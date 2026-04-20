use std::sync::{Arc, RwLock};
use std::{future::Future, str::FromStr};

use alloy::primitives::U256;
use alloy::signers::Signer;
use bon::Builder;
use dashmap::DashMap;
use reqwest::header::HeaderMap;
use reqwest::{Client as ReqwestClient, Method, Request};
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;
use serde::Deserialize;
use url::Url;

use crate::auth::state::{Authenticated, State, Unauthenticated};
use crate::auth::{Credentials, Kind, Normal};
use crate::clob::order_builder::{
    Limit, Market as MarketOrderKind, OrderBuilder, UserMarketOrder, UserOrder,
};
use crate::clob::types::{
    BalanceAllowanceRequest, BanStatus, BookParams, BuilderApiKey, BuilderApiKeyResponse,
    BuilderConfig, BuilderFeeRate, BuilderTrade, BuilderTradeParams, BuilderTradesResponse,
    DropNotificationsRequest, FeeInfo, FeeRateResponse, HeartbeatResponse, LastTradePriceRequest,
    LastTradePriceResponse, LastTradesPricesResponse, Market, MarketDetails, MarketPrice,
    MarketTradeEvent, MidpointRequest, MidpointResponse, MidpointsResponse, Notification,
    OpenOrder, OpenOrdersRequest, OrderBookRequest, OrderBookSummary, OrderResponse, OrderType,
    OrderMarketCancelRequest, Page,
    PriceHistoryFilterParams, PriceRequest, PriceResponse, PricesResponse, ReadonlyApiKeyResponse,
    RewardsPercentages, Side, SignatureTypeV2, SignableOrder, SignedOrder, SimplifiedMarket, SpreadRequest,
    SpreadResponse, SpreadsResponse, TickSize, TickSizeResponse, Trade, TradeParams,
    TradesPaginatedResponse, TotalUserEarning, UserEarning, UserRewardsEarning, sign_order as sign_v2_order,
};
use crate::config::exchange_contract;
use crate::error::{Error, Kind as ErrorKind};
use crate::types::{Address, Decimal};
use crate::{Result, Timestamp, auth};

fn read_lock<T: Copy>(lock: &RwLock<T>) -> T {
    *lock.read().unwrap_or_else(std::sync::PoisonError::into_inner)
}

pub struct AuthenticationBuilder<'signer, S: Signer, K: Kind = Normal> {
    client: Client<Unauthenticated>,
    signer: &'signer S,
    credentials: Option<Credentials>,
    nonce: Option<u32>,
    kind: K,
    funder: Option<Address>,
    signature_type: Option<SignatureTypeV2>,
}

impl<'signer, S: Signer, K: Kind> AuthenticationBuilder<'signer, S, K> {
    #[must_use]
    pub fn nonce(mut self, nonce: u32) -> Self {
        self.nonce = Some(nonce);
        self
    }

    #[must_use]
    pub fn credentials(mut self, credentials: Credentials) -> Self {
        self.credentials = Some(credentials);
        self
    }

    #[must_use]
    pub fn funder(mut self, funder: Address) -> Self {
        self.funder = Some(funder);
        self
    }

    #[must_use]
    pub fn signature_type(mut self, signature_type: SignatureTypeV2) -> Self {
        self.signature_type = Some(signature_type);
        self
    }

    #[must_use]
    pub fn kind<NextKind: Kind>(self, kind: NextKind) -> AuthenticationBuilder<'signer, S, NextKind> {
        AuthenticationBuilder {
            client: self.client,
            signer: self.signer,
            credentials: self.credentials,
            nonce: self.nonce,
            kind,
            funder: self.funder,
            signature_type: self.signature_type,
        }
    }

    pub async fn authenticate(self) -> Result<Client<Authenticated<K>>> {
        let inner = self.client.inner;

        let chain_id = self
            .signer
            .chain_id()
            .ok_or_else(|| Error::validation("Chain id not set on signer"))?;

        if chain_id != crate::POLYGON && chain_id != crate::AMOY {
            return Err(Error::validation(format!(
                "Only Polygon and Amoy are supported, got {chain_id}"
            )));
        }

        let credentials = match self.credentials {
            Some(_) if self.nonce.is_some() => {
                return Err(Error::validation(
                    "Credentials and nonce cannot be provided together",
                ))
            }
            Some(credentials) => credentials,
            None => inner.create_or_derive_api_key(self.signer, self.nonce).await?,
        };

        let state = Authenticated::new(self.signer.address(), credentials, self.kind);
        let cached_version = read_lock(&inner.cached_version);

        Ok(Client {
            inner: Arc::new(ClientInner {
                config: inner.config.clone(),
                state,
                host: inner.host.clone(),
                client: inner.client.clone(),
                tick_sizes: inner.tick_sizes.clone(),
                neg_risk: inner.neg_risk.clone(),
                fee_infos: inner.fee_infos.clone(),
                token_condition_map: inner.token_condition_map.clone(),
                builder_fee_rates: inner.builder_fee_rates.clone(),
                funder: self.funder,
                signature_type: self.signature_type.unwrap_or(SignatureTypeV2::Eoa),
                builder: inner.builder.clone(),
                cached_version: RwLock::new(cached_version),
            }),
        })
    }
}

#[derive(Clone, Debug)]
pub struct Client<S: State = Unauthenticated> {
    pub(crate) inner: Arc<ClientInner<S>>,
}

impl Default for Client<Unauthenticated> {
    fn default() -> Self {
        Self::new("https://clob.polymarket.com", Config::default())
            .expect("default client host is valid")
    }
}

#[derive(Clone, Debug, Builder, Default)]
pub struct Config {
    #[builder(default)]
    use_server_time: bool,
    #[builder(default)]
    retry_on_error: bool,
    #[builder(default)]
    throw_on_error: bool,
    builder: Option<BuilderConfig>,
}

#[derive(Debug)]
pub(crate) struct ClientInner<S: State> {
    config: Config,
    state: S,
    host: Url,
    client: ReqwestClient,
    tick_sizes: DashMap<U256, TickSize>,
    neg_risk: DashMap<U256, bool>,
    fee_infos: DashMap<U256, FeeInfo>,
    token_condition_map: DashMap<U256, String>,
    builder_fee_rates: DashMap<String, BuilderFeeRate>,
    funder: Option<Address>,
    signature_type: SignatureTypeV2,
    builder: Option<BuilderConfig>,
    cached_version: RwLock<Option<u32>>,
}

impl<S: State> ClientInner<S> {
    fn endpoint(&self, path: &str) -> Result<Url> {
        Ok(self.host.join(path.trim_start_matches('/'))?)
    }

    async fn request_json<Response, Query, Body>(
        &self,
        method: Method,
        path: &str,
        headers: Option<HeaderMap>,
        query: Option<&Query>,
        body: Option<&Body>,
    ) -> Result<Response>
    where
        Response: DeserializeOwned,
        Query: Serialize,
        Body: Serialize,
    {
        let mut request = self.client.request(method, self.endpoint(path)?);
        if let Some(query) = query {
            request = request.query(query);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        crate::request(
            &self.client,
            request.build()?,
            headers,
            self.config.retry_on_error,
        )
        .await
    }

    async fn server_time(&self) -> Result<Timestamp> {
        self.request_json(Method::GET, "/time", None, Option::<&()>::None, Option::<&()>::None)
            .await
    }

    async fn timestamp(&self) -> Result<Timestamp> {
        if self.config.use_server_time {
            self.server_time().await
        } else {
            Ok(chrono::Utc::now().timestamp())
        }
    }
}

impl ClientInner<Unauthenticated> {
    async fn create_headers<S: Signer>(&self, signer: &S, nonce: Option<u32>) -> Result<HeaderMap> {
        let chain_id = signer
            .chain_id()
            .ok_or_else(|| Error::validation("Chain id not set on signer"))?;
        auth::l1::create_headers(signer, chain_id, self.timestamp().await?, nonce).await
    }

    async fn create_api_key<S: Signer>(
        &self,
        signer: &S,
        nonce: Option<u32>,
    ) -> Result<Credentials> {
        let headers = self.create_headers(signer, nonce).await?;
        self.request_json(Method::POST, "/auth/api-key", Some(headers), Option::<&()>::None, Option::<&()>::None)
            .await
    }

    async fn derive_api_key<S: Signer>(
        &self,
        signer: &S,
        nonce: Option<u32>,
    ) -> Result<Credentials> {
        let headers = self.create_headers(signer, nonce).await?;
        self.request_json(
            Method::GET,
            "/auth/derive-api-key",
            Some(headers),
            Option::<&()>::None,
            Option::<&()>::None,
        )
        .await
    }

    async fn create_or_derive_api_key<S: Signer>(
        &self,
        signer: &S,
        nonce: Option<u32>,
    ) -> Result<Credentials> {
        match self.create_api_key(signer, nonce).await {
            Ok(credentials) => Ok(credentials),
            Err(error) if error.kind() == ErrorKind::Status => {
                self.derive_api_key(signer, nonce).await
            }
            Err(error) => Err(error),
        }
    }
}

impl<S: State> Client<S> {
    #[must_use]
    pub fn host(&self) -> &Url {
        &self.inner.host
    }

    pub fn invalidate_internal_caches(&self) {
        self.inner.tick_sizes.clear();
        self.inner.neg_risk.clear();
        self.inner.fee_infos.clear();
        self.inner.token_condition_map.clear();
        self.inner.builder_fee_rates.clear();
    }

    pub fn set_tick_size(&self, token_id: U256, tick_size: TickSize) {
        self.inner.tick_sizes.insert(token_id, tick_size);
    }

    pub fn set_neg_risk(&self, token_id: U256, neg_risk: bool) {
        self.inner.neg_risk.insert(token_id, neg_risk);
    }

    pub fn set_fee_info(&self, token_id: U256, fee_info: FeeInfo) {
        self.inner.fee_infos.insert(token_id, fee_info);
    }

    pub fn set_token_condition(&self, token_id: U256, condition_id: String) {
        self.inner.token_condition_map.insert(token_id, condition_id);
    }

    #[must_use]
    pub fn throw_on_error(&self) -> bool {
        self.inner.config.throw_on_error
    }

    pub async fn server_time(&self) -> Result<Timestamp> {
        self.inner.server_time().await
    }

    async fn get<Response, Query>(&self, path: &str, query: Option<&Query>) -> Result<Response>
    where
        Response: DeserializeOwned,
        Query: Serialize,
    {
        self.inner
            .request_json(Method::GET, path, None, query, Option::<&()>::None)
            .await
    }

    async fn post<Response, Body>(&self, path: &str, body: &Body) -> Result<Response>
    where
        Response: DeserializeOwned,
        Body: Serialize,
    {
        self.inner
            .request_json(Method::POST, path, None, Option::<&()>::None, Some(body))
            .await
    }

    async fn collect_pages<T, F, Fut>(&self, mut fetcher: F) -> Result<Vec<T>>
    where
        F: FnMut(Option<String>) -> Fut,
        Fut: Future<Output = Result<Page<T>>>,
    {
        let mut cursor = Some("MA==".to_owned());
        let mut items = Vec::new();

        while cursor.as_deref() != Some("LTE=") {
            let page = fetcher(cursor.clone()).await?;
            cursor = Some(page.next_cursor.clone());
            items.extend(page.data);
        }

        Ok(items)
    }

    pub async fn ok(&self) -> Result<String> {
        self.get("/ok", Option::<&()>::None).await
    }

    pub async fn version(&self) -> Result<u32> {
        if let Some(version) = read_lock(&self.inner.cached_version) {
            return Ok(version);
        }

        let version = self
            .get::<crate::clob::types::VersionResponse, ()>("/version", None)
            .await?
            .version
            .unwrap_or(2);

        *self
            .inner
            .cached_version
            .write()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(version);
        Ok(version)
    }

    pub async fn markets(&self, next_cursor: Option<String>) -> Result<Page<Market>> {
        #[derive(Serialize)]
        struct Query {
            next_cursor: Option<String>,
        }

        self.get("/markets", Some(&Query { next_cursor })).await
    }

    pub async fn market(&self, condition_id: &str) -> Result<Market> {
        self.get(&format!("/markets/{condition_id}"), Option::<&()>::None)
            .await
    }

    pub async fn simplified_markets(
        &self,
        next_cursor: Option<String>,
    ) -> Result<Page<SimplifiedMarket>> {
        #[derive(Serialize)]
        struct Query {
            next_cursor: Option<String>,
        }

        self.get("/simplified-markets", Some(&Query { next_cursor }))
            .await
    }

    pub async fn sampling_markets(&self, next_cursor: Option<String>) -> Result<Page<Market>> {
        #[derive(Serialize)]
        struct Query {
            next_cursor: Option<String>,
        }

        self.get("/sampling-markets", Some(&Query { next_cursor }))
            .await
    }

    pub async fn sampling_simplified_markets(
        &self,
        next_cursor: Option<String>,
    ) -> Result<Page<SimplifiedMarket>> {
        #[derive(Serialize)]
        struct Query {
            next_cursor: Option<String>,
        }

        self.get("/sampling-simplified-markets", Some(&Query { next_cursor }))
            .await
    }

    pub async fn clob_market_info(&self, condition_id: &str) -> Result<MarketDetails> {
        let details: MarketDetails = self
            .get(&format!("/clob-markets/{condition_id}"), Option::<&()>::None)
            .await?;

        for token in details.t.iter().flatten() {
            if let Ok(token_id) = U256::from_str(&token.token_id) {
                self.inner
                    .token_condition_map
                    .insert(token_id, details.condition_id.clone());
                self.inner.tick_sizes.insert(token_id, details.minimum_tick_size);
                self.inner.neg_risk.insert(token_id, details.neg_risk);

                let fee_info = details.fee_details.as_ref().map_or(
                    FeeInfo {
                        rate: 0,
                        exponent: 0,
                    },
                    |fee| FeeInfo {
                        rate: fee.rate.unwrap_or(0),
                        exponent: fee.exponent.unwrap_or(0),
                    },
                );
                self.inner.fee_infos.insert(token_id, fee_info);
            }
        }

        Ok(details)
    }

    pub async fn order_book(&self, token_id: U256) -> Result<OrderBookSummary> {
        self.get("/book", Some(&OrderBookRequest::builder().token_id(token_id).build()))
            .await
    }

    pub async fn order_books(&self, requests: &[BookParams]) -> Result<Vec<OrderBookSummary>> {
        self.post("/books", &requests).await
    }

    pub async fn tick_size(&self, token_id: U256) -> Result<TickSize> {
        if let Some(tick_size) = self.inner.tick_sizes.get(&token_id) {
            return Ok(*tick_size);
        }

        if let Some(condition_id) = self.inner.token_condition_map.get(&token_id) {
            self.clob_market_info(condition_id.value()).await?;
            if let Some(tick_size) = self.inner.tick_sizes.get(&token_id) {
                return Ok(*tick_size);
            }
        }

        let response: TickSizeResponse = self
            .get(
                "/tick-size",
                Some(&MidpointRequest::builder().token_id(token_id).build()),
            )
            .await?;
        self.inner
            .tick_sizes
            .insert(token_id, response.minimum_tick_size);
        Ok(response.minimum_tick_size)
    }

    pub async fn neg_risk(&self, token_id: U256) -> Result<bool> {
        if let Some(neg_risk) = self.inner.neg_risk.get(&token_id) {
            return Ok(*neg_risk);
        }

        if let Some(condition_id) = self.inner.token_condition_map.get(&token_id) {
            self.clob_market_info(condition_id.value()).await?;
            if let Some(neg_risk) = self.inner.neg_risk.get(&token_id) {
                return Ok(*neg_risk);
            }
        }

        #[derive(Serialize)]
        struct Query {
            token_id: String,
        }

        let response: crate::clob::types::NegRiskResponse = self
            .get(
                "/neg-risk",
                Some(&Query {
                    token_id: token_id.to_string(),
                }),
            )
            .await?;
        self.inner.neg_risk.insert(token_id, response.neg_risk);
        Ok(response.neg_risk)
    }

    pub async fn fee_rate_bps(&self, token_id: U256) -> Result<u32> {
        if let Some(fee_info) = self.inner.fee_infos.get(&token_id) {
            return Ok(fee_info.rate);
        }

        #[derive(Serialize)]
        struct Query {
            token_id: String,
        }

        let response: FeeRateResponse = self
            .get(
                "/fee-rate",
                Some(&Query {
                    token_id: token_id.to_string(),
                }),
            )
            .await?;
        let exponent = self.inner.fee_infos.get(&token_id).map_or(0, |info| info.exponent);
        self.inner.fee_infos.insert(
            token_id,
            FeeInfo {
                rate: response.base_fee,
                exponent,
            },
        );
        Ok(response.base_fee)
    }

    pub async fn fee_exponent(&self, token_id: U256) -> Result<u32> {
        if let Some(fee_info) = self.inner.fee_infos.get(&token_id) {
            return Ok(fee_info.exponent);
        }

        self.ensure_market_info_cached(token_id).await?;
        Ok(self
            .inner
            .fee_infos
            .get(&token_id)
            .map_or(0, |info| info.exponent))
    }

    pub fn order_book_hash(&self, order_book: &OrderBookSummary) -> Result<String> {
        order_book.hash()
    }

    pub async fn midpoint(&self, token_id: U256) -> Result<MidpointResponse> {
        self.get(
            "/midpoint",
            Some(&MidpointRequest::builder().token_id(token_id).build()),
        )
        .await
    }

    pub async fn midpoints(&self, requests: &[BookParams]) -> Result<MidpointsResponse> {
        self.post("/midpoints", &requests).await
    }

    pub async fn price(&self, token_id: U256, side: Side) -> Result<PriceResponse> {
        self.get(
            "/price",
            Some(&PriceRequest::builder().token_id(token_id).side(side).build()),
        )
        .await
    }

    pub async fn prices(&self, requests: &[BookParams]) -> Result<PricesResponse> {
        self.post("/prices", &requests).await
    }

    pub async fn spread(&self, token_id: U256) -> Result<SpreadResponse> {
        self.get(
            "/spread",
            Some(&SpreadRequest::builder().token_id(token_id).build()),
        )
        .await
    }

    pub async fn spreads(&self, requests: &[BookParams]) -> Result<SpreadsResponse> {
        self.post("/spreads", &requests).await
    }

    pub async fn last_trade_price(&self, token_id: U256) -> Result<LastTradePriceResponse> {
        self.get(
            "/last-trade-price",
            Some(&LastTradePriceRequest::builder().token_id(token_id).build()),
        )
        .await
    }

    pub async fn last_trades_prices(
        &self,
        requests: &[BookParams],
    ) -> Result<Vec<LastTradesPricesResponse>> {
        self.post("/last-trades-prices", &requests).await
    }

    pub async fn prices_history(
        &self,
        request: &PriceHistoryFilterParams,
    ) -> Result<Vec<MarketPrice>> {
        if request.interval.is_none() && (request.start_ts.is_none() || request.end_ts.is_none()) {
            return Err(Error::validation(
                "prices_history requires either interval or both start_ts and end_ts",
            ));
        }

        self.get("/prices-history", Some(request)).await
    }

    pub async fn calculate_market_price(
        &self,
        token_id: U256,
        side: Side,
        amount: Decimal,
        order_type: OrderType,
    ) -> Result<Decimal> {
        let book = self.order_book(token_id).await?;

        let positions = match side {
            Side::Buy => &book.asks,
            Side::Sell => &book.bids,
        };

        if positions.is_empty() {
            return Err(Error::validation("no match"));
        }

        let mut sum = Decimal::ZERO;
        for position in positions.iter().rev() {
            sum += if side == Side::Buy {
                position.size * position.price
            } else {
                position.size
            };

            if sum >= amount {
                return Ok(position.price);
            }
        }

        if matches!(order_type, OrderType::Fok) {
            return Err(Error::validation("no match"));
        }

        positions
            .first()
            .map(|position| position.price)
            .ok_or_else(|| Error::validation("no match"))
    }

    pub async fn market_trades_events(
        &self,
        condition_id: &str,
    ) -> Result<Vec<MarketTradeEvent>> {
        self.get(
            &format!("/markets/live-activity/{condition_id}"),
            Option::<&()>::None,
        )
        .await
    }

    async fn ensure_market_info_cached(&self, token_id: U256) -> Result<()> {
        if self.inner.fee_infos.contains_key(&token_id) {
            return Ok(());
        }

        if let Some(condition_id) = self.inner.token_condition_map.get(&token_id) {
            self.clob_market_info(condition_id.value()).await?;
            return Ok(());
        }

        #[derive(Deserialize)]
        struct MarketByTokenResponse {
            condition_id: String,
        }

        let response: MarketByTokenResponse = self
            .get(
                &format!("/markets-by-token/{token_id}"),
                Option::<&()>::None,
            )
            .await?;
        self.inner
            .token_condition_map
            .insert(token_id, response.condition_id.clone());
        self.clob_market_info(&response.condition_id).await?;
        Ok(())
    }
}

impl Client<Unauthenticated> {
    pub fn new(host: &str, config: Config) -> Result<Client<Unauthenticated>> {
        let normalized_host = if host.ends_with('/') {
            host.to_owned()
        } else {
            format!("{host}/")
        };

        let host = Url::parse(&normalized_host)?;
        let builder = config.builder.clone();
        Ok(Client {
            inner: Arc::new(ClientInner {
                config,
                state: Unauthenticated,
                host,
                client: ReqwestClient::new(),
                tick_sizes: DashMap::new(),
                neg_risk: DashMap::new(),
                fee_infos: DashMap::new(),
                token_condition_map: DashMap::new(),
                builder_fee_rates: DashMap::new(),
                funder: None,
                signature_type: SignatureTypeV2::Eoa,
                builder,
                cached_version: RwLock::new(None),
            }),
        })
    }

    #[must_use]
    pub fn authentication_builder<'a, S: Signer>(
        &self,
        signer: &'a S,
    ) -> AuthenticationBuilder<'a, S, Normal> {
        AuthenticationBuilder {
            client: self.clone(),
            signer,
            credentials: None,
            nonce: None,
            kind: Normal,
            funder: None,
            signature_type: None,
        }
    }

    pub async fn create_api_key<S: Signer>(
        &self,
        signer: &S,
        nonce: Option<u32>,
    ) -> Result<Credentials> {
        self.inner.create_api_key(signer, nonce).await
    }

    pub async fn derive_api_key<S: Signer>(
        &self,
        signer: &S,
        nonce: Option<u32>,
    ) -> Result<Credentials> {
        self.inner.derive_api_key(signer, nonce).await
    }

    pub async fn create_or_derive_api_key<S: Signer>(
        &self,
        signer: &S,
        nonce: Option<u32>,
    ) -> Result<Credentials> {
        self.inner.create_or_derive_api_key(signer, nonce).await
    }
}

impl<K: Kind> Client<Authenticated<K>> {
    #[must_use]
    pub fn state(&self) -> &Authenticated<K> {
        &self.inner.state
    }

    #[must_use]
    pub fn address(&self) -> Address {
        self.state().address()
    }

    #[must_use]
    pub fn credentials(&self) -> &Credentials {
        self.state().credentials()
    }

    #[must_use]
    pub fn funder(&self) -> Option<Address> {
        self.inner.funder
    }

    #[must_use]
    pub fn signature_type(&self) -> SignatureTypeV2 {
        self.inner.signature_type
    }

    #[must_use]
    pub fn builder_config(&self) -> Option<&BuilderConfig> {
        self.inner.builder.as_ref()
    }

    pub(crate) async fn create_headers(&self, request: &Request) -> Result<HeaderMap> {
        auth::l2::create_headers(self.state(), request, self.inner.timestamp().await?).await
    }

    async fn auth_request<Response, Query, Body>(
        &self,
        method: Method,
        path: &str,
        query: Option<&Query>,
        body: Option<&Body>,
    ) -> Result<Response>
    where
        Response: DeserializeOwned,
        Query: Serialize,
        Body: Serialize,
    {
        let mut request = self.inner.client.request(method, self.inner.endpoint(path)?);
        if let Some(query) = query {
            request = request.query(query);
        }
        if let Some(body) = body {
            request = request.json(body);
        }
        let request = request.build()?;
        let headers = self.create_headers(&request).await?;
        crate::request(
            &self.inner.client,
            request,
            Some(headers),
            self.inner.config.retry_on_error,
        )
        .await
    }

    async fn auth_get<Response, Query>(&self, path: &str, query: Option<&Query>) -> Result<Response>
    where
        Response: DeserializeOwned,
        Query: Serialize,
    {
        self.auth_request(Method::GET, path, query, Option::<&()>::None)
            .await
    }

    async fn auth_post<Response, Body>(&self, path: &str, body: &Body) -> Result<Response>
    where
        Response: DeserializeOwned,
        Body: Serialize,
    {
        self.auth_request(Method::POST, path, Option::<&()>::None, Some(body))
            .await
    }

    async fn auth_delete<Response, Query, Body>(
        &self,
        path: &str,
        query: Option<&Query>,
        body: Option<&Body>,
    ) -> Result<Response>
    where
        Response: DeserializeOwned,
        Query: Serialize,
        Body: Serialize,
    {
        self.auth_request(Method::DELETE, path, query, body).await
    }

    pub async fn heartbeat(&self, heartbeat_id: Option<String>) -> Result<HeartbeatResponse> {
        #[derive(Serialize)]
        struct Body {
            heartbeat_id: String,
        }

        self.auth_post(
            "/v1/heartbeats",
            &Body {
                heartbeat_id: heartbeat_id.unwrap_or_default(),
            },
        )
        .await
    }

    pub async fn api_keys(&self) -> Result<crate::clob::types::ApiKeysResponse> {
        self.auth_get("/auth/api-keys", Option::<&()>::None).await
    }

    pub async fn closed_only_mode(&self) -> Result<BanStatus> {
        self.auth_get("/auth/ban-status/closed-only", Option::<&()>::None)
            .await
    }

    pub async fn delete_api_key(&self) -> Result<Value> {
        self.auth_delete("/auth/api-key", Option::<&()>::None, Option::<&()>::None)
            .await
    }

    pub async fn create_readonly_api_key(&self) -> Result<ReadonlyApiKeyResponse> {
        self.auth_post("/auth/readonly-api-key", &()).await
    }

    pub async fn readonly_api_keys(&self) -> Result<Vec<String>> {
        self.auth_get("/auth/readonly-api-keys", Option::<&()>::None)
            .await
    }

    pub async fn delete_readonly_api_key(&self, key: String) -> Result<bool> {
        #[derive(Serialize)]
        struct Body {
            key: String,
        }

        self.auth_delete(
            "/auth/readonly-api-key",
            Option::<&()>::None,
            Some(&Body { key }),
        )
        .await
    }

    pub async fn order(&self, order_id: &str) -> Result<OpenOrder> {
        self.auth_get(&format!("/data/order/{order_id}"), Option::<&()>::None)
            .await
    }

    pub async fn orders_page(
        &self,
        request: &OpenOrdersRequest,
        next_cursor: Option<String>,
    ) -> Result<Page<OpenOrder>> {
        #[derive(Serialize)]
        struct Query<'a> {
            #[serde(flatten)]
            request: &'a OpenOrdersRequest,
            next_cursor: Option<String>,
        }

        self.auth_get(
            "/data/orders",
            Some(&Query {
                request,
                next_cursor,
            }),
        )
        .await
    }

    pub async fn open_orders(&self, request: &OpenOrdersRequest) -> Result<Vec<OpenOrder>> {
        self.collect_pages(|cursor| self.orders_page(request, cursor)).await
    }

    pub async fn orders(
        &self,
        request: &OpenOrdersRequest,
        next_cursor: Option<String>,
    ) -> Result<Page<OpenOrder>> {
        self.orders_page(request, next_cursor).await
    }

    pub async fn pre_migration_orders(&self) -> Result<Vec<OpenOrder>> {
        #[derive(Serialize)]
        struct Query {
            next_cursor: Option<String>,
        }

        self.collect_pages(|cursor| async move {
            self.auth_get("/data/pre-migration-orders", Some(&Query { next_cursor: cursor }))
                .await
        })
        .await
    }

    pub async fn trades(&self, request: &TradeParams) -> Result<Vec<Trade>> {
        self.collect_pages(|cursor| async {
            let page = self.trades_page(request, cursor).await?;
            Ok(Page {
                limit: page.limit,
                count: page.count,
                next_cursor: page.next_cursor,
                data: page.trades,
            })
        })
        .await
    }

    pub async fn trades_page(
        &self,
        request: &TradeParams,
        next_cursor: Option<String>,
    ) -> Result<TradesPaginatedResponse> {
        #[derive(Serialize)]
        struct Query<'a> {
            #[serde(flatten)]
            request: &'a TradeParams,
            next_cursor: Option<String>,
        }

        let page: Page<Trade> = self
            .auth_get(
                "/data/trades",
                Some(&Query {
                    request,
                    next_cursor,
                }),
            )
            .await?;

        Ok(TradesPaginatedResponse {
            trades: page.data,
            next_cursor: page.next_cursor,
            limit: page.limit,
            count: page.count,
        })
    }

    pub async fn trades_paginated(
        &self,
        request: &TradeParams,
        next_cursor: Option<String>,
    ) -> Result<TradesPaginatedResponse> {
        self.trades_page(request, next_cursor).await
    }

    pub async fn notifications(&self) -> Result<Vec<Notification>> {
        #[derive(Serialize)]
        struct Query {
            signature_type: SignatureTypeV2,
        }

        self.auth_get(
            "/notifications",
            Some(&Query {
                signature_type: self.signature_type(),
            }),
        )
        .await
    }

    pub async fn drop_notifications(
        &self,
        request: Option<&DropNotificationsRequest>,
    ) -> Result<()> {
        #[derive(Serialize)]
        struct Query {
            ids: Option<String>,
        }

        let ids = request.and_then(|request| {
            if request.ids.is_empty() {
                None
            } else {
                Some(request.ids.join(","))
            }
        });

        self.auth_delete("/notifications", Some(&Query { ids }), Option::<&()>::None)
            .await
    }

    pub async fn balance_allowance(
        &self,
        request: &BalanceAllowanceRequest,
    ) -> Result<crate::clob::types::BalanceAllowanceResponse> {
        let mut query = request.clone();
        if query.signature_type.is_none() {
            query.signature_type = Some(self.signature_type());
        }
        self.auth_get("/balance-allowance", Some(&query)).await
    }

    pub async fn update_balance_allowance(&self, request: &BalanceAllowanceRequest) -> Result<()> {
        let mut query = request.clone();
        if query.signature_type.is_none() {
            query.signature_type = Some(self.signature_type());
        }
        self.auth_get("/balance-allowance/update", Some(&query)).await
    }

    #[must_use]
    pub fn limit_order(&self) -> OrderBuilder<Limit, K> {
        OrderBuilder::new(self.clone())
    }

    #[must_use]
    pub fn market_order(&self) -> OrderBuilder<MarketOrderKind, K> {
        OrderBuilder::new(self.clone())
    }

    pub async fn sign<S: Signer>(
        &self,
        signer: &S,
        signable_order: SignableOrder,
    ) -> Result<SignedOrder> {
        let chain_id = signer
            .chain_id()
            .ok_or_else(|| Error::validation("Chain id not set on signer"))?;
        let neg_risk = self.neg_risk(signable_order.order.tokenId).await?;
        let verifying_contract = exchange_contract(chain_id, neg_risk)
            .ok_or_else(|| Error::missing_contract_config(chain_id, neg_risk))?;
        let signature = sign_v2_order(signer, &signable_order.order, chain_id, verifying_contract)
            .await?;

        Ok(SignedOrder {
            order: signable_order.order,
            signature,
            owner: self.credentials().key(),
            order_type: signable_order.order_type,
            post_only: signable_order.post_only,
            defer_exec: signable_order.defer_exec,
        })
    }

    pub async fn create_order<S: Signer>(
        &self,
        signer: &S,
        user_order: UserOrder,
    ) -> Result<SignedOrder> {
        let mut builder = self
            .limit_order()
            .token_id(user_order.token_id)
            .price(user_order.price)
            .size(user_order.size)
            .side(user_order.side);

        if let Some(metadata) = user_order.metadata {
            builder = builder.metadata(metadata);
        }
        if let Some(builder_code) = user_order.builder_code {
            builder = builder.builder_code(builder_code);
        }
        if let Some(expiration) = user_order.expiration {
            builder = builder.expiration(expiration);
        }

        self.sign(signer, builder.build().await?).await
    }

    pub async fn create_market_order<S: Signer>(
        &self,
        signer: &S,
        user_order: UserMarketOrder,
    ) -> Result<SignedOrder> {
        let mut builder = self
            .market_order()
            .token_id(user_order.token_id)
            .amount(user_order.amount)
            .side(user_order.side);

        if let Some(price) = user_order.price {
            builder = builder.price(price);
        }
        if let Some(order_type) = user_order.order_type {
            builder = builder.order_type(order_type);
        }
        if let Some(metadata) = user_order.metadata {
            builder = builder.metadata(metadata);
        }
        if let Some(builder_code) = user_order.builder_code {
            builder = builder.builder_code(builder_code);
        }
        if let Some(user_usdc_balance) = user_order.user_usdc_balance {
            builder = builder.user_usdc_balance(user_usdc_balance);
        }

        self.sign(signer, builder.build().await?).await
    }

    pub async fn create_and_post_order<S: Signer>(
        &self,
        signer: &S,
        user_order: UserOrder,
        order_type: OrderType,
        post_only: bool,
        defer_exec: bool,
    ) -> Result<OrderResponse> {
        let mut builder = self
            .limit_order()
            .token_id(user_order.token_id)
            .price(user_order.price)
            .size(user_order.size)
            .side(user_order.side)
            .order_type(order_type)
            .post_only(post_only)
            .defer_exec(defer_exec);

        if let Some(metadata) = user_order.metadata {
            builder = builder.metadata(metadata);
        }
        if let Some(builder_code) = user_order.builder_code {
            builder = builder.builder_code(builder_code);
        }
        if let Some(expiration) = user_order.expiration {
            builder = builder.expiration(expiration);
        }

        let order = self.sign(signer, builder.build().await?).await?;
        self.post_order(&order).await
    }

    pub async fn create_and_post_market_order<S: Signer>(
        &self,
        signer: &S,
        user_order: UserMarketOrder,
        order_type: OrderType,
        defer_exec: bool,
    ) -> Result<OrderResponse> {
        let mut builder = self
            .market_order()
            .token_id(user_order.token_id)
            .amount(user_order.amount)
            .side(user_order.side)
            .order_type(order_type)
            .defer_exec(defer_exec);

        if let Some(price) = user_order.price {
            builder = builder.price(price);
        }
        if let Some(metadata) = user_order.metadata {
            builder = builder.metadata(metadata);
        }
        if let Some(builder_code) = user_order.builder_code {
            builder = builder.builder_code(builder_code);
        }
        if let Some(user_usdc_balance) = user_order.user_usdc_balance {
            builder = builder.user_usdc_balance(user_usdc_balance);
        }

        let order = self.sign(signer, builder.build().await?).await?;
        self.post_order(&order).await
    }

    pub async fn post_order(&self, order: &SignedOrder) -> Result<OrderResponse> {
        self.auth_post("/order", &build_post_order_payload(order)?).await
    }

    pub async fn post_orders(&self, orders: &[SignedOrder]) -> Result<Vec<OrderResponse>> {
        let payloads = orders
            .iter()
            .map(build_post_order_payload)
            .collect::<Result<Vec<_>>>()?;
        self.auth_post("/orders", &payloads).await
    }

    pub async fn cancel_order(&self, order_id: String) -> Result<Value> {
        #[derive(Serialize)]
        struct Body {
            #[serde(rename = "orderID")]
            order_id: String,
        }

        self.auth_delete("/order", Option::<&()>::None, Some(&Body { order_id }))
            .await
    }

    pub async fn cancel_orders(&self, order_ids: &[String]) -> Result<Value> {
        let payload = order_ids.to_vec();
        self.auth_delete("/orders", Option::<&()>::None, Some(&payload))
            .await
    }

    pub async fn cancel_all_orders(&self) -> Result<Value> {
        self.auth_delete("/cancel-all", Option::<&()>::None, Option::<&()>::None)
            .await
    }

    pub async fn cancel_all(&self) -> Result<Value> {
        self.cancel_all_orders().await
    }

    pub async fn cancel_market_orders(
        &self,
        request: &OrderMarketCancelRequest,
    ) -> Result<Value> {
        self.auth_delete("/cancel-market-orders", Option::<&()>::None, Some(request))
            .await
    }

    pub async fn create_builder_api_key(&self) -> Result<BuilderApiKey> {
        self.auth_post("/auth/builder-api-key", &()).await
    }

    pub async fn builder_api_keys(&self) -> Result<Vec<BuilderApiKeyResponse>> {
        self.auth_get("/auth/builder-api-key", Option::<&()>::None)
            .await
    }

    pub async fn revoke_builder_api_key(&self) -> Result<Value> {
        self.auth_delete("/auth/builder-api-key", Option::<&()>::None, Option::<&()>::None)
            .await
    }

    pub async fn builder_trades(
        &self,
        request: &BuilderTradeParams,
        next_cursor: Option<String>,
    ) -> Result<BuilderTradesResponse> {
        let builder_code = request
            .builder_code
            .as_ref()
            .ok_or_else(|| Error::validation("builder_code is required"))?;

        if builder_code.eq_ignore_ascii_case(
            "0x0000000000000000000000000000000000000000000000000000000000000000",
        ) {
            return Err(Error::validation("builder_code cannot be zero"));
        }

        #[derive(Serialize)]
        struct Query<'a> {
            #[serde(flatten)]
            request: &'a BuilderTradeParams,
            next_cursor: Option<String>,
        }

        let page: Page<BuilderTrade> = self
            .auth_get(
                "/builder/trades",
                Some(&Query {
                    request,
                    next_cursor,
                }),
            )
            .await?;

        Ok(BuilderTradesResponse {
            trades: page.data,
            next_cursor: page.next_cursor,
            limit: page.limit,
            count: page.count,
        })
    }

    pub async fn order_scoring(&self, order_id: String) -> Result<crate::clob::types::OrderScoringResponse> {
        #[derive(Serialize)]
        struct Query {
            order_id: String,
        }

        self.auth_get("/order-scoring", Some(&Query { order_id })).await
    }

    pub async fn orders_scoring(&self, order_ids: &[String]) -> Result<crate::clob::types::OrdersScoringResponse> {
        self.auth_post("/orders-scoring", &order_ids.to_vec()).await
    }

    pub async fn earnings_for_user_for_day(&self, date: String) -> Result<Vec<UserEarning>> {
        #[derive(Serialize)]
        struct Query {
            date: String,
            signature_type: SignatureTypeV2,
            next_cursor: Option<String>,
        }

        self.collect_pages(|cursor| async {
            self.auth_get(
                "/rewards/user",
                Some(&Query {
                    date: date.clone(),
                    signature_type: self.signature_type(),
                    next_cursor: cursor,
                }),
            )
            .await
        })
        .await
    }

    pub async fn total_earnings_for_user_for_day(
        &self,
        date: String,
    ) -> Result<Vec<TotalUserEarning>> {
        #[derive(Serialize)]
        struct Query {
            date: String,
            signature_type: SignatureTypeV2,
        }

        self.auth_get(
            "/rewards/user/total",
            Some(&Query {
                date,
                signature_type: self.signature_type(),
            }),
        )
        .await
    }

    pub async fn user_earnings_and_markets_config(
        &self,
        date: String,
        order_by: String,
        position: String,
        no_competition: bool,
    ) -> Result<Vec<UserRewardsEarning>> {
        #[derive(Serialize)]
        struct Query {
            date: String,
            signature_type: SignatureTypeV2,
            next_cursor: Option<String>,
            order_by: String,
            position: String,
            no_competition: bool,
        }

        self.collect_pages(|cursor| async {
            self.auth_get(
                "/rewards/user/markets",
                Some(&Query {
                    date: date.clone(),
                    signature_type: self.signature_type(),
                    next_cursor: cursor,
                    order_by: order_by.clone(),
                    position: position.clone(),
                    no_competition,
                }),
            )
            .await
        })
        .await
    }

    pub async fn reward_percentages(&self) -> Result<RewardsPercentages> {
        #[derive(Serialize)]
        struct Query {
            signature_type: SignatureTypeV2,
        }

        self.auth_get(
            "/rewards/user/percentages",
            Some(&Query {
                signature_type: self.signature_type(),
            }),
        )
        .await
    }

    pub async fn current_rewards(&self) -> Result<Vec<crate::clob::types::MarketReward>> {
        #[derive(Serialize)]
        struct Query {
            next_cursor: Option<String>,
        }

        self.collect_pages(|cursor| async {
            self.get("/rewards/markets/current", Some(&Query { next_cursor: cursor }))
                .await
        })
        .await
    }

    pub async fn raw_rewards_for_market(
        &self,
        condition_id: &str,
    ) -> Result<Vec<crate::clob::types::MarketReward>> {
        #[derive(Serialize)]
        struct Query {
            next_cursor: Option<String>,
        }

        self.collect_pages(|cursor| async {
            self.get(
                &format!("/rewards/markets/{condition_id}"),
                Some(&Query { next_cursor: cursor }),
            )
            .await
        })
        .await
    }
}

#[derive(Serialize)]
struct PostOrderPayload {
    order: PostOrderEnvelope,
    owner: String,
    #[serde(rename = "orderType")]
    order_type: OrderType,
    #[serde(rename = "deferExec")]
    defer_exec: bool,
    #[serde(rename = "postOnly")]
    post_only: bool,
}

#[derive(Serialize)]
struct PostOrderEnvelope {
    salt: u64,
    maker: String,
    signer: String,
    #[serde(rename = "tokenId")]
    token_id: String,
    #[serde(rename = "makerAmount")]
    maker_amount: String,
    #[serde(rename = "takerAmount")]
    taker_amount: String,
    side: Side,
    #[serde(rename = "signatureType")]
    signature_type: SignatureTypeV2,
    timestamp: String,
    metadata: String,
    builder: String,
    expiration: String,
    signature: String,
}

fn build_post_order_payload(order: &SignedOrder) -> Result<PostOrderPayload> {
    Ok(PostOrderPayload {
        order: PostOrderEnvelope {
            salt: u64::try_from(order.order.salt)
                .map_err(|_| Error::validation("salt does not fit in u64"))?,
            maker: order.order.maker.to_string(),
            signer: order.order.signer.to_string(),
            token_id: order.order.tokenId.to_string(),
            maker_amount: order.order.makerAmount.to_string(),
            taker_amount: order.order.takerAmount.to_string(),
            side: Side::try_from(order.order.side)?,
            signature_type: match order.order.signatureType {
                0 => SignatureTypeV2::Eoa,
                1 => SignatureTypeV2::Proxy,
                2 => SignatureTypeV2::GnosisSafe,
                3 => SignatureTypeV2::Poly1271,
                other => {
                    return Err(Error::validation(format!(
                        "invalid signature type on order: {other}"
                    )))
                }
            },
            timestamp: order.order.timestamp.to_string(),
            metadata: order.order.metadata.to_string(),
            builder: order.order.builder.to_string(),
            expiration: order.order.expiration.to_string(),
            signature: order.signature.to_string(),
        },
        owner: order.owner.to_string(),
        order_type: order.order_type,
        defer_exec: order.defer_exec,
        post_only: order.post_only,
    })
}

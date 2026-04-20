use std::marker::PhantomData;
use std::str::FromStr as _;
use std::sync::Arc;

use alloy::primitives::{B256, U256};
use bon::Builder;
use futures::future::BoxFuture;
use rand::random;
use rust_decimal::RoundingStrategy::{AwayFromZero, MidpointAwayFromZero, ToZero};

use crate::Result;
use crate::auth::Kind as AuthKind;
use crate::auth::Signer;
use crate::auth::state::Authenticated;
use crate::clob::Client;
use crate::clob::types::{OrderType, Side, SignableOrder, SignatureTypeV2, new_order};
use crate::error::Error;
use crate::types::Address;
use crate::types::Decimal;

pub const USDC_DECIMALS: u32 = 6;
pub const LOT_SIZE_SCALE: u32 = 2;

pub type DynSigner = Box<dyn Signer + Send + Sync>;
pub type SignerFactory = Arc<dyn Fn() -> BoxFuture<'static, Result<DynSigner>> + Send + Sync>;

#[non_exhaustive]
#[derive(Debug)]
pub struct Limit;

#[non_exhaustive]
#[derive(Debug)]
pub struct Market;

#[non_exhaustive]
#[derive(Debug, Clone, Builder)]
pub struct UserOrder {
    pub token_id: U256,
    pub price: Decimal,
    pub size: Decimal,
    pub side: Side,
    pub metadata: Option<B256>,
    pub builder_code: Option<B256>,
    pub expiration: Option<u64>,
}

#[non_exhaustive]
#[derive(Debug, Clone, Builder)]
pub struct UserMarketOrder {
    pub token_id: U256,
    pub price: Option<Decimal>,
    pub amount: Decimal,
    pub side: Side,
    pub order_type: Option<OrderType>,
    pub user_usdc_balance: Option<Decimal>,
    pub metadata: Option<B256>,
    pub builder_code: Option<B256>,
}

pub struct OrderBuilder<OrderKind, K: AuthKind> {
    client: Client<Authenticated<K>>,
    signer: Address,
    maker: Address,
    signature_type: SignatureTypeV2,
    signer_factory: Option<SignerFactory>,
    token_id: Option<U256>,
    price: Option<Decimal>,
    size: Option<Decimal>,
    amount: Option<Decimal>,
    side: Option<Side>,
    metadata: Option<B256>,
    builder_code: Option<B256>,
    expiration: Option<u64>,
    order_type: Option<OrderType>,
    post_only: Option<bool>,
    defer_exec: Option<bool>,
    user_usdc_balance: Option<Decimal>,
    _kind: PhantomData<OrderKind>,
}

impl<OrderKind, K: AuthKind> std::fmt::Debug for OrderBuilder<OrderKind, K> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OrderBuilder")
            .field("signer", &self.signer)
            .field("maker", &self.maker)
            .field("signature_type", &self.signature_type)
            .field("has_signer_factory", &self.signer_factory.is_some())
            .field("token_id", &self.token_id)
            .field("price", &self.price)
            .field("size", &self.size)
            .field("amount", &self.amount)
            .field("side", &self.side)
            .field("metadata", &self.metadata)
            .field("builder_code", &self.builder_code)
            .field("expiration", &self.expiration)
            .field("order_type", &self.order_type)
            .field("post_only", &self.post_only)
            .field("defer_exec", &self.defer_exec)
            .field("user_usdc_balance", &self.user_usdc_balance)
            .finish_non_exhaustive()
    }
}

pub(crate) enum ResolvedSigner<'signer> {
    Borrowed(&'signer dyn Signer),
    Owned(DynSigner),
}

impl ResolvedSigner<'_> {
    pub(crate) fn as_ref(&self) -> &dyn Signer {
        match self {
            Self::Borrowed(signer) => *signer,
            Self::Owned(signer) => signer.as_ref(),
        }
    }

    pub(crate) fn address(&self) -> Address {
        self.as_ref().address()
    }
}

impl<OrderKind, K: AuthKind> OrderBuilder<OrderKind, K> {
    pub(crate) fn new(client: Client<Authenticated<K>>) -> Self {
        let signer = client.address();
        let maker = client.funder().unwrap_or(signer);
        let signature_type = client.signature_type();
        let builder_code = client
            .builder_config()
            .and_then(|config| B256::from_str(&config.builder_code).ok());
        let signer_factory = client.signer_factory();

        Self {
            client,
            signer,
            maker,
            signature_type,
            signer_factory,
            token_id: None,
            price: None,
            size: None,
            amount: None,
            side: None,
            metadata: None,
            builder_code,
            expiration: None,
            order_type: None,
            post_only: None,
            defer_exec: None,
            user_usdc_balance: None,
            _kind: PhantomData,
        }
    }

    #[must_use]
    pub fn token_id(mut self, token_id: U256) -> Self {
        self.token_id = Some(token_id);
        self
    }

    #[must_use]
    pub fn side(mut self, side: Side) -> Self {
        self.side = Some(side);
        self
    }

    #[must_use]
    pub fn metadata(mut self, metadata: B256) -> Self {
        self.metadata = Some(metadata);
        self
    }

    #[must_use]
    pub fn builder_code(mut self, builder_code: B256) -> Self {
        self.builder_code = Some(builder_code);
        self
    }

    #[must_use]
    pub fn order_type(mut self, order_type: OrderType) -> Self {
        self.order_type = Some(order_type);
        self
    }

    #[must_use]
    pub fn post_only(mut self, post_only: bool) -> Self {
        self.post_only = Some(post_only);
        self
    }

    #[must_use]
    pub fn defer_exec(mut self, defer_exec: bool) -> Self {
        self.defer_exec = Some(defer_exec);
        self
    }

    #[must_use]
    pub fn get_signer<F, Fut>(mut self, factory: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = Result<DynSigner>> + Send + 'static,
    {
        self.signer_factory = Some(Arc::new(move || Box::pin(factory())));
        self
    }

    pub(crate) async fn resolve_signer<'signer, S>(
        &self,
        signer: &'signer S,
    ) -> Result<ResolvedSigner<'signer>>
    where
        S: Signer + 'signer,
    {
        if let Some(factory) = &self.signer_factory {
            return Ok(ResolvedSigner::Owned(factory().await?));
        }

        Ok(ResolvedSigner::Borrowed(signer))
    }

    async fn resolve_signer_address(&self) -> Result<Address> {
        if let Some(factory) = &self.signer_factory {
            return Ok(factory().await?.address());
        }

        Ok(self.signer)
    }
}

impl<K: AuthKind> OrderBuilder<Limit, K> {
    #[must_use]
    pub fn price(mut self, price: Decimal) -> Self {
        self.price = Some(price);
        self
    }

    #[must_use]
    pub fn size(mut self, size: Decimal) -> Self {
        self.size = Some(size);
        self
    }

    #[must_use]
    pub fn expiration(mut self, expiration: u64) -> Self {
        self.expiration = Some(expiration);
        self
    }

    pub async fn build(self) -> Result<SignableOrder> {
        let signer = self.resolve_signer_address().await?;
        self.build_with_signer(signer).await
    }

    pub(crate) async fn build_with_signer(self, signer: Address) -> Result<SignableOrder> {
        let token_id = self
            .token_id
            .ok_or_else(|| Error::validation("token_id is required"))?;
        let side = self
            .side
            .ok_or_else(|| Error::validation("side is required"))?;
        let price = self
            .price
            .ok_or_else(|| Error::validation("price is required"))?;
        let order_size = self
            .size
            .ok_or_else(|| Error::validation("size is required"))?;

        validate_price(price)?;
        validate_size(order_size)?;

        let tick_size = self.client.tick_size(token_id).await?;
        let minimum_tick_size = tick_size.as_decimal();
        if price.normalize().scale() > minimum_tick_size.normalize().scale() {
            return Err(Error::validation(format!(
                "price has too many decimal places for tick size {tick_size}"
            )));
        }
        if price < minimum_tick_size || price > Decimal::ONE - minimum_tick_size {
            return Err(Error::validation(format!(
                "price {price} must be between {minimum_tick_size} and {}",
                Decimal::ONE - minimum_tick_size
            )));
        }

        let order_type = self.order_type.unwrap_or(OrderType::Gtc);
        let post_only = self.post_only.unwrap_or(false);
        let defer_exec = self.defer_exec.unwrap_or(false);
        let expiration = self.expiration.unwrap_or(0);

        if expiration > 0 && !matches!(order_type, OrderType::Gtd) {
            return Err(Error::validation(
                "Only GTD orders may specify a non-zero expiration",
            ));
        }

        if post_only && !matches!(order_type, OrderType::Gtc | OrderType::Gtd) {
            return Err(Error::validation(
                "post_only is only supported for GTC and GTD orders",
            ));
        }

        let round_config = tick_size.round_config();
        let (maker_amount, taker_amount) =
            get_limit_raw_amounts(side, order_size, price, round_config);

        Ok(SignableOrder {
            order: new_order(
                generate_salt(),
                self.maker,
                signer,
                token_id,
                to_scaled_u256(maker_amount, USDC_DECIMALS)?,
                to_scaled_u256(taker_amount, USDC_DECIMALS)?,
                side,
                self.signature_type,
                current_timestamp_millis()?,
                self.metadata.unwrap_or(B256::ZERO),
                self.builder_code.unwrap_or(B256::ZERO),
            ),
            expiration,
            order_type,
            post_only,
            defer_exec,
        })
    }
}

impl<K: AuthKind> OrderBuilder<Market, K> {
    #[must_use]
    pub fn price(mut self, price: Decimal) -> Self {
        self.price = Some(price);
        self
    }

    #[must_use]
    /// Sets the market-order amount.
    ///
    /// For BUY orders this value is denominated in USDC. For SELL orders it is
    /// denominated in shares, matching the TypeScript V2 client semantics.
    pub fn amount(mut self, amount: Decimal) -> Self {
        self.amount = Some(amount);
        self
    }

    #[must_use]
    pub fn user_usdc_balance(mut self, user_usdc_balance: Decimal) -> Self {
        self.user_usdc_balance = Some(user_usdc_balance);
        self
    }

    pub async fn build(self) -> Result<SignableOrder> {
        let signer = self.resolve_signer_address().await?;
        self.build_with_signer(signer).await
    }

    pub(crate) async fn build_with_signer(self, signer: Address) -> Result<SignableOrder> {
        let token_id = self
            .token_id
            .ok_or_else(|| Error::validation("token_id is required"))?;
        let side = self
            .side
            .ok_or_else(|| Error::validation("side is required"))?;
        let amount = self
            .amount
            .ok_or_else(|| Error::validation("amount is required"))?;

        if amount <= Decimal::ZERO {
            return Err(Error::validation("amount must be positive"));
        }

        let order_type = self.order_type.unwrap_or(OrderType::Fok);
        let post_only = self.post_only.unwrap_or(false);
        let defer_exec = self.defer_exec.unwrap_or(false);
        if post_only {
            return Err(Error::validation(
                "post_only is not supported for market orders",
            ));
        }

        let tick_size = self.client.tick_size(token_id).await?;
        let price = match self.price {
            Some(price) => price,
            None => {
                self.client
                    .calculate_market_price(token_id, side, amount, order_type)
                    .await?
            }
        };

        validate_price(price)?;
        let round_config = tick_size.round_config();
        let amount = if side == Side::Buy {
            match self.user_usdc_balance {
                Some(user_usdc_balance) => {
                    let builder_taker_fee_rate = match self.builder_code {
                        Some(builder_code) if builder_code != B256::ZERO => {
                            let fee_response =
                                self.client.builder_fees(&builder_code.to_string()).await?;
                            Decimal::from(fee_response.builder_taker_fee_rate_bps)
                                / Decimal::from(10_000_u32)
                        }
                        _ => Decimal::ZERO,
                    };

                    let fee_info = self.client.platform_fee_info(token_id).await?;
                    adjust_buy_amount_for_fees(
                        amount,
                        price,
                        user_usdc_balance,
                        fee_info.rate,
                        fee_info.exponent,
                        builder_taker_fee_rate,
                    )
                }
                None => amount,
            }
        } else {
            amount
        };
        let (maker_amount, taker_amount) =
            get_market_raw_amounts(side, amount, price, round_config)?;

        Ok(SignableOrder {
            order: new_order(
                generate_salt(),
                self.maker,
                signer,
                token_id,
                to_scaled_u256(maker_amount, USDC_DECIMALS)?,
                to_scaled_u256(taker_amount, USDC_DECIMALS)?,
                side,
                self.signature_type,
                current_timestamp_millis()?,
                self.metadata.unwrap_or(B256::ZERO),
                self.builder_code.unwrap_or(B256::ZERO),
            ),
            expiration: 0,
            order_type,
            post_only,
            defer_exec,
        })
    }
}

fn validate_price(price: Decimal) -> Result<()> {
    if price <= Decimal::ZERO {
        return Err(Error::validation("price must be positive"));
    }
    Ok(())
}

fn validate_size(size: Decimal) -> Result<()> {
    if size <= Decimal::ZERO {
        return Err(Error::validation("size must be positive"));
    }
    if size.normalize().scale() > LOT_SIZE_SCALE {
        return Err(Error::validation(format!(
            "size may have at most {LOT_SIZE_SCALE} decimal places"
        )));
    }
    Ok(())
}

fn generate_salt() -> U256 {
    // Polymarket's backend parses salts as IEEE 754 doubles, so values must stay
    // within JavaScript's max safe integer range to preserve the signed payload.
    U256::from(random::<u64>() & ((1_u64 << 53) - 1))
}

fn current_timestamp_millis() -> Result<u64> {
    u64::try_from(chrono::Utc::now().timestamp_millis())
        .map_err(|_conversion_error| Error::validation("timestamp is negative"))
}

fn round_normal(value: Decimal, decimals: u32) -> Decimal {
    if value.normalize().scale() <= decimals {
        value
    } else {
        value.round_dp_with_strategy(decimals, MidpointAwayFromZero)
    }
}

fn round_down(value: Decimal, decimals: u32) -> Decimal {
    if value.normalize().scale() <= decimals {
        value
    } else {
        value.round_dp_with_strategy(decimals, ToZero)
    }
}

fn round_up(value: Decimal, decimals: u32) -> Decimal {
    if value.normalize().scale() <= decimals {
        value
    } else {
        value.round_dp_with_strategy(decimals, AwayFromZero)
    }
}

fn decimal_places(value: Decimal) -> u32 {
    value.normalize().scale()
}

fn get_limit_raw_amounts(
    side: Side,
    order_size: Decimal,
    price: Decimal,
    round_config: crate::clob::types::RoundConfig,
) -> (Decimal, Decimal) {
    let raw_price = round_normal(price, round_config.price);

    match side {
        Side::Buy => {
            let share_amount = round_down(order_size, round_config.size);
            let mut quote_amount = share_amount * raw_price;
            if decimal_places(quote_amount) > round_config.amount {
                quote_amount = round_up(quote_amount, round_config.amount + 4);
                if decimal_places(quote_amount) > round_config.amount {
                    quote_amount = round_down(quote_amount, round_config.amount);
                }
            }
            (quote_amount, share_amount)
        }
        Side::Sell => {
            let share_amount = round_down(order_size, round_config.size);
            let mut quote_amount = share_amount * raw_price;
            if decimal_places(quote_amount) > round_config.amount {
                quote_amount = round_up(quote_amount, round_config.amount + 4);
                if decimal_places(quote_amount) > round_config.amount {
                    quote_amount = round_down(quote_amount, round_config.amount);
                }
            }
            (share_amount, quote_amount)
        }
    }
}

fn get_market_raw_amounts(
    side: Side,
    amount: Decimal,
    price: Decimal,
    round_config: crate::clob::types::RoundConfig,
) -> Result<(Decimal, Decimal)> {
    let raw_price = round_down(price, round_config.price);
    if raw_price.is_zero() {
        return Err(Error::validation(
            "price rounds to zero; cannot calculate market order amounts",
        ));
    }

    match side {
        Side::Buy => {
            let quote_amount = round_down(amount, round_config.size);
            let mut share_amount = quote_amount / raw_price;
            if decimal_places(share_amount) > round_config.amount {
                share_amount = round_up(share_amount, round_config.amount + 4);
                if decimal_places(share_amount) > round_config.amount {
                    share_amount = round_down(share_amount, round_config.amount);
                }
            }
            Ok((quote_amount, share_amount))
        }
        Side::Sell => {
            let share_amount = round_down(amount, round_config.size);
            let mut quote_amount = share_amount * raw_price;
            if decimal_places(quote_amount) > round_config.amount {
                quote_amount = round_up(quote_amount, round_config.amount + 4);
                if decimal_places(quote_amount) > round_config.amount {
                    quote_amount = round_down(quote_amount, round_config.amount);
                }
            }
            Ok((share_amount, quote_amount))
        }
    }
}

fn adjust_buy_amount_for_fees(
    amount: Decimal,
    price: Decimal,
    user_usdc_balance: Decimal,
    fee_rate: Decimal,
    fee_exponent: u32,
    builder_taker_fee_rate: Decimal,
) -> Decimal {
    let price_term = price * (Decimal::ONE - price);
    let mut price_term_power = Decimal::ONE;
    for _ in 0..fee_exponent {
        price_term_power *= price_term;
    }

    let platform_fee_rate = fee_rate * price_term_power;
    let platform_fee = (amount / price) * platform_fee_rate;
    let total_cost = amount + platform_fee + amount * builder_taker_fee_rate;

    if user_usdc_balance <= total_cost {
        user_usdc_balance / (Decimal::ONE + (platform_fee_rate / price) + builder_taker_fee_rate)
    } else {
        amount
    }
}

fn to_scaled_u256(value: Decimal, scale: u32) -> Result<U256> {
    let factor = Decimal::from(10_u64.pow(scale));
    let scaled = round_down(value * factor, 0).normalize();
    U256::from_str(&scaled.to_string()).map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_salts_stay_within_javascript_safe_integer_range() {
        let max_safe_integer = U256::from((1_u64 << 53) - 1);

        for _ in 0..1000 {
            assert!(generate_salt() <= max_safe_integer);
        }
    }

    #[test]
    fn buy_amount_is_adjusted_for_builder_fees_when_balance_is_tight() {
        let adjusted = adjust_buy_amount_for_fees(
            Decimal::from(100_u32),
            Decimal::new(5, 1),
            Decimal::from(100_u32),
            Decimal::ZERO,
            0,
            Decimal::new(2, 2),
        );

        assert_eq!(round_down(adjusted, 2), Decimal::new(9803, 2));
    }
}

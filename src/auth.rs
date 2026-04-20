//! Authentication helpers for Polymarket's Level 1, Level 2, and builder flows.

pub use alloy::signers::Signer;
pub use alloy::signers::local::{LocalSigner, PrivateKeySigner};
use async_trait::async_trait;
use base64::Engine as _;
use base64::engine::general_purpose::{STANDARD, URL_SAFE};
use hmac::{Hmac, Mac as _};
use reqwest::header::HeaderMap;
use reqwest::{Body, Request};
pub use secrecy::{ExposeSecret, SecretString};
use serde::Deserialize;
use sha2::Sha256;
pub use uuid::Uuid;

use crate::{Error, Result, Timestamp};

pub type ApiKey = Uuid;

/// API credentials used for Level 2 authenticated requests.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Credentials {
    #[serde(alias = "apiKey", rename = "apiKey")]
    pub(crate) key: ApiKey,
    pub(crate) secret: SecretString,
    pub(crate) passphrase: SecretString,
}

impl Credentials {
    #[must_use]
    pub fn new(key: Uuid, secret: String, passphrase: String) -> Self {
        Self {
            key,
            secret: SecretString::from(secret),
            passphrase: SecretString::from(passphrase),
        }
    }

    #[must_use]
    pub fn key(&self) -> ApiKey {
        self.key
    }

    #[must_use]
    pub fn secret(&self) -> &SecretString {
        &self.secret
    }

    #[must_use]
    pub fn passphrase(&self) -> &SecretString {
        &self.passphrase
    }
}

pub mod state {
    use crate::auth::{Credentials, Kind};
    use crate::types::Address;

    #[non_exhaustive]
    #[derive(Clone, Debug)]
    pub struct Unauthenticated;

    #[non_exhaustive]
    #[derive(Clone, Debug)]
    pub struct Authenticated<K: Kind> {
        pub(crate) address: Address,
        pub(crate) credentials: Credentials,
        pub(crate) kind: K,
    }

    impl<K: Kind> Authenticated<K> {
        #[must_use]
        pub fn new(address: Address, credentials: Credentials, kind: K) -> Self {
            Self {
                address,
                credentials,
                kind,
            }
        }

        #[must_use]
        pub fn address(&self) -> Address {
            self.address
        }

        #[must_use]
        pub fn credentials(&self) -> &Credentials {
            &self.credentials
        }
    }

    pub trait State: sealed::Sealed {}

    impl State for Unauthenticated {}
    impl sealed::Sealed for Unauthenticated {}

    impl<K: Kind> State for Authenticated<K> {}
    impl<K: Kind> sealed::Sealed for Authenticated<K> {}

    mod sealed {
        pub trait Sealed {}
    }
}

#[async_trait]
pub trait Kind: sealed::Sealed + Clone + Send + Sync + 'static {
    async fn extra_headers(&self, request: &Request, timestamp: Timestamp) -> Result<HeaderMap>;
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct Normal;

impl Normal {
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Default for Normal {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Kind for Normal {
    async fn extra_headers(&self, _request: &Request, _timestamp: Timestamp) -> Result<HeaderMap> {
        Ok(HeaderMap::new())
    }
}

impl sealed::Sealed for Normal {}

#[async_trait]
impl Kind for builder::Builder {
    async fn extra_headers(&self, request: &Request, timestamp: Timestamp) -> Result<HeaderMap> {
        self.create_headers(request, timestamp).await
    }
}

impl sealed::Sealed for builder::Builder {}

mod sealed {
    pub trait Sealed {}
}

pub mod l1 {
    use std::borrow::Cow;

    use alloy::core::sol;
    use alloy::dyn_abi::Eip712Domain;
    use alloy::hex::ToHexExt as _;
    use alloy::primitives::{ChainId, U256};
    use alloy::signers::Signer;
    use alloy::sol_types::SolStruct as _;
    use reqwest::header::HeaderMap;

    use crate::{Result, Timestamp};

    pub const POLY_ADDRESS: &str = "POLY_ADDRESS";
    pub const POLY_NONCE: &str = "POLY_NONCE";
    pub const POLY_SIGNATURE: &str = "POLY_SIGNATURE";
    pub const POLY_TIMESTAMP: &str = "POLY_TIMESTAMP";

    sol! {
        #[derive(Debug)]
        #[allow(
            clippy::exhaustive_structs,
            reason = "The typed-data struct must match Polymarket's fixed EIP-712 schema"
        )]
        struct ClobAuth {
            address address;
            string timestamp;
            uint256 nonce;
            string message;
        }
    }

    pub async fn create_headers<S: Signer>(
        signer: &S,
        chain_id: ChainId,
        timestamp: Timestamp,
        nonce: Option<u32>,
    ) -> Result<HeaderMap> {
        let nonce = nonce.unwrap_or(0);

        let auth = ClobAuth {
            address: signer.address(),
            timestamp: timestamp.to_string(),
            nonce: U256::from(nonce),
            message: "This message attests that I control the given wallet".to_owned(),
        };

        let domain = Eip712Domain {
            name: Some(Cow::Borrowed("ClobAuthDomain")),
            version: Some(Cow::Borrowed("1")),
            chain_id: Some(U256::from(chain_id)),
            ..Eip712Domain::default()
        };

        let hash = auth.eip712_signing_hash(&domain);
        let signature = signer.sign_hash(&hash).await?;

        let mut map = HeaderMap::new();
        map.insert(
            POLY_ADDRESS,
            signer.address().encode_hex_with_prefix().parse()?,
        );
        map.insert(POLY_NONCE, nonce.to_string().parse()?);
        map.insert(POLY_SIGNATURE, signature.to_string().parse()?);
        map.insert(POLY_TIMESTAMP, timestamp.to_string().parse()?);

        Ok(map)
    }
}

pub mod l2 {
    use alloy::hex::ToHexExt as _;
    use reqwest::Request;
    use reqwest::header::HeaderMap;
    use secrecy::ExposeSecret as _;

    use crate::auth::state::Authenticated;
    use crate::auth::{Kind, hmac, to_message};
    use crate::{Result, Timestamp};

    pub const POLY_ADDRESS: &str = "POLY_ADDRESS";
    pub const POLY_API_KEY: &str = "POLY_API_KEY";
    pub const POLY_PASSPHRASE: &str = "POLY_PASSPHRASE";
    pub const POLY_SIGNATURE: &str = "POLY_SIGNATURE";
    pub const POLY_TIMESTAMP: &str = "POLY_TIMESTAMP";

    pub async fn create_headers<K: Kind>(
        state: &Authenticated<K>,
        request: &Request,
        timestamp: Timestamp,
    ) -> Result<HeaderMap> {
        let signature = hmac(&state.credentials.secret, &to_message(request, timestamp)?)?;

        let mut map = HeaderMap::new();
        map.insert(
            POLY_ADDRESS,
            state.address.encode_hex_with_prefix().parse()?,
        );
        map.insert(POLY_API_KEY, state.credentials.key.to_string().parse()?);
        map.insert(
            POLY_PASSPHRASE,
            state.credentials.passphrase.expose_secret().parse()?,
        );
        map.insert(POLY_SIGNATURE, signature.parse()?);
        map.insert(POLY_TIMESTAMP, timestamp.to_string().parse()?);
        map.extend(state.kind.extra_headers(request, timestamp).await?);

        Ok(map)
    }
}

pub mod builder {
    use std::fmt;

    use reqwest::header::HeaderMap;
    use reqwest::{Client, Request};
    use secrecy::{ExposeSecret as _, SecretString};
    use serde::{Deserialize, Serialize};
    use serde_json::json;
    pub use url::Url;

    use crate::auth::{Credentials, body_to_string, hmac, to_message};
    use crate::{Result, Timestamp};

    pub const POLY_BUILDER_API_KEY: &str = "POLY_BUILDER_API_KEY";
    pub const POLY_BUILDER_PASSPHRASE: &str = "POLY_BUILDER_PASSPHRASE";
    pub const POLY_BUILDER_SIGNATURE: &str = "POLY_BUILDER_SIGNATURE";
    pub const POLY_BUILDER_TIMESTAMP: &str = "POLY_BUILDER_TIMESTAMP";

    #[derive(Clone, Debug, Deserialize, Serialize)]
    struct HeaderPayload {
        #[serde(rename = "POLY_BUILDER_API_KEY")]
        api_key: String,
        #[serde(rename = "POLY_BUILDER_TIMESTAMP")]
        timestamp: String,
        #[serde(rename = "POLY_BUILDER_PASSPHRASE")]
        passphrase: String,
        #[serde(rename = "POLY_BUILDER_SIGNATURE")]
        signature: String,
    }

    #[non_exhaustive]
    #[derive(Clone)]
    pub enum Config {
        Local(Credentials),
        Remote {
            host: Url,
            token: Option<SecretString>,
            allow_insecure: bool,
        },
    }

    fn validate_remote_host(host: &Url, allow_insecure: bool) -> Result<()> {
        match (host.scheme(), allow_insecure) {
            ("https", _) | ("http", true) => Ok(()),
            ("http", false) => Err(crate::Error::validation(
                "only HTTPS URLs are accepted for remote builder signing; use remote_insecure for local dev",
            )),
            _ => Err(crate::Error::validation(
                "remote builder signing URLs must use http:// or https://",
            )),
        }
    }

    impl Config {
        #[must_use]
        pub fn local(credentials: Credentials) -> Self {
            Self::Local(credentials)
        }

        pub fn remote(host: &str, token: Option<String>) -> Result<Self> {
            let host = Url::parse(host)?;
            validate_remote_host(&host, false)?;

            Ok(Self::Remote {
                host,
                token: token.map(SecretString::from),
                allow_insecure: false,
            })
        }

        pub fn remote_insecure(host: &str, token: Option<String>) -> Result<Self> {
            let host = Url::parse(host)?;
            validate_remote_host(&host, true)?;

            Ok(Self::Remote {
                host,
                token: token.map(SecretString::from),
                allow_insecure: true,
            })
        }
    }

    impl fmt::Debug for Config {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                Self::Local(_) => f.debug_tuple("Local").field(&"[REDACTED]").finish(),
                Self::Remote {
                    host,
                    token,
                    allow_insecure,
                } => f
                    .debug_struct("Remote")
                    .field("host", host)
                    .field("token", &token.as_ref().map(|_| "[REDACTED]"))
                    .field("allow_insecure", allow_insecure)
                    .finish(),
            }
        }
    }

    #[non_exhaustive]
    #[derive(Clone, Debug)]
    pub struct Builder {
        pub(crate) config: Config,
        pub(crate) client: Client,
    }

    impl Builder {
        #[must_use]
        pub fn new(config: Config) -> Self {
            Self {
                config,
                client: Client::new(),
            }
        }

        pub(crate) async fn create_headers(
            &self,
            request: &Request,
            timestamp: Timestamp,
        ) -> Result<HeaderMap> {
            match &self.config {
                Config::Local(credentials) => {
                    let signature = hmac(&credentials.secret, &to_message(request, timestamp)?)?;

                    let mut map = HeaderMap::new();
                    map.insert(POLY_BUILDER_API_KEY, credentials.key.to_string().parse()?);
                    map.insert(
                        POLY_BUILDER_PASSPHRASE,
                        credentials.passphrase.expose_secret().parse()?,
                    );
                    map.insert(POLY_BUILDER_SIGNATURE, signature.parse()?);
                    map.insert(POLY_BUILDER_TIMESTAMP, timestamp.to_string().parse()?);
                    Ok(map)
                }
                Config::Remote {
                    host,
                    token,
                    allow_insecure,
                } => {
                    validate_remote_host(host, *allow_insecure)?;

                    let payload = json!({
                        "method": request.method().as_str(),
                        "path": request.url().path(),
                        "body": request
                            .body()
                            .map(body_to_string)
                            .transpose()?
                            .unwrap_or_default(),
                        "timestamp": timestamp,
                    });

                    let mut headers = HeaderMap::new();
                    if let Some(token) = token {
                        headers.insert(
                            "Authorization",
                            format!("Bearer {}", token.expose_secret()).parse()?,
                        );
                    }

                    let response = self
                        .client
                        .post(host.clone())
                        .headers(headers)
                        .json(&payload)
                        .send()
                        .await?;

                    let remote_headers: HeaderPayload = response.error_for_status()?.json().await?;

                    let mut map = HeaderMap::new();
                    map.insert(POLY_BUILDER_SIGNATURE, remote_headers.signature.parse()?);
                    map.insert(POLY_BUILDER_TIMESTAMP, remote_headers.timestamp.parse()?);
                    map.insert(POLY_BUILDER_API_KEY, remote_headers.api_key.parse()?);
                    map.insert(POLY_BUILDER_PASSPHRASE, remote_headers.passphrase.parse()?);
                    Ok(map)
                }
            }
        }
    }
}

fn to_message(request: &Request, timestamp: Timestamp) -> Result<String> {
    let body = request
        .body()
        .map(body_to_string)
        .transpose()?
        .unwrap_or_default();
    Ok(format!(
        "{timestamp}{}{path}{body}",
        request.method(),
        path = request.url().path()
    ))
}

fn body_to_string(body: &Body) -> Result<String> {
    let bytes = body
        .as_bytes()
        .ok_or_else(|| Error::validation("request body is not available for HMAC signing"))?;
    let body = std::str::from_utf8(bytes).map_err(|_utf8_error| {
        Error::validation("request body is not valid UTF-8 for HMAC signing")
    })?;
    Ok(body.to_owned())
}

fn hmac(secret: &SecretString, message: &str) -> Result<String> {
    let decoded_secret = STANDARD.decode(secret.expose_secret())?;
    let mut mac = Hmac::<Sha256>::new_from_slice(&decoded_secret)?;
    mac.update(message.as_bytes());
    let result = mac.finalize().into_bytes();
    Ok(URL_SAFE.encode(result))
}

#[cfg(test)]
mod tests {
    use super::{SecretString, hmac};

    #[test]
    fn hmac_supports_standard_base64_secrets_with_plus_and_slash() {
        let signature = hmac(
            &SecretString::from("++//".to_owned()),
            "1700000000GET/auth/api-keys",
        )
        .expect("standard base64 secret should decode");

        assert_eq!(signature, "HbYWv2kCKePei7BtC17tq5d1fDwa21OBD_KBfuKXvjI=");
    }
}

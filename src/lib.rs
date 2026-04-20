#![allow(
    clippy::items_after_statements,
    clippy::module_name_repetitions,
    clippy::multiple_crate_versions,
    clippy::struct_excessive_bools,
    clippy::too_many_arguments
)]
#![cfg_attr(doc, doc = include_str!("../README.md"))]

pub mod auth;
#[cfg(feature = "clob")]
pub mod clob;
pub mod config;
pub mod error;
pub(crate) mod serde_helpers;
pub mod types;
#[cfg(feature = "ws")]
pub mod ws;

use std::fmt::Write as _;

use alloy::primitives::ChainId;
use reqwest::{Request, header::HeaderMap};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use tokio::time::{Duration, sleep};

pub use error::{Error, Kind};

pub type Result<T> = std::result::Result<T, Error>;

pub const POLYGON: ChainId = 137;
pub const AMOY: ChainId = 80002;
pub const PRIVATE_KEY_VAR: &str = "POLYMARKET_PRIVATE_KEY";

pub(crate) type Timestamp = i64;

pub trait ToQueryParams: Serialize {
    fn query_params(&self, next_cursor: Option<&str>) -> String {
        let mut params = serde_html_form::to_string(self).unwrap_or_default();

        if let Some(cursor) = next_cursor {
            if !params.is_empty() {
                params.push('&');
            }
            let _ = write!(params, "next_cursor={cursor}");
        }

        if params.is_empty() {
            String::new()
        } else {
            format!("?{params}")
        }
    }
}

impl<T: Serialize> ToQueryParams for T {}

pub(crate) async fn request<Response: DeserializeOwned>(
    client: &reqwest::Client,
    mut request: Request,
    headers: Option<HeaderMap>,
    retry_on_error: bool,
) -> Result<Response> {
    let mut retry_request = if retry_on_error {
        request.try_clone()
    } else {
        None
    };

    let method = request.method().clone();
    let path = request.url().path().to_owned();

    for attempt in 0..=1 {
        if let Some(header_map) = headers.clone() {
            *request.headers_mut() = header_map;
        }

        match client.execute(request).await {
            Ok(response) => {
                let status = response.status();
                if !status.is_success() {
                    if attempt == 0 && retry_request.is_some() && status.is_server_error() {
                        sleep(Duration::from_millis(30)).await;
                        request = retry_request
                            .take()
                            .expect("retry request exists when branch is taken");
                        continue;
                    }

                    let message = response.text().await.unwrap_or_default();
                    return Err(Error::status(status, method, path, message));
                }

                let text = response.text().await?;
                let value = if text.trim().is_empty() {
                    Value::Null
                } else {
                    serde_json::from_str(&text).unwrap_or(Value::String(text))
                };

                return crate::serde_helpers::deserialize_with_warnings(value);
            }
            Err(error) => {
                let should_retry =
                    attempt == 0 && retry_request.is_some() && (error.is_connect() || error.is_timeout());
                if should_retry {
                    sleep(Duration::from_millis(30)).await;
                    request = retry_request
                        .take()
                        .expect("retry request exists when branch is taken");
                    continue;
                }

                return Err(error.into());
            }
        }
    }

    unreachable!("request loop either returns or retries once")
}

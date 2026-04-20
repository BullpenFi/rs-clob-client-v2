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
use serde::Serialize;
use serde::de::DeserializeOwned;
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

fn status_message(body: &Value) -> String {
    match body.get("error") {
        Some(Value::String(message)) => message.clone(),
        Some(error) => error.to_string(),
        None => match body {
            Value::String(message) => message.clone(),
            other => other.to_string(),
        },
    }
}

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
        if let Some(header_map) = headers.as_ref() {
            for (name, value) in header_map {
                request.headers_mut().insert(name, value.clone());
            }
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

                    let raw_body = response.text().await.unwrap_or_default();
                    let body = serde_json::from_str::<Value>(&raw_body).ok();
                    let message = body
                        .as_ref()
                        .map_or_else(|| raw_body.clone(), status_message);
                    return Err(Error::status_with_payload(
                        status,
                        method.clone(),
                        path.clone(),
                        message,
                        body,
                        (!raw_body.is_empty()).then_some(raw_body),
                    ));
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
                let should_retry = attempt == 0
                    && retry_request.is_some()
                    && (error.is_connect() || error.is_timeout());
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

#[cfg(test)]
mod tests {
    use httpmock::Method::{GET, POST};
    use httpmock::MockServer;
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
    use serde::Deserialize;
    use serde_json::json;
    use tokio::time::{Duration, sleep};

    use super::request as send_request;

    #[derive(Debug, Deserialize)]
    struct OkResponse {
        ok: bool,
    }

    fn auth_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            HeaderName::from_static("poly_api_key"),
            HeaderValue::from_static("api-key"),
        );
        headers
    }

    #[tokio::test]
    async fn request_merges_existing_request_headers_with_supplied_headers() {
        let server = MockServer::start();
        let mock = server.mock(|when, then| {
            when.method(GET)
                .path("/merge")
                .header("x-test-existing", "keep-me")
                .header("poly_api_key", "api-key");
            then.status(200).json_body_obj(&json!({ "ok": true }));
        });

        let client = reqwest::Client::new();
        let built_request = client
            .get(server.url("/merge"))
            .header("x-test-existing", "keep-me")
            .build()
            .expect("request");

        let response: OkResponse = send_request(&client, built_request, Some(auth_headers()), false)
            .await
            .expect("response");

        mock.assert();
        assert!(response.ok);
    }

    #[tokio::test]
    async fn request_retry_preserves_existing_request_headers() {
        let server = MockServer::start_async().await;
        let first = server
            .mock_async(|when, then| {
                when.method(POST)
                    .path("/retry-merge")
                    .header("x-test-existing", "keep-me")
                    .header("poly_api_key", "api-key");
                then.status(500).body("boom");
            })
            .await;

        let client = reqwest::Client::new();
        let built_request = client
            .post(server.url("/retry-merge"))
            .header("x-test-existing", "keep-me")
            .json(&json!({ "ok": true }))
            .build()
            .expect("request");

        let switch_mock = async {
            loop {
                if first.calls_async().await == 1 {
                    first.delete_async().await;
                    return server
                        .mock_async(|when, then| {
                            when.method(POST)
                                .path("/retry-merge")
                                .header("x-test-existing", "keep-me")
                                .header("poly_api_key", "api-key");
                            then.status(200).json_body_obj(&json!({ "ok": true }));
                        })
                        .await;
                }

                sleep(Duration::from_millis(5)).await;
            }
        };

        let (second, response) = tokio::join!(
            switch_mock,
            send_request(&client, built_request, Some(auth_headers()), true)
        );
        let response: OkResponse = response.expect("response");

        second.assert_calls(1);
        assert!(response.ok);
    }
}

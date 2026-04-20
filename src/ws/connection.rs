use futures::{SinkExt as _, StreamExt as _};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio_tungstenite::WebSocketStream;
use tokio_tungstenite::connect_async;
use tokio_tungstenite::tungstenite::Message;

use crate::Result;

pub struct Connection {
    stream: WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
}

fn validate_config(config: &crate::ws::Config) -> Result<()> {
    match (config.url.scheme(), config.allow_insecure) {
        ("wss", _) | ("ws", true) => Ok(()),
        ("ws", false) => Err(crate::Error::validation(
            "only WSS URLs are accepted; set allow_insecure for local dev",
        )),
        _ => Err(crate::Error::validation(
            "websocket URLs must use ws:// or wss://",
        )),
    }
}

impl Connection {
    pub async fn connect(config: &crate::ws::Config) -> Result<Self> {
        validate_config(config)?;
        let (stream, _) = connect_async(config.url.as_str()).await?;
        Ok(Self { stream })
    }

    pub async fn send_json<T: Serialize>(&mut self, payload: &T) -> Result<()> {
        let text = serde_json::to_string(payload)?;
        self.stream.send(Message::Text(text.into())).await?;
        Ok(())
    }

    pub async fn next_json<T: DeserializeOwned>(&mut self) -> Result<Option<T>> {
        while let Some(message) = self.stream.next().await {
            match message? {
                Message::Text(text) => return Ok(Some(serde_json::from_str(&text)?)),
                Message::Binary(bytes) => return Ok(Some(serde_json::from_slice(&bytes)?)),
                Message::Ping(_) | Message::Pong(_) | Message::Frame(_) => {}
                Message::Close(_) => return Ok(None),
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use url::Url;

    use super::validate_config;

    #[test]
    fn validate_accepts_secure_wss_urls() {
        let config = crate::ws::Config::builder()
            .url(Url::parse("wss://example.com/ws").expect("wss url"))
            .build();

        validate_config(&config).expect("wss should validate");
    }

    #[test]
    fn validate_rejects_ws_without_opt_in() {
        let config = crate::ws::Config::builder()
            .url(Url::parse("ws://example.com/ws").expect("ws url"))
            .build();

        let error = validate_config(&config).expect_err("ws should be rejected by default");
        assert!(
            error
                .to_string()
                .contains("only WSS URLs are accepted; set allow_insecure for local dev")
        );
    }

    #[test]
    fn validate_accepts_ws_when_opted_in() {
        let config = crate::ws::Config::builder()
            .url(Url::parse("ws://example.com/ws").expect("ws url"))
            .allow_insecure(true)
            .build();

        validate_config(&config).expect("ws should validate when allow_insecure is set");
    }
}

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

impl Connection {
    pub async fn connect(config: &crate::ws::Config) -> Result<Self> {
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

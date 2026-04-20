#![allow(
    clippy::module_name_repetitions,
    reason = "The websocket configuration type intentionally uses the conventional Config name"
)]

use std::str::FromStr;

use bon::Builder;
use url::Url;

use crate::Result;

#[non_exhaustive]
#[derive(Clone, Debug, Builder)]
pub struct Config {
    pub url: Url,
    #[builder(default)]
    pub allow_insecure: bool,
}

impl Config {
    pub fn parse(url: &str) -> Result<Self> {
        let url = Url::parse(url)?;
        if url.scheme() != "wss" {
            return Err(crate::Error::validation(
                "only WSS URLs are accepted; set allow_insecure for local dev",
            ));
        }

        Ok(Self {
            url,
            allow_insecure: false,
        })
    }
}

impl FromStr for Config {
    type Err = crate::Error;

    fn from_str(url: &str) -> Result<Self> {
        Self::parse(url)
    }
}

#[cfg(test)]
mod tests {
    use super::Config;

    #[test]
    fn parse_accepts_wss_urls() {
        let config = Config::parse("wss://example.com/ws").expect("wss config");
        assert_eq!(config.url.as_str(), "wss://example.com/ws");
        assert!(!config.allow_insecure);
    }

    #[test]
    fn parse_rejects_non_wss_urls() {
        let error = Config::parse("ws://example.com/ws").expect_err("ws should fail");
        assert!(error
            .to_string()
            .contains("only WSS URLs are accepted; set allow_insecure for local dev"));
    }
}

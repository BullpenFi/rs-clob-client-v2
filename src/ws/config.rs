use std::str::FromStr;

use bon::Builder;
use url::Url;

use crate::Result;

#[derive(Clone, Debug, Builder)]
pub struct Config {
    pub url: Url,
}

impl Config {
    pub fn parse(url: &str) -> Result<Self> {
        Ok(Self {
            url: Url::parse(url)?,
        })
    }
}

impl FromStr for Config {
    type Err = crate::Error;

    fn from_str(url: &str) -> Result<Self> {
        Self::parse(url)
    }
}

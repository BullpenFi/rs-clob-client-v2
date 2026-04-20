use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};

use crate::error::Error;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u64)]
pub enum Chain {
    Polygon = 137,
    Amoy = 80002,
}

impl From<Chain> for u64 {
    fn from(value: Chain) -> Self {
        match value {
            Chain::Polygon => 137,
            Chain::Amoy => 80002,
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum Side {
    Buy = 0,
    Sell = 1,
}

impl Side {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Buy => "BUY",
            Self::Sell => "SELL",
        }
    }
}

impl fmt::Display for Side {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<u8> for Side {
    type Error = Error;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Self::Buy),
            1 => Ok(Self::Sell),
            other => Err(Error::validation(format!("invalid side: {other}"))),
        }
    }
}

impl Serialize for Side {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Side {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        match raw.to_ascii_uppercase().as_str() {
            "BUY" => Ok(Self::Buy),
            "SELL" => Ok(Self::Sell),
            _ => Err(serde::de::Error::custom(format!("invalid side: {raw}"))),
        }
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OrderType {
    #[serde(rename = "GTC", alias = "gtc")]
    Gtc,
    #[serde(rename = "FOK", alias = "fok")]
    Fok,
    #[serde(rename = "GTD", alias = "gtd")]
    Gtd,
    #[serde(rename = "FAK", alias = "fak")]
    Fak,
}

impl fmt::Display for OrderType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Gtc => "GTC",
            Self::Fok => "FOK",
            Self::Gtd => "GTD",
            Self::Fak => "FAK",
        };
        f.write_str(value)
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SignatureTypeV2 {
    Eoa = 0,
    Proxy = 1,
    GnosisSafe = 2,
    Poly1271 = 3,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AssetType {
    #[serde(rename = "COLLATERAL")]
    Collateral,
    #[serde(rename = "CONDITIONAL")]
    Conditional,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PriceHistoryInterval {
    #[serde(rename = "max")]
    Max,
    #[serde(rename = "1w")]
    OneWeek,
    #[serde(rename = "1d")]
    OneDay,
    #[serde(rename = "6h")]
    SixHours,
    #[serde(rename = "1h")]
    OneHour,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoundConfig {
    pub price: u32,
    pub size: u32,
    pub amount: u32,
}

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickSize {
    Tenth,
    Hundredth,
    Thousandth,
    TenThousandth,
}

impl TickSize {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Tenth => "0.1",
            Self::Hundredth => "0.01",
            Self::Thousandth => "0.001",
            Self::TenThousandth => "0.0001",
        }
    }

    #[must_use]
    pub fn as_decimal(self) -> Decimal {
        match self {
            Self::Tenth => Decimal::new(1, 1),
            Self::Hundredth => Decimal::new(1, 2),
            Self::Thousandth => Decimal::new(1, 3),
            Self::TenThousandth => Decimal::new(1, 4),
        }
    }

    #[must_use]
    pub const fn round_config(self) -> RoundConfig {
        match self {
            Self::Tenth => RoundConfig {
                price: 1,
                size: 2,
                amount: 3,
            },
            Self::Hundredth => RoundConfig {
                price: 2,
                size: 2,
                amount: 4,
            },
            Self::Thousandth => RoundConfig {
                price: 3,
                size: 2,
                amount: 5,
            },
            Self::TenThousandth => RoundConfig {
                price: 4,
                size: 2,
                amount: 6,
            },
        }
    }
}

impl fmt::Display for TickSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for TickSize {
    type Err = Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "0.1" => Ok(Self::Tenth),
            "0.01" => Ok(Self::Hundredth),
            "0.001" => Ok(Self::Thousandth),
            "0.0001" => Ok(Self::TenThousandth),
            other => Err(Error::validation(format!("invalid tick size: {other}"))),
        }
    }
}

impl Serialize for TickSize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for TickSize {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let raw = String::deserialize(deserializer)?;
        Self::from_str(&raw).map_err(serde::de::Error::custom)
    }
}

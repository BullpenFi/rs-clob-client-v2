use std::backtrace::Backtrace;
use std::error::Error as StdError;
use std::fmt;

use alloy::primitives::ChainId;
use alloy::primitives::ruint::ParseError;
use hmac::digest::InvalidLength;
pub use reqwest::Method;
pub use reqwest::StatusCode;
use reqwest::header;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Status,
    Validation,
    Synchronization,
    Internal,
    WebSocket,
    Geoblock,
}

#[derive(Debug)]
pub struct Error {
    kind: Kind,
    source: Option<Box<dyn StdError + Send + Sync + 'static>>,
    backtrace: Backtrace,
}

impl Error {
    pub fn with_source<S>(kind: Kind, source: S) -> Self
    where
        S: StdError + Send + Sync + 'static,
    {
        Self {
            kind,
            source: Some(Box::new(source)),
            backtrace: Backtrace::capture(),
        }
    }

    #[must_use]
    pub fn kind(&self) -> Kind {
        self.kind
    }

    pub fn backtrace(&self) -> &Backtrace {
        &self.backtrace
    }

    #[must_use]
    pub fn inner(&self) -> Option<&(dyn StdError + Send + Sync + 'static)> {
        self.source.as_deref()
    }

    #[must_use]
    pub fn validation<S: Into<String>>(message: S) -> Self {
        Validation {
            reason: message.into(),
        }
        .into()
    }

    #[must_use]
    pub fn status<S: Into<String>>(
        status_code: StatusCode,
        method: Method,
        path: String,
        message: S,
    ) -> Self {
        Status {
            status_code,
            method,
            path,
            message: message.into(),
        }
        .into()
    }

    #[must_use]
    pub fn missing_contract_config(chain_id: ChainId, neg_risk: bool) -> Self {
        MissingContractConfig { chain_id, neg_risk }.into()
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.source {
            Some(source) => write!(f, "{:?}: {source}", self.kind),
            None => write!(f, "{:?}", self.kind),
        }
    }
}

impl StdError for Error {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        self.source
            .as_deref()
            .map(|source| source as &(dyn StdError + 'static))
    }
}

#[non_exhaustive]
#[derive(Debug)]
pub struct Status {
    pub status_code: StatusCode,
    pub method: Method,
    pub path: String,
    pub message: String,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "error({}) making {} call to {} with {}",
            self.status_code, self.method, self.path, self.message
        )
    }
}

impl StdError for Status {}

#[non_exhaustive]
#[derive(Debug)]
pub struct Validation {
    pub reason: String,
}

impl fmt::Display for Validation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid: {}", self.reason)
    }
}

impl StdError for Validation {}

#[non_exhaustive]
#[derive(Debug)]
pub struct Synchronization;

impl fmt::Display for Synchronization {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "synchronization error: multiple threads attempted a client state transition"
        )
    }
}

impl StdError for Synchronization {}

#[non_exhaustive]
#[derive(Debug, Clone, Copy)]
pub struct MissingContractConfig {
    pub chain_id: ChainId,
    pub neg_risk: bool,
}

impl fmt::Display for MissingContractConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "missing contract config for chain id {} with neg_risk = {}",
            self.chain_id, self.neg_risk
        )
    }
}

impl StdError for MissingContractConfig {}

#[non_exhaustive]
#[derive(Debug, Clone)]
pub struct Geoblock {
    pub ip: String,
    pub country: String,
    pub region: String,
}

impl fmt::Display for Geoblock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "access blocked from country: {}, region: {}, ip: {}",
            self.country, self.region, self.ip
        )
    }
}

impl StdError for Geoblock {}

impl From<Status> for Error {
    fn from(error: Status) -> Self {
        Self::with_source(Kind::Status, error)
    }
}

impl From<Validation> for Error {
    fn from(error: Validation) -> Self {
        Self::with_source(Kind::Validation, error)
    }
}

impl From<Synchronization> for Error {
    fn from(error: Synchronization) -> Self {
        Self::with_source(Kind::Synchronization, error)
    }
}

impl From<MissingContractConfig> for Error {
    fn from(error: MissingContractConfig) -> Self {
        Self::with_source(Kind::Internal, error)
    }
}

impl From<Geoblock> for Error {
    fn from(error: Geoblock) -> Self {
        Self::with_source(Kind::Geoblock, error)
    }
}

impl From<base64::DecodeError> for Error {
    fn from(error: base64::DecodeError) -> Self {
        Self::with_source(Kind::Internal, error)
    }
}

impl From<reqwest::Error> for Error {
    fn from(error: reqwest::Error) -> Self {
        Self::with_source(Kind::Internal, error)
    }
}

impl From<header::InvalidHeaderValue> for Error {
    fn from(error: header::InvalidHeaderValue) -> Self {
        Self::with_source(Kind::Internal, error)
    }
}

impl From<InvalidLength> for Error {
    fn from(error: InvalidLength) -> Self {
        Self::with_source(Kind::Internal, error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::with_source(Kind::Internal, error)
    }
}

impl From<alloy::signers::Error> for Error {
    fn from(error: alloy::signers::Error) -> Self {
        Self::with_source(Kind::Internal, error)
    }
}

impl From<url::ParseError> for Error {
    fn from(error: url::ParseError) -> Self {
        Self::with_source(Kind::Internal, error)
    }
}

impl From<ParseError> for Error {
    fn from(error: ParseError) -> Self {
        Self::with_source(Kind::Internal, error)
    }
}

#[cfg(feature = "ws")]
impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(error: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::with_source(Kind::WebSocket, error)
    }
}

//! Shared primitive re-exports used throughout the client.

/// Alloy address and bytes primitives.
pub use alloy::primitives::{Address, B256, ChainId, Signature, U256, address, b256};
/// Common chrono date/time types used by request and response models.
pub use chrono::{DateTime, NaiveDate, Utc};
/// Decimal arithmetic type used for prices, sizes, and fees.
pub use rust_decimal::Decimal;
/// Decimal literal macro re-exported for ergonomic tests and examples.
pub use rust_decimal_macros::dec;

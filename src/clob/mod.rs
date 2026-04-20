//! Polymarket CLOB client implementation.
//!
//! This module exposes the type-state HTTP client, order builders, and the
//! strongly typed request and response models used by the Rust V2 client.

pub mod client;
pub mod order_builder;
pub mod types;
#[cfg(feature = "ws")]
pub mod ws;

pub use client::{AuthenticationBuilder, Client, Config};
pub use order_builder::{Limit, Market, OrderBuilder, UserMarketOrder, UserOrder};

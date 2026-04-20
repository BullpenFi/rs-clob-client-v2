#![allow(
    clippy::module_name_repetitions,
    reason = "Order model and helper names intentionally mirror the public API surface"
)]

use std::borrow::Cow;

use alloy::core::sol;
use alloy::dyn_abi::Eip712Domain;
use alloy::primitives::{Address, B256, ChainId, Signature, U256};
use alloy::signers::Signer;
use alloy::sol_types::SolStruct as _;

use crate::Result;
use crate::auth::ApiKey;
use crate::clob::types::{OrderType, SignatureTypeV2};

pub const CTF_EXCHANGE_V2_DOMAIN_NAME: &str = "Polymarket CTF Exchange";
pub const CTF_EXCHANGE_V2_DOMAIN_VERSION: &str = "2";

sol! {
    #[derive(Debug, PartialEq, Eq)]
    #[allow(
        clippy::exhaustive_structs,
        reason = "The signed order struct must exactly match Polymarket's EIP-712 schema"
    )]
    struct Order {
        uint256 salt;
        address maker;
        address signer;
        uint256 tokenId;
        uint256 makerAmount;
        uint256 takerAmount;
        uint8 side;
        uint8 signatureType;
        uint256 timestamp;
        bytes32 metadata;
        bytes32 builder;
    }
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignableOrder {
    pub order: Order,
    // Polymarket V2 sends expiration in the POST payload, but it is not part of the
    // typed-data struct or on-chain V2 Order tuple and must not affect the signing hash.
    pub expiration: u64,
    pub order_type: OrderType,
    // The TS V2 client always serializes `postOnly`, including `false` for FOK/FAK
    // orders. The server rejects only unsupported `true` combinations, so keeping a
    // concrete bool here is intentional.
    pub post_only: bool,
    pub defer_exec: bool,
}

#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedOrder {
    pub order: Order,
    pub expiration: u64,
    pub signature: Signature,
    pub owner: ApiKey,
    pub order_type: OrderType,
    pub post_only: bool,
    pub defer_exec: bool,
}

#[must_use]
pub fn signing_domain(chain_id: ChainId, verifying_contract: Address) -> Eip712Domain {
    Eip712Domain {
        name: Some(Cow::Borrowed(CTF_EXCHANGE_V2_DOMAIN_NAME)),
        version: Some(Cow::Borrowed(CTF_EXCHANGE_V2_DOMAIN_VERSION)),
        chain_id: Some(U256::from(chain_id)),
        verifying_contract: Some(verifying_contract),
        ..Eip712Domain::default()
    }
}

#[must_use]
pub fn signing_hash(order: &Order, chain_id: ChainId, verifying_contract: Address) -> B256 {
    order.eip712_signing_hash(&signing_domain(chain_id, verifying_contract))
}

pub async fn sign_order<S: Signer + ?Sized>(
    signer: &S,
    order: &Order,
    chain_id: ChainId,
    verifying_contract: Address,
) -> Result<Signature> {
    Ok(signer
        .sign_hash(&signing_hash(order, chain_id, verifying_contract))
        .await?)
}

#[must_use]
#[allow(
    clippy::too_many_arguments,
    reason = "The constructor mirrors Polymarket's fixed V2 order schema"
)]
pub fn new_order(
    salt: U256,
    maker: Address,
    signer: Address,
    token_id: U256,
    maker_amount: U256,
    taker_amount: U256,
    side: crate::clob::types::Side,
    signature_type: SignatureTypeV2,
    timestamp_ms: u64,
    metadata: B256,
    builder: B256,
) -> Order {
    Order {
        salt,
        maker,
        signer,
        tokenId: token_id,
        makerAmount: maker_amount,
        takerAmount: taker_amount,
        side: side as u8,
        signatureType: signature_type as u8,
        timestamp: U256::from(timestamp_ms),
        metadata,
        builder,
    }
}

#[cfg(test)]
mod tests {
    use alloy::primitives::{B256, U256, address, keccak256};

    use super::*;
    use crate::clob::types::Side;

    fn sample_order() -> Order {
        new_order(
            U256::from(1_u64),
            address!("0x0000000000000000000000000000000000000001"),
            address!("0x0000000000000000000000000000000000000002"),
            U256::from(123_u64),
            U256::from(1_000_000_u64),
            U256::from(2_000_000_u64),
            Side::Buy,
            SignatureTypeV2::Eoa,
            1_700_000_000_000,
            B256::ZERO,
            B256::ZERO,
        )
    }

    #[test]
    fn order_type_hash_matches_ts_v2_typed_data() {
        let expected = keccak256(
            "Order(uint256 salt,address maker,address signer,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint8 side,uint8 signatureType,uint256 timestamp,bytes32 metadata,bytes32 builder)",
        );

        assert_eq!(sample_order().eip712_type_hash(), expected);
    }
}

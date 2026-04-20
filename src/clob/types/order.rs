use std::borrow::Cow;

use alloy::core::sol;
use alloy::dyn_abi::Eip712Domain;
use alloy::primitives::{Address, B256, ChainId, Signature, U256};
use alloy::signers::Signer;
use alloy::sol_types::SolStruct as _;

use crate::auth::ApiKey;
use crate::clob::types::{OrderType, SignatureTypeV2};
use crate::Result;

pub const CTF_EXCHANGE_V2_DOMAIN_NAME: &str = "Polymarket CTF Exchange";
pub const CTF_EXCHANGE_V2_DOMAIN_VERSION: &str = "2";

sol! {
    #[derive(Debug, PartialEq, Eq)]
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
        uint256 expiration;
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignableOrder {
    pub order: Order,
    pub order_type: OrderType,
    pub post_only: bool,
    pub defer_exec: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignedOrder {
    pub order: Order,
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

pub async fn sign_order<S: Signer>(
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
    expiration: u64,
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
        expiration: U256::from(expiration),
    }
}

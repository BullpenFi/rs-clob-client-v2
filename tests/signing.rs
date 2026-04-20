#![allow(
    clippy::exhaustive_structs,
    reason = "The sol! helper mirrors a fixed EIP-712 test layout"
)]

mod common;

use alloy::primitives::{B256, U256, address, keccak256};
use alloy::sol;
use alloy::sol_types::{SolStruct as _, SolValue as _};
use polymarket_clob_client_v2::clob::types::new_order;
use polymarket_clob_client_v2::clob::types::{
    Order, Side, SignatureTypeV2, sign_order, signing_domain, signing_hash,
};
use polymarket_clob_client_v2::config::exchange_contract;
use polymarket_clob_client_v2::{AMOY, POLYGON};

fn sample_order() -> Order {
    new_order(
        U256::from(1_u64),
        address!("0x0000000000000000000000000000000000000001"),
        common::signer().address(),
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

sol! {
    struct DomainFields {
        bytes32 typeHash;
        bytes32 nameHash;
        bytes32 versionHash;
        uint256 chainId;
        address verifyingContract;
    }
}

fn expected_domain_separator(
    chain_id: u64,
    verifying_contract: polymarket_clob_client_v2::types::Address,
) -> B256 {
    let fields = DomainFields {
        typeHash: keccak256(
            "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)",
        ),
        nameHash: keccak256("Polymarket CTF Exchange"),
        versionHash: keccak256("2"),
        chainId: U256::from(chain_id),
        verifyingContract: verifying_contract,
    };

    keccak256(fields.abi_encode())
}

#[tokio::test]
async fn deterministic_signature() {
    let signer = common::signer();
    let order = sample_order();
    let verifying_contract = exchange_contract(POLYGON, false).expect("exchange contract");

    let first = sign_order(&signer, &order, POLYGON, verifying_contract)
        .await
        .expect("first signature");
    let second = sign_order(&signer, &order, POLYGON, verifying_contract)
        .await
        .expect("second signature");

    assert_eq!(first, second);
}

#[test]
fn type_hash_matches_keccak() {
    let expected = keccak256(
        "Order(uint256 salt,address maker,address signer,uint256 tokenId,uint256 makerAmount,uint256 takerAmount,uint8 side,uint8 signatureType,uint256 timestamp,bytes32 metadata,bytes32 builder)",
    );

    assert_eq!(sample_order().eip712_type_hash(), expected);
}

#[test]
fn domain_separator_polygon_matches_manual_keccak() {
    let verifying_contract = exchange_contract(POLYGON, false).expect("exchange contract");

    assert_eq!(
        signing_domain(POLYGON, verifying_contract).separator(),
        expected_domain_separator(POLYGON, verifying_contract)
    );
}

#[test]
fn domain_separator_amoy_matches_manual_keccak() {
    let verifying_contract = exchange_contract(AMOY, false).expect("exchange contract");

    assert_eq!(
        signing_domain(AMOY, verifying_contract).separator(),
        expected_domain_separator(AMOY, verifying_contract)
    );
}

#[test]
fn neg_risk_exchange_routing_changes_signing_hash() {
    let order = sample_order();
    let standard_exchange = exchange_contract(POLYGON, false).expect("standard exchange");
    let neg_risk_exchange = exchange_contract(POLYGON, true).expect("neg risk exchange");

    assert_ne!(standard_exchange, neg_risk_exchange);
    assert_ne!(
        signing_hash(&order, POLYGON, standard_exchange),
        signing_hash(&order, POLYGON, neg_risk_exchange)
    );
}

#[tokio::test]
async fn sign_then_recover_round_trip() {
    let signer = common::signer();
    let order = sample_order();
    let verifying_contract = exchange_contract(POLYGON, false).expect("exchange contract");
    let hash = signing_hash(&order, POLYGON, verifying_contract);
    let signature = sign_order(&signer, &order, POLYGON, verifying_contract)
        .await
        .expect("sign order");

    assert_eq!(
        signature
            .recover_address_from_prehash(&hash)
            .expect("recover signer"),
        signer.address()
    );
}

use polymarket_clob_client_v2::clob::types::{Side, TickSize};
use polymarket_clob_client_v2::config::contract_config;
use polymarket_clob_client_v2::error::{Kind, Method, StatusCode};
use polymarket_clob_client_v2::{AMOY, Error};

#[test]
fn error_validation_sets_kind() {
    let error = Error::validation("invalid request");
    assert_eq!(error.kind(), Kind::Validation);
}

#[test]
fn error_status_sets_kind() {
    let error = Error::status(
        StatusCode::BAD_REQUEST,
        Method::GET,
        "/markets".to_owned(),
        "bad request",
    );
    assert_eq!(error.kind(), Kind::Status);
}

#[test]
fn amoy_contract_config_matches_reference() {
    let config = contract_config(AMOY).expect("missing amoy config");
    assert_eq!(
        config.exchange_v2,
        polymarket_clob_client_v2::types::address!("0xE111180000d2663C0091e4f400237545B87B996B")
    );
}

#[test]
fn side_round_trip_serialization() {
    let value = serde_json::to_string(&Side::Buy).expect("serialize side");
    assert_eq!(value, "\"BUY\"");

    let round_trip: Side = serde_json::from_str(&value).expect("deserialize side");
    assert_eq!(round_trip, Side::Buy);
}

#[test]
fn tick_size_rounding_config_matches_ts_reference() {
    let config = TickSize::Hundredth.round_config();
    assert_eq!(config.price, 2);
    assert_eq!(config.size, 2);
    assert_eq!(config.amount, 4);
}

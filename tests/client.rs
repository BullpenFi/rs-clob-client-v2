use std::str::FromStr as _;

use alloy::signers::Signer as _;
use httpmock::Method::GET;
use httpmock::MockServer;
use polymarket_clob_client_v2::auth::{Credentials, PrivateKeySigner};
use polymarket_clob_client_v2::clob::{Client, Config};
use uuid::Uuid;

#[tokio::test]
async fn public_ok_endpoint_works_against_httpmock() {
    let server = MockServer::start();
    let mock = server.mock(|when, then| {
        when.method(GET).path("/ok");
        then.status(200).body("OK");
    });

    let client = Client::new(&server.base_url(), Config::default()).expect("client");
    let response = client.ok().await.expect("ok response");

    mock.assert();
    assert_eq!(response, "OK");
}

#[tokio::test]
async fn authenticated_api_keys_endpoint_signs_l2_headers() {
    let server = MockServer::start();
    let signer = PrivateKeySigner::from_str(
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80",
    )
    .expect("valid private key")
    .with_chain_id(Some(polymarket_clob_client_v2::POLYGON));

    let mock = server.mock(|when, then| {
        when.method(GET)
            .path("/auth/api-keys")
            .header("POLY_API_KEY", "00000000-0000-0000-0000-000000000000");
        then.status(200)
            .json_body_obj(&serde_json::json!({ "apiKeys": [] }));
    });

    let client = Client::new(&server.base_url(), Config::default())
        .expect("client")
        .authentication_builder(&signer)
        .credentials(Credentials::new(
            Uuid::nil(),
            "c2VjcmV0".to_owned(),
            "passphrase".to_owned(),
        ))
        .authenticate()
        .await
        .expect("authenticated client");

    let response = client.api_keys().await.expect("api keys");
    mock.assert();
    assert!(response.api_keys.is_empty());
}

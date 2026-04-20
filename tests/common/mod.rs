use std::str::FromStr as _;

use polymarket_clob_client_v2::POLYGON;
use polymarket_clob_client_v2::auth::{Credentials, Normal, PrivateKeySigner, Signer as _};
use polymarket_clob_client_v2::clob::{Client, Config};
use uuid::Uuid;

pub const PRIVATE_KEY: &str = "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80";
pub const SECRET: &str = "c2VjcmV0";
pub const PASSPHRASE: &str = "passphrase";
#[allow(dead_code)]
pub const TEST_HOST: &str = "https://example.com";

pub fn signer() -> PrivateKeySigner {
    PrivateKeySigner::from_str(PRIVATE_KEY)
        .expect("valid private key")
        .with_chain_id(Some(POLYGON))
}

pub fn credentials() -> Credentials {
    Credentials::new(Uuid::nil(), SECRET.to_owned(), PASSPHRASE.to_owned())
}

#[allow(dead_code)]
pub async fn create_authenticated(
    host: &str,
    config: Config,
) -> Client<polymarket_clob_client_v2::auth::state::Authenticated<Normal>> {
    Client::new(host, config)
        .expect("client")
        .authentication_builder(&signer())
        .credentials(credentials())
        .authenticate()
        .await
        .expect("authenticated client")
}

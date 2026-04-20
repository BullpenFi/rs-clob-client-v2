use alloy::primitives::ChainId;
use phf::phf_map;

use crate::types::{Address, address};

pub const COLLATERAL_TOKEN_DECIMALS: u32 = 6;
pub const CONDITIONAL_TOKEN_DECIMALS: u32 = 6;

#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContractConfig {
    pub exchange_v2: Address,
    pub neg_risk_exchange_v2: Address,
    pub collateral: Address,
    pub conditional_tokens: Address,
    pub neg_risk_adapter: Address,
}

static CONTRACTS: phf::Map<ChainId, ContractConfig> = phf_map! {
    137_u64 => ContractConfig {
        exchange_v2: address!("0xE111180000d2663C0091e4f400237545B87B996B"),
        neg_risk_exchange_v2: address!("0xe2222d279d744050d28e00520010520000310F59"),
        collateral: address!("0xC011a7E12a19f7B1f670d46F03B03f3342E82DFB"),
        conditional_tokens: address!("0x4D97DCd97eC945f40cF65F87097ACe5EA0476045"),
        neg_risk_adapter: address!("0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296"),
    },
    80002_u64 => ContractConfig {
        exchange_v2: address!("0xE111180000d2663C0091e4f400237545B87B996B"),
        neg_risk_exchange_v2: address!("0xe2222d279d744050d28e00520010520000310F59"),
        collateral: address!("0xC011a7E12a19f7B1f670d46F03B03f3342E82DFB"),
        conditional_tokens: address!("0x69308FB512518e39F9b16112fA8d994F4e2Bf8bB"),
        neg_risk_adapter: address!("0xd91E80cF2E7be2e162c6513ceD06f1dD0dA35296"),
    },
};

#[must_use]
pub fn contract_config(chain_id: ChainId) -> Option<&'static ContractConfig> {
    CONTRACTS.get(&chain_id)
}

#[must_use]
pub fn exchange_contract(chain_id: ChainId, neg_risk: bool) -> Option<Address> {
    contract_config(chain_id).map(|config| {
        if neg_risk {
            config.neg_risk_exchange_v2
        } else {
            config.exchange_v2
        }
    })
}

#[cfg(test)]
mod tests {
    use crate::{AMOY, POLYGON};

    use super::{contract_config, exchange_contract};

    #[test]
    fn contains_polygon_config() {
        let config = contract_config(POLYGON).expect("missing polygon config");
        assert_eq!(
            config.exchange_v2,
            crate::types::address!("0xE111180000d2663C0091e4f400237545B87B996B")
        );
    }

    #[test]
    fn contains_amoy_neg_risk_exchange() {
        let exchange = exchange_contract(AMOY, true).expect("missing amoy config");
        assert_eq!(
            exchange,
            crate::types::address!("0xe2222d279d744050d28e00520010520000310F59")
        );
    }
}

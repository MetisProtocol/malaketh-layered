use crate::ValidatorSet;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Genesis {
    pub validator_set: ValidatorSet,
}

#[cfg(test)]
mod tests {
    use super::*;

    const VALIDATOR_SET_JSON: &str = r#"
    {
        "validator_set": {
            "validators": [
            {
                "consensus_address": "0x0754445aeda0441230d3ab099b0942181915186c",
                "operator_address": "0xEf6A32d98b3d7C8933164dEE6F14fbdC09767AEc",
                "public_key": {
                "type": "tendermint/PubKeyEd25519",
                "value": "lwB6erO0yiT4uI5tzrdk/ov/gQv0X8Fu978JQfy9eic="
                },
                "voting_power": 1
            },
            {
                "consensus_address": "0x3f8f2908b1b5b6ef3eec1968fcdf8340a6bec221",
                "operator_address": "0x7A0aF7D1a9c5701b7FBcde045aa3D4439A10103a",
                "public_key": {
                "type": "tendermint/PubKeyEd25519",
                "value": "2sSy+F3l4EwwGgd7CCVvZZ3d82o5V4NhsZmd9WI3q44="
                },
                "voting_power": 1
            },
            {
                "consensus_address": "0x9ab1a8b89460fccd8eb6739352300988915c71fe",
                "operator_address": "0x680Cc4Ad7cdD0bF294b5D400EB74AfEF8fD1DCF8",
                "public_key": {
                "type": "tendermint/PubKeyEd25519",
                "value": "G0lKW8Y0v6FAwfW492XHwCA6XTpziDVC7D3Q2q/DYVc="
                },
                "voting_power": 1
            }
            ]
        }
    }
    "#;

    #[test]
    fn test_genesis() {
        let genesis: Genesis = serde_json::from_str(VALIDATOR_SET_JSON).unwrap();
        
        assert_eq!(genesis.validator_set.validators.len(), 3);
        assert_eq!(genesis.validator_set.validators[0].voting_power, 1);
    }

    #[test]
    fn test_genesis_from_file() {
        let genesis_str = std::fs::read_to_string("../nodes_config_bin/0/config/genesis.json").unwrap();
        let genesis: Genesis = serde_json::from_str(&genesis_str).unwrap();

        assert_eq!(genesis.validator_set.validators.len(), 3);
        assert_eq!(genesis.validator_set.validators[0].voting_power, 1);
    }
}

use alloy_genesis::{ChainConfig, Genesis, GenesisAccount};
use alloy_primitives::{Address, FixedBytes, B256, U256};
use alloy_signer_local::{coins_bip39::English, LocalSigner, MnemonicBuilder};
use chrono::NaiveDate;
use color_eyre::eyre::Result;
use k256::ecdsa::SigningKey;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fs, str::FromStr};

/// Test mnemonics for wallet generation
const TEST_MNEMONICS: [&str; 3] = [
    "test test test test test test test test test test test junk",
    "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about",
    "zero zero zero zero zero zero zero zero zero zero zero zoo",
];

/// Validator information from genesis file
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisValidator {
    pub consensus_address: String, // Tendermint address for consensus
    pub operator_address: String,  // Ethereum address for smart contract operations
    pub public_key: GenesisPublicKey,
    pub voting_power: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisPublicKey {
    #[serde(rename = "type")]
    pub key_type: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisValidatorSet {
    pub validators: Vec<GenesisValidator>,
}

const VALIDATOR_SET_MANAGER_BYTECODE: &str = "608060405234801561000f575f5ffd5b506004361061014b575f3560e01c80637071688a116100c1578063c0f531c11161007a578063c0f531c11461038c578063cfe8a73b146103aa578063d5a6151a146103c8578063e5358c4f146103e6578063f851a44014610402578063fa52c7d8146104205761014b565b80637071688a146102b557806386d54506146102d35780638a11d7c914610303578063904b1cbf14610333578063973e35b61461034f578063a944dcb6146103705761014b565b80633e47158c116101135780633e47158c146101f357806347c026611461021157806354eea7961461022d578063569c77271461024957806357d775f8146102795780635c60da1b146102975761014b565b80631394890a1461014f57806314f64c781461016d5780631af60f721461019d5780631cfe4f0b146101b95780633659cfe6146101d7575b5f5ffd5b610157610453565b6040516101649190611741565b60405180910390f35b6101876004803603810190610182919061178c565b610459565b60405161019491906117f6565b60405180910390f35b6101b760048036038101906101b29190611839565b610494565b005b6101c16104a0565b6040516101ce9190611741565b60405180910390f35b6101f160048036038101906101ec9190611839565b6104a9565b005b6101fb610669565b60405161020891906117f6565b60405180910390f35b61022b60048036038101906102269190611839565b61068e565b005b6102476004803603810190610242919061178c565b6107ce565b005b610263600480360381019061025e9190611864565b6108a9565b60405161027091906117f6565b60405180910390f35b6102816108f1565b60405161028e9190611741565b60405180910390f35b61029f6108f7565b6040516102ac91906117f6565b60405180910390f35b6102bd61091c565b6040516102ca9190611741565b60405180910390f35b6102ed60048036038101906102e89190611839565b610928565b6040516102fa91906117f6565b60405180910390f35b61031d60048036038101906103189190611839565b610958565b60405161032a919061192b565b60405180910390f35b61034d60048036038101906103489190611a4f565b610a6b565b005b610357610cef565b6040516103679493929190611d3d565b60405180910390f35b61038a6004803603810190610385919061178c565b611113565b005b6103946111ee565b6040516103a19190611741565b60405180910390f35b6103b26111f7565b6040516103bf9190611741565b60405180910390f35b6103d0611200565b6040516103dd9190611741565b60405180910390f35b61040060048036038101906103fb9190611dc6565b611206565b005b61040a611218565b60405161041791906117f6565b60405180910390f35b61043a60048036038101906104359190611839565b61123d565b60405161044a9493929190611e39565b60405180910390f35b60065481565b60038181548110610468575f80fd5b905f5260205f20015f915054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b61049d816112a6565b50565b5f600454905090565b60095f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614610538576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161052f90611ed6565b60405180910390fd5b5f73ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff16036105a6576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161059d90611f3e565b60405180910390fd5b5f60085f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1690508160085f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055508173ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff167f3684250ce1e33b790ed973c23080f312db0adb21a6d98c61a5c9ff99e4babc1760405160405180910390a35050565b60095f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b60095f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff161461071d576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161071490611ed6565b60405180910390fd5b5f73ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff160361078b576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161078290611fa6565b60405180910390fd5b8060095f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555050565b60075f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff161461085d576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016108549061200e565b60405180910390fd5b5f811161089f576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161089690612076565b60405180910390fd5b8060058190555050565b6002602052815f5260405f2081815481106108c2575f80fd5b905f5260205f20015f915091509054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b60055481565b60085f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b5f600380549050905090565b6001602052805f5260405f205f915054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b6109606116d9565b5f5f8373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f206040518060800160405290815f82015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001600182015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001600282015481526020016003820154815250509050919050565b5f73ffffffffffffffffffffffffffffffffffffffff1660075f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1614610afa576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401610af1906120de565b60405180910390fd5b3360075f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055503360095f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055508060058190555060156004819055508686905089899050148015610ba357508484905089899050145b8015610bb457508282905089899050145b610bf3576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401610bea90612146565b60405180910390fd5b6003898990501015610c3a576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401610c31906121ae565b60405180910390fd5b5f5f90505b89899050811015610ce357610cd68a8a83818110610c6057610c5f6121cc565b5b9050602002016020810190610c759190611839565b898984818110610c8857610c876121cc565b5b9050602002016020810190610c9d9190611839565b888885818110610cb057610caf6121cc565b5b90506020020135878786818110610cca57610cc96121cc565b5b90506020020135611469565b8080600101915050610c3f565b50505050505050505050565b6060806060805f60038054905067ffffffffffffffff811115610d1557610d146121f9565b5b604051908082528060200260200182016040528015610d435781602001602082028036833780820191505090505b5090505f60038054905067ffffffffffffffff811115610d6657610d656121f9565b5b604051908082528060200260200182016040528015610d945781602001602082028036833780820191505090505b5090505f60038054905067ffffffffffffffff811115610db757610db66121f9565b5b604051908082528060200260200182016040528015610de55781602001602082028036833780820191505090505b5090505f60038054905067ffffffffffffffff811115610e0857610e076121f9565b5b604051908082528060200260200182016040528015610e365781602001602082028036833780820191505090505b5090505f5f90505b6003805490508110156110fc5760038181548110610e5f57610e5e6121cc565b5b905f5260205f20015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff16858281518110610e9a57610e996121cc565b5b602002602001019073ffffffffffffffffffffffffffffffffffffffff16908173ffffffffffffffffffffffffffffffffffffffff16815250505f5f60038381548110610eea57610ee96121cc565b5b905f5260205f20015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f206001015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff16848281518110610f8157610f806121cc565b5b602002602001019073ffffffffffffffffffffffffffffffffffffffff16908173ffffffffffffffffffffffffffffffffffffffff16815250505f5f60038381548110610fd157610fd06121cc565b5b905f5260205f20015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f2060020154838281518110611049576110486121cc565b5b6020026020010181815250505f5f6003838154811061106b5761106a6121cc565b5b905f5260205f20015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f20600301548282815181106110e3576110e26121cc565b5b6020026020010181815250508080600101915050610e3e565b508383838397509750975097505050505090919293565b60075f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16146111a2576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016111999061200e565b60405180910390fd5b5f81116111e4576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016111db90612270565b60405180910390fd5b8060048190555050565b5f600654905090565b5f600554905090565b60045481565b61121284848484611469565b50505050565b60075f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b5f602052805f5260405f205f91509050805f015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1690806001015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff16908060020154908060030154905084565b5f5f90505b600380549050811015611422578173ffffffffffffffffffffffffffffffffffffffff16600382815481106112e3576112e26121cc565b5b905f5260205f20015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1603611415576003600160038054905061133a91906122bb565b8154811061134b5761134a6121cc565b5b905f5260205f20015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1660038281548110611387576113866121cc565b5b905f5260205f20015f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555060038054806113de576113dd6122ee565b5b600190038181905f5260205f20015f6101000a81549073ffffffffffffffffffffffffffffffffffffffff02191690559055611422565b80806001019150506112ab565b508073ffffffffffffffffffffffffffffffffffffffff167fe1434e25d6611e0db941968fdc97811c982ac1602e951637d206f5fdda9dd8f160405160405180910390a250565b60405180608001604052808573ffffffffffffffffffffffffffffffffffffffff1681526020018473ffffffffffffffffffffffffffffffffffffffff168152602001838152602001828152505f5f8673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f205f820151815f015f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055506020820151816001015f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555060408201518160020155606082015181600301559050508260015f8673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f205f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff160217905550600384908060018154018082558091505060019003905f5260205f20015f9091909190916101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055508273ffffffffffffffffffffffffffffffffffffffff168473ffffffffffffffffffffffffffffffffffffffff167fe04cca73492eae55b9a3507ddb917fbe17f1fc77fe360f7cea9fe34c8f9393e6846040516116cb9190611741565b60405180910390a350505050565b60405180608001604052805f73ffffffffffffffffffffffffffffffffffffffff1681526020015f73ffffffffffffffffffffffffffffffffffffffff1681526020015f81526020015f81525090565b5f819050919050565b61173b81611729565b82525050565b5f6020820190506117545f830184611732565b92915050565b5f5ffd5b5f5ffd5b61176b81611729565b8114611775575f5ffd5b50565b5f8135905061178681611762565b92915050565b5f602082840312156117a1576117a061175a565b5b5f6117ae84828501611778565b91505092915050565b5f73ffffffffffffffffffffffffffffffffffffffff82169050919050565b5f6117e0826117b7565b9050919050565b6117f0816117d6565b82525050565b5f6020820190506118095f8301846117e7565b92915050565b611818816117d6565b8114611822575f5ffd5b50565b5f813590506118338161180f565b92915050565b5f6020828403121561184e5761184d61175a565b5b5f61185b84828501611825565b91505092915050565b5f5f6040838503121561187a5761187961175a565b5b5f61188785828601611778565b925050602061189885828601611778565b9150509250929050565b6118ab816117d6565b82525050565b6118ba81611729565b82525050565b5f819050919050565b6118d2816118c0565b82525050565b608082015f8201516118ec5f8501826118a2565b5060208201516118ff60208501826118a2565b50604082015161191260408501826118b1565b50606082015161192560608501826118c9565b50505050565b5f60808201905061193e5f8301846118d8565b92915050565b5f5ffd5b5f5ffd5b5f5ffd5b5f5f83601f84011261196557611964611944565b5b8235905067ffffffffffffffff81111561198257611981611948565b5b60208301915083602082028301111561199e5761199d61194c565b5b9250929050565b5f5f83601f8401126119ba576119b9611944565b5b8235905067ffffffffffffffff8111156119d7576119d6611948565b5b6020830191508360208202830111156119f3576119f261194c565b5b9250929050565b5f5f83601f840112611a0f57611a0e611944565b5b8235905067ffffffffffffffff811115611a2c57611a2b611948565b5b602083019150836020820283011115611a4857611a4761194c565b5b9250929050565b5f5f5f5f5f5f5f5f5f60a08a8c031215611a6c57611a6b61175a565b5b5f8a013567ffffffffffffffff811115611a8957611a8861175e565b5b611a958c828d01611950565b995099505060208a013567ffffffffffffffff811115611ab857611ab761175e565b5b611ac48c828d01611950565b975097505060408a013567ffffffffffffffff811115611ae757611ae661175e565b5b611af38c828d016119a5565b955095505060608a013567ffffffffffffffff811115611b1657611b1561175e565b5b611b228c828d016119fa565b93509350506080611b358c828d01611778565b9150509295985092959850929598565b5f81519050919050565b5f82825260208201905092915050565b5f819050602082019050919050565b5f611b7983836118a2565b60208301905092915050565b5f602082019050919050565b5f611b9b82611b45565b611ba58185611b4f565b9350611bb083611b5f565b805f5b83811015611be0578151611bc78882611b6e565b9750611bd283611b85565b925050600181019050611bb3565b5085935050505092915050565b5f81519050919050565b5f82825260208201905092915050565b5f819050602082019050919050565b5f611c2183836118b1565b60208301905092915050565b5f602082019050919050565b5f611c4382611bed565b611c4d8185611bf7565b9350611c5883611c07565b805f5b83811015611c88578151611c6f8882611c16565b9750611c7a83611c2d565b925050600181019050611c5b565b5085935050505092915050565b5f81519050919050565b5f82825260208201905092915050565b5f819050602082019050919050565b5f611cc983836118c9565b60208301905092915050565b5f602082019050919050565b5f611ceb82611c95565b611cf58185611c9f565b9350611d0083611caf565b805f5b83811015611d30578151611d178882611cbe565b9750611d2283611cd5565b925050600181019050611d03565b5085935050505092915050565b5f6080820190508181035f830152611d558187611b91565b90508181036020830152611d698186611b91565b90508181036040830152611d7d8185611c39565b90508181036060830152611d918184611ce1565b905095945050505050565b611da5816118c0565b8114611daf575f5ffd5b50565b5f81359050611dc081611d9c565b92915050565b5f5f5f5f60808587031215611dde57611ddd61175a565b5b5f611deb87828801611825565b9450506020611dfc87828801611825565b9350506040611e0d87828801611778565b9250506060611e1e87828801611db2565b91505092959194509250565b611e33816118c0565b82525050565b5f608082019050611e4c5f8301876117e7565b611e5960208301866117e7565b611e666040830185611732565b611e736060830184611e2a565b95945050505050565b5f82825260208201905092915050565b7f4f6e6c792070726f78792061646d696e000000000000000000000000000000005f82015250565b5f611ec0601083611e7c565b9150611ecb82611e8c565b602082019050919050565b5f6020820190508181035f830152611eed81611eb4565b9050919050565b7f496e76616c696420696d706c656d656e746174696f6e000000000000000000005f82015250565b5f611f28601683611e7c565b9150611f3382611ef4565b602082019050919050565b5f6020820190508181035f830152611f5581611f1c565b9050919050565b7f496e76616c69642061646d696e000000000000000000000000000000000000005f82015250565b5f611f90600d83611e7c565b9150611f9b82611f5c565b602082019050919050565b5f6020820190508181035f830152611fbd81611f84565b9050919050565b7f4f6e6c792061646d696e000000000000000000000000000000000000000000005f82015250565b5f611ff8600a83611e7c565b915061200382611fc4565b602082019050919050565b5f6020820190508181035f83015261202581611fec565b9050919050565b7f496e76616c69642065706f6368206c656e6774680000000000000000000000005f82015250565b5f612060601483611e7c565b915061206b8261202c565b602082019050919050565b5f6020820190508181035f83015261208d81612054565b9050919050565b7f416c726561647920696e697469616c697a6564000000000000000000000000005f82015250565b5f6120c8601383611e7c565b91506120d382612094565b602082019050919050565b5f6020820190508181035f8301526120f5816120bc565b9050919050565b7f496e76616c696420696e707574000000000000000000000000000000000000005f82015250565b5f612130600d83611e7c565b915061213b826120fc565b602082019050919050565b5f6020820190508181035f83015261215d81612124565b9050919050565b7f4e656564206174206c6561737420332076616c696461746f72730000000000005f82015250565b5f612198601a83611e7c565b91506121a382612164565b602082019050919050565b5f6020820190508181035f8301526121c58161218c565b9050919050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52603260045260245ffd5b7f4e487b71000000000000000000000000000000000000000000000000000000005f52604160045260245ffd5b7f496e76616c69642076616c696461746f72206e756d62657200000000000000005f82015250565b5f61225a601883611e7c565b915061226582612226565b602082019050919050565b5f6020820190508181035f8301526122878161224e565b9050919050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52601160045260245ffd5b5f6122c582611729565b91506122d083611729565b92508282039050818111156122e8576122e761228e565b5b92915050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52603160045260245ffdfea26469706673582212205daba3f8216855b53c455f14c2c0c5b8334db0cc5f5e37cd2263366d775bc06364736f6c634300081e0033";
const VALIDATOR_SET_MANAGER_ADDRESS: &str = "0x0000000000000000000000000000000000000800";
const STAKING_MANAGER_ADDRESS: &str = "0x0000000000000000000000000000000000000801";
const REWARD_DISTRIBUTOR_ADDRESS: &str = "0x0000000000000000000000000000000000000802";
const SLASHING_MANAGER_ADDRESS: &str = "0x0000000000000000000000000000000000000803";

/// System parameters
const EPOCH_LENGTH: u64 = 100;

/// Storage slot constants - defined according to actual contract storage layout
///
/// According to VALIDATOR_SET_MANAGER_STORAGE_LAYOUT.md document:
/// 0: bool initialized (1)
/// 1: uint256 epochLength (100)
/// 2: uint256 minStakeAmount (1 ETH)
/// 3: uint256 maxValidators (100)
/// 4: uint256 genesisValidatorCount (3)
/// 5: uint256 activeValidatorCount (3)
/// 6+: mappings and arrays and other variables
const INITIALIZED_SLOT: u8 = 0; // bool public initialized;
const _EPOCH_LENGTH_SLOT: u8 = 1; // uint256 public epochLength;
const _MIN_STAKE_AMOUNT_SLOT: u8 = 2; // uint256 public minStakeAmount;
const _MAX_VALIDATORS_SLOT: u8 = 3; // uint256 public maxValidators;
const _GENESIS_VALIDATOR_COUNT_SLOT: u8 = 4; // uint256 public genesisValidatorCount;
const _ACTIVE_VALIDATOR_COUNT_SLOT: u8 = 5; // uint256 public activeValidatorCount;

/// Create a signer from a mnemonic.
pub(crate) fn make_signer(mnemonic: &str) -> LocalSigner<SigningKey> {
    MnemonicBuilder::<English>::default()
        .phrase(mnemonic)
        .build()
        .expect("Failed to create wallet")
}

/// Read validator_set from validator config file
fn read_genesis_validator_set(validator_config_path: &str) -> Result<GenesisValidatorSet> {
    let content = fs::read_to_string(validator_config_path).map_err(|e| {
        color_eyre::eyre::eyre!(
            "Failed to read validator config file {}: {}",
            validator_config_path,
            e
        )
    })?;

    // Parse complete genesis file structure
    #[derive(Deserialize)]
    struct GenesisFile {
        validator_set: GenesisValidatorSet,
    }

    let genesis_file: GenesisFile = serde_json::from_str(&content).map_err(|e| {
        color_eyre::eyre::eyre!(
            "Failed to parse validator config file {}: {}",
            validator_config_path,
            e
        )
    })?;

    Ok(genesis_file.validator_set)
}

/// Convert base64 encoded public key to 32-byte array
fn decode_public_key(base64_key: &str) -> Result<[u8; 32]> {
    use base64::{engine::general_purpose, Engine as _};

    let decoded = general_purpose::STANDARD
        .decode(base64_key)
        .map_err(|e| color_eyre::eyre::eyre!("Failed to decode base64 public key: {}", e))?;

    if decoded.len() != 32 {
        return Err(color_eyre::eyre::eyre!(
            "Invalid public key length: expected 32 bytes, got {}",
            decoded.len()
        ));
    }

    let mut key_bytes = [0u8; 32];
    key_bytes.copy_from_slice(&decoded);
    Ok(key_bytes)
}

pub(crate) fn make_signers() -> Vec<LocalSigner<SigningKey>> {
    TEST_MNEMONICS
        .iter()
        .map(|&mnemonic| make_signer(mnemonic))
        .collect()
}

/// Create initialized state storage item
fn create_initialized_storage() -> (FixedBytes<32>, FixedBytes<32>) {
    let mut init_key = [0u8; 32];
    init_key[31] = INITIALIZED_SLOT;
    let mut init_value = [0u8; 32];
    init_value[31] = 1; // initialized = true
    (FixedBytes::from(init_key), FixedBytes::from(init_value))
}

/// Calculate storage location for a key in mapping
/// For mapping(address => ValidatorInfo) validators, storage location is keccak256(abi.encodePacked(validator_address, validators_slot))
/// Note: According to Solidity storage layout, validators mapping is at slot 0
fn calculate_validator_storage_key(validator_address: Address) -> FixedBytes<32> {
    use alloy_primitives::keccak256;

    let mut data = [0u8; 64];
    // Put address in first 32 bytes
    data[12..32].copy_from_slice(validator_address.as_slice());
    // Put slot in last 32 bytes (validators mapping is at slot 0)
    data[63] = 0;

    let hash = keccak256(data);
    FixedBytes::from(hash)
}

/// Calculate storage location for an epoch in epochValidators mapping
/// For mapping(uint256 => address[]) epochValidators, storage location is keccak256(abi.encodePacked(epoch, epoch_validators_slot))
/// Note: According to contract storage layout, mapping starts from slot 2
fn calculate_epoch_validators_storage_key(epoch: u64) -> FixedBytes<32> {
    use alloy_primitives::keccak256;

    let mut data = [0u8; 64];
    // Put epoch in first 32 bytes
    let epoch_bytes = epoch.to_be_bytes();
    data[24..32].copy_from_slice(&epoch_bytes);
    // Put slot in last 32 bytes (mapping starts from slot 2)
    data[63] = 2;

    let hash = keccak256(data);
    FixedBytes::from(hash)
}

/// Utility function: u64 → slot (B256)
fn slot_u64(n: u64) -> B256 {
    B256::from(U256::from(n))
}

/// Create complete validator storage mapping
///
/// According to ValidatorSetManager contract, initialize the following storage items:
/// - Slot 0: mapping(address => ValidatorInfo) validators - validator information mapping
/// - Slot 1: mapping(address => address) consensusToOperator - consensus to operator address mapping
/// - Slot 2: mapping(uint256 => address[]) epochValidators - validator list for each epoch
/// - Slot 3: address[] activeValidators - current active validator array
/// - Slot 4: uint256 validatorNum - number of validators
/// - Slot 5: uint256 epochLength - epoch length
/// - Slot 6: uint256 updateHeight - update height
/// - Slot 7: address admin - admin address
/// - Slot 8: address implementation - implementation contract address
/// - Slot 9: address proxyAdmin - proxy admin address
///
/// ValidatorInfo struct contains: consensusAddress, operatorAddress, votingPower, publicKey
fn create_validator_storage(genesis_data: &GenesisValidatorSet) -> Result<BTreeMap<B256, B256>> {
    use alloy_primitives::keccak256;

    // storage mapping
    let mut storage: BTreeMap<B256, B256> = BTreeMap::new();

    let mut consensus_addresses = Vec::new();
    let mut operator_addresses = Vec::new();
    let mut powers = Vec::new();
    let mut public_keys = Vec::new();

    for validator in &genesis_data.validators {
        let consensus_address = validator
            .consensus_address
            .parse::<Address>()
            .map_err(|e| {
                color_eyre::eyre::eyre!(
                    "Invalid consensus address {}: {}",
                    validator.consensus_address,
                    e
                )
            })?;
        let operator_address = validator.operator_address.parse::<Address>().map_err(|e| {
            color_eyre::eyre::eyre!(
                "Invalid operator address {}: {}",
                validator.operator_address,
                e
            )
        })?;
        let public_key = decode_public_key(&validator.public_key.value)?;

        consensus_addresses.push(consensus_address);
        operator_addresses.push(operator_address);
        powers.push(validator.voting_power);
        public_keys.push(public_key);
    }

    let (
        genesis_consensus_addresses,
        genesis_operator_addresses,
        genesis_powers,
        genesis_public_keys,
    ) = (consensus_addresses, operator_addresses, powers, public_keys);

    // Slot 4: validatorNum = number of validators
    storage.insert(
        slot_u64(4),
        slot_u64(genesis_consensus_addresses.len() as u64),
    );

    // Slot 5: epochLength = 100
    storage.insert(slot_u64(5), slot_u64(EPOCH_LENGTH));

    // Slot 6: updateHeight = 0
    storage.insert(slot_u64(6), slot_u64(0));

    // Slot 7: admin (use first validator as admin)
    storage.insert(
        slot_u64(7),
        B256::from(genesis_consensus_addresses[0].into_word()),
    );

    // Slot 8: implementation (proxy implementation address, set to 0 for now)
    storage.insert(slot_u64(8), B256::ZERO);

    // Slot 9: proxyAdmin (proxy admin, use first validator)
    storage.insert(
        slot_u64(9),
        B256::from(genesis_consensus_addresses[0].into_word()),
    );

    // Initialize ValidatorInfo for each validator
    for (i, consensus_addr) in genesis_consensus_addresses.iter().enumerate() {
        let operator_addr = &genesis_operator_addresses[i];

        // Calculate storage key for validators mapping
        let validator_key = calculate_validator_storage_key(*consensus_addr);

        // ValidatorInfo struct layout in storage:
        // Each field occupies one slot, stored in order
        let base_slot = validator_key;

        // consensusAddress (address) - consensus address
        storage.insert(base_slot, B256::from(consensus_addr.into_word()));

        // operatorAddress (address) - operator address
        let operator_slot = B256::from(U256::try_from(base_slot).unwrap() + U256::from(1));
        storage.insert(operator_slot, B256::from(operator_addr.into_word()));

        // votingPower (uint256) - voting power (read from genesis file)
        let voting_power_slot = B256::from(U256::try_from(base_slot).unwrap() + U256::from(2));
        storage.insert(voting_power_slot, slot_u64(genesis_powers[i]));

        // publicKey (bytes32) - public key (read from genesis file)
        let public_key_slot = B256::from(U256::try_from(base_slot).unwrap() + U256::from(3));
        storage.insert(public_key_slot, B256::from(genesis_public_keys[i]));

        // Set consensusToOperator mapping
        let mapping_key = calculate_consensus_to_operator_key(*consensus_addr);
        storage.insert(mapping_key, B256::from(operator_addr.into_word()));
    }

    // Initialize activeValidators array
    // Array length stored in slot 3
    storage.insert(
        slot_u64(3),
        slot_u64(genesis_consensus_addresses.len() as u64),
    ); // array length

    // Array elements stored in keccak256(slot) + index
    let array_slot = slot_u64(3);
    let array_start = keccak256(array_slot.as_slice());
    let array_start_b256 = B256::from(array_start);

    for (i, consensus_addr) in genesis_consensus_addresses.iter().enumerate() {
        let element_slot = B256::from(U256::try_from(array_start_b256).unwrap() + U256::from(i));
        storage.insert(element_slot, B256::from(consensus_addr.into_word()));
    }

    // Initialize epochValidators mapping
    // Set validator list for epoch 0
    let epoch = 0u64;
    let epoch_key = calculate_epoch_validators_storage_key(epoch);

    // Array length
    storage.insert(
        epoch_key,
        slot_u64(genesis_consensus_addresses.len() as u64),
    ); // array length

    // Array elements
    let epoch_array_start = keccak256(epoch_key.as_slice());
    let epoch_array_start_b256 = B256::from(epoch_array_start);
    for (i, consensus_addr) in genesis_consensus_addresses.iter().enumerate() {
        let element_slot =
            B256::from(U256::try_from(epoch_array_start_b256).unwrap() + U256::from(i));
        storage.insert(element_slot, B256::from(consensus_addr.into_word()));
    }

    Ok(storage)
}

/// Calculate storage key for consensusToOperator mapping
fn calculate_consensus_to_operator_key(consensus_address: Address) -> B256 {
    use alloy_primitives::keccak256;

    // consensusToOperator mapping is at slot 1
    let mapping_slot = slot_u64(1);

    // For mapping(address => address), the key is keccak256(abi.encodePacked(consensus_address, mapping_slot))
    let mut data = [0u8; 64];
    data[12..32].copy_from_slice(consensus_address.as_slice());
    data[32..64].copy_from_slice(mapping_slot.as_slice());

    B256::from(keccak256(data))
}

/// Create basic admin storage mapping (for other contracts)
fn create_basic_admin_storage() -> BTreeMap<FixedBytes<32>, FixedBytes<32>> {
    let mut storage = BTreeMap::new();
    // For other contracts, we only set initialization state
    let (initialized_key, initialized_value) = create_initialized_storage();
    storage.insert(initialized_key, initialized_value);
    storage
}

pub(crate) fn generate_genesis_with_contracts(validator_config_path: &str) -> Result<()> {
    let genesis_file = "./assets/genesis.json";

    // Read validator addresses from config file
    let genesis_data = read_genesis_validator_set(validator_config_path)?;

    // Extract operator addresses (Ethereum addresses) for genesis allocation
    let operator_addresses: Vec<Address> = genesis_data
        .validators
        .iter()
        .map(|validator| {
            validator.operator_address.parse::<Address>().map_err(|e| {
                color_eyre::eyre::eyre!(
                    "Invalid operator address {}: {}",
                    validator.operator_address,
                    e
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    println!("Using validator addresses from config:");
    for (i, validator) in genesis_data.validators.iter().enumerate() {
        println!("Validator {i}:");
        println!("  Consensus (Tendermint): {}", validator.consensus_address);
        println!("  Operator (Ethereum): {}", validator.operator_address);
    }

    // Create genesis configuration with pre-funded accounts using operator addresses
    let mut alloc = BTreeMap::new();
    for addr in &operator_addresses {
        alloc.insert(
            *addr,
            GenesisAccount {
                balance: U256::from_str("15000000000000000000000").unwrap(), // 15000 ETH
                ..Default::default()
            },
        );
    }
    alloc.insert(
        Address::from_str("0x52732ef09590c920a8AA5161FE224e21fC85fD26").unwrap(),
        GenesisAccount {
            balance: U256::from_str("15000000000000000000000").unwrap(), // 15000 ETH
            ..Default::default()
        },
    );

    // Create validator storage
    let validator_storage = create_validator_storage(&genesis_data)?;

    // ValidatorSetManager contract
    let validator_set_manager_address = Address::from_str(VALIDATOR_SET_MANAGER_ADDRESS).unwrap();
    let bytecode = hex::decode(VALIDATOR_SET_MANAGER_BYTECODE).unwrap();

    alloc.insert(
        validator_set_manager_address,
        GenesisAccount {
            code: Some(bytecode.into()),
            storage: Some(validator_storage),
            balance: U256::from(123), // Set initial balance to 123
            nonce: Some(0),
            private_key: None,
            // storage: None,
        },
    );

    for (address_str, _name) in [
        (STAKING_MANAGER_ADDRESS, "StakingManager"),
        (REWARD_DISTRIBUTOR_ADDRESS, "RewardDistributor"),
        (SLASHING_MANAGER_ADDRESS, "SlashingManager"),
    ] {
        let address = Address::from_str(address_str).unwrap();
        let storage = create_basic_admin_storage();

        // TODO bytecode
        let bytecode = hex::decode(VALIDATOR_SET_MANAGER_BYTECODE).unwrap();
        alloc.insert(
            address,
            GenesisAccount {
                code: Some(bytecode.into()),
                storage: Some(storage),
                balance: U256::ZERO,
                nonce: Some(0),
                private_key: None,
            },
        );
    }

    // The Ethereum Cancun-Deneb (Dencun) upgrade was activated on the mainnet
    // on March 13, 2024, at epoch 269,568.
    let date = NaiveDate::from_ymd_opt(2024, 3, 14).unwrap();
    let datetime = date.and_hms_opt(0, 0, 0).unwrap();
    let valid_cancun_timestamp = datetime.and_utc().timestamp() as u64;

    // Create genesis configuration
    let genesis = Genesis {
        config: ChainConfig {
            chain_id: 1,
            homestead_block: Some(0),
            eip150_block: Some(0),
            eip155_block: Some(0),
            eip158_block: Some(0),
            byzantium_block: Some(0),
            constantinople_block: Some(0),
            petersburg_block: Some(0),
            istanbul_block: Some(0),
            berlin_block: Some(0),
            london_block: Some(0),
            shanghai_time: Some(0),
            cancun_time: Some(0),
            terminal_total_difficulty: Some(U256::ZERO),
            terminal_total_difficulty_passed: true,
            ..Default::default()
        },
        alloc,
        ..Default::default()
    }
    .with_gas_limit(30_000_000)
    .with_timestamp(valid_cancun_timestamp);

    // Create data directory if it doesn't exist
    std::fs::create_dir_all("./assets")?;

    // Write genesis to file
    let genesis_json = serde_json::to_string_pretty(&genesis)?;
    std::fs::write(genesis_file, genesis_json)?;
    println!("Genesis configuration written to {genesis_file}");

    Ok(())
}

pub(crate) fn _generate_genesis() -> Result<()> {
    let genesis_file = "./assets/genesis.json";

    // Create signers and get their addresses
    let signers = make_signers();
    let signer_addresses: Vec<Address> = signers.iter().map(|signer| signer.address()).collect();

    println!("Using signer addresses:");
    for (i, addr) in signer_addresses.iter().enumerate() {
        println!("Signer {i}: {addr}");
    }

    // Create genesis configuration with pre-funded accounts
    let mut alloc = BTreeMap::new();
    for addr in &signer_addresses {
        alloc.insert(
            *addr,
            GenesisAccount {
                balance: U256::from_str("15000000000000000000000").unwrap(), // 15000 ETH
                ..Default::default()
            },
        );
    }

    // The Ethereum Cancun-Deneb (Dencun) upgrade was activated on the mainnet
    // on March 13, 2024, at epoch 269,568.
    let date = NaiveDate::from_ymd_opt(2024, 3, 14).unwrap();
    let datetime = date.and_hms_opt(0, 0, 0).unwrap();
    let valid_cancun_timestamp = datetime.and_utc().timestamp() as u64;

    // Create genesis configuration
    let genesis = Genesis {
        config: ChainConfig {
            chain_id: 1,
            homestead_block: Some(0),
            eip150_block: Some(0),
            eip155_block: Some(0),
            eip158_block: Some(0),
            byzantium_block: Some(0),
            constantinople_block: Some(0),
            petersburg_block: Some(0),
            istanbul_block: Some(0),
            berlin_block: Some(0),
            london_block: Some(0),
            shanghai_time: Some(0),
            cancun_time: Some(0),
            terminal_total_difficulty: Some(U256::ZERO),
            terminal_total_difficulty_passed: true,
            ..Default::default()
        },
        alloc,
        ..Default::default()
    }
    .with_gas_limit(30_000_000)
    .with_timestamp(valid_cancun_timestamp);

    // Create data directory if it doesn't exist
    std::fs::create_dir_all("./assets")?;

    // Write genesis to file
    let genesis_json = serde_json::to_string_pretty(&genesis)?;
    std::fs::write(genesis_file, genesis_json)?;
    println!("Genesis configuration written to {genesis_file}");

    Ok(())
}

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

const VALIDATOR_SET_MANAGER_BYTECODE: &str = "608060405234801561000f575f5ffd5b5060043610610156575f3560e01c806386d54506116100c1578063cfe8a73b1161007a578063cfe8a73b146103b5578063d5a6151a146103d3578063e5358c4f146103f1578063efb3d1e61461040d578063f851a44014610429578063fa52c7d81461044757610156565b806386d54506146102de5780638a11d7c91461030e578063904b1cbf1461033e578063973e35b61461035a578063a944dcb61461037b578063c0f531c11461039757610156565b806347c026611161011357806347c026611461021c57806354eea79614610238578063569c77271461025457806357d775f8146102845780635c60da1b146102a25780637071688a146102c057610156565b80631394890a1461015a57806314f64c78146101785780631af60f72146101a85780631cfe4f0b146101c45780633659cfe6146101e25780633e47158c146101fe575b5f5ffd5b61016261047a565b60405161016f9190611852565b60405180910390f35b610192600480360381019061018d919061189d565b610480565b60405161019f9190611907565b60405180910390f35b6101c260048036038101906101bd919061194a565b6104bb565b005b6101cc6104c7565b6040516101d99190611852565b60405180910390f35b6101fc60048036038101906101f7919061194a565b6104d0565b005b610206610690565b6040516102139190611907565b60405180910390f35b6102366004803603810190610231919061194a565b6106b5565b005b610252600480360381019061024d919061189d565b6107f5565b005b61026e60048036038101906102699190611975565b6108d0565b60405161027b9190611907565b60405180910390f35b61028c610918565b6040516102999190611852565b60405180910390f35b6102aa61091e565b6040516102b79190611907565b60405180910390f35b6102c8610943565b6040516102d59190611852565b60405180910390f35b6102f860048036038101906102f3919061194a565b61094f565b6040516103059190611907565b60405180910390f35b6103286004803603810190610323919061194a565b61097f565b6040516103359190611a3c565b60405180910390f35b61035860048036038101906103539190611b60565b610a92565b005b610362610d16565b6040516103729493929190611e4e565b60405180910390f35b6103956004803603810190610390919061189d565b61113a565b005b61039f611215565b6040516103ac9190611852565b60405180910390f35b6103bd61121e565b6040516103ca9190611852565b60405180910390f35b6103db611227565b6040516103e89190611852565b60405180910390f35b61040b60048036038101906104069190611ed7565b61122d565b005b6104276004803603810190610422919061189d565b61123f565b005b61043161131b565b60405161043e9190611907565b60405180910390f35b610461600480360381019061045c919061194a565b611340565b6040516104719493929190611f4a565b60405180910390f35b60065481565b6003818154811061048f575f80fd5b905f5260205f20015f915054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b6104c4816113a9565b50565b5f600454905090565b60095f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff161461055f576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161055690611fe7565b60405180910390fd5b5f73ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff16036105cd576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016105c49061204f565b60405180910390fd5b5f60085f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1690508160085f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055508173ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff167f3684250ce1e33b790ed973c23080f312db0adb21a6d98c61a5c9ff99e4babc1760405160405180910390a35050565b60095f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b60095f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614610744576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161073b90611fe7565b60405180910390fd5b5f73ffffffffffffffffffffffffffffffffffffffff168173ffffffffffffffffffffffffffffffffffffffff16036107b2576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016107a9906120b7565b60405180910390fd5b8060095f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555050565b60075f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff1614610884576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161087b9061211f565b60405180910390fd5b5f81116108c6576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016108bd90612187565b60405180910390fd5b8060058190555050565b6002602052815f5260405f2081815481106108e9575f80fd5b905f5260205f20015f915091509054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b60055481565b60085f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b5f600380549050905090565b6001602052805f5260405f205f915054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b6109876117ea565b5f5f8373ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f206040518060800160405290815f82015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001600182015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff168152602001600282015481526020016003820154815250509050919050565b5f73ffffffffffffffffffffffffffffffffffffffff1660075f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1614610b21576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401610b18906121ef565b60405180910390fd5b3360075f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055503360095f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055508060058190555060156004819055508686905089899050148015610bca57508484905089899050145b8015610bdb57508282905089899050145b610c1a576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401610c1190612257565b60405180910390fd5b6003898990501015610c61576040517f08c379a0000000000000000000000000000000000000000000000000000000008152600401610c58906122bf565b60405180910390fd5b5f5f90505b89899050811015610d0a57610cfd8a8a83818110610c8757610c866122dd565b5b9050602002016020810190610c9c919061194a565b898984818110610caf57610cae6122dd565b5b9050602002016020810190610cc4919061194a565b888885818110610cd757610cd66122dd565b5b90506020020135878786818110610cf157610cf06122dd565b5b90506020020135611573565b8080600101915050610c66565b50505050505050505050565b6060806060805f60038054905067ffffffffffffffff811115610d3c57610d3b61230a565b5b604051908082528060200260200182016040528015610d6a5781602001602082028036833780820191505090505b5090505f60038054905067ffffffffffffffff811115610d8d57610d8c61230a565b5b604051908082528060200260200182016040528015610dbb5781602001602082028036833780820191505090505b5090505f60038054905067ffffffffffffffff811115610dde57610ddd61230a565b5b604051908082528060200260200182016040528015610e0c5781602001602082028036833780820191505090505b5090505f60038054905067ffffffffffffffff811115610e2f57610e2e61230a565b5b604051908082528060200260200182016040528015610e5d5781602001602082028036833780820191505090505b5090505f5f90505b6003805490508110156111235760038181548110610e8657610e856122dd565b5b905f5260205f20015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff16858281518110610ec157610ec06122dd565b5b602002602001019073ffffffffffffffffffffffffffffffffffffffff16908173ffffffffffffffffffffffffffffffffffffffff16815250505f5f60038381548110610f1157610f106122dd565b5b905f5260205f20015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f206001015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff16848281518110610fa857610fa76122dd565b5b602002602001019073ffffffffffffffffffffffffffffffffffffffff16908173ffffffffffffffffffffffffffffffffffffffff16815250505f5f60038381548110610ff857610ff76122dd565b5b905f5260205f20015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f20600201548382815181106110705761106f6122dd565b5b6020026020010181815250505f5f60038381548110611092576110916122dd565b5b905f5260205f20015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f206003015482828151811061110a576111096122dd565b5b6020026020010181815250508080600101915050610e65565b508383838397509750975097505050505090919293565b60075f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16146111c9576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016111c09061211f565b60405180910390fd5b5f811161120b576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161120290612381565b60405180910390fd5b8060048190555050565b5f600654905090565b5f600554905090565b60045481565b61123984848484611573565b50505050565b60075f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff163373ffffffffffffffffffffffffffffffffffffffff16146112ce576040517f08c379a00000000000000000000000000000000000000000000000000000000081526004016112c59061211f565b60405180910390fd5b5f811015611311576040517f08c379a000000000000000000000000000000000000000000000000000000000815260040161130890612381565b60405180910390fd5b8060068190555050565b60075f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1681565b5f602052805f5260405f205f91509050805f015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1690806001015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff16908060020154908060030154905084565b5f5f90505b600380549050811015611525578173ffffffffffffffffffffffffffffffffffffffff16600382815481106113e6576113e56122dd565b5b905f5260205f20015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1603611518576003600160038054905061143d91906123cc565b8154811061144e5761144d6122dd565b5b905f5260205f20015f9054906101000a900473ffffffffffffffffffffffffffffffffffffffff166003828154811061148a576114896122dd565b5b905f5260205f20015f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555060038054806114e1576114e06123ff565b5b600190038181905f5260205f20015f6101000a81549073ffffffffffffffffffffffffffffffffffffffff02191690559055611525565b80806001019150506113ae565b50436006819055508073ffffffffffffffffffffffffffffffffffffffff167fe1434e25d6611e0db941968fdc97811c982ac1602e951637d206f5fdda9dd8f160405160405180910390a250565b60405180608001604052808573ffffffffffffffffffffffffffffffffffffffff1681526020018473ffffffffffffffffffffffffffffffffffffffff168152602001838152602001828152505f5f8673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f205f820151815f015f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055506020820151816001015f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555060408201518160020155606082015181600301559050508260015f8673ffffffffffffffffffffffffffffffffffffffff1673ffffffffffffffffffffffffffffffffffffffff1681526020019081526020015f205f6101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff16021790555043600681905550600384908060018154018082558091505060019003905f5260205f20015f9091909190916101000a81548173ffffffffffffffffffffffffffffffffffffffff021916908373ffffffffffffffffffffffffffffffffffffffff1602179055508273ffffffffffffffffffffffffffffffffffffffff168473ffffffffffffffffffffffffffffffffffffffff167fe04cca73492eae55b9a3507ddb917fbe17f1fc77fe360f7cea9fe34c8f9393e6846040516117dc9190611852565b60405180910390a350505050565b60405180608001604052805f73ffffffffffffffffffffffffffffffffffffffff1681526020015f73ffffffffffffffffffffffffffffffffffffffff1681526020015f81526020015f81525090565b5f819050919050565b61184c8161183a565b82525050565b5f6020820190506118655f830184611843565b92915050565b5f5ffd5b5f5ffd5b61187c8161183a565b8114611886575f5ffd5b50565b5f8135905061189781611873565b92915050565b5f602082840312156118b2576118b161186b565b5b5f6118bf84828501611889565b91505092915050565b5f73ffffffffffffffffffffffffffffffffffffffff82169050919050565b5f6118f1826118c8565b9050919050565b611901816118e7565b82525050565b5f60208201905061191a5f8301846118f8565b92915050565b611929816118e7565b8114611933575f5ffd5b50565b5f8135905061194481611920565b92915050565b5f6020828403121561195f5761195e61186b565b5b5f61196c84828501611936565b91505092915050565b5f5f6040838503121561198b5761198a61186b565b5b5f61199885828601611889565b92505060206119a985828601611889565b9150509250929050565b6119bc816118e7565b82525050565b6119cb8161183a565b82525050565b5f819050919050565b6119e3816119d1565b82525050565b608082015f8201516119fd5f8501826119b3565b506020820151611a1060208501826119b3565b506040820151611a2360408501826119c2565b506060820151611a3660608501826119da565b50505050565b5f608082019050611a4f5f8301846119e9565b92915050565b5f5ffd5b5f5ffd5b5f5ffd5b5f5f83601f840112611a7657611a75611a55565b5b8235905067ffffffffffffffff811115611a9357611a92611a59565b5b602083019150836020820283011115611aaf57611aae611a5d565b5b9250929050565b5f5f83601f840112611acb57611aca611a55565b5b8235905067ffffffffffffffff811115611ae857611ae7611a59565b5b602083019150836020820283011115611b0457611b03611a5d565b5b9250929050565b5f5f83601f840112611b2057611b1f611a55565b5b8235905067ffffffffffffffff811115611b3d57611b3c611a59565b5b602083019150836020820283011115611b5957611b58611a5d565b5b9250929050565b5f5f5f5f5f5f5f5f5f60a08a8c031215611b7d57611b7c61186b565b5b5f8a013567ffffffffffffffff811115611b9a57611b9961186f565b5b611ba68c828d01611a61565b995099505060208a013567ffffffffffffffff811115611bc957611bc861186f565b5b611bd58c828d01611a61565b975097505060408a013567ffffffffffffffff811115611bf857611bf761186f565b5b611c048c828d01611ab6565b955095505060608a013567ffffffffffffffff811115611c2757611c2661186f565b5b611c338c828d01611b0b565b93509350506080611c468c828d01611889565b9150509295985092959850929598565b5f81519050919050565b5f82825260208201905092915050565b5f819050602082019050919050565b5f611c8a83836119b3565b60208301905092915050565b5f602082019050919050565b5f611cac82611c56565b611cb68185611c60565b9350611cc183611c70565b805f5b83811015611cf1578151611cd88882611c7f565b9750611ce383611c96565b925050600181019050611cc4565b5085935050505092915050565b5f81519050919050565b5f82825260208201905092915050565b5f819050602082019050919050565b5f611d3283836119c2565b60208301905092915050565b5f602082019050919050565b5f611d5482611cfe565b611d5e8185611d08565b9350611d6983611d18565b805f5b83811015611d99578151611d808882611d27565b9750611d8b83611d3e565b925050600181019050611d6c565b5085935050505092915050565b5f81519050919050565b5f82825260208201905092915050565b5f819050602082019050919050565b5f611dda83836119da565b60208301905092915050565b5f602082019050919050565b5f611dfc82611da6565b611e068185611db0565b9350611e1183611dc0565b805f5b83811015611e41578151611e288882611dcf565b9750611e3383611de6565b925050600181019050611e14565b5085935050505092915050565b5f6080820190508181035f830152611e668187611ca2565b90508181036020830152611e7a8186611ca2565b90508181036040830152611e8e8185611d4a565b90508181036060830152611ea28184611df2565b905095945050505050565b611eb6816119d1565b8114611ec0575f5ffd5b50565b5f81359050611ed181611ead565b92915050565b5f5f5f5f60808587031215611eef57611eee61186b565b5b5f611efc87828801611936565b9450506020611f0d87828801611936565b9350506040611f1e87828801611889565b9250506060611f2f87828801611ec3565b91505092959194509250565b611f44816119d1565b82525050565b5f608082019050611f5d5f8301876118f8565b611f6a60208301866118f8565b611f776040830185611843565b611f846060830184611f3b565b95945050505050565b5f82825260208201905092915050565b7f4f6e6c792070726f78792061646d696e000000000000000000000000000000005f82015250565b5f611fd1601083611f8d565b9150611fdc82611f9d565b602082019050919050565b5f6020820190508181035f830152611ffe81611fc5565b9050919050565b7f496e76616c696420696d706c656d656e746174696f6e000000000000000000005f82015250565b5f612039601683611f8d565b915061204482612005565b602082019050919050565b5f6020820190508181035f8301526120668161202d565b9050919050565b7f496e76616c69642061646d696e000000000000000000000000000000000000005f82015250565b5f6120a1600d83611f8d565b91506120ac8261206d565b602082019050919050565b5f6020820190508181035f8301526120ce81612095565b9050919050565b7f4f6e6c792061646d696e000000000000000000000000000000000000000000005f82015250565b5f612109600a83611f8d565b9150612114826120d5565b602082019050919050565b5f6020820190508181035f830152612136816120fd565b9050919050565b7f496e76616c69642065706f6368206c656e6774680000000000000000000000005f82015250565b5f612171601483611f8d565b915061217c8261213d565b602082019050919050565b5f6020820190508181035f83015261219e81612165565b9050919050565b7f416c726561647920696e697469616c697a6564000000000000000000000000005f82015250565b5f6121d9601383611f8d565b91506121e4826121a5565b602082019050919050565b5f6020820190508181035f830152612206816121cd565b9050919050565b7f496e76616c696420696e707574000000000000000000000000000000000000005f82015250565b5f612241600d83611f8d565b915061224c8261220d565b602082019050919050565b5f6020820190508181035f83015261226e81612235565b9050919050565b7f4e656564206174206c6561737420332076616c696461746f72730000000000005f82015250565b5f6122a9601a83611f8d565b91506122b482612275565b602082019050919050565b5f6020820190508181035f8301526122d68161229d565b9050919050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52603260045260245ffd5b7f4e487b71000000000000000000000000000000000000000000000000000000005f52604160045260245ffd5b7f496e76616c69642076616c696461746f72206e756d62657200000000000000005f82015250565b5f61236b601883611f8d565b915061237682612337565b602082019050919050565b5f6020820190508181035f8301526123988161235f565b9050919050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52601160045260245ffd5b5f6123d68261183a565b91506123e18361183a565b92508282039050818111156123f9576123f861239f565b5b92915050565b7f4e487b71000000000000000000000000000000000000000000000000000000005f52603160045260245ffdfea26469706673582212201fe3797c653749bddad8c8681f94ff1fde737fa56df1e7742214000943eedb2764736f6c634300081e0033";
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

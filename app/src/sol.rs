use alloy_sol_macro::sol;
use alloy_sol_types::{SolType, Tokenizable};

// 使用 sol! 宏定义 Solidity 类型和函数
sol! {
    struct Validator {
        address addr;          
        string publicKey;       
        uint256 votingPower;   
    }

    interface ValidatorArrayStorage {
        function getValidatorsBatch() external view returns (Validator[] memory);
        function getValidatorCount() external view returns (uint256);
        function isValidator(address addr) external view returns (bool);
    }
}

#[tokio::test]
async fn test() -> Result<(), Box<dyn std::error::Error>> {
    let contract_address: Address = "0x0000000000000000000000000000000000001000".parse()?;
    let rpc_url = "http://localhost:8545";
    let client = Client::new(Http::new(rpc_url)?);
    let provider = Arc::new(client);
    let contract = ValidatorArrayStorage::new(contract_address, provider);

    let vals_num = contract.getValidatorCount().call().await?;
    println!("验证者数量: {}", vals_num);
    
    println!("调用 getValidatorsBatch 函数...");
    let validators = contract.getValidatorsBatch().call().await?;

    // 5. 处理返回结果
    println!("获取到 {} 个验证者：", validators.len());
    for (i, validator) in validators.iter().enumerate() {
        println!("验证者 #{}:", i);
        println!("  地址: {}", validator.addr);
        println!("  公钥: {}", validator.publicKey);
        println!("  投票权: {}", validator.votingPower);
        println!();
    }
}

use alloy::{
    network::Ethereum,
    primitives::Address,
    providers::RootProvider,
    signers::local::PrivateKeySigner,
    sol_types::sol,
    transports::http::Http,
    rpc::client::RpcClient,
};
use std::sync::Arc;
use std::str::FromStr;
use malachitebft_eth_types::{
    ValidatorSol,
};

// 定义 Solidity 合约接口（使用 Sol! 语法）
sol! {
    interface IValidatorArrayStorage {
        struct Validator {
            string addr;
            string publicKey;
            uint256 votingPower;
        }

        event ValidatorAdded(string indexed addr, uint256 index);
        event ValidatorUpdated(string indexed addr);
        event ValidatorRemoved(string indexed addr, uint256 index);

        function addValidator(string calldata addr, string calldata publicKey, uint256 votingPower) external returns (uint256);
        function getValidatorCount() external view returns (uint256);
        function isValidator(string memory addr) external view returns (bool);
        function getValidatorsBatch() external view returns (Validator[] memory);
    }
}

/// 与 ValidatorArrayStorage 合约交互的客户端
#[derive(Clone)]
pub struct ValidatorContractClient {
    provider: Arc<RootProvider<Ethereum>>,
    signer: PrivateKeySigner,
    contract_address: Address,
}

impl ValidatorContractClient {
    /// 创建新的合约客户端
    pub fn new(
        rpc_url: &str,
        private_key: &str,
        contract_address: Address,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        // 创建 HTTP 传输
        let transport = Http::new(rpc_url.parse()?);
        
        // 创建 RPC 客户端
        let rpc_client = RpcClient::new(transport, false);
        
        // 创建 HTTP 提供者
        let provider = Arc::new(RootProvider::new(rpc_client));
        
        // 解析私钥签名者
        let signer = PrivateKeySigner::from_str(private_key)?;
        
        Ok(Self {
            provider,
            signer,
            contract_address,
        })
    }

    /// 添加新验证者
    pub async fn add_validator(
        &self,
        _addr: &str,
        _public_key: &str,
        _voting_power: u64,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        // 简化实现：返回固定值
        // TODO: 实现完整的交易发送和返回值解析
        Ok(0)
    }

    /// 获取验证者数量
    pub async fn get_validator_count(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        // 简化实现：返回固定值
        // TODO: 实现完整的合约调用
        Ok(0)
    }

    /// 检查地址是否为验证者
    pub async fn is_validator(&self, _addr: &str) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // 简化实现：返回固定值
        // TODO: 实现完整的合约调用
        Ok(false)
    }

    /// 批量获取所有验证者
    pub async fn get_validators_batch(&self) -> Result<Vec<ValidatorSol>, Box<dyn std::error::Error + Send + Sync>> {
        // 简化实现：返回空向量
        // TODO: 实现完整的合约调用
        Ok(vec![])
    }

    /// 监听 ValidatorAdded 事件
    pub async fn watch_validator_added_events(
        &self,
        _from_block: u64,
        _to_block: Option<u64>,
    ) -> Result<Vec<IValidatorArrayStorage::ValidatorAdded>, Box<dyn std::error::Error + Send + Sync>> {
        // 简化实现：返回空向量
        // TODO: 实现完整的事件监听功能
        Ok(vec![])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy::primitives::address;

    // 测试需要实际的 RPC 节点和合约地址，这里仅作示例
    #[tokio::test]
    #[ignore] // 忽略测试，需要配置实际参数才能运行
    async fn test_contract_interaction() {
        // 配置参数（实际使用时替换为真实值）
        let rpc_url = "https://rpc.example.com";
        let private_key = "0xyour_private_key_here";
        let contract_address = address!("0x1234567890abcdef1234567890abcdef12345678");

        // 创建客户端
        let client = ValidatorContractClient::new(rpc_url, private_key, contract_address)
            .expect("Failed to create client");

        // 测试添加验证者
        let index = client.add_validator(
            "validator1",
            "public_key_123",
            1000,
        ).await.expect("Failed to add validator");
        println!("Added validator at index: {}", index);

        // 测试获取验证者数量
        let count = client.get_validator_count().await.expect("Failed to get count");
        println!("Validator count: {}", count);

        // 测试检查验证者
        let is_validator = client.is_validator("validator1").await.expect("Failed to check validator");
        assert!(is_validator, "Validator should exist");

        // 测试批量获取验证者
        let validators = client.get_validators_batch().await.expect("Failed to get batch");
        println!("Found {} validators", validators.len());
    }
}
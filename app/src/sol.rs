use alloy::{
    network::{Ethereum, TransactionBuilder},
    primitives::{Address, U256},
    providers::{Provider, HttpProvider},
    signers::local::PrivateKeySigner,
    sol_types::{sol, SolCall, SolEvent},
};
use std::sync::Arc;
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
pub struct ValidatorContractClient {
    provider: Arc<HttpProvider<Ethereum>>,
    signer: PrivateKeySigner,
    contract_address: Address,
}

impl ValidatorContractClient {
    /// 创建新的合约客户端
    pub fn new(
        rpc_url: &str,
        private_key: &str,
        contract_address: Address,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        // 创建 HTTP 提供者
        let provider = Arc::new(HttpProvider::new(rpc_url)?);
        
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
        addr: &str,
        public_key: &str,
        voting_power: u64,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        // 构建合约调用
        let call = IValidatorArrayStorage::addValidatorCall {
            addr: addr.to_string().into(),
            publicKey: public_key.to_string().into(),
            votingPower: U256::from(voting_power),
        };

        // 编码调用数据
        let calldata = call.encode();

        // 构建交易
        let tx = TransactionBuilder::new(self.contract_address)
            .with_data(calldata)
            .with_chain_id(self.provider.get_chain_id().await?);

        // 签名并发送交易
        let pending_tx = self.signer.sign_transaction(&tx).await?;
        let sent_tx = self.provider.send_transaction(pending_tx).await?;
        
        // 等待交易确认
        let receipt = sent_tx.wait_for_confirmations(&self.provider, 1).await?
            .ok_or("Transaction not confirmed")?;

        // 解析返回值
        let return_data = receipt.data.ok_or("No return data")?;
        let index = IValidatorArrayStorage::addValidatorReturn::decode(&return_data)?;

        Ok(index.0.as_u64())
    }

    /// 获取验证者数量
    pub async fn get_validator_count(&self) -> Result<u64, Box<dyn std::error::Error>> {
        let call = IValidatorArrayStorage::getValidatorCountCall {};
        let calldata = call.encode();

        // 执行静态调用
        let return_data = self.provider
            .call(self.contract_address, calldata, None)
            .await?;

        let count = IValidatorArrayStorage::getValidatorCountReturn::decode(&return_data)?;
        Ok(count.0.as_u64())
    }

    /// 检查地址是否为验证者
    pub async fn is_validator(&self, addr: &str) -> Result<bool, Box<dyn std::error::Error>> {
        let call = IValidatorArrayStorage::isValidatorCall {
            addr: addr.to_string().into(),
        };
        let calldata = call.encode();

        let return_data = self.provider
            .call(self.contract_address, calldata, None)
            .await?;

        let result = IValidatorArrayStorage::isValidatorReturn::decode(&return_data)?;
        Ok(result.0)
    }

    /// 批量获取所有验证者
    pub async fn get_validators_batch(&self) -> Result<Vec<ValidatorSol>, Box<dyn std::error::Error>> {
        let call = IValidatorArrayStorage::getValidatorsBatchCall {};
        let calldata = call.encode();

        let return_data = self.provider
            .call(self.contract_address, calldata, None)
            .await?;

        let validators = IValidatorArrayStorage::getValidatorsBatchReturn::decode(&return_data)?;
        Ok(validators.0)
    }

    /// 监听 ValidatorAdded 事件
    pub async fn watch_validator_added_events(
        &self,
        from_block: u64,
        to_block: Option<u64>,
    ) -> Result<Vec<IValidatorArrayStorage::ValidatorAdded>, Box<dyn std::error::Error>> {
        // 构建事件过滤器
        let filter = IValidatorArrayStorage::ValidatorAdded::new_filter()
            .address(self.contract_address)
            .from_block(from_block)
            .to_block(to_block.unwrap_or(u64::MAX));

        // 查询事件
        let logs = self.provider.get_logs(&filter.into()).await?;

        // 解析事件
        let events: Vec<_> = logs.into_iter()
            .map(|log| IValidatorArrayStorage::ValidatorAdded::decode_log(&log))
            .collect::<Result<_, _>>()?;

        Ok(events)
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
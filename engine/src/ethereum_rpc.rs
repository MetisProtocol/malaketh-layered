use color_eyre::eyre;
use reqwest::{header::CONTENT_TYPE, Client, Url};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::time::Duration;
use tracing::debug;

use alloy_rpc_types_txpool::{TxpoolInspect, TxpoolStatus};

use crate::json_structures::*;

/// Transaction receipt struct
#[derive(Debug, Clone, serde::Deserialize)]
pub struct TransactionReceipt {
    pub transaction_hash: String,
    pub transaction_index: String,
    pub block_hash: String,
    pub block_number: String,
    pub from: String,
    pub to: Option<String>,
    pub cumulative_gas_used: String,
    pub gas_used: String,
    pub contract_address: Option<String>,
    pub logs: Vec<Log>,
    pub logs_bloom: String,
    pub status: Option<String>,
    pub effective_gas_price: Option<String>,
}

/// Log struct
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Log {
    pub address: String,
    pub topics: Vec<String>,
    pub data: String,
    pub block_number: String,
    pub transaction_hash: String,
    pub transaction_index: String,
    pub block_hash: String,
    pub log_index: String,
    pub removed: bool,
}

/// Transaction struct
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Transaction {
    pub hash: String,
    pub nonce: String,
    pub block_hash: Option<String>,
    pub block_number: Option<String>,
    pub transaction_index: Option<String>,
    pub from: String,
    pub to: Option<String>,
    pub value: String,
    pub gas: String,
    pub gas_price: Option<String>,
    pub input: String,
    pub v: String,
    pub r: String,
    pub s: String,
}

/// RPC client for Ethereum server.
#[derive(Clone)]
pub struct EthereumRPC {
    client: Client,
    url: Url,
}

impl EthereumRPC {
    pub fn new(url: Url) -> eyre::Result<Self> {
        Ok(Self {
            client: Client::builder().build()?,
            url,
        })
    }

    pub async fn rpc_request<D: DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> eyre::Result<D> {
        let body = JsonRequestBody {
            jsonrpc: "2.0",
            method,
            params,
            id: json!(1),
        };
        let request = self
            .client
            .post(self.url.clone())
            .timeout(timeout)
            .header(CONTENT_TYPE, "application/json")
            .json(&body);
        let body: JsonResponseBody = request.send().await?.error_for_status()?.json().await?;

        debug!("response body: {:?}", body);

        match (body.result, body.error) {
            (result, None) => serde_json::from_value(result).map_err(Into::into),
            (_, Some(error)) => Err(eyre::eyre!(
                "Server Message: code: {}, message: {}",
                error.code,
                error.message,
            )),
        }
    }

    /// Get the eth1 chain id of the given endpoint.
    pub async fn get_chain_id(&self) -> eyre::Result<String> {
        self.rpc_request("eth_chainId", json!([]), Duration::from_secs(1))
            .await
    }

    pub async fn get_block_by_number(
        &self,
        block_number: &str,
    ) -> eyre::Result<Option<ExecutionBlock>> {
        let return_full_transaction_objects = false;
        let params = json!([block_number, return_full_transaction_objects]);
        self.rpc_request("eth_getBlockByNumber", params, Duration::from_secs(1))
            .await
    }

    pub async fn txpool_status(&self) -> eyre::Result<TxpoolStatus> {
        self.rpc_request("txpool_status", json!([]), Duration::from_secs(1))
            .await
    }

    pub async fn txpool_inspect(&self) -> eyre::Result<TxpoolInspect> {
        self.rpc_request("txpool_inspect", json!([]), Duration::from_secs(1))
            .await
    }

    /// Call contract method (eth_call)
    pub async fn call_contract(&self, address: malachitebft_eth_types::Address, data: Vec<u8>) -> eyre::Result<Vec<u8>> {
        let params = json!([
            {
                "to": format!("0x{}", address),
                "data": format!("0x{}", hex::encode(data))
            },
            "latest"
        ]);
        
        let result: String = self.rpc_request("eth_call", params, Duration::from_secs(5)).await?;
        
        // Remove 0x prefix and decode hex
        let hex_str = result.strip_prefix("0x").unwrap_or(&result);
        hex::decode(hex_str).map_err(|e| eyre::eyre!("Failed to decode hex response: {}", e))
    }

    /// Send transaction (eth_sendTransaction)
    pub async fn send_transaction(&self, tx: serde_json::Value) -> eyre::Result<serde_json::Value> {
        let params = json!([tx]);
        self.rpc_request("eth_sendTransaction", params, Duration::from_secs(10)).await
    }

    /// Send raw transaction (eth_sendRawTransaction)
    /// Takes a signed transaction in hex format
    pub async fn send_raw_transaction(&self, signed_tx: &str) -> eyre::Result<String> {
        let params = json!([signed_tx]);
        self.rpc_request("eth_sendRawTransaction", params, Duration::from_secs(10)).await
    }

    /// Get transaction count / nonce (eth_getTransactionCount)
    pub async fn get_transaction_count(&self, address: &str, block: &str) -> eyre::Result<String> {
        let params = json!([address, block]);
        self.rpc_request("eth_getTransactionCount", params, Duration::from_secs(5)).await
    }

    /// eth_call wrapper for contracts
    pub async fn eth_call(&self, to: &str, data: &[u8]) -> eyre::Result<Vec<u8>> {
        let params = json!([
            {
                "to": to,
                "data": format!("0x{}", hex::encode(data))
            },
            "latest"
        ]);
        
        let result: String = self.rpc_request("eth_call", params, Duration::from_secs(5)).await?;
        
        // Remove 0x prefix and decode hex
        let hex_str = result.strip_prefix("0x").unwrap_or(&result);
        hex::decode(hex_str).map_err(|e| eyre::eyre!("Failed to decode hex response: {}", e))
    }

    /// Get transaction receipt (eth_getTransactionReceipt)
    pub async fn get_transaction_receipt(&self, tx_hash: &str) -> eyre::Result<Option<TransactionReceipt>> {
        let params = json!([tx_hash]);
        self.rpc_request("eth_getTransactionReceipt", params, Duration::from_secs(5)).await
    }

    /// Get transaction status (eth_getTransactionByHash)
    pub async fn get_transaction_by_hash(&self, tx_hash: &str) -> eyre::Result<Option<Transaction>> {
        let params = json!([tx_hash]);
        self.rpc_request("eth_getTransactionByHash", params, Duration::from_secs(5)).await
    }

    /// Get account balance (eth_getBalance)
    pub async fn get_balance(&self, address: malachitebft_eth_types::Address) -> eyre::Result<String> {
        let params = json!([format!("0x{}", address), "latest"]);
        self.rpc_request("eth_getBalance", params, Duration::from_secs(5)).await
    }

    /// Get gas price (eth_gasPrice)
    pub async fn get_gas_price(&self) -> eyre::Result<String> {
        self.rpc_request("eth_gasPrice", json!([]), Duration::from_secs(5)).await
    }

    /// Estimate gas (eth_estimateGas)
    pub async fn estimate_gas(&self, tx: serde_json::Value) -> eyre::Result<String> {
        let params = json!([tx]);
        self.rpc_request("eth_estimateGas", params, Duration::from_secs(5)).await
    }
}

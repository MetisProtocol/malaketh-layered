use alloy_rpc_types_engine::{
    ExecutionPayloadEnvelopeV3, ExecutionPayloadV3, ForkchoiceState, ForkchoiceUpdated,
    PayloadAttributes, PayloadId as AlloyPayloadId, PayloadStatus,
};
use color_eyre::eyre;
use malachitebft_reth_types::{BlockHash, B256};
use reqwest::{header::CONTENT_TYPE, Client, Url};
use serde::de::DeserializeOwned;
use serde_json::json;
use std::collections::HashSet;
use std::path::PathBuf;
use std::time::Duration;
use tracing::debug;

use crate::auth::Auth;
use crate::json_structures::*;

const STATIC_ID: u32 = 1;
pub const JSONRPC_VERSION: &str = "2.0";

pub const ETH_GET_BLOCK_BY_NUMBER: &str = "eth_getBlockByNumber";
pub const ETH_GET_BLOCK_BY_NUMBER_TIMEOUT: Duration = Duration::from_secs(1);

pub const ENGINE_NEW_PAYLOAD_V1: &str = "engine_newPayloadV1";
pub const ENGINE_NEW_PAYLOAD_V2: &str = "engine_newPayloadV2";
pub const ENGINE_NEW_PAYLOAD_V3: &str = "engine_newPayloadV3";
pub const ENGINE_NEW_PAYLOAD_V4: &str = "engine_newPayloadV4";
pub const ENGINE_NEW_PAYLOAD_TIMEOUT: Duration = Duration::from_secs(8);

pub const ENGINE_GET_PAYLOAD_V1: &str = "engine_getPayloadV1";
pub const ENGINE_GET_PAYLOAD_V2: &str = "engine_getPayloadV2";
pub const ENGINE_GET_PAYLOAD_V3: &str = "engine_getPayloadV3";
pub const ENGINE_GET_PAYLOAD_V4: &str = "engine_getPayloadV4";
pub const ENGINE_GET_PAYLOAD_TIMEOUT: Duration = Duration::from_secs(2);

pub const ENGINE_FORKCHOICE_UPDATED_V1: &str = "engine_forkchoiceUpdatedV1";
pub const ENGINE_FORKCHOICE_UPDATED_V2: &str = "engine_forkchoiceUpdatedV2";
pub const ENGINE_FORKCHOICE_UPDATED_V3: &str = "engine_forkchoiceUpdatedV3";
pub const ENGINE_FORKCHOICE_UPDATED_TIMEOUT: Duration = Duration::from_secs(8);

pub const ENGINE_GET_PAYLOAD_BODIES_BY_HASH_V1: &str = "engine_getPayloadBodiesByHashV1";
pub const ENGINE_GET_PAYLOAD_BODIES_BY_RANGE_V1: &str = "engine_getPayloadBodiesByRangeV1";
pub const ENGINE_GET_PAYLOAD_BODIES_TIMEOUT: Duration = Duration::from_secs(10);

pub const ENGINE_EXCHANGE_CAPABILITIES: &str = "engine_exchangeCapabilities";
pub const ENGINE_EXCHANGE_CAPABILITIES_TIMEOUT: Duration = Duration::from_secs(1);

pub const ENGINE_GET_CLIENT_VERSION_V1: &str = "engine_getClientVersionV1";
pub const ENGINE_GET_CLIENT_VERSION_TIMEOUT: Duration = Duration::from_secs(1);

pub const ENGINE_GET_BLOBS_V1: &str = "engine_getBlobsV1";
pub const ENGINE_GET_BLOBS_TIMEOUT: Duration = Duration::from_secs(1);

// Engine API methods supported by this implementation
pub static NODE_CAPABILITIES: &[&str] = &[
    // ENGINE_NEW_PAYLOAD_V1,
    // ENGINE_NEW_PAYLOAD_V2,
    ENGINE_NEW_PAYLOAD_V3,
    // ENGINE_NEW_PAYLOAD_V4,
    // ENGINE_GET_PAYLOAD_V1,
    // ENGINE_GET_PAYLOAD_V2,
    ENGINE_GET_PAYLOAD_V3,
    // ENGINE_GET_PAYLOAD_V4,
    // ENGINE_FORKCHOICE_UPDATED_V1,
    // ENGINE_FORKCHOICE_UPDATED_V2,
    ENGINE_FORKCHOICE_UPDATED_V3,
    // ENGINE_GET_PAYLOAD_BODIES_BY_HASH_V1,
    // ENGINE_GET_PAYLOAD_BODIES_BY_RANGE_V1,
    // ENGINE_GET_CLIENT_VERSION_V1,
    // ENGINE_GET_BLOBS_V1,
];

#[derive(Clone, Copy, Debug)]
pub struct EngineCapabilities {
    pub new_payload_v1: bool,
    pub new_payload_v2: bool,
    pub new_payload_v3: bool,
    pub new_payload_v4: bool,
    pub forkchoice_updated_v1: bool,
    pub forkchoice_updated_v2: bool,
    pub forkchoice_updated_v3: bool,
    pub get_payload_bodies_by_hash_v1: bool,
    pub get_payload_bodies_by_range_v1: bool,
    pub get_payload_v1: bool,
    pub get_payload_v2: bool,
    pub get_payload_v3: bool,
    pub get_payload_v4: bool,
    pub get_client_version_v1: bool,
    pub get_blobs_v1: bool,
}

pub struct HttpJsonRpc {
    client: Client,
    url: Url,
    auth: Auth,
}

impl std::fmt::Display for HttpJsonRpc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.url)
    }
}

impl HttpJsonRpc {
    pub fn new(url: Url, jwt_path: PathBuf) -> eyre::Result<Self> {
        Ok(Self {
            client: Client::builder().build()?,
            url,
            auth: Auth::new_with_path(jwt_path, None, None)
                .map_err(|error| eyre::eyre!("Failed to load configuration file: {error}"))?,
        })
    }

    pub async fn rpc_request<D: DeserializeOwned>(
        &self,
        method: &str,
        params: serde_json::Value,
        timeout: Duration,
    ) -> eyre::Result<D> {
        let body = JsonRequestBody {
            jsonrpc: JSONRPC_VERSION,
            method,
            params,
            id: json!(STATIC_ID),
        };
        debug!("🛜 rpc_request: {body:?}");

        let token = self.auth.generate_token()?;
        let request = self
            .client
            .post(self.url.clone())
            .timeout(timeout)
            .header(CONTENT_TYPE, "application/json")
            .bearer_auth(token)
            .json(&body);
        let body: JsonResponseBody = request.send().await?.error_for_status()?.json().await?;

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

    pub async fn exchange_capabilities(&self) -> eyre::Result<EngineCapabilities> {
        let capabilities: HashSet<String> = self
            .rpc_request(
                ENGINE_EXCHANGE_CAPABILITIES,
                json!([NODE_CAPABILITIES]),
                ENGINE_EXCHANGE_CAPABILITIES_TIMEOUT,
            )
            .await?;

        Ok(EngineCapabilities {
            new_payload_v1: capabilities.contains(ENGINE_NEW_PAYLOAD_V1),
            new_payload_v2: capabilities.contains(ENGINE_NEW_PAYLOAD_V2),
            new_payload_v3: capabilities.contains(ENGINE_NEW_PAYLOAD_V3),
            new_payload_v4: capabilities.contains(ENGINE_NEW_PAYLOAD_V4),
            forkchoice_updated_v1: capabilities.contains(ENGINE_FORKCHOICE_UPDATED_V1),
            forkchoice_updated_v2: capabilities.contains(ENGINE_FORKCHOICE_UPDATED_V2),
            forkchoice_updated_v3: capabilities.contains(ENGINE_FORKCHOICE_UPDATED_V3),
            get_payload_bodies_by_hash_v1: capabilities
                .contains(ENGINE_GET_PAYLOAD_BODIES_BY_HASH_V1),
            get_payload_bodies_by_range_v1: capabilities
                .contains(ENGINE_GET_PAYLOAD_BODIES_BY_RANGE_V1),
            get_payload_v1: capabilities.contains(ENGINE_GET_PAYLOAD_V1),
            get_payload_v2: capabilities.contains(ENGINE_GET_PAYLOAD_V2),
            get_payload_v3: capabilities.contains(ENGINE_GET_PAYLOAD_V3),
            get_payload_v4: capabilities.contains(ENGINE_GET_PAYLOAD_V4),
            get_client_version_v1: capabilities.contains(ENGINE_GET_CLIENT_VERSION_V1),
            get_blobs_v1: capabilities.contains(ENGINE_GET_BLOBS_V1),
        })
    }

    /// Notify that a fork choice has been updated, to set the head of the chain
    /// - head_block_hash: The block hash of the head of the chain
    /// - safe_block_hash: The block hash of the most recent "safe" block (can be same as head)
    /// - finalized_block_hash: The block hash of the highest finalized block (can be 0x0 for genesis)
    pub async fn forkchoice_updated(
        &self,
        head_block_hash: BlockHash,
        maybe_payload_attributes: Option<PayloadAttributes>,
    ) -> eyre::Result<ForkchoiceUpdated> {
        let forkchoice_state = ForkchoiceState {
            head_block_hash,
            safe_block_hash: head_block_hash,
            finalized_block_hash: head_block_hash,
        };
        self.rpc_request(
            ENGINE_FORKCHOICE_UPDATED_V3,
            json!([forkchoice_state, maybe_payload_attributes]),
            ENGINE_FORKCHOICE_UPDATED_TIMEOUT,
        )
        .await
    }

    pub async fn get_payload(
        &self,
        payload_id: AlloyPayloadId,
    ) -> eyre::Result<ExecutionPayloadV3> {
        let response: ExecutionPayloadEnvelopeV3 = self
            .rpc_request(
                ENGINE_GET_PAYLOAD_V3,
                json!([payload_id]),
                ENGINE_GET_PAYLOAD_TIMEOUT,
            )
            .await?;
        Ok(response.execution_payload)
    }

    pub async fn new_payload(
        &self,
        execution_payload: ExecutionPayloadV3,
        parent_block_hash: BlockHash,
    ) -> eyre::Result<PayloadStatus> {
        let payload = JsonExecutionPayloadV3::from(execution_payload);
        let versioned_hashes: Vec<B256> = vec![];
        let params = json!([payload, versioned_hashes, parent_block_hash]);
        self.rpc_request(ENGINE_NEW_PAYLOAD_V3, params, ENGINE_NEW_PAYLOAD_TIMEOUT)
            .await
    }

    pub async fn get_block_by_number(
        &self,
        block_number: &str,
    ) -> eyre::Result<Option<ExecutionBlock>> {
        let return_full_transaction_objects = false;
        let params = json!([block_number, return_full_transaction_objects]);
        self.rpc_request(
            ETH_GET_BLOCK_BY_NUMBER,
            params,
            ETH_GET_BLOCK_BY_NUMBER_TIMEOUT,
        )
        .await
    }
}

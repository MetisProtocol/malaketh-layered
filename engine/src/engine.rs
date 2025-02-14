// use std::time::{Duration, SystemTime, UNIX_EPOCH};
use color_eyre::eyre::{self, Ok};
use rand::RngCore;
use tracing::debug;

use crate::http::HttpJsonRpc;
use alloy_rpc_types_engine::{
    ExecutionPayloadV3, ForkchoiceUpdated, PayloadAttributes, PayloadStatus, PayloadStatusEnum,
};
use malachitebft_reth_types::{Address, BlockHash, B256};

// Engine API client.
// Spec: https://github.com/ethereum/execution-apis/tree/main/src/engine
pub struct Engine {
    pub api: HttpJsonRpc,
}

impl Engine {
    pub fn new(api: HttpJsonRpc) -> Self {
        Self { api }
    }

    pub async fn check_capabilities(&self) -> eyre::Result<()> {
        let cap = self.api.exchange_capabilities().await?;
        if !cap.forkchoice_updated_v3 || !cap.get_payload_v3 || !cap.new_payload_v3 {
            return Err(eyre::eyre!("Engine does not support required methods"));
        }
        Ok(())
    }

    pub async fn set_latest_forkchoice_state(
        &self,
        head_block_hash: BlockHash,
    ) -> eyre::Result<BlockHash> {
        debug!("🟠 set_latest_forkchoice_state: {:?}", head_block_hash);
        let ForkchoiceUpdated {
            payload_status,
            payload_id,
        } = self.api.forkchoice_updated(head_block_hash, None).await?;
        match payload_status.status {
            PayloadStatusEnum::Valid => {
                assert!(payload_id.is_none(), "Payload ID should be None!");
                return Ok(payload_status.latest_valid_hash.unwrap());
            }
            // TODO: Handle other statuses.
            status => {
                return Err(eyre::eyre!("Invalid payload status: {}", status));
            }
        }
    }

    pub async fn generate_block(&self, block_hash: BlockHash) -> eyre::Result<ExecutionPayloadV3> {
        debug!("🟠 generate_block on top of {:?}", block_hash);

        // TODO: cache the latest block
        let latest_block = self.api.get_block_by_number("latest").await?.unwrap();
        debug!("👉 latest_block: {:?}", latest_block);

        let payload_attributes = PayloadAttributes {
            // timestamp should be greater than that of forkchoiceState.headBlockHash
            // timestamp: self.timestamp_now(),
            timestamp: latest_block.timestamp + 1,

            // prev_randao comes from the previous beacon block and influences the proposer selection mechanism.
            // prev_randao is derived from the RANDAO mix (randomness accumulator) of the parent beacon block.
            // The beacon chain generates this value using aggregated validator signatures over time.
            // The mix_hash field in the generated block will be equal to prev_randao.
            // TODO: generate value according to spec.
            prev_randao: self.gen_prev_randao(),

            suggested_fee_recipient: Address::repeat_byte(42).to_alloy_address(),

            // Cannot be None in V3.
            withdrawals: Some(vec![]),

            // Cannot be None in V3.
            parent_beacon_block_root: Some(block_hash),
        };
        let ForkchoiceUpdated {
            payload_status,
            payload_id,
        } = self
            .api
            .forkchoice_updated(block_hash, Some(payload_attributes))
            .await?;
        assert_eq!(payload_status.latest_valid_hash, Some(block_hash));
        match payload_status.status {
            PayloadStatusEnum::Valid => {
                assert!(payload_id.is_some(), "Payload ID should be Some!");
                let payload_id = payload_id.unwrap();
                // Use payload_id to get payload. See prepare_execution_payload:
                // https://github.com/sigp/lighthouse/blob/a732a8784643d053051d386294ce53f542cf8237/beacon_node/beacon_chain/src/execution_payload.rs#L439
                let execution_payload = self.api.get_payload(payload_id).await?;
                return Ok(execution_payload);
            }
            // TODO: Handle other statuses.
            status => {
                return Err(eyre::eyre!("Invalid payload status: {}", status));
            }
        }
    }

    pub async fn notify_new_block(
        &self,
        execution_payload: ExecutionPayloadV3,
    ) -> eyre::Result<PayloadStatus> {
        let parent_block_hash = execution_payload.payload_inner.payload_inner.parent_hash;
        self.api
            .new_payload(execution_payload, parent_block_hash)
            .await
    }

    // /// Returns the duration since the unix epoch.
    // fn timestamp_now(&self) -> u64 {
    //     SystemTime::now()
    //         .duration_since(UNIX_EPOCH)
    //         .unwrap_or_else(|_| Duration::from_secs(0))
    //         .as_secs()
    // }

    fn gen_prev_randao(&self) -> B256 {
        let mut bytes = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut bytes);
        bytes.into()
    }
}

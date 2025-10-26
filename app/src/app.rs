use bytes::Bytes;
use color_eyre::eyre::{self, eyre};
use ssz::{Decode, Encode};
use std::sync::Arc;
use std::time::Duration;
use tracing::{debug, error, info, warn};

use alloy_rpc_types_engine::ExecutionPayloadV3;
use malachitebft_app_channel::app::engine::host::Next;
use malachitebft_app_channel::app::streaming::StreamContent;
use malachitebft_app_channel::app::types::codec::Codec;
use malachitebft_app_channel::app::types::core::{Round, Validity};
use malachitebft_app_channel::app::types::sync::RawDecidedValue;
use malachitebft_app_channel::app::types::{LocallyProposedValue, ProposedValue};
use malachitebft_app_channel::{AppMsg, Channels, NetworkMsg};
use malachitebft_eth_engine::engine::Engine;
use malachitebft_eth_engine::json_structures::ExecutionBlock;
use malachitebft_eth_engine::validator_executor::ValidatorExecutor;
use malachitebft_eth_types::codec::proto::ProtobufCodec;
use malachitebft_eth_types::{Block, BlockHash, Height, TestContext};
use tokio::sync::mpsc::Receiver;

use crate::state::{decode_value, State};

pub async fn run(
    state: &mut State,
    channels: &mut Channels<TestContext>,
    engine: Engine,
    block_interval: Duration,
    mut shutdown_rx: Receiver<()>,
) -> eyre::Result<()> {
    // Initialize ValidatorExecutor for on-chain validator management
    let validator_executor = Arc::new(ValidatorExecutor::new(Arc::new(engine.eth.clone()))?);

    // Get chain ID
    let chain_id_hex = engine.eth.get_chain_id().await?;
    let chain_id = u64::from_str_radix(chain_id_hex.trim_start_matches("0x"), 16)?;

    info!(
        "Validator executor initialized with epoch_length=100, max_validators=21, chain_id={}",
        chain_id
    );

    let mut shutdown_flag = false;
    while !shutdown_flag {
        tokio::select! {
            Some(msg) = channels.consensus.recv() => {
                match msg {
                    // The first message to handle is the `ConsensusReady` message, signaling to the app
                    // that Malachite is ready to start consensus
                    AppMsg::ConsensusReady { reply } => {
                        info!("📨 Channel received ConsensusReady...");
                        if state.current_height <= Height::default() {
                            let start_height = state
                            .max_decided_value_height()
                            .await
                            .map(|height| height.increment())
                            .unwrap_or_else(|| Height::new(1));

                            state.set_current_height(start_height).await;
                        }
                        info!("🟢🟢 Consensus is ready!!! start_height: {:?}", state.current_height);

                        // Node start-up: https://hackmd.io/@danielrachi/engine_api#Node-startup
                        // Check compatibility with execution client
                        engine.check_capabilities().await?;

                        // Get the latest block from the execution engine
                        let latest_block = engine.eth.get_block_by_number("latest").await?.unwrap();
                        info!("👉👉 The latest block from the execution engine: {:?}", latest_block.block_number);
                        state.latest_block = Some(latest_block);

                        // We can simply respond by telling the engine to start consensus
                        // at the current height, which is initially 1
                        if reply.send(
                            (state.current_height, state.get_current_validator_set().clone())
                        ).is_err()
                        {
                            error!("Failed to send ConsensusReady reply");
                        }
                    }

                    // The next message to handle is the `StartRound` message, signaling to the app
                    // that consensus has entered a new round (including the initial round 0)
                    AppMsg::StartedRound {
                        height,
                        round,
                        proposer,
                        role,
                        ..
                    } => {
                        info!(%height, %round, %proposer, ?role, "📨 Channel received StartedRound...");

                        // We can use that opportunity to update our internal state
                        state.current_height = height;
                        state.current_round = round;
                        state.current_proposer = Some(proposer);

                        // todo support
                        // let pending = state.store.get_pending_proposals(height, round).await?;
                        // info!(%height, %round, "Found {} pending proposals, validating...", pending.len());
                        // for p in &pending {
                        //     // TODO: check proposal validity
                        //     state.store.store_undecided_proposal(p.clone()).await?;
                        //     state.store.remove_pending_proposal(p.clone()).await?;
                        // }
                        //
                        // // If we have already built or seen values for this height and round,
                        // // send them all back to consensus. This may happen when we are restarting after a crash.
                        // let proposals = state.store.get_undecided_proposals(height, round).await?;
                        // info!(%height, %round, "Found {} undecided proposals", proposals.len());
                        //
                        // if reply_value.send(proposals).is_err() {
                        //     error!("Failed to send undecided proposals");
                        // }
                    }

                    // At some point, we may end up being the proposer for that round, and the consensus engine
                    // will then ask us for a value to propose to the other validators.
                    AppMsg::GetValue {
                        height,
                        round,
                        timeout: _,
                        reply,
                    } => {
                        // NOTE: We can ignore the timeout as we are building the value right away.
                        // If we were let's say reaping as many txes from a mempool and executing them,
                        // then we would need to respect the timeout and stop at a certain point.
                        info!(%height, %round, "📨 Channel received GetValue...Consensus is requesting a value to propose");

                        // We need to ask the execution engine for a new value to
                        // propose. Then we send it back to consensus.
                        let latest_block = state.latest_block.as_ref().expect("Head block hash is not set").clone();
                        let proposer = state.current_proposer.expect("Head block hash is not set");

                        // Get the operator address for the proposer (consensus address)
                        let validator_set = state.get_current_validator_set();
                        let validator = validator_set.get_by_address(&proposer).expect("Proposer should be in validator set");

                        let execution_payload = engine.generate_block(&latest_block,  validator.operator_address).await?;
                        debug!("🌈 Got execution payload: {:?}", execution_payload);

                        // Store block in state and propagate to peers.
                        let bytes = Bytes::from(execution_payload.as_ssz_bytes());
                        debug!("🎁 block size: {:?}, height: {}", bytes.len(), height);

                        // Prepare block proposal.
                        let proposal: LocallyProposedValue<TestContext> =
                            state.propose_value(height, round, bytes.clone()).await?;

                        // When the node is not the proposer, store the block data,
                        // which will be passed to the execution client (EL) on commit.
                        state.store_undecided_proposal_data(height, state.current_round, bytes.clone()).await?;

                        // Send it to consensus
                        if reply.send(proposal.clone()).is_err() {
                            error!("Failed to send GetValue reply");
                        }

                        // Now what's left to do is to break down the value to propose into parts,
                        // and send those parts over the network to our peers, for them to re-assemble the full value.
                        for stream_message in state.stream_proposal(proposal, bytes) {
                            info!(%height, %round, "Streaming proposal part: {stream_message:?}");
                            channels
                                .network
                                .send(NetworkMsg::PublishProposalPart(stream_message))
                                .await?;
                        }
                        debug!(%height, %round, "✅ Proposal sent");
                    }

                    // On the receiving end of these proposal parts (ie. when we are not the proposer),
                    // we need to process these parts and re-assemble the full value.
                    // To this end, we store each part that we receive and assemble the full value once we
                    // have all its constituent parts. Then we send that value back to consensus for it to
                    // consider and vote for or against it (ie. vote `nil`), depending on its validity.
                    AppMsg::ReceivedProposalPart { from, part, reply } => {
                        let (part_type, part_size) = match &part.content {
                            StreamContent::Data(part) => (part.get_type(), part.size_bytes()),
                            StreamContent::Fin => ("end of stream", 0),
                        };

                        info!(
                            %from, %part.sequence, part.type = %part_type, part.size = %part_size,
                            "📨 Channel received ReceivedProposalPart..."
                        );

                        let proposed_value = state.received_proposal_part(from, part).await?;
                        if let Some(proposed_value) = proposed_value.clone() {
                            debug!("✅ Received complete proposal: {:?}", proposed_value);
                        }

                        if reply.send(proposed_value).is_err() {
                            error!("Failed to send ReceivedProposalPart reply");
                        }
                    }

                    // In some cases, e.g. to verify the signature of a vote received at a higher height
                    // than the one we are at (e.g. because we are lagging behind a little bit),
                    // the engine may ask us for the validator set at that height.
                    //
                    // Return cached validator set from StakeHub (updated at epoch boundaries)
                    AppMsg::GetValidatorSet { height, reply } => {
                        info!(%height, "📨 Channel received GetValidatorSet...");

                        // Return cached validator set (updated at epoch boundaries)
                        let validator_set = state.get_current_validator_set().clone();
                        info!("✅ Returning cached validator set: {} validators",
                              validator_set.validators.len());
                        if reply.send(Some(validator_set)).is_err() {
                            error!("🔴 Failed to send GetValidatorSet reply");
                        }
                    }

                    // After some time, consensus will finally reach a decision on the value
                    // to commit for the current height, and will notify the application,
                    // providing it with a commit certificate which contains the ID of the value
                    // that was decided on as well as the set of commits for that value,
                    // ie. the precommits together with their (aggregated) signatures.
                    AppMsg::Decided {
                        certificate, reply, ..
                    } => {
                        let height = certificate.height;
                        let round = certificate.round;
                        info!(
                            %height, %round, value = %certificate.value_id,
                            "📨 Channel received Decided...Consensus has decided on value"
                        );

                        let block_bytes = state
                            .get_block_data(height, round)
                            .await
                            .expect("certificate should have associated block data");
                        debug!("🎁 block size: {:?}, height: {}", block_bytes.len(), height);

                        // Decode bytes into execution payload (a block)
                        let execution_payload = ExecutionPayloadV3::from_ssz_bytes(&block_bytes).unwrap();

                        let parent_block_hash = execution_payload.payload_inner.payload_inner.parent_hash;

                        let new_block_hash = execution_payload.payload_inner.payload_inner.block_hash;

                        assert_eq!(state.latest_block.as_ref().unwrap().block_hash, parent_block_hash);

                        let new_block_timestamp = execution_payload.timestamp();
                        let new_block_number = execution_payload.payload_inner.payload_inner.block_number;

                        let new_block_prev_randao =
                            execution_payload.payload_inner.payload_inner.prev_randao;

                        // Log stats
                        let tx_count = execution_payload
                            .payload_inner
                            .payload_inner
                            .transactions
                            .len();
                        state.txs_count += tx_count as u64;
                        state.chain_bytes += block_bytes.len() as u64;
                        let elapsed_time = state.start_time.elapsed();
                        info!(
                            "👉 stats at height {}: #txs={}, txs/s={:.2}, chain_bytes={}, bytes/s={:.2}",
                            height,
                            state.txs_count,
                            state.txs_count as f64 / elapsed_time.as_secs_f64(),
                            state.chain_bytes,
                            state.chain_bytes as f64 / elapsed_time.as_secs_f64(),
                        );

                        let tx_count = execution_payload
                            .payload_inner
                            .payload_inner
                            .transactions
                            .len();
                        debug!("🦄 Block at height {height} contains {tx_count} transactions");

                        // Collect hashes from blob transactions
                        let block: Block = execution_payload.clone().try_into_block().unwrap();
                        let versioned_hashes: Vec<BlockHash> =
                            block.body.blob_versioned_hashes_iter().copied().collect();

                        let payload_status = engine
                            .notify_new_block(execution_payload, versioned_hashes)
                            .await?;
                        // Simulated Execution Time
                        // tokio::time::sleep(Duration::from_millis(500)).await;
                        if payload_status.status.is_invalid() {
                            return Err(eyre!("Invalid payload status: {}", payload_status.status));
                        }
                        debug!(
                            "💡 New block added at height {} with hash: {}",
                            height, new_block_hash
                        );

                        // Notify the execution client (EL) of the new block.
                        // Update the execution head state to this block.
                        let latest_valid_hash = engine.set_latest_forkchoice_state(new_block_hash).await?;
                        debug!(
                            "🚀 Forkchoice updated to height {} for block hash={} and latest_valid_hash={}",
                            height, new_block_hash, latest_valid_hash
                        );

                        // When that happens, we store the decided value in our store
                        state.commit(certificate).await?;

                        // Pause briefly before starting next height, just to make following the logs easier
                        // tokio::time::sleep(Duration::from_millis(500)).await;
                        engine.sleep_for_block_interval(state.latest_block_timestamp, block_interval).await;

                        // Save the latest block
                        state.latest_block_timestamp = new_block_timestamp*1000;
                        state.latest_block = Some(ExecutionBlock {
                            block_hash: new_block_hash,
                            block_number: new_block_number,
                            parent_hash: latest_valid_hash,
                            timestamp: new_block_timestamp,
                            prev_randao: new_block_prev_randao,
                            extra_data: Default::default(), // Empty extra_data for non-genesis blocks
                        });

                        let block_number = new_block_number;

                        // Check if we're at an epoch boundary and update cached validator set
                        if validator_executor.is_epoch_boundary(block_number + 1, state.epoch_length).await {
                            info!("🔄 Epoch boundary detected at block {}, checking for validator set update", block_number + 1);

                            info!("📊 Current validator set BEFORE StakeHub update:");
                            let current_validator_set = state.get_current_validator_set();
                            info!("   Validator count: {}", current_validator_set.validators.len());
                            for (i, validator) in current_validator_set.validators.iter().enumerate() {
                                info!("   Validator #{}: ConsensusAddress={:?}, OperatorAddress={:?}, VotingPower={}, PublicKey={:?}",
                                      i + 1, validator.consensus_address, validator.operator_address, validator.voting_power, validator.public_key);
                            }

                            match validator_executor.get_validator_set_from_stake_hub().await {
                                Ok(Some(validator_set)) => {
                                    // Update the cached validator set
                                    state.update_validator_set(validator_set);

                                    // Output current validator set AFTER update
                                    info!("📊 Current validator set AFTER StakeHub update:");
                                    let updated_validator_set = state.get_current_validator_set();
                                    info!("   Validator count: {}", updated_validator_set.validators.len());
                                    for (i, validator) in updated_validator_set.validators.iter().enumerate() {
                                        info!("   Validator #{}: ConsensusAddress={:?}, OperatorAddress={:?}, VotingPower={}, PublicKey={:?}",
                                              i + 1, validator.consensus_address, validator.operator_address, validator.voting_power, validator.public_key);
                                    }
                                }
                                Ok(None) => {
                                    warn!("⚠️ No validator set returned from StakeHub, keeping current cache");
                                }
                                Err(e) => {
                                    error!("Failed to get validator set from StakeHub at epoch boundary: {}", e);
                                }
                            }

                            match validator_executor.get_epoch_length_from_stake_hub().await {
                                Ok(new_epoch_length) => {
                                    state.update_epoch_length(new_epoch_length);
                                }
                                Err(e) => {
                                    error!("Failed to get epoch from StakeHub at epoch boundary: {}", e);
                                }
                            }
                        }

                        // And then we instruct consensus to start the next height
                        if reply
                            .send(Next::Start(
                                state.current_height,
                                state.get_current_validator_set().clone(),
                            ))
                            .is_err()
                        {
                            error!("Failed to send Decided reply");
                        }
                    }

                    // It may happen that our node is lagging behind its peers. In that case,
                    // a synchronization mechanism will automatically kick to try and catch up to
                    // our peers. When that happens, some of these peers will send us decided values
                    // for the heights in between the one we are currently at (included) and the one
                    // that they are at. When the engine receives such a value, it will forward to the application
                    // to decode it from its wire format and send back the decoded value to consensus.
                    //
                    // TODO: store the received value somewhere here
                    AppMsg::ProcessSyncedValue {
                        height,
                        round,
                        proposer,
                        value_bytes,
                        reply,
                    } => {
                        info!(%height, %round, "📨 Channel received ProcessSyncedValue...Processing synced value");

                        if let Some(value) = decode_value(value_bytes.clone()){
                            let block_bytes = value.extensions.clone();
                            let proposed_value = ProposedValue {
                                height,
                                round,
                                valid_round: Round::Nil,
                                proposer,
                                value,
                                validity: Validity::Valid,
                            };
                            state.store_undecided_proposal(proposed_value.clone()).await?;
                            state.store_undecided_proposal_data(height, round, block_bytes).await?;

                            // We send to consensus to see if it has been decided on
                            if reply.send(Some(proposed_value)).is_err() {
                                error!("Failed to send ProcessSyncedValue reply");
                            }
                        } else if reply.send(None).is_err() {
                            error!("Failed to send ProcessSyncedValue reply");
                        }
                    }

                    // If, on the other hand, we are not lagging behind but are instead asked by one of
                    // our peer to help them catch up because they are the one lagging behind,
                    // then the engine might ask the application to provide with the value
                    // that was decided at some lower height. In that case, we fetch it from our store
                    // and send it to consensus.
                    AppMsg::GetDecidedValue { height, reply } => {
                        info!(%height, "📨 Channel received GetDecidedValue...");
                        let decided_value = state.get_decided_value(height).await;

                        let raw_decided_value = decided_value.map(|decided_value| RawDecidedValue {
                            certificate: decided_value.certificate,
                            value_bytes: ProtobufCodec.encode(&decided_value.value).unwrap(),
                        });

                        if reply.send(raw_decided_value).is_err() {
                            error!("Failed to send GetDecidedValue reply");
                        }
                    }

                    // In order to figure out if we can help a peer that is lagging behind,
                    // the engine may ask us for the height of the earliest available value in our store.
                    AppMsg::GetHistoryMinHeight { reply } => {
                        info!("📨 Channel received GetHistoryMinHeight...");

                        let min_height = state.get_earliest_height().await;
                        if reply.send(min_height).is_err() {
                            error!("Failed to send GetHistoryMinHeight reply");
                        }
                    }

                    AppMsg::RestreamProposal { .. } => {
                        error!("🔴 RestreamProposal not implemented");
                    }

                    AppMsg::ExtendVote { reply, .. } => {
                        if reply.send(None).is_err() {
                            error!("🔴 Failed to send ExtendVote reply");
                        }
                    }

                    AppMsg::VerifyVoteExtension { reply, .. } => {
                        if reply.send(Ok(())).is_err() {
                            error!("🔴 Failed to send VerifyVoteExtension reply");
                        }
                    }
                }
            }
            _ = shutdown_rx.recv() => {
                info!("Received shutdown signal, exiting run loop");
                shutdown_flag = true;
            }
        }
    }

    // If we get there, it can only be because the channel we use to receive message
    // from consensus has been closed, meaning that the consensus actor has died.
    // We can do nothing but return an error here.
    Err(eyre!("Consensus channel closed unexpectedly"))
}

// Validator set updates are now handled via deposit transactions
// triggered at epoch boundaries in the AppMsg::Decided handler above

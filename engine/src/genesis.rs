// Genesis block extraData parser
// Parses validator addresses from genesis block header's extraData field
// Format (similar to BSC Parlia):
// - First 32 bytes: vanity (all zeros)
// - Middle: validator
// - Last 65 bytes: seal (all zeros in genesis)

use alloy_primitives::Address;
use color_eyre::eyre::{eyre, Result};
use tracing::info;

const EXTRA_VANITY_LEN: usize = 32;
const EXTRA_SEAL_LEN: usize = 65;

/// Validator information from genesis extraData
#[derive(Debug, Clone)]
pub struct GenesisValidatorInfo {
    pub consensus_address: Address, // Consensus address (primary identifier)
    pub operator_address: Address,  // Operator address (for contract interactions)
    pub tendermint_pubkey: Vec<u8>, // 32 bytes Ed25519 public key
    pub voting_power: u64,          // Voting power from genesis
}

/// Parse validators WITH complete information from extraData
///
/// Format: vanity(32) + [consensusAddr(20) + operatorAddr(20) + votingPower(8) + tendermintPubKey(32)] * N + epochLength(8) + seal(65)
///
/// # Arguments
/// * `extra_data` - The extraData bytes from genesis block header
///
/// # Returns
/// * `Vec<GenesisValidatorInfo>` - List of validators with complete information
/// * `u64` - Epoch length in blocks
pub fn parse_validators_from_extra_data(extra_data: &[u8]) -> Result<(Vec<GenesisValidatorInfo>, u64)> {
    // Minimum length check
    let min_len = EXTRA_VANITY_LEN + EXTRA_SEAL_LEN;

    if extra_data.len() < min_len {
        return Err(eyre!(
            "extraData too short: {} bytes, expected at least {}",
            extra_data.len(),
            min_len
        ));
    }

    // Calculate middle_data = total - vanity - seal
    let middle_data_len = extra_data.len() - EXTRA_VANITY_LEN - EXTRA_SEAL_LEN;

    // Format: N * (20 + 20 + 8 + 32) + 8 = N * 80 + 8
    // Check if middle_data_len >= 8 (at least epoch_length)
    if middle_data_len < 8 {
        return Err(eyre!(
            "Invalid extraData format: middle section length {} is too short. \
             Expected format: vanity(32) + [consensusAddr(20) + operatorAddr(20) + votingPower(8) + tendermintPubKey(32)] * N + epochLength(8) + seal(65)",
            middle_data_len
        ));
    }

    // Extract epoch_length (last 8 bytes before seal)
    let epoch_length_bytes = &extra_data[extra_data.len() - EXTRA_SEAL_LEN - 8..extra_data.len() - EXTRA_SEAL_LEN];
    let epoch_length = u64::from_be_bytes([
        epoch_length_bytes[0],
        epoch_length_bytes[1],
        epoch_length_bytes[2],
        epoch_length_bytes[3],
        epoch_length_bytes[4],
        epoch_length_bytes[5],
        epoch_length_bytes[6],
        epoch_length_bytes[7],
    ]);

    // Calculate validator data length (excluding epoch_length)
    let validator_data_len = middle_data_len - 8;

    // Format: N * (20 + 20 + 8 + 32) = N * 80
    if validator_data_len % 80 != 0 {
        return Err(eyre!(
            "Invalid extraData format: validator data length {} is not a multiple of 80. \
             Expected format: vanity(32) + [consensusAddr(20) + operatorAddr(20) + votingPower(8) + tendermintPubKey(32)] * N + epochLength(8) + seal(65)",
            validator_data_len
        ));
    }

    let validator_count = validator_data_len / 80;

    info!("📖 Parsing extraData format:");
    info!("   Total length: {} bytes", extra_data.len());
    info!("   Validator count: {}", validator_count);
    info!("   Epoch length: {} blocks", epoch_length);
    info!("   Format: vanity(32) + [consensusAddr(20) + operatorAddr(20) + votingPower(8) + tendermintPubKey(32)] * {} + epochLength(8) + seal(65)", validator_count);

    let mut result = Vec::new();
    for i in 0..validator_count {
        let validator_start = EXTRA_VANITY_LEN + (i * 80);

        // Extract consensus address (20 bytes)
        let consensus_addr =
            Address::from_slice(&extra_data[validator_start..validator_start + 20]);

        // Extract operator address (20 bytes)
        let operator_addr =
            Address::from_slice(&extra_data[validator_start + 20..validator_start + 40]);

        // Extract voting power (8 bytes, big-endian uint64)
        let voting_power_bytes = &extra_data[validator_start + 40..validator_start + 48];
        let voting_power = u64::from_be_bytes([
            voting_power_bytes[0],
            voting_power_bytes[1],
            voting_power_bytes[2],
            voting_power_bytes[3],
            voting_power_bytes[4],
            voting_power_bytes[5],
            voting_power_bytes[6],
            voting_power_bytes[7],
        ]);

        // Extract tendermint public key (32 bytes)
        let tendermint_pubkey = extra_data[validator_start + 48..validator_start + 80].to_vec();

        info!(
            "  Validator #{}: consensus={}, operator={}, voting_power={}, pubkey={:?}",
            i + 1,
            consensus_addr,
            operator_addr,
            voting_power,
            hex::encode(&tendermint_pubkey)
        );

        result.push(GenesisValidatorInfo {
            consensus_address: consensus_addr,
            operator_address: operator_addr,
            tendermint_pubkey,
            voting_power,
        });
    }

    info!(
        "✅ Parsed {} validators from extraData format, epoch_length: {} blocks",
        result.len(),
        epoch_length
    );
    Ok((result, epoch_length))
}

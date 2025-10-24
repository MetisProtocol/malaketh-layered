// Genesis block extraData parser
// Parses validator addresses from genesis block header's extraData field
// Format (similar to BSC Parlia):
// - First 32 bytes: vanity (all zeros)
// - Middle: validator consensus addresses (20 bytes each)
// - Last 65 bytes: seal (all zeros in genesis)

use alloy_primitives::Address;
use color_eyre::eyre::{Result, eyre};
use tracing::info;

const EXTRA_VANITY_LEN: usize = 32;
const EXTRA_SEAL_LEN: usize = 65;
const ADDRESS_LENGTH: usize = 20;
const TENDERMINT_PUBKEY_LENGTH: usize = 32;

/// Validator information from genesis extraData (extended format)
#[derive(Debug, Clone)]
pub struct GenesisValidatorInfo {
    pub address: Address,
    pub tendermint_pubkey: Vec<u8>, // 32 bytes Ed25519 public key
    pub voting_power: u64,           // Default to 1 for genesis
}

/// Parse validator addresses from genesis extraData
/// 
/// # Arguments
/// * `extra_data` - The extraData bytes from genesis block header
/// 
/// # Returns
/// * `Vec<Address>` - List of validator consensus addresses
pub fn parse_validators_from_extra_data(extra_data: &[u8]) -> Result<Vec<Address>> {
    let extra_len = extra_data.len();
    
    // Check minimum length: vanity + seal = 97 bytes
    if extra_len < EXTRA_VANITY_LEN + EXTRA_SEAL_LEN {
        return Err(eyre!(
            "Invalid extraData length: {}, expected at least {}",
            extra_len,
            EXTRA_VANITY_LEN + EXTRA_SEAL_LEN
        ));
    }
    
    // Extract validator bytes (between vanity and seal)
    let validator_bytes_len = extra_len - EXTRA_VANITY_LEN - EXTRA_SEAL_LEN;
    
    // Check if validator bytes length is multiple of ADDRESS_LENGTH
    if validator_bytes_len % ADDRESS_LENGTH != 0 {
        return Err(eyre!(
            "Invalid validator bytes length: {}, not a multiple of {}",
            validator_bytes_len,
            ADDRESS_LENGTH
        ));
    }
    
    let validator_count = validator_bytes_len / ADDRESS_LENGTH;
    
    if validator_count == 0 {
        return Err(eyre!("No validators found in extraData"));
    }
    
    info!(
        "📖 Parsing {} validators from genesis extraData (length: {} bytes)",
        validator_count,
        validator_bytes_len
    );
    
    // Parse validator addresses
    let mut validators = Vec::with_capacity(validator_count);
    
    for i in 0..validator_count {
        let start = EXTRA_VANITY_LEN + (i * ADDRESS_LENGTH);
        let end = start + ADDRESS_LENGTH;
        let address = Address::from_slice(&extra_data[start..end]);
        validators.push(address);
        
        info!("  Validator #{}: {}", i + 1, address);
    }
    
    Ok(validators)
}

/// Parse validator addresses from hex-encoded extraData string
/// 
/// # Arguments
/// * `extra_data_hex` - The extraData hex string (with or without 0x prefix)
/// 
/// # Returns
/// * `Vec<Address>` - List of validator consensus addresses
pub fn parse_validators_from_extra_data_hex(extra_data_hex: &str) -> Result<Vec<Address>> {
    let hex_str = extra_data_hex.strip_prefix("0x").unwrap_or(extra_data_hex);
    let extra_data = hex::decode(hex_str)
        .map_err(|e| eyre!("Failed to decode extraData hex: {}", e))?;
    
    parse_validators_from_extra_data(&extra_data)
}

/// Parse validators WITH Tendermint public keys from extended extraData format
/// 
/// ACTUAL Format: vanity(32) + validator_addrs(N*20) + tendermint_keys(N*32) + seal(65)
/// 
/// This function supports both:
/// - Standard format: vanity(32) + addresses(N*20) + seal(65) - returns placeholder Tendermint keys
/// - Extended format: vanity(32) + addresses(N*20) + tendermint_keys(N*32) + seal(65)
/// 
/// # Arguments
/// * `extra_data` - The extraData bytes from genesis block header
/// 
/// # Returns
/// * `Vec<GenesisValidatorInfo>` - List of validators with Tendermint public keys
pub fn parse_validators_with_tendermint_keys(extra_data: &[u8]) -> Result<Vec<GenesisValidatorInfo>> {
    // Minimum length check
    let min_len = EXTRA_VANITY_LEN + EXTRA_SEAL_LEN;
    
    if extra_data.len() < min_len {
        return Err(eyre!("extraData too short: {} bytes, expected at least {}", extra_data.len(), min_len));
    }

    // ACTUAL Format: vanity(32) + addresses(N*20) + tendermint_keys(N*32) + seal(65)
    // Standard format: vanity(32) + addresses(N*20) + seal(65)
    
    // Step 1: Try to determine format
    // Calculate middle_data = total - vanity - seal
    let middle_data_len = extra_data.len() - EXTRA_VANITY_LEN - EXTRA_SEAL_LEN;
    
    // Check if middle_data is just addresses (standard format)
    let is_standard_format = middle_data_len % ADDRESS_LENGTH == 0;
    
    // Check if middle_data is addresses + tendermint keys (extended format)
    // middle_data should be N*20 + N*32 = N*52
    let is_extended_format = middle_data_len % (ADDRESS_LENGTH + TENDERMINT_PUBKEY_LENGTH) == 0;
    
    if is_standard_format && !is_extended_format {
        // Standard format: only addresses
        let addresses = parse_validators_from_extra_data(extra_data)?;
        
        tracing::warn!("⚠️  Genesis extraData uses standard format (no Tendermint keys)");
        tracing::warn!("   Returning placeholder keys. Please regenerate genesis with extended format.");
        
        return Ok(addresses.into_iter().map(|address| GenesisValidatorInfo {
            address,
            tendermint_pubkey: vec![0u8; TENDERMINT_PUBKEY_LENGTH], // Placeholder
            voting_power: 1,
        }).collect());
    }
    
    if !is_extended_format {
        return Err(eyre!(
            "Invalid extraData format: middle section length {} is neither N*20 (standard) nor N*52 (extended)",
            middle_data_len
        ));
    }
    
    // Extended format: vanity(32) + addresses(N*20) + tendermint_keys(N*32) + seal(65)
    let validator_count = middle_data_len / (ADDRESS_LENGTH + TENDERMINT_PUBKEY_LENGTH);
    let addresses_len = validator_count * ADDRESS_LENGTH;
    let tendermint_keys_len = validator_count * TENDERMINT_PUBKEY_LENGTH;
    
    info!("📖 Parsing extended extraData format:");
    info!("   Total length: {} bytes", extra_data.len());
    info!("   Validator count: {}", validator_count);
    info!("   Format: vanity(32) + addresses({}) + tendermint_keys({}) + seal(65)", 
          addresses_len, 
          tendermint_keys_len);
    
    // Step 2: Extract parts
    let addresses_start = EXTRA_VANITY_LEN;
    let addresses_end = addresses_start + addresses_len;
    let tendermint_keys_start = addresses_end;
    let tendermint_keys_end = tendermint_keys_start + tendermint_keys_len;
    let seal_start = tendermint_keys_end;
    let seal_end = seal_start + EXTRA_SEAL_LEN;
    
    if seal_end != extra_data.len() {
        return Err(eyre!(
            "Invalid extraData length: expected {}, got {}",
            seal_end,
            extra_data.len()
        ));
    }
    
    let address_bytes = &extra_data[addresses_start..addresses_end];
    let tendermint_keys_data = &extra_data[tendermint_keys_start..tendermint_keys_end];
    
    // Step 5: Build result
    let mut result = Vec::new();
    for i in 0..validator_count {
        let addr_start = i * ADDRESS_LENGTH;
        let addr_end = addr_start + ADDRESS_LENGTH;
        let address = Address::from_slice(&address_bytes[addr_start..addr_end]);
        
        let key_start = i * TENDERMINT_PUBKEY_LENGTH;
        let key_end = key_start + TENDERMINT_PUBKEY_LENGTH;
        let tendermint_pubkey = tendermint_keys_data[key_start..key_end].to_vec();
        
        // Check if it's a placeholder (all zeros)
        if tendermint_pubkey.iter().all(|&b| b == 0) {
            tracing::warn!("⚠️  Validator {} has placeholder Tendermint public key (all zeros)", address);
        }
        
        result.push(GenesisValidatorInfo {
            address,
            tendermint_pubkey,
            voting_power: 1, // All validators have equal power in genesis
        });
    }
    
    info!("✅ Parsed {} validators from extended extraData format", result.len());
    Ok(result)
}

/// Parse validators from extraData hex string (with Tendermint keys)
/// 
/// # Arguments
/// * `extra_data_hex` - The extraData hex string (with or without 0x prefix)
/// 
/// # Returns
/// * `Vec<GenesisValidatorInfo>` - List of validators with Tendermint public keys
pub fn parse_validators_with_tendermint_keys_hex(extra_data_hex: &str) -> Result<Vec<GenesisValidatorInfo>> {
    let hex_str = extra_data_hex.strip_prefix("0x").unwrap_or(extra_data_hex);
    let extra_data = hex::decode(hex_str)
        .map_err(|e| eyre!("Failed to decode extraData hex: {}", e))?;
    
    parse_validators_with_tendermint_keys(&extra_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_validators_from_extra_data() {
        // Test data: 32 bytes vanity + 3x20 bytes addresses + 65 bytes seal
        // Validators:
        // 0x0754445aeda0441230d3ab099b0942181915186c
        // 0x3f8f2908b1b5b6ef3eec1968fcdf8340a6bec221
        // 0x9ab1a8b89460fccd8eb6739352300988915c71fe
        let extra_data_hex = "0x00000000000000000000000000000000000000000000000000000000000000000754445aeda0441230d3ab099b0942181915186c3f8f2908b1b5b6ef3eec1968fcdf8340a6bec2219ab1a8b89460fccd8eb6739352300988915c71fe0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
        
        let validators = parse_validators_from_extra_data_hex(extra_data_hex).unwrap();
        
        assert_eq!(validators.len(), 3);
        assert_eq!(
            validators[0],
            "0x0754445aeda0441230d3ab099b0942181915186c".parse::<Address>().unwrap()
        );
        assert_eq!(
            validators[1],
            "0x3f8f2908b1b5b6ef3eec1968fcdf8340a6bec221".parse::<Address>().unwrap()
        );
        assert_eq!(
            validators[2],
            "0x9ab1a8b89460fccd8eb6739352300988915c71fe".parse::<Address>().unwrap()
        );
    }

    #[test]
    fn test_parse_empty_validators() {
        // Only vanity + seal, no validators
        let extra_data = vec![0u8; EXTRA_VANITY_LEN + EXTRA_SEAL_LEN];
        
        let result = parse_validators_from_extra_data(&extra_data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("No validators"));
    }

    #[test]
    fn test_parse_invalid_length() {
        // Not enough bytes
        let extra_data = vec![0u8; 50];
        
        let result = parse_validators_from_extra_data(&extra_data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid extraData length"));
    }

    #[test]
    fn test_parse_invalid_validator_bytes() {
        // vanity + 25 bytes (not multiple of 20) + seal
        let mut extra_data = vec![0u8; EXTRA_VANITY_LEN];
        extra_data.extend_from_slice(&[0u8; 25]);
        extra_data.extend_from_slice(&[0u8; EXTRA_SEAL_LEN]);
        
        let result = parse_validators_from_extra_data(&extra_data);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not a multiple of"));
    }
}


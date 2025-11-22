use crate::PubkeyCmd;
use base64::{engine::general_purpose, Engine};
use color_eyre::eyre::Result;
use ed25519_consensus::SigningKey;
use serde::Deserialize;
use sha3::{Digest, Keccak256};
use std::fs;

#[derive(Debug, Deserialize)]
struct PrivKeyFile {
    value: String, // base64 private key seed (32 bytes)
    #[serde(default)]
    pub_key: Option<PubKey>, // optional embedded public key
}

#[derive(Debug, Deserialize)]
struct PubKey {
    value: String, // base64 public key (32 bytes)
}

pub fn run_pubkey(cmd: PubkeyCmd) -> Result<()> {
    let contents = fs::read_to_string(&cmd.key_file)?;
    let key_file: PrivKeyFile = serde_json::from_str(&contents)?;

    // If file contains pub_key.value, use it; otherwise derive from private key seed
    let pk32: [u8; 32] = if let Some(pk) = key_file.pub_key {
        let bytes = general_purpose::STANDARD.decode(pk.value)?;
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes[..32]);
        arr
    } else {
        let sk = general_purpose::STANDARD.decode(key_file.value)?; // 32 bytes seed
        let sk = SigningKey::try_from(&sk[..32]).expect("invalid ed25519 private key");
        sk.verification_key().to_bytes()
    };

    // PubKey hex
    let pk_hex = format!("0x{}", hex::encode(pk32));

    // Derive address per project types::Address::from_public_key:
    // address = first 20 bytes of Keccak256(pubkey)
    let hash = Keccak256::digest(&pk32);
    let addr_hex = format!("0x{}", hex::encode(&hash[..20]));

    println!("consensus pubkey: {}", pk_hex);
    println!("consensus address: {}", addr_hex);
    Ok(())
}

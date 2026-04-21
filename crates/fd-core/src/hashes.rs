use crate::FdError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, PartialOrd, Ord)]
#[serde(transparent)]
pub struct TaggedHash(pub String);

impl TaggedHash {
    pub fn parse_sha256(text: &str) -> Result<Self, FdError> {
        validate_tagged_hash(text, "sha256", 64)?;
        Ok(Self(text.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub fn sha256_tagged(bytes: &[u8]) -> TaggedHash {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    TaggedHash(format!("sha256:{}", hex::encode(hasher.finalize())))
}

pub fn validate_tagged_hash(text: &str, expected_tag: &str, expected_hex_len: usize) -> Result<(), FdError> {
    let Some((tag, hex_part)) = text.split_once(':') else {
        return Err(FdError::InvalidHashTag);
    };
    if tag != expected_tag {
        return Err(FdError::InvalidHashTag);
    }
    if hex_part.len() != expected_hex_len {
        return Err(FdError::InvalidHashLength);
    }
    if !hex_part.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {
        return Err(FdError::InvalidHashHex);
    }
    Ok(())
}

pub fn sha256_event_hash(bytes: &[u8]) -> TaggedHash {
    sha256_tagged(bytes)
}

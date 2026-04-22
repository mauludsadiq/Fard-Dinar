use crate::{canonical_json_bytes, crypto::validate_public_key_hex, sha256_tagged, FdError, MerchantRegistrySnapshot, OracleSetSnapshot};
use serde::de::DeserializeOwned;
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ObjectStore {
    root: PathBuf,
}

impl ObjectStore {
    pub fn new(root: impl AsRef<Path>) -> Self {
        Self {
            root: root.as_ref().to_path_buf(),
        }
    }

    pub fn load_registry(&self, hash: &str) -> Result<MerchantRegistrySnapshot, FdError> {
        let snapshot: MerchantRegistrySnapshot = self.load_validated_json(hash)?;
        validate_unique_keys(&snapshot.merchants, "merchant registry")?;
        for merchant in &snapshot.merchants {
            validate_public_key_hex(merchant)?;
        }
        Ok(MerchantRegistrySnapshot {
            version: snapshot.version,
            merchants: normalize_strings(snapshot.merchants),
        })
    }

    pub fn load_oracle_set(&self, hash: &str) -> Result<OracleSetSnapshot, FdError> {
        let snapshot: OracleSetSnapshot = self.load_validated_json(hash)?;
        validate_unique_keys(&snapshot.oracles, "oracle set")?;
        for oracle in &snapshot.oracles {
            validate_public_key_hex(oracle)?;
        }
        Ok(OracleSetSnapshot {
            version: snapshot.version,
            oracles: normalize_strings(snapshot.oracles),
        })
    }

    pub fn load_bytes(&self, tagged_hash: &str) -> Result<Vec<u8>, FdError> {
        let (tag, hex_part) = tagged_hash.split_once(':').ok_or(FdError::InvalidHashTag)?;
        if tag != "ahd1024" {
            return Err(FdError::InvalidHashTag);
        }
        let path = self.root.join(hex_part);
        let bytes = fs::read(&path).map_err(|_| FdError::ObjectNotFound(tagged_hash.to_string()))?;
        let actual = sha256_tagged(&bytes);
        if actual.as_str() != tagged_hash {
            return Err(FdError::ObjectHashMismatch {
                expected: tagged_hash.to_string(),
                actual: actual.0,
            });
        }
        Ok(bytes)
    }

    pub fn write_bytes(&self, bytes: &[u8]) -> Result<String, std::io::Error> {
        let hash = sha256_tagged(bytes);
        let hex_part = hash.as_str().split_once(':').expect("tagged hash must contain colon").1;
        fs::create_dir_all(&self.root)?;
        fs::write(self.root.join(hex_part), bytes)?;
        Ok(hash.0)
    }

    fn load_validated_json<T: DeserializeOwned + serde::Serialize>(&self, hash: &str) -> Result<T, FdError> {
        let bytes = self.load_bytes(hash)?;
        let parsed: T = serde_json::from_slice(&bytes).map_err(|e| FdError::InvalidJson(e.to_string()))?;
        let canonical_bytes = canonical_json_bytes(&parsed).map_err(|e| FdError::InvalidJson(e.to_string()))?;
        let canonical_hash = sha256_tagged(&canonical_bytes);
        if canonical_hash.as_str() != hash {
            return Err(FdError::CanonicalMismatch(hash.to_string()));
        }
        Ok(parsed)
    }
}

fn validate_unique_keys(values: &[String], context: &str) -> Result<(), FdError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value.clone()) {
            return Err(FdError::DuplicateValues(context.to_string()));
        }
    }
    Ok(())
}

fn normalize_strings(mut values: Vec<String>) -> Vec<String> {
    values.sort_by(|a, b| a.as_bytes().cmp(b.as_bytes()));
    values
}

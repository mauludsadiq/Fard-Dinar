use crate::FdError;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};

pub fn validate_public_key_hex(text: &str) -> Result<(), FdError> {
    if text.len() != 64 || !text.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {
        return Err(FdError::InvalidPublicKey);
    }
    Ok(())
}

pub fn validate_signature_hex(text: &str) -> Result<(), FdError> {
    if text.len() != 128 || !text.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()) {
        return Err(FdError::InvalidSignature);
    }
    Ok(())
}

pub fn verify_ed25519(public_key_hex: &str, message: &[u8], signature_hex: &str) -> Result<(), FdError> {
    validate_public_key_hex(public_key_hex)?;
    validate_signature_hex(signature_hex)?;

    let pk_bytes = hex::decode(public_key_hex).map_err(|_| FdError::InvalidPublicKey)?;
    let sig_bytes = hex::decode(signature_hex).map_err(|_| FdError::InvalidSignature)?;

    let pk_array: [u8; 32] = pk_bytes.try_into().map_err(|_| FdError::InvalidPublicKey)?;
    let sig_array: [u8; 64] = sig_bytes.try_into().map_err(|_| FdError::InvalidSignature)?;

    let verifying_key = VerifyingKey::from_bytes(&pk_array).map_err(|_| FdError::InvalidPublicKey)?;
    let signature = Signature::from_bytes(&sig_array);
    verifying_key
        .verify(message, &signature)
        .map_err(|_| FdError::SignatureVerificationFailed)
}

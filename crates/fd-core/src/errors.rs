use thiserror::Error;

#[derive(Debug, Error)]
pub enum FdError {
    #[error("hash text must start with a supported tag")]
    InvalidHashTag,
    #[error("hash text has invalid length")]
    InvalidHashLength,
    #[error("hash text contains invalid hex")]
    InvalidHashHex,
    #[error("public key must be 64 lowercase hex characters")]
    InvalidPublicKey,
    #[error("signature must be 128 lowercase hex characters")]
    InvalidSignature,
    #[error("object not found: {0}")]
    ObjectNotFound(String),
    #[error("object hash mismatch: expected {expected}, got {actual}")]
    ObjectHashMismatch { expected: String, actual: String },
    #[error("invalid json: {0}")]
    InvalidJson(String),
    #[error("canonical mismatch for object {0}")]
    CanonicalMismatch(String),
    #[error("duplicate values are not allowed in {0}")]
    DuplicateValues(String),
    #[error("account has insufficient balance")]
    InsufficientBalance,
    #[error("invalid nonce: expected {expected}, got {actual}")]
    InvalidNonce { expected: u64, actual: u64 },
    #[error("self transfer is not allowed")]
    SelfTransfer,
    #[error("deposit has already been consumed")]
    DepositAlreadyConsumed,
    #[error("oracle id is not authorized")]
    UnauthorizedOracle,
    #[error("signature verification failed")]
    SignatureVerificationFailed,
    #[error("unsupported event kind: {0}")]
    UnsupportedEventKind(String),
    #[error("registry snapshot is invalid: {0}")]
    InvalidRegistry(String),
    #[error("oracle set snapshot is invalid: {0}")]
    InvalidOracleSet(String),
    #[error("transition failed because dependent object resolution failed: {0}")]
    DependencyResolution(String),
}

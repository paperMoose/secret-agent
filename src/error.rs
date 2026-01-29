use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("secret '{0}' not found")]
    SecretNotFound(String),

    #[error("secret '{0}' already exists")]
    SecretAlreadyExists(String),

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("encryption error: {0}")]
    Encryption(String),

    #[error("decryption error: {0}")]
    Decryption(String),

    #[error("keychain error: {0}")]
    Keychain(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid secret name: {0}")]
    InvalidSecretName(String),
}

pub type Result<T> = std::result::Result<T, Error>;

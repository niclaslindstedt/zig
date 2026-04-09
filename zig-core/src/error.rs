use thiserror::Error;

#[derive(Debug, Error)]
pub enum ZigError {
    #[error("parse error: {0}")]
    Parse(String),

    #[error("validation error: {0}")]
    Validation(String),

    #[error("I/O error: {0}")]
    Io(String),

    #[error("serialization error: {0}")]
    Serialize(String),

    #[error("zag error: {0}")]
    Zag(String),
}

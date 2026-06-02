//! Error types for squelch-matrix.

use thiserror::Error;

/// Errors that can occur in the Matrix signaling layer.
#[derive(Debug, Error)]
pub enum MatrixError {
    #[error("login failed: {0}")]
    Login(String),

    #[error("room error: {0}")]
    Room(String),

    #[error("signaling error: {0}")]
    Signaling(String),

    #[error("sync error: {0}")]
    Sync(String),

    #[error("sdk error: {0}")]
    Sdk(#[from] matrix_sdk::Error),
}

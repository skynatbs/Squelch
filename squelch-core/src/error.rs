//! Central error types for Squelch.

use thiserror::Error;

/// Top-level error type shared across all Squelch crates.
#[derive(Debug, Error)]
pub enum SquelchError {
    #[error("audio error: {0}")]
    Audio(String),

    #[error("signaling error: {0}")]
    Signaling(String),

    #[error("webrtc error: {0}")]
    WebRtc(String),

    #[error("squad error: {0}")]
    Squad(String),
}

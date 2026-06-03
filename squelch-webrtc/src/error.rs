//! Error types for squelch-webrtc.

use thiserror::Error;

/// Errors that can occur in the WebRTC layer.
#[derive(Debug, Error)]
pub enum WebRtcError {
    #[error("SDP error: {0}")]
    Sdp(String),

    #[error("ICE error: {0}")]
    Ice(String),

    #[error("codec error: {0}")]
    Codec(String),

    #[error("peer not found: {0}")]
    PeerNotFound(String),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("str0m error: {0}")]
    Str0m(String),
}

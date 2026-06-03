//! squelch-matrix — Matrix signaling backend.
//!
//! Responsibilities:
//! - Login / session management via matrix-rust-sdk
//! - Squad room creation and membership
//! - Sending and receiving WebRTC signaling messages (SDP offer/answer, ICE candidates)
//!   as unencrypted to-device events (`io.squelch.*`)
//!
//! Audio data never passes through Matrix — only the WebRTC handshake does.

pub mod client;
pub mod error;
pub mod event_types;
pub mod signaling;

pub use client::{MatrixClient, MatrixConfig, SyncHandle};
pub use error::MatrixError;
pub use signaling::{IceCandidate, SdpMessage, SignalingEvent};

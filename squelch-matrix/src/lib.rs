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
pub mod signaling;

/// Custom Matrix event types used for WebRTC signaling.
pub mod event_types {
    pub const SDP_OFFER:     &str = "io.squelch.sdp_offer";
    pub const SDP_ANSWER:    &str = "io.squelch.sdp_answer";
    pub const ICE_CANDIDATE: &str = "io.squelch.ice_candidate";
    pub const CALL_MEMBER:   &str = "io.squelch.call_member";
}

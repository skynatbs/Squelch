//! WebRTC signaling messages sent as Matrix to-device events.

use serde::{Deserialize, Serialize};

/// A WebRTC SDP offer or answer payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdpMessage {
    pub call_id: String,
    pub room_id: String,
    pub sdp:     String,
}

/// A WebRTC ICE candidate payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    pub call_id:   String,
    pub room_id:   String,
    pub candidate: String,
    /// SDP media line index for this candidate.
    pub sdp_m_line_index: Option<u32>,
}

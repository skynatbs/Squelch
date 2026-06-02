//! WebRTC signaling messages sent as Matrix to-device events.
//!
//! All payloads are JSON objects sent via `io.squelch.*` to-device events.
//! They are unencrypted in MVP — the WebRTC DTLS-SRTP layer encrypts the
//! actual audio. E2EE signaling is a post-MVP concern.

use serde::{Deserialize, Serialize};

// ── Outgoing payloads ──────────────────────────────────────────────────────

/// A WebRTC SDP offer or answer sent to a remote peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdpMessage {
    /// Unique call identifier (e.g. UUID).
    pub call_id: String,
    /// Matrix room ID this call belongs to.
    pub room_id: String,
    /// SDP string as produced by str0m.
    pub sdp: String,
}

/// A WebRTC ICE candidate sent to a remote peer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IceCandidate {
    /// Unique call identifier matching the SDP exchange.
    pub call_id: String,
    /// Matrix room ID this call belongs to.
    pub room_id: String,
    /// ICE candidate string (e.g. `candidate:1 1 UDP ...`).
    pub candidate: String,
    /// SDP m-line index for this candidate.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sdp_m_line_index: Option<u32>,
}

// ── Incoming events ────────────────────────────────────────────────────────

/// A signaling event received from a remote peer via Matrix to-device.
#[derive(Debug, Clone)]
pub enum SignalingEvent {
    /// Remote peer is offering a WebRTC connection.
    SdpOffer {
        from: String,
        payload: SdpMessage,
    },
    /// Remote peer answered our offer.
    SdpAnswer {
        from: String,
        payload: SdpMessage,
    },
    /// Remote peer sent an ICE candidate.
    IceCandidate {
        from: String,
        payload: IceCandidate,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sdp_message_roundtrip() {
        let msg = SdpMessage {
            call_id: "test-call-1".into(),
            room_id: "!abc:example.org".into(),
            sdp: "v=0\r\no=- 0 0 IN IP4 127.0.0.1\r\n".into(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        let decoded: SdpMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.call_id, msg.call_id);
        assert_eq!(decoded.sdp, msg.sdp);
    }

    #[test]
    fn ice_candidate_optional_field_omitted() {
        let ice = IceCandidate {
            call_id: "test-call-1".into(),
            room_id: "!abc:example.org".into(),
            candidate: "candidate:1 1 UDP 2130706431 192.168.1.1 54400 typ host".into(),
            sdp_m_line_index: None,
        };
        let json = serde_json::to_string(&ice).unwrap();
        assert!(!json.contains("sdp_m_line_index"), "None field should be omitted");
    }

    #[test]
    fn ice_candidate_with_index_serialized() {
        let ice = IceCandidate {
            call_id: "c".into(),
            room_id: "!r:e.org".into(),
            candidate: "candidate:1 1 UDP 2 1.2.3.4 1234 typ host".into(),
            sdp_m_line_index: Some(0),
        };
        let json = serde_json::to_string(&ice).unwrap();
        assert!(json.contains("sdp_m_line_index"));
    }
}

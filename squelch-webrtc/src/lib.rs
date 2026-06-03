//! squelch-webrtc — WebRTC peer connections and audio streams.
//!
//! Responsibilities:
//! - One [`PeerConnection`] (wrapping str0m's `Rtc`) per remote peer
//! - SDP offer/answer negotiation via squelch-matrix signaling
//! - ICE + DTLS handshake (str0m handles internally)
//! - Sending encoded audio frames (Opus f32 PCM → writer) to remote peers
//! - Receiving decoded audio frames from remote peers → channel to squelch-audio
//!
//! # Mesh topology
//!
//! For a 4-player squad (2 duos), each player maintains 3 [`PeerConnection`]s:
//!
//! ```text
//!   Self ──── duo_partner  (always-on audio)
//!        ├─── other_leader (leader-net, PTT-gated)
//!        └─── other_member (leader-net, PTT-gated)
//! ```
//!
//! The [`PeerMesh`] manages all connections for one local player.

pub mod error;
pub mod peer;
pub mod mesh;

pub use error::WebRtcError;
pub use peer::{PeerConnection, PeerRole};
pub use mesh::PeerMesh;

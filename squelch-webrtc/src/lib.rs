//! squelch-webrtc — WebRTC peer connections and audio streams.
//!
//! Responsibilities:
//! - One `Rtc` instance (str0m) per remote peer
//! - SDP offer/answer negotiation (coordinated via squelch-matrix signaling)
//! - ICE + DTLS handshake (str0m handles internally)
//! - Sending encoded audio frames (Opus) to remote peers
//! - Receiving audio frames from remote peers → forwarding to squelch-audio
//!
//! This crate owns the str0m run loop (one thread per peer connection).
//! It communicates with squelch-audio via channels (encoded audio in/out).
//!
//! # Mesh topology
//!
//! For a 4-player squad (2 duos):
//!   Each player maintains 3 `PeerConnection` instances (one per remote peer).
//!   Channels: duo partner (always-on) + 2 others (leader-net gated).

pub mod peer;
pub mod mesh;

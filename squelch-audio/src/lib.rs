//! squelch-audio — Microphone capture, mixing, and playback.
//!
//! Responsibilities:
//! - Capture microphone input via cpal (f32, device-default sample rate)
//! - Route captured PCM to two logical channels:
//!     - Duo channel: always-on, forwarded to duo partner's WebRTC stream
//!     - Leader net:  PTT-gated, forwarded to all leaders' WebRTC streams
//! - Mix incoming PCM from remote peers into a single output stream
//! - Manage PTT state (AtomicBool, set by squelch-app on hotkey press/release)
//!
//! # Pipeline
//!
//! ```text
//! cpal input  → [duo_ring]    → squelch-webrtc (encode → str0m → network)
//!             → [leader_ring] → squelch-webrtc (only when ptt_active=true)
//!
//! squelch-webrtc (network → str0m → decode) → [remote_ring(s)]
//!                                           → cpal output (mix + clamp)
//! ```

pub mod pipeline;
pub mod ptt;

//! squelch-audio — Microphone capture, mixing, and playback.
//!
//! Responsibilities:
//! - Capture microphone input via cpal (f32, device-default sample rate)
//! - Encode mono PCM to Opus frames (20ms @ 48kHz) for WebRTC
//! - Route encoded audio to two logical channels:
//!     - Duo channel: always-on
//!     - Leader net: PTT-gated
//! - Decode incoming Opus frames from remote peers
//! - Mix all remote streams into a single cpal output stream
//!
//! # Pipeline
//!
//! ```text
//! cpal input → encode(Opus) → duo_ch   → squelch-webrtc (duo peer)
//!                           → leader_ch → squelch-webrtc (leaders, PTT only)
//!
//! squelch-webrtc → decode(Opus) → mix → cpal output
//! ```

pub mod error;
pub mod pipeline;
pub mod ptt;

pub use error::AudioError;
pub use pipeline::{AudioConfig, AudioHandles, AudioPipeline, OPUS_FRAME_SAMPLES};
pub use ptt::PttState;

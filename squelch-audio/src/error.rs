//! Error types for squelch-audio.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum AudioError {
    #[error("no input device available")]
    NoInputDevice,

    #[error("no output device available")]
    NoOutputDevice,

    #[error("cpal stream error: {0}")]
    Stream(String),

    #[error("opus encoder error: {0}")]
    OpusEncode(#[from] opus::Error),

    #[error("unsupported sample format: {0:?}")]
    UnsupportedFormat(cpal::SampleFormat),
}

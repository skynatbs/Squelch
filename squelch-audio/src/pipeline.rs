//! cpal input/output stream management, channel routing, and Opus codec.
//!
//! # Data flow
//!
//! ```text
//!                    ┌─────────────────────────────────────────────┐
//!   Microphone ──────┤ cpal input callback (f32 PCM)               │
//!                    │                                             │
//!                    │  always ──→ duo_ring ──→ Opus encode ──→ net │
//!                    │  if PTT ──→ leader_ring ─→ Opus encode ──→ net│
//!                    └─────────────────────────────────────────────┘
//!
//!                    ┌─────────────────────────────────────────────┐
//!   Network ─────────┤ Opus decode ──→ remote_ring(s)              │
//!                    │                                             │
//!                    │  sum of all remote_rings ──→ cpal output    │
//!                    └─────────────────────────────────────────────┘
//! ```
//!
//! The pipeline runs entirely on cpal's audio threads. PCM samples are
//! `f32` mono at the Opus canonical rate of 48kHz. If the device uses a
//! different rate, samples are passed through unchanged and Opus will encode
//! at the device rate — proper resampling is a post-MVP concern.

use std::sync::{Arc, atomic::Ordering};

use cpal::{
    SampleFormat, StreamConfig,
    traits::{DeviceTrait, HostTrait, StreamTrait},
};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::{error::AudioError, ptt::PttState};

/// Opus frame size: 20ms at 48kHz mono = 960 samples.
pub const OPUS_FRAME_SAMPLES: usize = 960;
/// Opus target bitrate in bps (24kbps — good quality for voice).
const OPUS_BITRATE: opus::Bitrate = opus::Bitrate::Bits(24_000);
/// Pre-fill latency in milliseconds — prevents output starvation at startup.
const LATENCY_MS: f32 = 100.0;

// ── Config ─────────────────────────────────────────────────────────────────

/// Configuration for the audio pipeline.
#[derive(Debug, Clone)]
pub struct AudioConfig {
    /// Shared PTT state (set by the UI/hotkey layer).
    pub ptt: PttState,
}

// ── Pipeline ───────────────────────────────────────────────────────────────

/// Handles created by the audio pipeline for sending/receiving encoded audio.
pub struct AudioHandles {
    /// Send encoded Opus bytes to the duo channel (always-on).
    /// Connect to `squelch-webrtc` for the duo peer.
    pub duo_opus_tx: mpsc::Receiver<Vec<u8>>,
    /// Send encoded Opus bytes to the leader-net channel (PTT-gated).
    /// Connect to `squelch-webrtc` for leader peers.
    pub leader_opus_tx: mpsc::Receiver<Vec<u8>>,
    /// Push encoded Opus bytes from any remote peer here for playback.
    /// Call `AudioPipeline::add_remote_peer` to get a sender per peer.
    pub add_remote_fn: Box<dyn FnMut() -> mpsc::Sender<Vec<u8>> + Send>,
}

/// The active audio pipeline — keeps cpal streams alive via RAII.
///
/// Drop this to stop all audio I/O.
pub struct AudioPipeline {
    _input_stream: cpal::Stream,
    _output_stream: cpal::Stream,
}

impl AudioPipeline {
    /// Build and start the audio pipeline.
    ///
    /// Returns the pipeline (keep alive as long as audio is needed) and
    /// the [`AudioHandles`] for connecting to `squelch-webrtc`.
    pub fn start(cfg: AudioConfig) -> Result<(AudioPipeline, AudioHandles), AudioError> {
        let host = cpal::default_host();

        let input_dev = host
            .default_input_device()
            .ok_or(AudioError::NoInputDevice)?;
        let output_dev = host
            .default_output_device()
            .ok_or(AudioError::NoOutputDevice)?;

        let in_cfg_supported = input_dev
            .default_input_config()
            .map_err(|e| AudioError::Stream(e.to_string()))?;
        let out_cfg_supported = output_dev
            .default_output_config()
            .map_err(|e| AudioError::Stream(e.to_string()))?;

        if in_cfg_supported.sample_format() != SampleFormat::F32 {
            return Err(AudioError::UnsupportedFormat(
                in_cfg_supported.sample_format(),
            ));
        }

        let in_stream_cfg: StreamConfig = in_cfg_supported.into();
        let out_stream_cfg: StreamConfig = out_cfg_supported.into();
        let sample_rate = in_stream_cfg.sample_rate.0;
        let channels = in_stream_cfg.channels as usize;

        info!(sample_rate, channels, "audio pipeline starting");

        // ── Ring buffers for local mic routing ────────────────────────────
        // Used only for pre-filling silence to cover initial latency.
        // The actual routing happens inline in the input callback via the
        // duo_pcm_buf and leader_pcm_buf accumulators below.
        let latency_samples = ((LATENCY_MS / 1000.0) * sample_rate as f32) as usize * channels;
        let buf_size = latency_samples * 2;

        // Unused in the current inline-encoder design but kept to document
        // the latency constant. Ring buffers are used only for the remote
        // peer output path (see remote_senders below).
        let _ = buf_size;

        // ── Channels: mic PCM → Opus encoder tasks ─────────────────────
        // We produce f32 PCM frames from the ring buffers in the output
        // callback, then encode to Opus in a separate task.
        // For simplicity in Phase 3 the encoding happens in the input callback.

        let (duo_opus_tx, duo_opus_rx) = mpsc::channel::<Vec<u8>>(64);
        let (leader_opus_tx, leader_opus_rx) = mpsc::channel::<Vec<u8>>(64);

        // ── Remote peer ring buffers (output mixing) ──────────────────────
        // Each remote peer gets a HeapRb. We support up to MAX_REMOTE_PEERS.
        // The add_remote_fn closure creates a new sender/ring pair per peer.

        // We use a Vec of producers (protected by Arc<Mutex>) that the input
        // callback / encoder task fills, and a Vec of consumers that the
        // output callback drains and mixes.
        //
        // For thread-safety across the closure boundary we use mpsc channels
        // for remote audio too: each remote peer pushes Opus bytes, we decode
        // in the output callback.

        let remote_senders: Arc<std::sync::Mutex<Vec<mpsc::Receiver<Vec<u8>>>>> =
            Arc::new(std::sync::Mutex::new(Vec::new()));
        let remote_senders_clone = remote_senders.clone();

        let add_remote_fn = move || -> mpsc::Sender<Vec<u8>> {
            let (tx, rx) = mpsc::channel::<Vec<u8>>(64);
            remote_senders_clone.lock().unwrap().push(rx);
            tx
        };

        // ── Opus encoder — mono, 48kHz ────────────────────────────────────
        // We create one encoder per channel (duo + leader). They run on the
        // input callback thread (single-threaded, no Mutex needed).
        let mut duo_encoder =
            opus::Encoder::new(sample_rate, opus::Channels::Mono, opus::Application::Voip)?;
        duo_encoder.set_bitrate(OPUS_BITRATE)?;

        let mut leader_encoder =
            opus::Encoder::new(sample_rate, opus::Channels::Mono, opus::Application::Voip)?;
        leader_encoder.set_bitrate(OPUS_BITRATE)?;

        let ptt = cfg.ptt.clone();

        // Accumulation buffers for building full Opus frames
        let mut duo_pcm_buf: Vec<f32> = Vec::with_capacity(OPUS_FRAME_SAMPLES * 2);
        let mut leader_pcm_buf: Vec<f32> = Vec::with_capacity(OPUS_FRAME_SAMPLES * 2);
        let mut opus_out_buf = vec![0u8; 4096];

        // ── Input callback ────────────────────────────────────────────────
        let input_fn = move |data: &[f32], _: &cpal::InputCallbackInfo| {
            let ptt_on = ptt.0.load(Ordering::Relaxed);

            // Downmix stereo → mono if necessary (average channels)
            let mono: Vec<f32> = if channels == 1 {
                data.to_vec()
            } else {
                data.chunks_exact(channels)
                    .map(|ch| ch.iter().sum::<f32>() / channels as f32)
                    .collect()
            };

            duo_pcm_buf.extend_from_slice(&mono);
            if ptt_on {
                leader_pcm_buf.extend_from_slice(&mono);
            }

            // Encode full Opus frames (960 samples each)
            while duo_pcm_buf.len() >= OPUS_FRAME_SAMPLES {
                let frame: Vec<f32> = duo_pcm_buf.drain(..OPUS_FRAME_SAMPLES).collect();
                match duo_encoder.encode_float(&frame, &mut opus_out_buf) {
                    Ok(n) => {
                        let _ = duo_opus_tx.try_send(opus_out_buf[..n].to_vec());
                    }
                    Err(e) => warn!("duo opus encode error: {e}"),
                }
            }

            while ptt_on && leader_pcm_buf.len() >= OPUS_FRAME_SAMPLES {
                let frame: Vec<f32> = leader_pcm_buf.drain(..OPUS_FRAME_SAMPLES).collect();
                match leader_encoder.encode_float(&frame, &mut opus_out_buf) {
                    Ok(n) => {
                        let _ = leader_opus_tx.try_send(opus_out_buf[..n].to_vec());
                    }
                    Err(e) => warn!("leader opus encode error: {e}"),
                }
            }

            // If PTT turned off, discard accumulated leader PCM
            if !ptt_on {
                leader_pcm_buf.clear();
            }
        };

        // ── Opus decoders for remote peers ────────────────────────────────
        // We create decoders lazily as remote peers are added. Since we can't
        // know the count at startup, we allocate a fixed pool and decode
        // in the output callback.
        let mut decoders: Vec<opus::Decoder> = Vec::new();
        let mut decode_buf = vec![0f32; OPUS_FRAME_SAMPLES * 2];

        let remote_senders_out = remote_senders.clone();

        // ── Output callback ───────────────────────────────────────────────
        let output_fn = move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            // Drain all pending Opus packets from remote peers, decode and mix
            let mut mixed = vec![0f32; data.len()];

            let mut receivers = remote_senders_out.lock().unwrap();

            // Grow decoder pool if new peers were added
            while decoders.len() < receivers.len() {
                match opus::Decoder::new(sample_rate, opus::Channels::Mono) {
                    Ok(d) => decoders.push(d),
                    Err(e) => {
                        warn!("failed to create opus decoder: {e}");
                        break;
                    }
                }
            }

            for (i, rx) in receivers.iter_mut().enumerate() {
                let decoder = match decoders.get_mut(i) {
                    Some(d) => d,
                    None => break,
                };
                while let Ok(opus_bytes) = rx.try_recv() {
                    match decoder.decode_float(&opus_bytes, &mut decode_buf, false) {
                        Ok(n) => {
                            // Upmix mono → stereo if output is stereo
                            let out_ch = out_stream_cfg.channels as usize;
                            for (j, s) in decode_buf[..n].iter().enumerate() {
                                for c in 0..out_ch {
                                    let idx = j * out_ch + c;
                                    if idx < mixed.len() {
                                        mixed[idx] += s;
                                    }
                                }
                            }
                        }
                        Err(e) => warn!("opus decode error: {e}"),
                    }
                }
            }
            drop(receivers);

            // Write clamped mix to output buffer
            for (out, mix) in data.iter_mut().zip(mixed.iter()) {
                *out = mix.clamp(-1.0, 1.0);
            }
        };

        // ── Build cpal streams ────────────────────────────────────────────
        let input_stream = input_dev
            .build_input_stream(
                &in_stream_cfg,
                input_fn,
                |e| warn!("input stream error: {e}"),
                None,
            )
            .map_err(|e| AudioError::Stream(e.to_string()))?;

        let output_stream = output_dev
            .build_output_stream(
                &out_stream_cfg,
                output_fn,
                |e| warn!("output stream error: {e}"),
                None,
            )
            .map_err(|e| AudioError::Stream(e.to_string()))?;

        input_stream
            .play()
            .map_err(|e| AudioError::Stream(e.to_string()))?;
        output_stream
            .play()
            .map_err(|e| AudioError::Stream(e.to_string()))?;

        info!("audio pipeline started");

        Ok((
            AudioPipeline {
                _input_stream: input_stream,
                _output_stream: output_stream,
            },
            AudioHandles {
                duo_opus_tx: duo_opus_rx,
                leader_opus_tx: leader_opus_rx,
                add_remote_fn: Box::new(add_remote_fn),
            },
        ))
    }
}

// ── Codec helpers (used by squelch-webrtc TODO items) ─────────────────────

/// Encode a mono f32 PCM frame (exactly `OPUS_FRAME_SAMPLES` samples) to Opus.
///
/// Returns the encoded bytes. The encoder is owned by the caller so it can
/// maintain continuity across frames.
pub fn encode_frame(
    encoder: &mut opus::Encoder,
    pcm: &[f32],
    out: &mut Vec<u8>,
) -> Result<usize, AudioError> {
    out.resize(4096, 0);
    let n = encoder.encode_float(pcm, out)?;
    Ok(n)
}

/// Decode an Opus packet to mono f32 PCM.
///
/// Returns the number of samples written to `out`.
pub fn decode_packet(
    decoder: &mut opus::Decoder,
    packet: &[u8],
    out: &mut [f32],
) -> Result<usize, AudioError> {
    let n = decoder.decode_float(packet, out, false)?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opus_encode_decode_roundtrip() {
        let sample_rate = 48_000u32;
        let mut encoder =
            opus::Encoder::new(sample_rate, opus::Channels::Mono, opus::Application::Voip)
                .expect("encoder");
        let mut decoder = opus::Decoder::new(sample_rate, opus::Channels::Mono).expect("decoder");

        // Generate a simple sine wave at 440 Hz
        let pcm: Vec<f32> = (0..OPUS_FRAME_SAMPLES)
            .map(|i| {
                let t = i as f32 / sample_rate as f32;
                (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 0.5
            })
            .collect();

        let mut encoded = vec![0u8; 4096];
        let n = encoder.encode_float(&pcm, &mut encoded).expect("encode");
        assert!(n > 0, "encoded size should be > 0");

        let mut decoded = vec![0f32; OPUS_FRAME_SAMPLES];
        let m = decoder
            .decode_float(&encoded[..n], &mut decoded, false)
            .expect("decode");
        assert_eq!(m, OPUS_FRAME_SAMPLES, "decoded frame size should match");

        // Verify the decoded signal is not silent (Opus is lossy but not that lossy)
        let energy: f32 = decoded.iter().map(|s| s * s).sum::<f32>() / m as f32;
        assert!(
            energy > 0.01,
            "decoded signal should not be silent: energy={energy}"
        );
    }

    #[test]
    fn encode_frame_helper() {
        let mut encoder = opus::Encoder::new(48_000, opus::Channels::Mono, opus::Application::Voip)
            .expect("encoder");
        let pcm = vec![0.0f32; OPUS_FRAME_SAMPLES];
        let mut out = Vec::new();
        let n = encode_frame(&mut encoder, &pcm, &mut out).expect("encode_frame");
        assert!(n > 0);
    }

    #[test]
    fn decode_packet_helper() {
        let mut encoder = opus::Encoder::new(48_000, opus::Channels::Mono, opus::Application::Voip)
            .expect("encoder");
        let mut decoder = opus::Decoder::new(48_000, opus::Channels::Mono).expect("decoder");

        let pcm = vec![0.1f32; OPUS_FRAME_SAMPLES];
        let mut out = Vec::new();
        let n = encode_frame(&mut encoder, &pcm, &mut out).expect("encode");

        let mut decoded = vec![0.0f32; OPUS_FRAME_SAMPLES];
        let m = decode_packet(&mut decoder, &out[..n], &mut decoded).expect("decode");
        assert_eq!(m, OPUS_FRAME_SAMPLES);
    }
}

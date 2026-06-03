//! Single WebRTC peer connection — wraps str0m's `Rtc` instance.
//!
//! Each `PeerConnection` runs its own event loop in a dedicated thread
//! (str0m is sync + sans-IO, not async). It communicates with the outside
//! world via two channels:
//!
//! - `audio_tx`: sends decoded PCM frames received from the remote peer
//!   → squelch-audio mixes these into the output stream
//! - `audio_rx`: receives raw PCM frames to encode and send to the remote peer
//!   → squelch-audio pushes mic samples here
//!
//! SDP and ICE are handled externally: the caller drives the negotiation via
//! [`PeerConnection::create_offer`], [`PeerConnection::accept_answer`], and
//! [`PeerConnection::add_ice_candidate`].

use std::{
    net::{SocketAddr, UdpSocket},
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use str0m::{
    Candidate, Event, IceConnectionState, Input, Output, Rtc,
    media::{Direction, Frequency, MediaKind, MediaTime, Mid},
};
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::error::WebRtcError;

/// Role of this peer in the WebRTC negotiation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerRole {
    /// We initiate the connection (send the SDP offer).
    Offerer,
    /// The remote peer initiates (we send the SDP answer).
    Answerer,
}

/// Channel capacity for audio frames.
const AUDIO_CHAN_SIZE: usize = 128;

/// How many PCM f32 samples per Opus frame (20ms @ 48kHz mono).
pub const OPUS_FRAME_SAMPLES: usize = 960;

/// A single WebRTC peer connection.
///
/// Owns a str0m `Rtc` instance and a UDP socket. After construction,
/// call [`PeerConnection::run`] in a dedicated thread to drive the event loop.
pub struct PeerConnection {
    /// Identifier of the remote peer (their Matrix user ID).
    pub remote_id: String,
    /// Role in SDP negotiation.
    pub role: PeerRole,
    /// Shared str0m state — accessible before the run loop starts.
    rtc: Arc<Mutex<Rtc>>,
    /// Local UDP socket address.
    local_addr: SocketAddr,
    /// Audio MID assigned during SDP negotiation.
    mid: Option<Mid>,
    /// Sends decoded PCM from remote → squelch-audio.
    pub audio_out_tx: mpsc::Sender<Vec<f32>>,
    /// Receives PCM from squelch-audio → encode + send to remote.
    pub audio_in_tx: mpsc::Sender<Vec<f32>>,
    audio_in_rx: Arc<Mutex<Option<mpsc::Receiver<Vec<f32>>>>>,
}

impl PeerConnection {
    /// Create a new peer connection bound to a random local UDP port.
    ///
    /// Returns the connection and the channel to receive decoded audio
    /// from the remote peer (connect this to squelch-audio's mixer).
    pub fn new(
        remote_id: impl Into<String>,
        role: PeerRole,
    ) -> Result<(Self, mpsc::Receiver<Vec<f32>>), WebRtcError> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let local_addr = socket.local_addr()?;

        let now = Instant::now();
        let rtc = Rtc::new(now);

        let (audio_out_tx, audio_out_rx) = mpsc::channel(AUDIO_CHAN_SIZE);
        let (audio_in_tx, audio_in_rx)   = mpsc::channel(AUDIO_CHAN_SIZE);

        let conn = Self {
            remote_id: remote_id.into(),
            role,
            rtc: Arc::new(Mutex::new(rtc)),
            local_addr,
            mid: None,
            audio_out_tx,
            audio_in_tx,
            audio_in_rx: Arc::new(Mutex::new(Some(audio_in_rx))),
        };

        Ok((conn, audio_out_rx))
    }

    /// Returns the local UDP address for this connection.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    // ── SDP / ICE ─────────────────────────────────────────────────────────

    /// Create an SDP offer (Offerer role).
    ///
    /// Returns the SDP string to send to the remote peer via Matrix.
    pub fn create_offer(&mut self) -> Result<String, WebRtcError> {
        let mut rtc = self.rtc.lock().unwrap();

        // Add local ICE candidate
        rtc.add_local_candidate(
            Candidate::host(self.local_addr, "udp")
                .map_err(|e| WebRtcError::Ice(e.to_string()))?,
        );

        let mut sdp = rtc.sdp_api();
        let mid = sdp.add_media(MediaKind::Audio, Direction::SendRecv, None, None, None);

        let (offer, _pending) = sdp
            .apply()
            .ok_or_else(|| WebRtcError::Sdp("no changes to apply".into()))?;

        self.mid = Some(mid);
        Ok(offer.to_sdp_string())
    }

    /// Accept an SDP offer from the remote peer (Answerer role).
    ///
    /// Returns the SDP answer string to send back via Matrix.
    pub fn accept_offer(&mut self, offer_sdp: &str) -> Result<String, WebRtcError> {
        let mut rtc = self.rtc.lock().unwrap();

        rtc.add_local_candidate(
            Candidate::host(self.local_addr, "udp")
                .map_err(|e| WebRtcError::Ice(e.to_string()))?,
        );

        let offer = str0m::change::SdpOffer::from_sdp_string(offer_sdp)
            .map_err(|e| WebRtcError::Sdp(e.to_string()))?;

        let answer = rtc
            .sdp_api()
            .accept_offer(offer)
            .map_err(|e| WebRtcError::Sdp(e.to_string()))?;

        Ok(answer.to_sdp_string())
    }

    /// Apply the remote peer's SDP answer (Offerer role, after `create_offer`).
    pub fn accept_answer(&mut self, answer_sdp: &str) -> Result<(), WebRtcError> {
        let mut rtc = self.rtc.lock().unwrap();

        // Re-create pending state by re-applying the offer
        let mut sdp = rtc.sdp_api();
        let _mid = sdp.add_media(MediaKind::Audio, Direction::SendRecv, None, None, None);
        let (_offer, pending) = sdp
            .apply()
            .ok_or_else(|| WebRtcError::Sdp("no changes to apply for answer".into()))?;

        let answer = str0m::change::SdpAnswer::from_sdp_string(answer_sdp)
            .map_err(|e| WebRtcError::Sdp(e.to_string()))?;

        rtc.sdp_api()
            .accept_answer(pending, answer)
            .map_err(|e| WebRtcError::Sdp(e.to_string()))?;

        Ok(())
    }

    /// Add a remote ICE candidate.
    pub fn add_ice_candidate(&self, candidate_str: &str) -> Result<(), WebRtcError> {
        let candidate = Candidate::from_sdp_string(candidate_str)
            .map_err(|e| WebRtcError::Ice(e.to_string()))?;

        let mut rtc = self.rtc.lock().unwrap();
        rtc.add_remote_candidate(candidate);
        Ok(())
    }

    // ── Run loop ──────────────────────────────────────────────────────────

    /// Drive the str0m event loop. Call this in a dedicated `std::thread`.
    ///
    /// Runs until the remote peer disconnects or `shutdown_rx` fires.
    pub fn run(
        self,
        socket: UdpSocket,
        mut shutdown_rx: tokio::sync::oneshot::Receiver<()>,
    ) {
        let mut buf = vec![0u8; 2048];
        let mut media_ts: u64 = 0;
        let mut audio_pt = None;
        let mut connected = false;
        let mut audio_mid = self.mid;

        // Take the audio_in receiver
        let mut audio_in_rx = self.audio_in_rx.lock().unwrap().take().unwrap();

        socket.set_nonblocking(true).expect("set_nonblocking");

        info!(remote = %self.remote_id, "peer connection run loop started");

        loop {
            // Check shutdown signal
            if shutdown_rx.try_recv().is_ok() {
                info!(remote = %self.remote_id, "peer connection shutting down");
                break;
            }

            let now = Instant::now();
            let mut rtc = self.rtc.lock().unwrap();

            // ── Drain poll_output ─────────────────────────────────────────
            loop {
                match rtc.poll_output() {
                    Err(e) => { warn!("poll_output error: {e}"); break; }
                    Ok(Output::Timeout(_)) => break,
                    Ok(Output::Transmit(t)) => {
                        if let Err(e) = socket.send_to(&t.contents, t.destination) {
                            debug!("send_to error: {e}");
                        }
                    }
                    Ok(Output::Event(Event::IceConnectionStateChange(s))) => {
                        info!(remote = %self.remote_id, state = ?s, "ICE state");
                        connected = matches!(
                            s,
                            IceConnectionState::Connected | IceConnectionState::Completed
                        );
                        if s == IceConnectionState::Disconnected { break; }
                    }
                    Ok(Output::Event(Event::MediaAdded(media))) => {
                        if media.kind == MediaKind::Audio && audio_mid.is_none() {
                            audio_mid = Some(media.mid);
                            info!(mid = ?media.mid, "audio track added (answerer side)");
                        }
                    }
                    Ok(Output::Event(Event::MediaData(data))) => {
                        // Raw Opus payload received — forward as-is to squelch-audio
                        // (squelch-audio will decode with the opus crate)
                        let samples = data.data.to_vec();
                        // Reinterpret bytes as f32 placeholder until opus decode is wired
                        // TODO: decode Opus bytes → f32 PCM via opus crate
                        let _ = self.audio_out_tx.try_send(
                            samples.chunks_exact(4)
                                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                                .collect()
                        );
                    }
                    Ok(Output::Event(_)) => {}
                }
            }

            // ── Read incoming UDP ─────────────────────────────────────────
            loop {
                match socket.recv_from(&mut buf) {
                    Ok((n, src)) => {
                        let data = match buf[..n].try_into() {
                            Ok(d) => d,
                            Err(_) => break,
                        };
                        let _ = rtc.handle_input(Input::Receive(
                            Instant::now(),
                            str0m::net::Receive {
                                proto: str0m::net::Protocol::Udp,
                                source: src,
                                destination: self.local_addr,
                                contents: data,
                            },
                        ));
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                    Err(e) => { debug!("recv_from error: {e}"); break; }
                }
            }

            // ── Advance time ──────────────────────────────────────────────
            let _ = rtc.handle_input(Input::Timeout(now));

            // ── Send audio if connected ───────────────────────────────────
            if connected {
                // Resolve PT once after DTLS handshake
                if audio_pt.is_none() {
                    if let Some(mid) = audio_mid {
                        if let Some(writer) = rtc.writer(mid) {
                            audio_pt = writer.payload_params().next().map(|p| p.pt());
                            if let Some(pt) = audio_pt {
                                info!(?pt, "audio payload type resolved");
                            }
                        }
                    }
                }

                if let (Some(mid), Some(pt)) = (audio_mid, audio_pt) {
                    // Drain all pending audio frames from squelch-audio
                    while let Ok(pcm) = audio_in_rx.try_recv() {
                        media_ts += OPUS_FRAME_SAMPLES as u64;
                        // TODO: encode PCM → Opus bytes via opus crate
                        // For now send raw f32 bytes as placeholder
                        let raw: Vec<u8> = pcm.iter()
                            .flat_map(|s| s.to_le_bytes())
                            .collect();
                        if let Some(writer) = rtc.writer(mid) {
                            let _ = writer.write(
                                pt,
                                Instant::now(),
                                MediaTime::new(media_ts, Frequency::FORTY_EIGHT_KHZ),
                                raw.as_slice(),
                            );
                        }
                        // After write() drain poll_output (single-mutation invariant)
                        loop {
                            match rtc.poll_output() {
                                Ok(Output::Timeout(_)) => break,
                                Ok(Output::Transmit(t)) => {
                                    let _ = socket.send_to(&t.contents, t.destination);
                                }
                                Ok(Output::Event(_)) => {}
                                Err(_) => break,
                            }
                        }
                    }
                }
            }

            drop(rtc);
            std::thread::sleep(Duration::from_millis(10));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_connection_creation() {
        let (conn, _rx) = PeerConnection::new("@alice:example.org", PeerRole::Offerer)
            .expect("should create peer connection");
        assert_eq!(conn.remote_id, "@alice:example.org");
        assert_eq!(conn.role, PeerRole::Offerer);
        assert!(conn.local_addr().port() > 0, "should have a bound port");
    }

    #[test]
    fn peer_role_offerer_answerer_distinct() {
        assert_ne!(PeerRole::Offerer, PeerRole::Answerer);
    }
}

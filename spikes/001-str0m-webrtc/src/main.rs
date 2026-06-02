/// Spike 001 – str0m WebRTC Loopback
///
/// Zwei Rtc-Instanzen (L = Offerer, R = Answerer) in einem Prozess.
/// SDP-Handshake in-process (simuliert Matrix-Signaling).
/// ICE läuft über zwei loopback-UDP-Sockets.
/// L sendet synthetische Audio-Bytes (fake Opus), R empfängt sie.
///
/// Erfolgskriterium:
///   - Verbindung kommt zustande (ICE Completed)
///   - R empfängt mindestens 5 RTP-Pakete von L
///   - Kein Panic, kein Deadlock
use std::{
    net::{SocketAddr, UdpSocket},
    time::{Duration, Instant},
};

use anyhow::{bail, Result};
use str0m::{
    media::{Direction, Frequency, MediaKind, MediaTime},
    net::{Protocol, Receive},
    Candidate, Event, IceConnectionState, Input, Output, Rtc,
};

fn main() -> Result<()> {
    // ── Sockets ──────────────────────────────────────────────────────────────
    let sock_l = UdpSocket::bind("127.0.0.1:0")?;
    let sock_r = UdpSocket::bind("127.0.0.1:0")?;
    let addr_l: SocketAddr = sock_l.local_addr()?;
    let addr_r: SocketAddr = sock_r.local_addr()?;

    println!("[init] L bindet auf {addr_l}");
    println!("[init] R bindet auf {addr_r}");

    // ── Rtc-Instanzen ────────────────────────────────────────────────────────
    let now = Instant::now();
    let mut rtc_l = Rtc::new(now);
    let mut rtc_r = Rtc::new(now);

    // ICE-Kandidaten eintragen
    rtc_l.add_local_candidate(Candidate::host(addr_l, "udp")?);
    rtc_r.add_local_candidate(Candidate::host(addr_r, "udp")?);
    rtc_l.add_remote_candidate(Candidate::host(addr_r, "udp")?);
    rtc_r.add_remote_candidate(Candidate::host(addr_l, "udp")?);

    // ── SDP Offer/Answer ─────────────────────────────────────────────────────
    let mut sdp_l = rtc_l.sdp_api();
    let mid = sdp_l.add_media(MediaKind::Audio, Direction::SendRecv, None, None, None);
    let (offer, pending) = sdp_l
        .apply()
        .ok_or_else(|| anyhow::anyhow!("sdp_l.apply() gab None – keine Änderungen"))?;

    println!("[sdp]  Offer erstellt ({} Bytes)", offer.to_sdp_string().len());

    let answer = rtc_r.sdp_api().accept_offer(offer)?;
    println!("[sdp]  Answer generiert ({} Bytes)", answer.to_sdp_string().len());

    rtc_l.sdp_api().accept_answer(pending, answer)?;
    println!("[sdp]  Handshake abgeschlossen – starte Run-Loop");
    println!("[sdp]  Audio-MID: {mid:?}");

    // ── Hilfsfunktion: poll_output für einen Peer drainieren ─────────────────
    // Gibt (ice_connected, packets_received_delta) zurück.
    // Pakete von diesem Peer werden über `send_sock` an `dest` weitergeleitet.

    // ── Run-Loop ─────────────────────────────────────────────────────────────
    sock_l.set_nonblocking(true)?;
    sock_r.set_nonblocking(true)?;

    let mut buf = vec![0u8; 2048];
    let mut l_connected = false;
    let mut r_connected = false;
    let mut packets_received = 0u32;
    let mut audio_pt = None;
    let mut media_ts: u64 = 0;
    let deadline = Instant::now() + Duration::from_secs(10);

    loop {
        if Instant::now() > deadline {
            bail!("Timeout nach 10s – {} RTP-Pakete empfangen", packets_received);
        }

        let now = Instant::now();

        // ── L drainieren ─────────────────────────────────────────────────
        loop {
            match rtc_l.poll_output()? {
                Output::Timeout(_) => break,
                Output::Transmit(t) => { sock_l.send_to(&t.contents, addr_r)?; }
                Output::Event(Event::IceConnectionStateChange(s)) => {
                    println!("[L] ICE: {s:?}");
                    if matches!(s, IceConnectionState::Connected | IceConnectionState::Completed) {
                        l_connected = true;
                    }
                    if s == IceConnectionState::Disconnected { bail!("L: ICE Disconnected"); }
                }
                Output::Event(e) => { println!("[L] {e:?}"); }
            }
        }

        // ── R drainieren ─────────────────────────────────────────────────
        loop {
            match rtc_r.poll_output()? {
                Output::Timeout(_) => break,
                Output::Transmit(t) => { sock_r.send_to(&t.contents, addr_l)?; }
                Output::Event(Event::IceConnectionStateChange(s)) => {
                    println!("[R] ICE: {s:?}");
                    if matches!(s, IceConnectionState::Connected | IceConnectionState::Completed) {
                        r_connected = true;
                    }
                    if s == IceConnectionState::Disconnected { bail!("R: ICE Disconnected"); }
                }
                Output::Event(Event::MediaData(data)) => {
                    packets_received += 1;
                    if packets_received == 1 || packets_received % 10 == 0 {
                        println!("[R] MediaData #{packets_received} – {} Bytes, time={:?}", data.data.len(), data.time);
                    }
                    if packets_received >= 5 {
                        println!("\n✓ SPIKE VALIDIERT: {} Pakete empfangen (PT={:?}, 48kHz Opus), Verbindung steht.", packets_received, data.pt);
                        return Ok(());
                    }
                }
                Output::Event(e) => { println!("[R] {e:?}"); }
            }
        }

        // ── UDP-Pakete einlesen (L) ───────────────────────────────────────
        loop {
            match sock_l.recv_from(&mut buf) {
                Ok((n, src)) => {
                    let data: str0m::net::DatagramRecv = buf[..n].try_into()?;
                    rtc_l.handle_input(Input::Receive(
                        Instant::now(),
                        Receive { proto: Protocol::Udp, source: src, destination: addr_l, contents: data },
                    ))?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }

        // ── UDP-Pakete einlesen (R) ───────────────────────────────────────
        loop {
            match sock_r.recv_from(&mut buf) {
                Ok((n, src)) => {
                    let data: str0m::net::DatagramRecv = buf[..n].try_into()?;
                    rtc_r.handle_input(Input::Receive(
                        Instant::now(),
                        Receive { proto: Protocol::Udp, source: src, destination: addr_r, contents: data },
                    ))?;
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(e) => return Err(e.into()),
            }
        }

        // ── Timeout vorschieben ───────────────────────────────────────────
        rtc_l.handle_input(Input::Timeout(now))?;
        rtc_r.handle_input(Input::Timeout(now))?;

        // ── Audio senden sobald beide verbunden ───────────────────────────
        // WICHTIG: write() ist eine Mutation → danach sofort L drainieren
        if l_connected && r_connected {
            // PT beim ersten Mal ermitteln (nach DTLS-Handshake verfügbar)
            if audio_pt.is_none() {
                if let Some(writer) = rtc_l.writer(mid) {
                    if let Some(p) = writer.payload_params().next() {
                        audio_pt = Some(p.pt());
                        println!("[L] Audio-PT ermittelt: {:?}", audio_pt.unwrap());
                    }
                }
            }

            if let Some(pt) = audio_pt {
                media_ts += 960; // 20ms bei 48kHz

                // Fake Opus comfort noise – 6 Byte reichen für den Test
                let fake_opus: &[u8] = &[0xf8, 0xff, 0xfe, 0x00, 0x00, 0x00];

                if let Some(writer) = rtc_l.writer(mid) {
                    let _ = writer.write(
                        pt,
                        Instant::now(),
                        MediaTime::new(media_ts, Frequency::FORTY_EIGHT_KHZ),
                        fake_opus,
                    );
                }

                // Nach write() MUSS L komplett drainiert werden (single-mutation invariant)
                loop {
                    match rtc_l.poll_output()? {
                        Output::Timeout(_) => break,
                        Output::Transmit(t) => { sock_l.send_to(&t.contents, addr_r)?; }
                        Output::Event(e) => { println!("[L post-write] {e:?}"); }
                    }
                }
            }
        }

        std::thread::sleep(Duration::from_millis(20));
    }
}

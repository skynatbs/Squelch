/// Spike 001 – str0m WebRTC Loopback
///
/// Two Rtc instances (L = offerer, R = answerer) in one process.
/// SDP handshake is exchanged in-process (simulates Matrix signaling).
/// ICE runs over two loopback UDP sockets.
/// L sends synthetic audio bytes (fake Opus), R receives them.
///
/// Success criteria:
///   - Connection established (ICE Completed)
///   - R receives at least 5 MediaData packets from L
///   - No panic, no deadlock
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

    println!("[init] L bound to {addr_l}");
    println!("[init] R bound to {addr_r}");

    // ── Rtc instances ────────────────────────────────────────────────────────
    let now = Instant::now();
    let mut rtc_l = Rtc::new(now);
    let mut rtc_r = Rtc::new(now);

    // ICE candidates (host candidate = direct loopback address)
    rtc_l.add_local_candidate(Candidate::host(addr_l, "udp")?);
    rtc_r.add_local_candidate(Candidate::host(addr_r, "udp")?);
    rtc_l.add_remote_candidate(Candidate::host(addr_r, "udp")?);
    rtc_r.add_remote_candidate(Candidate::host(addr_l, "udp")?);

    // ── SDP Offer/Answer ─────────────────────────────────────────────────────
    let mut sdp_l = rtc_l.sdp_api();
    let mid = sdp_l.add_media(MediaKind::Audio, Direction::SendRecv, None, None, None);
    let (offer, pending) = sdp_l
        .apply()
        .ok_or_else(|| anyhow::anyhow!("sdp_l.apply() returned None – no changes present"))?;

    println!("[sdp]  Offer created ({} bytes)", offer.to_sdp_string().len());

    let answer = rtc_r.sdp_api().accept_offer(offer)?;
    println!("[sdp]  Answer generated ({} bytes)", answer.to_sdp_string().len());

    rtc_l.sdp_api().accept_answer(pending, answer)?;
    println!("[sdp]  Handshake complete – starting run loop");
    println!("[sdp]  Audio MID: {mid:?}");

    // ── Run loop ─────────────────────────────────────────────────────────────
    // Both peers stay single-threaded to demonstrate the sans-IO nature.
    // Strategy: alternate polling L and R, forwarding UDP between them.
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
            bail!("Timeout after 10s – {} packets received", packets_received);
        }

        let now = Instant::now();

        // ── Drain L ──────────────────────────────────────────────────────
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

        // ── Drain R ──────────────────────────────────────────────────────
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
                        println!("[R] MediaData #{packets_received} – {} bytes, time={:?}", data.data.len(), data.time);
                    }
                    if packets_received >= 5 {
                        println!("\n✓ SPIKE VALIDATED: {} packets received (PT={:?}, 48kHz Opus), connection established.", packets_received, data.pt);
                        return Ok(());
                    }
                }
                Output::Event(e) => { println!("[R] {e:?}"); }
            }
        }

        // ── Read incoming UDP for L ───────────────────────────────────────
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

        // ── Read incoming UDP for R ───────────────────────────────────────
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

        // ── Advance time ──────────────────────────────────────────────────
        rtc_l.handle_input(Input::Timeout(now))?;
        rtc_r.handle_input(Input::Timeout(now))?;

        // ── Send audio once both peers are connected ───────────────────────
        // IMPORTANT: write() is a mutation → drain L immediately afterward
        if l_connected && r_connected {
            // Resolve PT on first opportunity (available after DTLS handshake)
            if audio_pt.is_none() {
                if let Some(writer) = rtc_l.writer(mid) {
                    if let Some(p) = writer.payload_params().next() {
                        audio_pt = Some(p.pt());
                        println!("[L] Audio PT resolved: {:?}", audio_pt.unwrap());
                    }
                }
            }

            if let Some(pt) = audio_pt {
                media_ts += 960; // 20ms at 48kHz

                // Fake Opus comfort noise – 6 bytes sufficient for the test
                let fake_opus: &[u8] = &[0xf8, 0xff, 0xfe, 0x00, 0x00, 0x00];

                if let Some(writer) = rtc_l.writer(mid) {
                    let _ = writer.write(
                        pt,
                        Instant::now(),
                        MediaTime::new(media_ts, Frequency::FORTY_EIGHT_KHZ),
                        fake_opus,
                    );
                }

                // After write() L MUST be fully drained (single-mutation invariant)
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

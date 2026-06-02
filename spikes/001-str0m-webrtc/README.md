# Spike 001 – str0m WebRTC Loopback

## Question

Can str0m connect two peers within a single Rust program (SDP handshake + ICE via
loopback UDP) and exchange audio packets? Is the sans-IO design manageable?

## Approach

Two `Rtc` instances in one process, connected via loopback UdpSocket.
SDP offer/answer is exchanged in-process (simulating what Matrix will do later).
ICE runs over localhost UDP. Synthetic Opus frames (6 bytes, 20ms @ 48kHz)
are sent from peer L and received by peer R as `MediaData` events.

## Output

```
[init] L binds on 127.0.0.1:34358
[init] R binds on 127.0.0.1:41534
[sdp]  Offer created (1211 bytes)
[sdp]  Answer generated (1211 bytes)
[sdp]  Handshake complete – starting run loop
[sdp]  Audio MID: Mid(LP5)
[L] ICE: Checking
[R] ICE: Checking
[L] ICE: Completed
[R] ICE: Completed
[L] Audio PT resolved: Pt(111)
[R] Connected
[R] MediaAdded(...)
[L post-write] Connected
[R] StreamPaused(..., paused: false)
[R] MediaData #1 – 6 bytes, time=MediaTime(960, Frequency(48000))

✓ SPIKE VALIDATED: 5 packets received (PT=Pt(111), 48kHz Opus), connection established.
```

---

## Verdict: VALIDATED

### What worked

- SDP offer/answer fully in Rust without external tools
- ICE Checking → Completed in < 1 second (loopback)
- DTLS handshake automatic after ICE (str0m handles it completely)
- Audio frames arrive as `MediaData` events (PT 111 = Opus, 48kHz)
- Payload timestamps monotonically increasing (960-step increments = correct 20ms frames)
- Sans-IO design is manageable: clear mutate → drain pattern, no hidden threading

### What was not immediately obvious (learning curve)

- `IceConnectionState::Completed` rather than `Connected` is the stable state
  (both mean "connected", Completed = ICE finalization after STUN checks)
- `MediaData` not `RtpPacket`: str0m 0.20 delivers assembled audio frames as
  `Event::MediaData`, not raw RTP packets (the latter requires `enable_raw_packets()`)
- `writer.write()` is a mutation → `poll_output` MUST be drained to `Timeout`
  afterward (single-mutation invariant, documented in the crate)
- `MediaAdded` fires on the peer that *receives* the offer (R), not the offerer (L)
  — but the MID is directly available from `sdp_api.add_media()`

### Surprises

- No custom DTLS handling needed — str0m negotiates certificates internally
  (rcgen generates a self-signed cert, aws-lc-rs for crypto)
- `Frequency::FORTY_EIGHT_KHZ` as a constant rather than raw `48_000`
- `MediaTime::new(numer: u64, denom: Frequency)` — not `i64`

### Recommendation for the real build

- str0m is the right choice. The sans-IO design fits perfectly with the planned
  cpal audio thread: cpal delivers PCM samples, we encode with opus-rs to Opus,
  write via `writer.write()`, drain. The run loop runs in its own `std::thread`,
  no Tokio required.
- Matrix signaling replaces the in-process SDP/ICE exchange — same offer/answer
  API, only the transport runs over Matrix events.
- For 4 peers: 3 connections per peer (mesh), each with its own `Rtc` instance.
  Manageable in `squelch-webrtc`.
- webrtc-rs does not need to be spiked — str0m is clearly preferable.

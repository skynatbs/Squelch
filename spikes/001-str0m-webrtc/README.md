# Spike 001 – str0m WebRTC Loopback

## Frage

Kann str0m zwei Peers in einem Rust-Programm verbinden (SDP-Handshake + ICE via
loopback UDP) und Audio-Pakete austauschen? Ist das Sans-IO Design handhabbar?

## Ansatz

Zwei `Rtc`-Instanzen in einem Prozess, verbunden via loopback UdpSocket.
SDP Offer/Answer wird direkt in-process getauscht (simuliert was Matrix später macht).
ICE läuft über localhost UDP. Synthetische Opus-Frames (6 Byte, 20ms @ 48kHz)
werden von Peer L gesendet und von Peer R als `MediaData`-Events empfangen.

## Ergebnis (Ausgabe)

```
[init] L bindet auf 127.0.0.1:34358
[init] R bindet auf 127.0.0.1:41534
[sdp]  Offer erstellt (1211 Bytes)
[sdp]  Answer generiert (1211 Bytes)
[sdp]  Handshake abgeschlossen – starte Run-Loop
[sdp]  Audio-MID: Mid(LP5)
[L] ICE: Checking
[R] ICE: Checking
[L] ICE: Completed
[R] ICE: Completed
[L] Audio-PT ermittelt: Pt(111)
[R] Connected
[R] MediaAdded(...)
[L post-write] Connected
[R] StreamPaused(..., paused: false)
[R] MediaData #1 – 6 Bytes, time=MediaTime(960, Frequency(48000))

✓ SPIKE VALIDIERT: 5 Pakete empfangen (PT=Pt(111), 48kHz Opus), Verbindung steht.
```

---

## Verdict: VALIDIERT

### Was funktioniert hat

- SDP Offer/Answer vollständig in Rust ohne externe Tools
- ICE Checking → Completed in < 1 Sekunde (loopback)
- DTLS-Handshake automatisch nach ICE (str0m übernimmt das vollständig)
- Audio-Frames kommen als `MediaData`-Events an (PT 111 = Opus, 48kHz)
- Payload-Timestamps monoton steigend (960er-Schritte = korrekte 20ms-Frames)
- Sans-IO Design handhabbar: klares mutate → drain-Muster, kein verstecktes Threading

### Was nicht sofort offensichtlich war (Lernkurve)

- `IceConnectionState::Completed` statt `Connected` ist der stabile Zustand
  (beide bedeuten "verbunden", Completed = ICE-Abschluss nach STUN-Checks)
- `MediaData` statt `RtpPacket`: str0m 0.20 liefert assemblierten Audio-Frame
  als `Event::MediaData`, nicht als rohe RTP-Pakete (letzteres erfordert
  `enable_raw_packets()`)
- `writer.write()` ist eine Mutation → danach MUSS `poll_output` bis `Timeout`
  drainiert werden (single-mutation invariant, in der Doku beschrieben)
- `MediaAdded` feuert auf dem Peer der das Offer *empfängt* (R), nicht auf
  dem Offerer (L) – MID ist aber aus `sdp_api.add_media()` direkt bekannt

### Surprises

- Kein eigenes DTLS-Handling nötig – str0m verhandelt Zertifikate intern
  (rcgen generiert self-signed cert, aws-lc-rs für Crypto)
- `Frequency::FORTY_EIGHT_KHZ` als Konstante statt roher `48_000`-Zahl
- `MediaTime::new(numer: u64, denom: Frequency)` – nicht `i64`

### Recommendation für den echten Build

- str0m ist die richtige Wahl. Das Sans-IO Design passt exzellent zum
  geplanten cpal Audio-Thread: cpal liefert PCM-Samples, wir kodieren
  mit opus-rs zu Opus, schreiben via `writer.write()`, drainieren.
  Der Run-Loop läuft in einem eigenen `std::thread`, kein Tokio nötig.
- Matrix-Signaling ersetzt den in-process SDP/ICE-Tausch – gleiche
  Offer/Answer API, nur die Übertragung läuft über Matrix-Events.
- Für 4 Peers: 3 Verbindungen pro Peer (Mesh), jede mit eigenem `Rtc`.
  Das ist manageable in `squelch-webrtc`.
- webrtc-rs muss nicht mehr gespiked werden – str0m ist klar vorzuziehen.

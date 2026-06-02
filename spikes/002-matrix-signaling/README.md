# Spike 002 – Matrix Signaling via To-Device Messages

## Question

Can matrix-rust-sdk be used to:
1. Log in two clients programmatically?
2. Create a Matrix room and have both peers join it?
3. Exchange WebRTC SDP offer/answer and ICE candidates as unencrypted to-device messages?
4. Is the API manageable for a background signaling thread?

## Homeserver

asra.gr — open registration via m.login.terms + m.login.dummy (no CAPTCHA).
Accounts were pre-registered manually; credentials are stored in `.env` (gitignored).

Note: asra.gr has aggressive rate-limiting on registration (~2-3 min cooldown per account).
For development, pre-register accounts once and reuse them. For production, users bring
their own Matrix account from any homeserver.

## Approach

Two `Client` instances (A = offerer, B = answerer) with pre-registered accounts.
A creates a room, invites B. B joins.
A sends a fake SDP offer as a to-device message to B's device.
B responds with a fake SDP answer and a fake ICE candidate.
A receives both in a single polling loop that handles multiple events per sync batch.

## Output

```
[init] User A: @squelch_spike_a_y4vrl1qs:asra.gr
[init] User B: @squelch_spike_b_p6an4ge7:asra.gr
[login] Logging in A...
[login] Logging in B...
[login]  A: @squelch_spike_a_y4vrl1qs:asra.gr device=AHXLEROOUM
[login]  B: @squelch_spike_b_p6an4ge7:asra.gr device=SVUAOPCCLF
[sync] Initial sync...
[sync] Done
[room] A creates squad room and invites B...
[room] Created: !r8WtDSENs6qIrfKO0qThhAeoKfBAs_MKkNGBy52b69s
[room] B joined
[signal] A → B: SDP offer
[sync]   B waiting for offer...
[B] ✓ SDP offer received: call_id="spike-002-call-1"
[signal] B → A: SDP answer
[signal] B → A: ICE candidate
[sync]   A waiting for answer + ICE...
[A] ✓ SDP answer received: call_id="spike-002-call-1"
[A] ✓ ICE candidate received: "candidate:1 1 UDP 2130706431 192.168.1.100 54400 typ host"

✓ SPIKE VALIDATED:
  - Both clients logged in on https://asra.gr
  - Room created and joined
  - SDP offer delivered A → B via to-device
  - SDP answer delivered B → A via to-device
  - ICE candidate delivered B → A via to-device
  matrix-rust-sdk is usable as a P2P signaling bus for Squelch.
```

---

## Verdict: VALIDATED

### What worked

- Login with `matrix_auth().login_username().send()` — clean and straightforward
- `Client::send(send_to_device::Request)` works for unencrypted custom to-device events
- Custom event types (`io.squelch.*`) pass through without issues
- `sync_once()` with `SyncSettings::default().timeout(Duration)` reliably delivers
  to-device events in `SyncResponse.to_device` as `ProcessedToDeviceEvent::PlainText`
- Multiple to-device events sent in sequence arrive in the same or next sync batch
- Room creation and invite/join flow works without issues
- Total round-trip time for full signaling exchange: ~2 seconds (including network)

### What was not immediately obvious (learning curve)

- `matrix_sdk::ruma::uiaa` does not exist — `uiaa` lives in `ruma::api::client::uiaa`
  (requires adding `ruma` as a direct dependency)
- `ProcessedToDeviceEvent` is in `matrix-sdk-common`, not re-exported by `matrix-sdk`
  (requires adding `matrix-sdk-common` as a direct dependency)
- `matrix_sdk::Client::register()` logs in automatically — do not call `login_username`
  afterward or it panics with `AlreadyInitializedError`
- SQLite store caches device IDs — delete the store directory between runs if you change
  accounts or create a new login session
- The 404 "Account data not found" errors on login are harmless — matrix-sdk tries to
  load key-backup state that doesn't exist for fresh accounts
- `send_to_device::Request::new_raw()` not `::new()` — the struct is built with `new_raw`
- `SyncResponse.to_device` must be iterated carefully: multiple events can arrive in
  one sync batch; a sequential `wait_for_to_device` loop will miss earlier events

### Surprises

- matrix-sdk 0.18's `Client::builder().server_name_or_homeserver_url()` is required
  instead of `.homeserver_url()` to avoid hanging on well-known discovery
- The `experimental-send-custom-to-device` feature flag is required to send custom
  unencrypted to-device events (would be hidden E2EE in production)
- End-to-end latency for to-device message delivery: ~200-500ms on asra.gr

### What this means for squelch-matrix

Matrix is confirmed as a viable signaling bus. The `squelch-matrix` crate will:

1. Use `matrix-sdk` for login, room management, and event sending
2. Use `ProcessedToDeviceEvent::PlainText` for unencrypted signaling (MVP)
   or `ProcessedToDeviceEvent::Decrypted` for E2EE signaling (post-MVP)
3. Define custom event types: `io.squelch.sdp_offer`, `io.squelch.sdp_answer`,
   `io.squelch.ice_candidate` (+ optionally `io.squelch.call_member` for presence)
4. Run a background `sync_once` loop in a dedicated thread, dispatching events
   to str0m via a channel (validated in Spike 001)
5. No dependency on MatrixRTC/LiveKit — pure to-device P2P signaling

### Recommendation for the real build

- `squelch-matrix` crate: `matrix-sdk` + `matrix-sdk-common` + `ruma` (for uiaa types)
- Separate thread for the sync loop, channels for signaling messages in/out
- One Matrix room per squad, one to-device message per peer-pair per signal
- For 4 peers (2 duos): A↔C, A↔D, B↔C, B↔D = 4 signaling channels, all via same room
- Spike 003 (audio mixing) is the last open risk before the workspace can be structured
"""

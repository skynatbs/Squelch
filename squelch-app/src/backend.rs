//! Backend — async state machine that manages Matrix login, squad room,
//! audio pipeline, and WebRTC peer connections.
//!
//! The UI only talks to the `Backend` via `BackendCmd` and reads status from
//! `BackendState`. No matrix-sdk or cpal types leak into the UI layer.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use matrix_sdk::ruma::OwnedRoomId;
use squelch_audio::{AudioConfig, AudioPipeline, PttState};
use squelch_matrix::{MatrixClient, SignalingEvent, client::SyncHandle, event_types};
use squelch_webrtc::{PeerConnection, PeerRole};
use tokio::{
    runtime::Handle,
    sync::{
        mpsc::{self, UnboundedSender},
        oneshot,
    },
};
use tracing::{error, info, warn};

use crate::config::AppConfig;

// ── Commands (UI → Backend) ────────────────────────────────────────────────

#[derive(Debug)]
pub enum BackendCmd {
    Login {
        homeserver: String,
        username: String,
        password: String,
    },
    CreateRoom {
        name: String,
    },
    JoinRoom {
        room_id: String,
    },
    LeaveSession,
    DisbandSquad,
    Logout,
}

// ── Status (Backend → UI) ─────────────────────────────────────────────────

#[derive(Debug, Clone, Default)]
pub struct BackendState {
    pub status: String,
    pub error: Option<String>,
    pub user_id: Option<String>,
    pub room_id: Option<String>,
    pub audio_active: bool,
    pub logged_in: bool,
    /// Connected peer user IDs (for the running screen member list)
    pub peers: Vec<String>,
}

// ── Backend handle ────────────────────────────────────────────────────────

#[derive(Clone)]
pub struct Backend {
    cmd_tx: UnboundedSender<BackendCmd>,
    pub state: Arc<Mutex<BackendState>>,
}

impl Backend {
    pub fn spawn(ptt: PttState, _cfg: AppConfig, rt: Handle) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let state = Arc::new(Mutex::new(BackendState {
            status: "Not connected".into(),
            ..Default::default()
        }));

        // Audio pipeline on its own thread (cpal not Send)
        let audio_state = state.clone();
        let ptt_audio = ptt.clone();
        std::thread::Builder::new()
            .name("squelch-audio".into())
            .spawn(
                move || match AudioPipeline::start(AudioConfig { ptt: ptt_audio }) {
                    Ok(_) => {
                        audio_state.lock().unwrap().audio_active = true;
                        loop {
                            std::thread::sleep(std::time::Duration::from_secs(3600));
                        }
                    }
                    Err(e) => warn!("audio pipeline failed: {e}"),
                },
            )
            .expect("failed to spawn audio thread");

        let state_clone = state.clone();
        rt.spawn(async move { run(cmd_rx, state_clone).await });

        Self { cmd_tx, state }
    }

    pub fn send(&self, cmd: BackendCmd) {
        if let Err(e) = self.cmd_tx.send(cmd) {
            error!("backend cmd send error: {e}");
        }
    }

    pub fn snapshot(&self) -> BackendState {
        self.state.lock().unwrap().clone()
    }
}

// ── Backend task ──────────────────────────────────────────────────────────

struct Inner {
    matrix: Option<MatrixClient>,
    _sync: Option<SyncHandle>,
    room_id: Option<OwnedRoomId>,
    /// shutdown senders for active WebRTC run loops
    peer_shutdown: HashMap<String, oneshot::Sender<()>>,
}

async fn run(mut cmd_rx: mpsc::UnboundedReceiver<BackendCmd>, state: Arc<Mutex<BackendState>>) {
    let mut inner = Inner {
        matrix: None,
        _sync: None,
        room_id: None,
        peer_shutdown: HashMap::new(),
    };

    while let Some(cmd) = cmd_rx.recv().await {
        state.lock().unwrap().error = None;

        match cmd {
            // ── Login ──────────────────────────────────────────────────────
            BackendCmd::Login {
                homeserver,
                username,
                password,
            } => {
                set_status(&state, "Logging in to Matrix…");

                let cfg = squelch_matrix::MatrixConfig {
                    homeserver: homeserver.clone(),
                    username: username.clone(),
                    password: password.clone(),
                    store_path: dirs::data_local_dir()
                        .unwrap_or_else(|| std::path::PathBuf::from("."))
                        .join("squelch")
                        .join("matrix-store"),
                };

                match MatrixClient::login(&cfg).await {
                    Err(e) => {
                        error!("matrix login failed: {e}");
                        set_error(&state, &format!("Login failed: {e}"));
                    }
                    Ok(client) => {
                        let uid = client.user_id().to_string();
                        info!(%uid, "matrix login ok");

                        let (sync_handle, mut sig_rx) = client.start_sync();

                        // Dispatch signaling events
                        let state2 = state.clone();
                        let client2 = client.clone();
                        tokio::spawn(async move {
                            while let Some(event) = sig_rx.recv().await {
                                handle_signaling_event(event, &state2, &client2).await;
                            }
                        });

                        inner.matrix = Some(client);
                        inner._sync = Some(sync_handle);

                        set_status(&state, &format!("Logged in as {uid}"));
                        let mut s = state.lock().unwrap();
                        s.logged_in = true;
                        s.user_id = Some(uid);
                        s.error = None;
                    }
                }
            }

            // ── Create room ────────────────────────────────────────────────
            BackendCmd::CreateRoom { name } => {
                let Some(client) = &inner.matrix else {
                    set_error(&state, "Not logged in.");
                    continue;
                };
                set_status(&state, "Creating squad room…");
                match client.create_squad_room(&name, &[]).await {
                    Err(e) => set_error(&state, &format!("Could not create room: {e}")),
                    Ok(room_id) => {
                        info!(%room_id, "squad room created");
                        let id_str = room_id.to_string();
                        inner.room_id = Some(room_id);
                        state.lock().unwrap().room_id = Some(id_str.clone());
                        set_status(&state, &format!("Room ready — share ID: {id_str}"));
                    }
                }
            }

            // ── Join room ──────────────────────────────────────────────────
            BackendCmd::JoinRoom { room_id } => {
                let Some(client) = &inner.matrix else {
                    set_error(&state, "Not logged in.");
                    continue;
                };
                set_status(&state, "Joining squad room…");

                let parsed = match room_id.parse::<OwnedRoomId>() {
                    Ok(id) => id,
                    Err(_) => {
                        set_error(&state, "Invalid room ID.");
                        continue;
                    }
                };

                match client.join_room(&parsed).await {
                    Err(e) => set_error(&state, &format!("Could not join room: {e}")),
                    Ok(()) => {
                        info!(%parsed, "joined squad room");

                        // Discover existing members and offer WebRTC to each
                        let my_uid = client.user_id().to_string();
                        if let Ok(members) = client.room_members(&parsed).await {
                            for member_uid in &members {
                                let uid_str = member_uid.to_string();
                                if uid_str == my_uid {
                                    continue;
                                }
                                connect_to_peer(
                                    client,
                                    member_uid,
                                    &parsed,
                                    &state,
                                    &mut inner.peer_shutdown,
                                )
                                .await;
                            }
                        }

                        let id_str = parsed.to_string();
                        inner.room_id = Some(parsed);
                        state.lock().unwrap().room_id = Some(id_str);
                        set_status(&state, "Joined squad room.");
                    }
                }
            }

            // ── Leave session ──────────────────────────────────────────────
            BackendCmd::LeaveSession => {
                shutdown_peers(&mut inner.peer_shutdown);
                if let (Some(client), Some(room_id)) = (&inner.matrix, &inner.room_id) {
                    let _ = client.leave_room(room_id).await;
                }
                inner.room_id = None;
                let mut s = state.lock().unwrap();
                s.room_id = None;
                s.peers = vec![];
                drop(s);
                set_status(&state, "Left session. Room still exists.");
            }

            // ── Disband squad ──────────────────────────────────────────────
            BackendCmd::DisbandSquad => {
                shutdown_peers(&mut inner.peer_shutdown);
                if let (Some(client), Some(room_id)) = (&inner.matrix, &inner.room_id) {
                    // Send disband to all members
                    if let Ok(members) = client.room_members(room_id).await {
                        let my_uid = client.user_id().to_string();
                        for m in &members {
                            if m.as_str() == my_uid {
                                continue;
                            }
                            let _ = client
                                .send_to_all_devices(
                                    m,
                                    event_types::DISBAND,
                                    serde_json::json!({ "room_id": room_id.as_str() }),
                                )
                                .await;
                        }
                    }
                    let _ = client.leave_room(room_id).await;
                }
                inner.room_id = None;
                let mut s = state.lock().unwrap();
                s.room_id = None;
                s.peers = vec![];
                drop(s);
                set_status(&state, "Squad disbanded.");
            }

            // ── Logout ─────────────────────────────────────────────────────
            BackendCmd::Logout => {
                shutdown_peers(&mut inner.peer_shutdown);
                if let Some(client) = &inner.matrix {
                    let _ = client.logout().await;
                }
                inner = Inner {
                    matrix: None,
                    _sync: None,
                    room_id: None,
                    peer_shutdown: HashMap::new(),
                };
                let mut s = state.lock().unwrap();
                *s = BackendState {
                    status: "Logged out.".into(),
                    ..Default::default()
                };
            }
        }
    }
}

// ── WebRTC helpers ────────────────────────────────────────────────────────

/// Initiate a WebRTC connection to a peer: send SDP offer via Matrix.
async fn connect_to_peer(
    client: &MatrixClient,
    peer_uid: &matrix_sdk::ruma::OwnedUserId,
    room_id: &OwnedRoomId,
    state: &Arc<Mutex<BackendState>>,
    shutdowns: &mut HashMap<String, oneshot::Sender<()>>,
) {
    let uid_str = peer_uid.to_string();
    if shutdowns.contains_key(&uid_str) {
        info!(%uid_str, "already connected, skipping");
        return;
    }

    let (conn, _audio_rx) = match PeerConnection::new(uid_str.clone(), PeerRole::Offerer) {
        Ok(c) => c,
        Err(e) => {
            warn!("PeerConnection::new failed: {e}");
            return;
        }
    };

    let offer_sdp = match conn.create_offer() {
        Ok(s) => s,
        Err(e) => {
            warn!("create_offer failed: {e}");
            return;
        }
    };

    let call_id = format!("{}-{}", room_id.as_str(), uid_str);
    let payload = squelch_matrix::SdpMessage {
        call_id: call_id.clone(),
        room_id: room_id.to_string(),
        sdp: offer_sdp,
    };

    if let Err(e) = client
        .send_to_all_devices(
            peer_uid,
            event_types::SDP_OFFER,
            serde_json::to_value(&payload).unwrap_or_default(),
        )
        .await
    {
        warn!("send SDP offer failed: {e}");
        return;
    }

    info!(%uid_str, "SDP offer sent, waiting for answer");

    let (shutdown_tx, shutdown_rx) = oneshot::channel();
    shutdowns.insert(uid_str.clone(), shutdown_tx);

    std::thread::Builder::new()
        .name(format!("webrtc-{uid_str}"))
        .spawn(move || conn.run(shutdown_rx))
        .ok();

    state.lock().unwrap().peers.push(uid_str);
}

/// Abort all active WebRTC run loops.
fn shutdown_peers(shutdowns: &mut HashMap<String, oneshot::Sender<()>>) {
    for (uid, tx) in shutdowns.drain() {
        info!(%uid, "shutting down peer connection");
        let _ = tx.send(());
    }
}

// ── Signaling event handler ───────────────────────────────────────────────

async fn handle_signaling_event(
    event: SignalingEvent,
    state: &Arc<Mutex<BackendState>>,
    client: &MatrixClient,
) {
    match event {
        SignalingEvent::SdpOffer { from, payload } => {
            info!(sender = %from, "received SDP offer — answering");

            let peer_uid: matrix_sdk::ruma::OwnedUserId = match from.parse() {
                Ok(u) => u,
                Err(_) => {
                    warn!("invalid user ID in SDP offer: {from}");
                    return;
                }
            };

            let (conn, _audio_rx) = match PeerConnection::new(from.clone(), PeerRole::Answerer) {
                Ok(c) => c,
                Err(e) => {
                    warn!("PeerConnection::new (answerer) failed: {e}");
                    return;
                }
            };

            let answer_sdp = match conn.accept_offer(&payload.sdp) {
                Ok(s) => s,
                Err(e) => {
                    warn!("accept_offer failed: {e}");
                    return;
                }
            };

            let answer = squelch_matrix::SdpMessage {
                call_id: payload.call_id,
                room_id: payload.room_id.clone(),
                sdp: answer_sdp,
            };

            if let Err(e) = client
                .send_to_all_devices(
                    &peer_uid,
                    event_types::SDP_ANSWER,
                    serde_json::to_value(&answer).unwrap_or_default(),
                )
                .await
            {
                warn!("send SDP answer failed: {e}");
                return;
            }

            let (_shutdown_tx, shutdown_rx) = oneshot::channel();
            let uid_str = from.clone();
            std::thread::Builder::new()
                .name(format!("webrtc-{uid_str}"))
                .spawn(move || conn.run(shutdown_rx))
                .ok();
            // TODO Phase 5b: store shutdown_tx so we can abort this connection

            let mut s = state.lock().unwrap();
            if !s.peers.contains(&uid_str) {
                s.peers.push(uid_str);
            }
        }

        SignalingEvent::SdpAnswer { from, payload } => {
            info!(sender = %from, "received SDP answer");
            // The PeerConnection's run loop is already started (from connect_to_peer).
            // We need to feed the answer to it. For now we log — full answer routing
            // requires storing PeerConnection references (Phase 5b).
            // TODO Phase 5b: route answer to existing PeerConnection
            let _ = payload;
        }

        SignalingEvent::IceCandidate { from, payload } => {
            info!(sender = %from, "received ICE candidate");
            // TODO Phase 5b: route ICE candidate to PeerConnection
            let _ = payload;
        }

        SignalingEvent::Disband { from, room_id } => {
            warn!(sender = %from, %room_id, "disband event received — leaving room");
            state.lock().unwrap().room_id = None;
            state.lock().unwrap().peers = vec![];
            set_status(state, "Squad disbanded by leader.");
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────

fn set_status(state: &Arc<Mutex<BackendState>>, msg: &str) {
    state.lock().unwrap().status = msg.to_owned();
}

fn set_error(state: &Arc<Mutex<BackendState>>, err: &str) {
    let mut s = state.lock().unwrap();
    s.error = Some(err.to_owned());
    s.status = "Error".into();
}

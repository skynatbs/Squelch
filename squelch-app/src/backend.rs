//! Backend — async state machine that manages Matrix login, squad room,
//! audio pipeline, and (later) WebRTC peer mesh.
//!
//! The UI only talks to the `Backend` via `BackendCmd` and reads status from
//! `BackendState`. No matrix-sdk or cpal types leak into the UI layer.

use std::sync::{Arc, Mutex};

use squelch_audio::{AudioConfig, AudioHandles, AudioPipeline, PttState};
use squelch_matrix::{MatrixClient, SignalingEvent, client::SyncHandle};

// Ruma types via matrix-sdk re-export (access through squelch-matrix dependency)
use matrix_sdk::ruma::OwnedRoomId;
use tokio::{
    runtime::Handle,
    sync::mpsc::{self, UnboundedSender},
};
use tracing::{error, info, warn};

use crate::config::AppConfig;

// ── Commands (UI → Backend) ────────────────────────────────────────────────

/// Commands the UI sends to the backend task.
#[derive(Debug)]
pub enum BackendCmd {
    /// Log in to Matrix and start the audio pipeline.
    Login {
        homeserver: String,
        username: String,
        password: String,
    },
    /// Create a new squad room and become its leader.
    CreateRoom { name: String },
    /// Join an existing squad room by ID.
    JoinRoom { room_id: String },
    /// Leave the current session (room stays).
    LeaveSession,
    /// Disband the squad (leader only — sends disband to all members, leaves room).
    DisbandSquad,
    /// Log out and stop everything.
    Logout,
}

// ── Status (Backend → UI) ─────────────────────────────────────────────────

/// Current backend status — read by the UI on every frame.
#[derive(Debug, Clone, Default)]
pub struct BackendState {
    /// Human-readable status line shown in the UI.
    pub status: String,
    /// Non-empty when the last action produced an error.
    pub error: Option<String>,
    /// Matrix user ID after login (e.g. `@alice:matrix.org`).
    pub user_id: Option<String>,
    /// Active room ID (set after create/join).
    pub room_id: Option<String>,
    /// Whether the audio pipeline is active.
    pub audio_active: bool,
    /// Whether we are currently logged in to Matrix.
    pub logged_in: bool,
}

// ── Backend handle ────────────────────────────────────────────────────────

/// Handle used by the UI to send commands and read state.
#[derive(Clone)]
pub struct Backend {
    cmd_tx: UnboundedSender<BackendCmd>,
    /// Shared state — written by the backend task, read by the UI.
    pub state: Arc<Mutex<BackendState>>,
}

impl Backend {
    /// Spawn the backend task on the given Tokio runtime handle.
    ///
    /// The `AudioPipeline` is started in a dedicated `std::thread` because
    /// cpal streams are not `Send` on all platforms (Linux/ALSA especially).
    pub fn spawn(ptt: PttState, cfg: AppConfig, rt: Handle) -> Self {
        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let state = Arc::new(Mutex::new(BackendState {
            status: "Not connected".into(),
            ..Default::default()
        }));

        // Start audio pipeline on a dedicated thread (cpal not Send)
        let audio_state = state.clone();
        let ptt_audio = ptt.clone();
        std::thread::Builder::new()
            .name("squelch-audio".into())
            .spawn(move || {
                match AudioPipeline::start(AudioConfig { ptt: ptt_audio }) {
                    Ok((_pipeline, _handles)) => {
                        audio_state.lock().unwrap().audio_active = true;
                        // Keep pipeline alive by blocking this thread
                        loop {
                            std::thread::sleep(std::time::Duration::from_secs(3600));
                        }
                    }
                    Err(e) => {
                        warn!("audio pipeline failed to start: {e}");
                    }
                }
            })
            .expect("failed to spawn audio thread");

        let state_clone = state.clone();
        rt.spawn(async move {
            run(cmd_rx, state_clone, ptt, cfg).await;
        });

        Self { cmd_tx, state }
    }

    /// Send a command to the backend.
    pub fn send(&self, cmd: BackendCmd) {
        if let Err(e) = self.cmd_tx.send(cmd) {
            error!("backend cmd send error: {e}");
        }
    }

    /// Read a snapshot of the current backend state.
    pub fn snapshot(&self) -> BackendState {
        self.state.lock().unwrap().clone()
    }
}

// ── Backend task ──────────────────────────────────────────────────────────

struct BackendInner {
    matrix: Option<MatrixClient>,
    _handles: Option<AudioHandles>,
    _sync: Option<SyncHandle>,
    room_id: Option<OwnedRoomId>,
}

async fn run(
    mut cmd_rx: mpsc::UnboundedReceiver<BackendCmd>,
    state: Arc<Mutex<BackendState>>,
    _ptt: PttState,
    _cfg: AppConfig,
) {
    let mut inner = BackendInner {
        matrix: None,
        _handles: None,
        _sync: None,
        room_id: None,
    };

    let set_status = |state: &Arc<Mutex<BackendState>>, msg: &str| {
        state.lock().unwrap().status = msg.to_owned();
    };
    let set_error = |state: &Arc<Mutex<BackendState>>, err: &str| {
        state.lock().unwrap().error = Some(err.to_owned());
        state.lock().unwrap().status = "Error".into();
    };
    let clear_error = |state: &Arc<Mutex<BackendState>>| {
        state.lock().unwrap().error = None;
    };

    while let Some(cmd) = cmd_rx.recv().await {
        clear_error(&state);
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

                        // Start sync loop
                        let (sync_handle, mut sig_rx) = client.start_sync();

                        // Spawn signaling event dispatcher
                        let state2 = state.clone();
                        tokio::spawn(async move {
                            while let Some(event) = sig_rx.recv().await {
                                handle_signaling_event(event, &state2).await;
                            }
                        });

                        inner.matrix = Some(client);
                        inner._sync = Some(sync_handle);

                        // Audio pipeline is already running (started in Backend::spawn)
                        set_status(&state, &format!("Logged in as {uid}"));
                        state.lock().unwrap().logged_in = true;
                        state.lock().unwrap().user_id = Some(uid);
                        state.lock().unwrap().error = None;
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

                let parsed = match room_id.parse::<matrix_sdk::ruma::OwnedRoomId>() {
                    Ok(id) => id,
                    Err(_) => {
                        set_error(&state, "Invalid room ID format.");
                        continue;
                    }
                };

                match client.join_room(&parsed).await {
                    Err(e) => set_error(&state, &format!("Could not join room: {e}")),
                    Ok(()) => {
                        info!(%parsed, "joined squad room");
                        let id_str = parsed.to_string();
                        inner.room_id = Some(parsed);
                        state.lock().unwrap().room_id = Some(id_str);
                        set_status(&state, "Joined squad room.");
                    }
                }
            }

            // ── Leave session ──────────────────────────────────────────────
            BackendCmd::LeaveSession => {
                let (Some(client), Some(room_id)) = (&inner.matrix, &inner.room_id) else {
                    continue;
                };
                set_status(&state, "Leaving session…");
                if let Err(e) = client.leave_room(room_id).await {
                    warn!("leave_room error: {e}");
                }
                inner.room_id = None;
                state.lock().unwrap().room_id = None;
                set_status(&state, "Left session. Room still exists.");
            }

            // ── Disband squad ──────────────────────────────────────────────
            BackendCmd::DisbandSquad => {
                let (Some(client), Some(room_id)) = (&inner.matrix, &inner.room_id) else {
                    set_error(&state, "Not in a squad room.");
                    continue;
                };
                set_status(&state, "Disbanding squad…");

                // TODO: send disband to-device to all members before leaving
                // (requires fetching room member list + their device IDs)
                // For now: just leave the room ourselves
                info!(%room_id, "disbanding squad");
                if let Err(e) = client.leave_room(room_id).await {
                    warn!("leave_room on disband: {e}");
                }
                inner.room_id = None;
                state.lock().unwrap().room_id = None;
                set_status(&state, "Squad disbanded.");
            }

            // ── Logout ─────────────────────────────────────────────────────
            BackendCmd::Logout => {
                if let Some(client) = &inner.matrix {
                    let _ = client.logout().await;
                }
                inner = BackendInner {
                    matrix: None,
                    _handles: None,
                    _sync: None,
                    room_id: None,
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

/// Handle incoming signaling events from the Matrix sync loop.
async fn handle_signaling_event(event: SignalingEvent, state: &Arc<Mutex<BackendState>>) {
    match event {
        SignalingEvent::Disband { from, room_id } => {
            // TODO: validate that `from` is the current squad leader
            warn!(sender = %from, %room_id, "received disband event — leaving room");
            // UI will pick this up on the next frame via state.room_id = None
            state.lock().unwrap().room_id = None;
            state.lock().unwrap().status = "Squad disbanded by leader.".into();
        }
        SignalingEvent::SdpOffer { from, .. } => {
            info!(sender = %from, "received SDP offer — WebRTC peer wiring is Phase 5");
        }
        SignalingEvent::SdpAnswer { from, .. } => {
            info!(sender = %from, "received SDP answer");
        }
        SignalingEvent::IceCandidate { from, .. } => {
            info!(sender = %from, "received ICE candidate");
        }
    }
}

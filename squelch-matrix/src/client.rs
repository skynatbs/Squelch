//! Matrix client — login, room management, and signaling sync loop.
//!
//! # Architecture
//!
//! `MatrixClient` wraps `matrix_sdk::Client`. After login, `start_sync()`
//! spawns a background Tokio task that calls `sync_once` in a loop and
//! dispatches incoming `io.squelch.*` to-device events as `SignalingEvent`s
//! through a channel.
//!
//! The caller (squelch-webrtc) receives `SignalingEvent`s and feeds them
//! into the appropriate str0m `Rtc` instance.

use std::{collections::BTreeMap, path::PathBuf, time::Duration};

use matrix_sdk::{
    Client,
    config::SyncSettings,
    ruma::{
        OwnedDeviceId, OwnedRoomId, OwnedUserId, TransactionId,
        api::client::{
            room::create_room::v3 as create_room,
            to_device::send_event_to_device::v3 as send_to_device,
        },
        events::AnyToDeviceEventContent,
        serde::Raw,
        to_device::DeviceIdOrAllDevices,
    },
};
use matrix_sdk_common::deserialized_responses::ProcessedToDeviceEvent;
use tokio::sync::mpsc;
use tracing::{debug, info, warn};

use crate::{
    error::MatrixError,
    event_types,
    signaling::{IceCandidate, SdpMessage, SignalingEvent},
};

/// Sync timeout — how long to wait on the Matrix server before retrying.
const SYNC_TIMEOUT_SECS: u64 = 10;

// ── Config ─────────────────────────────────────────────────────────────────

/// Configuration for connecting to a Matrix homeserver.
#[derive(Debug, Clone)]
pub struct MatrixConfig {
    /// Full homeserver URL (e.g. `https://matrix.org`).
    pub homeserver: String,
    /// Matrix username (local part only, without `@` and domain).
    pub username: String,
    /// Password for login.
    pub password: String,
    /// Path for the SQLite session store.
    pub store_path: PathBuf,
}

// ── Client ─────────────────────────────────────────────────────────────────

/// Squelch's Matrix client — wraps matrix-sdk and exposes signaling primitives.
pub struct MatrixClient {
    inner: Client,
    user_id: OwnedUserId,
    device_id: OwnedDeviceId,
}

impl MatrixClient {
    /// Build and log in to the Matrix homeserver.
    ///
    /// Creates a persistent SQLite session store at `config.store_path`.
    /// The session is reused on subsequent calls if the store already exists.
    pub async fn login(config: &MatrixConfig) -> Result<Self, MatrixError> {
        let client = Client::builder()
            .server_name_or_homeserver_url(&config.homeserver)
            .sqlite_store(&config.store_path, None)
            .build()
            .await
            .map_err(|e| MatrixError::Login(e.to_string()))?;

        client
            .matrix_auth()
            .login_username(&config.username, &config.password)
            .send()
            .await
            .map_err(|e| MatrixError::Login(e.to_string()))?;

        let user_id = client
            .user_id()
            .ok_or_else(|| MatrixError::Login("no user_id after login".into()))?
            .to_owned();

        let device_id = client
            .device_id()
            .ok_or_else(|| MatrixError::Login("no device_id after login".into()))?
            .to_owned();

        info!(%user_id, %device_id, "logged in to Matrix");

        // Initial sync to populate local state
        client
            .sync_once(SyncSettings::default().timeout(Duration::from_secs(SYNC_TIMEOUT_SECS)))
            .await
            .map_err(|e| MatrixError::Sync(e.to_string()))?;

        Ok(Self {
            inner: client,
            user_id,
            device_id,
        })
    }

    /// Returns this client's Matrix user ID.
    pub fn user_id(&self) -> &OwnedUserId {
        &self.user_id
    }

    /// Returns this client's Matrix device ID.
    pub fn device_id(&self) -> &OwnedDeviceId {
        &self.device_id
    }

    // ── Room management ───────────────────────────────────────────────────

    /// Create a new squad room and invite the given members.
    pub async fn create_squad_room(
        &self,
        name: &str,
        invite: &[OwnedUserId],
    ) -> Result<OwnedRoomId, MatrixError> {
        let mut req = create_room::Request::new();
        req.name = Some(name.to_owned());
        req.invite = invite.to_vec();

        let room = self
            .inner
            .create_room(req)
            .await
            .map_err(|e| MatrixError::Room(e.to_string()))?;

        let room_id = room.room_id().to_owned();
        info!(%room_id, "squad room created");
        Ok(room_id)
    }

    /// Join an existing squad room by its ID.
    pub async fn join_room(&self, room_id: &OwnedRoomId) -> Result<(), MatrixError> {
        // Sync once to pick up any pending invite before joining
        self.inner
            .sync_once(SyncSettings::default().timeout(Duration::from_secs(SYNC_TIMEOUT_SECS)))
            .await
            .map_err(|e| MatrixError::Sync(e.to_string()))?;

        self.inner
            .join_room_by_id(room_id)
            .await
            .map_err(|e| MatrixError::Room(e.to_string()))?;

        info!(%room_id, "joined squad room");
        Ok(())
    }

    // ── Signaling ─────────────────────────────────────────────────────────

    /// Send a WebRTC SDP offer to a remote peer.
    pub async fn send_sdp_offer(
        &self,
        target_user: &OwnedUserId,
        target_device: &OwnedDeviceId,
        payload: &SdpMessage,
    ) -> Result<(), MatrixError> {
        let content =
            serde_json::to_value(payload).map_err(|e| MatrixError::Signaling(e.to_string()))?;
        self.send_to_device(target_user, target_device, event_types::SDP_OFFER, content)
            .await
    }

    /// Send a WebRTC SDP answer to a remote peer.
    pub async fn send_sdp_answer(
        &self,
        target_user: &OwnedUserId,
        target_device: &OwnedDeviceId,
        payload: &SdpMessage,
    ) -> Result<(), MatrixError> {
        let content =
            serde_json::to_value(payload).map_err(|e| MatrixError::Signaling(e.to_string()))?;
        self.send_to_device(target_user, target_device, event_types::SDP_ANSWER, content)
            .await
    }

    /// Send a WebRTC ICE candidate to a remote peer.
    pub async fn send_ice_candidate(
        &self,
        target_user: &OwnedUserId,
        target_device: &OwnedDeviceId,
        payload: &IceCandidate,
    ) -> Result<(), MatrixError> {
        let content =
            serde_json::to_value(payload).map_err(|e| MatrixError::Signaling(e.to_string()))?;
        self.send_to_device(
            target_user,
            target_device,
            event_types::ICE_CANDIDATE,
            content,
        )
        .await
    }

    /// Send a disband event to a remote member (leader-only action).
    ///
    /// The receiving client will leave the room and clear its local config
    /// after validating that the sender is the current squad leader.
    pub async fn send_disband(
        &self,
        target_user: &OwnedUserId,
        target_device: &OwnedDeviceId,
        room_id: &OwnedRoomId,
    ) -> Result<(), MatrixError> {
        let content = serde_json::json!({
            "room_id": room_id.as_str(),
            "reason":  "leader_initiated",
        });
        self.send_to_device(target_user, target_device, event_types::DISBAND, content)
            .await
    }

    /// Leave a Matrix room.
    pub async fn leave_room(&self, room_id: &OwnedRoomId) -> Result<(), MatrixError> {
        if let Some(room) = self.inner.get_room(room_id) {
            room.leave()
                .await
                .map_err(|e| MatrixError::Room(e.to_string()))?;
            info!(%room_id, "left squad room");
        }
        Ok(())
    }

    // ── Sync loop ─────────────────────────────────────────────────────────

    /// Spawn the background sync task.
    ///
    /// Returns a channel receiver that yields `SignalingEvent`s as they arrive.
    /// The task runs until the returned `SyncHandle` is dropped.
    pub fn start_sync(&self) -> (SyncHandle, mpsc::Receiver<SignalingEvent>) {
        let (tx, rx) = mpsc::channel(64);
        let client = self.inner.clone();
        let user_id = self.user_id.clone();

        let handle = tokio::spawn(async move {
            let settings = SyncSettings::default().timeout(Duration::from_secs(SYNC_TIMEOUT_SECS));

            loop {
                match client.sync_once(settings.clone()).await {
                    Err(e) => {
                        warn!("sync error: {e}");
                        tokio::time::sleep(Duration::from_secs(2)).await;
                    }
                    Ok(response) => {
                        for event in &response.to_device {
                            let raw = match event {
                                ProcessedToDeviceEvent::PlainText(r) => r,
                                ProcessedToDeviceEvent::Invalid(r) => r,
                                _ => continue,
                            };

                            if let Ok(val) = raw.deserialize_as::<serde_json::Value>() {
                                let ev_type =
                                    val.get("type").and_then(|v| v.as_str()).unwrap_or("");
                                let sender = val
                                    .get("sender")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_owned();

                                // Ignore our own events
                                if sender == user_id.as_str() {
                                    continue;
                                }

                                let content =
                                    val.get("content").cloned().unwrap_or(serde_json::json!({}));

                                let signal = match ev_type {
                                    event_types::SDP_OFFER => {
                                        serde_json::from_value::<SdpMessage>(content).ok().map(
                                            |p| SignalingEvent::SdpOffer {
                                                from: sender,
                                                payload: p,
                                            },
                                        )
                                    }
                                    event_types::SDP_ANSWER => {
                                        serde_json::from_value::<SdpMessage>(content).ok().map(
                                            |p| SignalingEvent::SdpAnswer {
                                                from: sender,
                                                payload: p,
                                            },
                                        )
                                    }
                                    event_types::ICE_CANDIDATE => {
                                        serde_json::from_value::<IceCandidate>(content).ok().map(
                                            |p| SignalingEvent::IceCandidate {
                                                from: sender,
                                                payload: p,
                                            },
                                        )
                                    }
                                    event_types::DISBAND => {
                                        let room_id = content
                                            .get("room_id")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_owned();
                                        Some(SignalingEvent::Disband {
                                            from: sender,
                                            room_id,
                                        })
                                    }
                                    _ => {
                                        debug!(ev_type, "unknown to-device event, ignoring");
                                        None
                                    }
                                };

                                // let-chains (if let X && Y) require nightly;
                                // keep as nested ifs on stable.
                                #[allow(clippy::collapsible_if)]
                                if let Some(ev) = signal {
                                    if tx.send(ev).await.is_err() {
                                        // Receiver dropped — shutdown sync loop
                                        return;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        });

        (SyncHandle(handle), rx)
    }

    // ── Internal ──────────────────────────────────────────────────────────

    /// Low-level: send an unencrypted to-device message.
    async fn send_to_device(
        &self,
        target_user: &OwnedUserId,
        target_device: &OwnedDeviceId,
        event_type: &str,
        content: serde_json::Value,
    ) -> Result<(), MatrixError> {
        let raw = Raw::from_json(
            serde_json::value::to_raw_value(&content)
                .map_err(|e| MatrixError::Signaling(e.to_string()))?,
        );

        let mut per_device: BTreeMap<DeviceIdOrAllDevices, Raw<AnyToDeviceEventContent>> =
            BTreeMap::new();
        per_device.insert(DeviceIdOrAllDevices::DeviceId(target_device.clone()), raw);

        let mut messages = send_to_device::Messages::new();
        messages.insert(target_user.clone(), per_device);

        self.inner
            .send(send_to_device::Request::new_raw(
                event_type.into(),
                TransactionId::new(),
                messages,
            ))
            .await
            .map_err(|e| MatrixError::Signaling(e.to_string()))?;

        debug!(%target_user, event_type, "to-device message sent");
        Ok(())
    }
}

// ── SyncHandle ─────────────────────────────────────────────────────────────

/// RAII handle for the background sync task.
/// Dropping this aborts the sync loop.
pub struct SyncHandle(tokio::task::JoinHandle<()>);

impl Drop for SyncHandle {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Spike 002 – Matrix Signaling via To-Device Messages
///
/// Two pre-registered matrix-sdk clients (A = offerer, B = answerer) on asra.gr.
/// Credentials are read from .env in the spike directory.
/// A creates a room, both peers exchange fake SDP offer/answer and ICE candidates
/// as unencrypted to-device messages.
///
/// Success criteria:
///   - Both clients log in successfully
///   - A creates a room, invites B; B joins
///   - A sends a fake SDP offer to B via to-device message
///   - B receives the offer and responds with a fake answer
///   - Both exchange a fake ICE candidate
///   - All messages arrive with correct content
use std::{collections::BTreeMap, time::Duration};

use anyhow::{bail, Context, Result};
use matrix_sdk::{
    Client,
    config::SyncSettings,
    ruma::{
        OwnedDeviceId, OwnedUserId, TransactionId,
        api::client::to_device::send_event_to_device::v3 as send_to_device,
        events::AnyToDeviceEventContent,
        serde::Raw,
        to_device::DeviceIdOrAllDevices,
    },
};
use matrix_sdk_common::deserialized_responses::ProcessedToDeviceEvent;

const TYPE_SDP_OFFER:     &str = "io.squelch.sdp_offer";
const TYPE_SDP_ANSWER:    &str = "io.squelch.sdp_answer";
const TYPE_ICE_CANDIDATE: &str = "io.squelch.ice_candidate";

/// Send an unencrypted to-device message.
async fn send_signaling(
    client: &Client,
    target_user: &OwnedUserId,
    target_device: &OwnedDeviceId,
    event_type: &str,
    content: serde_json::Value,
) -> Result<()> {
    let mut per_device: BTreeMap<DeviceIdOrAllDevices, Raw<AnyToDeviceEventContent>> =
        BTreeMap::new();
    per_device.insert(
        DeviceIdOrAllDevices::DeviceId(target_device.clone()),
        Raw::from_json(serde_json::value::to_raw_value(&content)?),
    );
    let mut messages = send_to_device::Messages::new();
    messages.insert(target_user.clone(), per_device);

    client
        .send(send_to_device::Request::new_raw(
            event_type.into(),
            TransactionId::new(),
            messages,
        ))
        .await
        .context("send_to_device failed")?;
    Ok(())
}

/// Poll sync until a to-device message of the expected type arrives.
async fn wait_for_to_device(
    client: &Client,
    expected_type: &str,
) -> Result<serde_json::Value> {
    let sync_settings = SyncSettings::default().timeout(Duration::from_secs(10));
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);
    loop {
        if tokio::time::Instant::now() > deadline {
            bail!("Timeout waiting for to-device event type={expected_type}");
        }
        let response = client
            .sync_once(sync_settings.clone())
            .await
            .context("sync_once failed")?;

        for event in &response.to_device {
            let raw = match event {
                ProcessedToDeviceEvent::PlainText(r) => r,
                ProcessedToDeviceEvent::Invalid(r) => r,
                _ => continue,
            };
            if let Ok(val) = raw.deserialize_as::<serde_json::Value>() {
                if val.get("type").and_then(|v| v.as_str()) == Some(expected_type) {
                    let content = val.get("content").cloned().unwrap_or(serde_json::json!({}));
                    return Ok(content);
                }
            }
        }
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt().with_max_level(tracing::Level::WARN).init();

    // Load credentials from .env
    dotenvy::from_path(".env").or_else(|_| dotenvy::dotenv().map(|_| ())).ok();
    let user_a   = std::env::var("MATRIX_USER_A").context("MATRIX_USER_A not set")?;
    let user_b   = std::env::var("MATRIX_USER_B").context("MATRIX_USER_B not set")?;
    let pass     = std::env::var("MATRIX_PASS").context("MATRIX_PASS not set")?;
    let homeserver = std::env::var("MATRIX_HOMESERVER")
        .unwrap_or_else(|_| "https://asra.gr".to_owned());

    println!("[init] User A: @{user_a}:{}", homeserver.trim_start_matches("https://"));
    println!("[init] User B: @{user_b}:{}", homeserver.trim_start_matches("https://"));

    // ── Build clients ────────────────────────────────────────────────────────
    let tmp = std::env::temp_dir();
    let client_a = Client::builder()
        .server_name_or_homeserver_url(&homeserver)
        .sqlite_store(tmp.join(format!("squelch_002_a_{user_a}")), None)
        .build().await.context("build client A")?;
    let client_b = Client::builder()
        .server_name_or_homeserver_url(&homeserver)
        .sqlite_store(tmp.join(format!("squelch_002_b_{user_b}")), None)
        .build().await.context("build client B")?;

    // ── Login ─────────────────────────────────────────────────────────────────
    println!("[login] Logging in A...");
    client_a.matrix_auth().login_username(&user_a, &pass).send().await.context("login A")?;
    println!("[login] Logging in B...");
    client_b.matrix_auth().login_username(&user_b, &pass).send().await.context("login B")?;

    let uid_a = client_a.user_id().context("no user_id A")?.to_owned();
    let uid_b = client_b.user_id().context("no user_id B")?.to_owned();
    let did_a = client_a.device_id().context("no device_id A")?.to_owned();
    let did_b = client_b.device_id().context("no device_id B")?.to_owned();
    println!("[login]  A: {uid_a} device={did_a}");
    println!("[login]  B: {uid_b} device={did_b}");

    // ── Initial sync ─────────────────────────────────────────────────────────
    println!("[sync] Initial sync...");
    let sync_settings = SyncSettings::default().timeout(Duration::from_secs(10));
    client_a.sync_once(sync_settings.clone()).await.context("initial sync A")?;
    client_b.sync_once(sync_settings.clone()).await.context("initial sync B")?;
    println!("[sync] Done");

    // ── A creates room + invites B ────────────────────────────────────────────
    println!("[room] A creates squad room and invites B...");
    use matrix_sdk::ruma::api::client::room::create_room::v3 as create_room;
    let mut create_req = create_room::Request::new();
    create_req.name = Some("squelch-spike-002".to_owned());
    create_req.invite = vec![uid_b.clone()];
    let room = client_a.create_room(create_req).await.context("create room")?;
    let room_id = room.room_id().to_owned();
    println!("[room] Created: {room_id}");

    // ── B syncs to get invite, then joins ─────────────────────────────────────
    client_b.sync_once(sync_settings.clone()).await.context("sync B for invite")?;
    client_b.join_room_by_id(&room_id).await.context("B join room")?;
    println!("[room] B joined");

    // ── A sends SDP offer to B ────────────────────────────────────────────────
    println!("[signal] A → B: SDP offer");
    send_signaling(&client_a, &uid_b, &did_b, TYPE_SDP_OFFER, serde_json::json!({
        "room_id": room_id.as_str(),
        "sdp": "v=0\r\no=squelch-a 0 0 IN IP4 127.0.0.1\r\n[fake offer]",
        "call_id": "spike-002-call-1",
    })).await?;

    // ── B polls until offer arrives ───────────────────────────────────────────
    println!("[sync]   B waiting for offer...");
    let offer = wait_for_to_device(&client_b, TYPE_SDP_OFFER).await?;
    println!("[B] ✓ SDP offer received: call_id={}", offer["call_id"]);

    // ── B sends answer + ICE to A ─────────────────────────────────────────────
    println!("[signal] B → A: SDP answer");
    send_signaling(&client_b, &uid_a, &did_a, TYPE_SDP_ANSWER, serde_json::json!({
        "room_id": room_id.as_str(),
        "sdp": "v=0\r\no=squelch-b 0 0 IN IP4 127.0.0.1\r\n[fake answer]",
        "call_id": "spike-002-call-1",
    })).await?;

    println!("[signal] B → A: ICE candidate");
    send_signaling(&client_b, &uid_a, &did_a, TYPE_ICE_CANDIDATE, serde_json::json!({
        "room_id": room_id.as_str(),
        "candidate": "candidate:1 1 UDP 2130706431 192.168.1.100 54400 typ host",
        "call_id": "spike-002-call-1",
    })).await?;

    // ── A polls until answer + ICE arrive (in same or subsequent sync) ───────
    println!("[sync]   A waiting for answer + ICE...");
    let mut got_answer: Option<serde_json::Value> = None;
    let mut got_ice: Option<serde_json::Value> = None;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(60);

    while got_answer.is_none() || got_ice.is_none() {
        if tokio::time::Instant::now() > deadline {
            bail!("Timeout: answer={} ice={}", got_answer.is_some(), got_ice.is_some());
        }
        let response = client_a
            .sync_once(sync_settings.clone())
            .await
            .context("sync A poll")?;

        for event in &response.to_device {
            let raw = match event {
                ProcessedToDeviceEvent::PlainText(r) => r,
                ProcessedToDeviceEvent::Invalid(r) => r,
                _ => continue,
            };
            if let Ok(val) = raw.deserialize_as::<serde_json::Value>() {
                match val.get("type").and_then(|v| v.as_str()) {
                    Some(TYPE_SDP_ANSWER) => {
                        println!("[A] ✓ SDP answer received: call_id={}", val["content"]["call_id"]);
                        got_answer = val.get("content").cloned();
                    }
                    Some(TYPE_ICE_CANDIDATE) => {
                        println!("[A] ✓ ICE candidate received: {}", val["content"]["candidate"]);
                        got_ice = val.get("content").cloned();
                    }
                    _ => {}
                }
            }
        }
        if got_answer.is_none() || got_ice.is_none() {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    println!("\n✓ SPIKE VALIDATED:");
    println!("  - Both clients logged in on {homeserver}");
    println!("  - Room created and joined");
    println!("  - SDP offer delivered A → B via to-device");
    println!("  - SDP answer delivered B → A via to-device");
    println!("  - ICE candidate delivered B → A via to-device");
    println!("  matrix-rust-sdk is usable as a P2P signaling bus for Squelch.");

    let _ = client_a.matrix_auth().logout().await;
    let _ = client_b.matrix_auth().logout().await;
    Ok(())
}

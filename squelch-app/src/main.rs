//! Squelch — entry point, egui UI, global PTT hotkey, and application wiring.

mod app;
mod backend;
mod config;
mod hotkey;

use anyhow::Result;
use tokio::runtime::Runtime;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // ── Logging ───────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                EnvFilter::new(
                    "squelch=info,squelch_matrix=info,squelch_webrtc=info,\
                     squelch_audio=info,\
                     matrix_sdk_crypto::backups=off,\
                     matrix_sdk_crypto::identities=off,\
                     warn",
                )
            }),
        )
        .init();

    info!("Squelch starting");

    // ── Tokio runtime (multi-thread for Matrix async work) ────────────────
    let rt = Runtime::new()?;

    // ── Config ────────────────────────────────────────────────────────────
    let cfg = config::AppConfig::load()?;

    // ── PTT state ─────────────────────────────────────────────────────────
    let ptt = squelch_audio::PttState::new();

    // ── Hotkey manager (must be on main thread on some platforms) ─────────
    // Non-fatal: if the hotkey is already registered (e.g. another instance
    // is running), log a warning and continue without PTT hotkey support.
    let _hotkey_manager = hotkey::HotkeyManager::new(ptt.clone(), cfg.ptt_key.as_deref())
        .map_err(|e| tracing::warn!("PTT hotkey not available: {e}"))
        .ok();

    // ── Backend task ──────────────────────────────────────────────────────
    let backend = backend::Backend::spawn(ptt.clone(), cfg.clone(), rt.handle().clone());

    // ── egui application ─────────────────────────────────────────────────
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Squelch")
            .with_inner_size([480.0, 560.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Squelch",
        native_options,
        Box::new(move |cc| Ok(Box::new(app::SquelchApp::new(cc, ptt, cfg, backend)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    // Keep runtime alive until eframe exits
    drop(rt);
    Ok(())
}

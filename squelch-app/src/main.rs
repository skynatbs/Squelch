//! Squelch — entry point, egui UI, global PTT hotkey, and application wiring.
//!
//! Lifecycle:
//!   1. Load config
//!   2. Build PttState (shared between hotkey poller and audio pipeline)
//!   3. Start AudioPipeline
//!   4. Login to Matrix, start sync loop
//!   5. Launch egui window for squad setup
//!   6. On setup complete: hide window, show tray icon
//!   7. PTT hotkey pressed/released → PttState::press()/release()

mod app;
mod config;
mod hotkey;

use anyhow::Result;
use tracing::info;
use tracing_subscriber::EnvFilter;

fn main() -> Result<()> {
    // ── Logging ───────────────────────────────────────────────────────────
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("squelch=info,warn")),
        )
        .init();

    info!("Squelch starting");

    // ── Config ────────────────────────────────────────────────────────────
    let cfg = config::AppConfig::load()?;
    info!(?cfg, "config loaded");

    // ── PTT state (shared across all threads) ─────────────────────────────
    let ptt = squelch_audio::PttState::new();

    // ── Hotkey manager ────────────────────────────────────────────────────
    // Must be created on the main thread on some platforms.
    let _hotkey_manager = hotkey::HotkeyManager::new(ptt.clone(), cfg.ptt_key.as_deref())?;

    // ── egui application ─────────────────────────────────────────────────
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Squelch")
            .with_inner_size([480.0, 540.0])
            .with_resizable(false),
        ..Default::default()
    };

    eframe::run_native(
        "Squelch",
        native_options,
        Box::new(|cc| Ok(Box::new(app::SquelchApp::new(cc, ptt, cfg)))),
    )
    .map_err(|e| anyhow::anyhow!("eframe error: {e}"))?;

    Ok(())
}

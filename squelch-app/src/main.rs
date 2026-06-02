//! squelch-app — Entry point, egui UI, tray icon, and global PTT hotkey.
//!
//! Responsibilities:
//! - Parse CLI args / config
//! - Initialize tracing
//! - Launch egui window for squad setup
//! - Register global PTT hotkey and forward events to squelch-audio::PttState
//! - After setup: minimize to tray, keep audio pipeline running in background

fn main() {
    tracing_subscriber::fmt::init();
    tracing::info!("Squelch starting");

    // TODO: launch egui window (Phase 4)
    println!("Squelch – tactical voice communication");
    println!("(UI not yet implemented)");
}

//! Global PTT hotkey registration and polling.
//!
//! `HotkeyManager` registers a global hotkey and polls for press/release events
//! in a background thread, updating the shared `PttState` accordingly.

use anyhow::{Context, Result};
use global_hotkey::{
    GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState,
    hotkey::{Code, HotKey},
};
use squelch_audio::PttState;
use tracing::{info, warn};

/// Default PTT key when none is configured: CapsLock (unusual in gaming, avoids conflicts).
const DEFAULT_PTT_KEY: &str = "CapsLock";

/// Manages the global PTT hotkey lifecycle.
///
/// The background polling thread runs until this is dropped.
pub struct HotkeyManager {
    _manager: GlobalHotKeyManager,
    _thread: std::thread::JoinHandle<()>,
}

impl HotkeyManager {
    /// Register the PTT hotkey and start the polling thread.
    ///
    /// `key_str` is a key string like `"CapsLock"`, `"F1"`, or `"ctrl+F1"`.
    /// Falls back to `DEFAULT_PTT_KEY` if `None` or unparseable.
    pub fn new(ptt: PttState, key_str: Option<&str>) -> Result<Self> {
        let manager =
            GlobalHotKeyManager::new().context("failed to create global hotkey manager")?;

        let key_str = key_str.unwrap_or(DEFAULT_PTT_KEY);
        let hotkey = Self::parse_hotkey(key_str);

        manager
            .register(hotkey)
            .context("failed to register PTT hotkey")?;

        info!(key = key_str, "PTT hotkey registered");

        let thread = std::thread::Builder::new()
            .name("squelch-hotkey".into())
            .spawn(move || Self::poll_loop(ptt))
            .context("failed to spawn hotkey thread")?;

        Ok(Self {
            _manager: manager,
            _thread: thread,
        })
    }

    /// Parse a hotkey string. Falls back to CapsLock on error.
    fn parse_hotkey(s: &str) -> HotKey {
        s.parse::<HotKey>().unwrap_or_else(|e| {
            warn!("could not parse PTT key \"{s}\": {e} — falling back to CapsLock");
            HotKey::new(None, Code::CapsLock)
        })
    }

    /// Background loop: poll for hotkey events and update PttState.
    fn poll_loop(ptt: PttState) {
        loop {
            if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                match event.state() {
                    HotKeyState::Pressed => ptt.press(),
                    HotKeyState::Released => ptt.release(),
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}

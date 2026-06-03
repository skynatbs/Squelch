//! Push-to-talk state — shared between the audio thread and the UI/hotkey thread.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

/// Shared PTT state. Clone-able, cheap to share across threads.
#[derive(Clone, Debug)]
pub struct PttState(pub(crate) Arc<AtomicBool>);

impl PttState {
    /// Create a new PTT state, initially inactive.
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Activate PTT (key pressed).
    pub fn press(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    /// Deactivate PTT (key released).
    pub fn release(&self) {
        self.0.store(false, Ordering::Relaxed);
    }

    /// Returns true if PTT is currently active.
    pub fn is_active(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for PttState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ptt_press_release() {
        let ptt = PttState::new();
        assert!(!ptt.is_active());
        ptt.press();
        assert!(ptt.is_active());
        ptt.release();
        assert!(!ptt.is_active());
    }

    #[test]
    fn ptt_shared_across_clones() {
        let ptt_a = PttState::new();
        let ptt_b = ptt_a.clone();
        ptt_a.press();
        assert!(ptt_b.is_active(), "clone should see the same atomic state");
    }
}

# ADR-0002 – egui as UI Framework
**Date:** 2026-06-02
**Status:** Accepted
**Author:** Christian / SetScallywag

---

## Context

Squelch is a utility tool. The user interacts with the UI only during setup (creating a squad, assigning duos, setting leaders) and then the app runs in the background as a tray icon. A full WebView-based UI stack is disproportionate for this use case.

Tauri v2 + Vue 3 was initially considered (familiar from another project in the same ecosystem), but Squelch has a fundamentally different character: no persistent visible editor interface, no complex frontend state management, no rich text rendering.

---

## Decision

Squelch uses **egui / eframe** as its UI framework — pure Rust, no separate frontend stack.

---

## Rationale

The app lifecycle is: short setup window → tray icon → runs in background. For this purpose egui is ideal: pure Rust without WebView overhead, simple build setup, small binary, no second technology stack (no JavaScript/TypeScript/Vue). The UI does not need to be polished — it needs to be functional. Nobody admires a voice chat client; it just needs to work.

Global hotkeys work on Linux and Windows without Tauri via the `global-hotkey` crate (based on `tao`); tray icon via the `tray-icon` crate.

---

## Alternatives Considered

| Option | Why rejected |
|--------|-------------|
| Tauri v2 + Vue 3 | Overengineered for a pure utility tool; two tech stacks for minimal UI |
| Iced | Less mature ecosystem than egui, API still in flux |
| No UI (pure CLI) | Squad setup and leader management require minimal graphical interaction |
| Native widgets (gtk-rs / win32) | Platform-specific, significantly more effort |

---

## Consequences

**Positive:**
- Pure Rust — one single tech stack across the entire project
- Small binary, no WebView overhead
- Simple build setup, no Node.js dependency
- egui is immediate-mode — easy to understand for external contributors

**Negative / Risks:**
- UI styling more limited compared to CSS/HTML
- egui ecosystem smaller than the web ecosystem
- Tray icon and global hotkey must be wired up separately (no Tauri plugin)

---

## Related ADRs

- ADR-0001 – Cargo Workspace as Project Structure
- ADR-0005 – Communication Model

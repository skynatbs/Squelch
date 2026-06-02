# ADR-0002 – Tauri v2 als Desktop-Shell
**Datum:** 2026-06-02  
**Status:** Angenommen  
**Autor:** Christian / SetScallywag

---

## Kontext

Squelch benötigt eine Desktop-App mit globalem PTT-Hotkey. Ein Browser-Tab kann keinen globalen Tastendruck abfangen wenn ein anderes Fenster (das Spiel) im Fokus ist. Eine native Desktop-Lösung ist daher zwingend. Der bekannte Stack aus dem Torval-Projekt soll wo sinnvoll wiederverwendet werden.

---

## Entscheidung

Squelch wird als **Tauri v2 Desktop-App** mit Rust-Backend gebaut.

---

## Begründung

Tauri v2 ist stable, Rust-nativ und ermöglicht globale Hotkeys via `tauri-plugin-global-shortcut`. Das Team kennt den Stack bereits aus Torval. Tauri erzeugt kleine Binaries und hat keine Electron-typischen Overhead-Probleme. Die WebView-Schicht erlaubt ein modernes UI ohne nativen Widget-Aufwand.

---

## Betrachtete Alternativen

| Option | Warum verworfen |
|---|---|
| Browser-App (WebRTC im Tab) | Kein globaler Hotkey möglich wenn Spielfenster fokussiert |
| Electron | Zu groß, zu langsam, kein Rust-Backend |
| Iced / egui (reines Rust GUI) | Weniger ausgereifte Ökosysteme, höherer UI-Aufwand |

---

## Konsequenzen

**Positiv:**
- Globaler PTT-Hotkey funktioniert auch während des Spiels
- Bekannter Stack, kurze Einarbeitungszeit
- Kleine Binary, kein Electron-Overhead
- Rust-Backend für Audio-Processing und WebRTC direkt nutzbar

**Negativ / Risiken:**
- WebView-Verhalten je Betriebssystem leicht unterschiedlich
- Tauri-Plugin-Ökosystem noch wachsend

---

## Verwandte ADRs

- ADR-0001 – Cargo Workspace als Projektstruktur
- ADR-0003 – Matrix als Signaling-Backend

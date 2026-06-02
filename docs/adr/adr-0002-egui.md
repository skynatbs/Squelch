# ADR-0002 – egui als UI-Framework
**Datum:** 2026-06-02  
**Status:** Angenommen  
**Autor:** Christian / SetScallywag

---

## Kontext

Squelch ist ein Utility-Tool. Der Nutzer interagiert mit der UI ausschließlich beim Start (Squad einrichten, Duo zuweisen, Leader setzen) und danach läuft die App im Hintergrund als Tray-Icon. Eine vollwertige WebView-basierte UI ist für diesen Use Case unverhältnismäßig aufwändig.

Tauri v2 mit Vue 3 wurde initial als Stack erwogen (bekannt aus Torval), aber Squelch hat einen anderen Charakter als Torval: kein dauerhaft sichtbares Editor-Interface, kein komplexes State-Management im Frontend, keine Rich-Text-Darstellung.

---

## Entscheidung

Squelch nutzt **egui / eframe** als UI-Framework – reines Rust, kein separater Frontend-Stack.

---

## Begründung

Der Lebenszyklus der App ist: kurzes Setup-Fenster → Tray-Icon → läuft im Hintergrund. Für diesen Zweck ist egui ideal: reines Rust ohne WebView-Overhead, einfaches Build-Setup, kleine Binary, kein zweiter Technologie-Stack (kein JavaScript/TypeScript/Vue). Die UI muss nicht poliert sein – sie muss funktional sein. Niemand bewundert einen Voice-Chat-Client, er soll einfach funktionieren.

Globale Hotkeys funktionieren unter Linux und Windows ohne Tauri via `global-hotkey` crate (basiert auf `tao`), Tray-Icon via `tray-icon` crate.

---

## Betrachtete Alternativen

| Option | Warum verworfen |
|---|---|
| Tauri v2 + Vue 3 | Überdimensioniert für ein reines Utility-Tool; zwei Tech-Stacks für minimale UI |
| Iced | Weniger ausgereiftes Ökosystem als egui, API noch in Bewegung |
| Keine UI (reines CLI-Tool) | Squad-Setup und Leader-Verwaltung brauchen minimale grafische Interaktion |
| Native Widgets (gtk-rs / win32) | Plattformspezifisch, deutlich mehr Aufwand |

---

## Konsequenzen

**Positiv:**
- Reines Rust – ein einziger Tech-Stack im gesamten Projekt
- Kleine Binary, kein WebView-Overhead
- Einfaches Build-Setup, keine Node.js-Abhängigkeit
- egui ist immediate-mode – einfach zu verstehen für externe Mitwirkende

**Negativ / Risiken:**
- UI-Styling eingeschränkt gegenüber CSS/HTML
- egui-Ökosystem kleiner als Web-Ökosystem
- Tray-Icon und globaler Hotkey müssen separat über Crates eingebunden werden (kein Tauri-Plugin)

---

## Verwandte ADRs

- ADR-0001 – Cargo Workspace als Projektstruktur
- ADR-0005 – Kommunikationsmodell

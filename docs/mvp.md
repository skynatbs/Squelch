# Squelch – MVP Definition
**Version:** 0.2  
**Status:** Konzeptionsphase  
**Datum:** 2026-06-02

---

## 1. Vision

Squelch ist eine Open-Source Desktop-App für Gaming-Squads die strukturierte Sprachkommunikation ohne Channel-Wechsel ermöglicht. Inspiriert von militärischen Funknetzen und dem Kommunikationsmodell des Spiels *Squad*: das eigene Duo hört sich permanent, Squad-Leader koordinieren sich per PTT im Leader-Net. Kein Discord-Channel-Wechsel, keine Unterbrechung des Spielflusses.

---

## 2. Zielgruppe

**Primär (MVP):** 4-köpfige Gaming-Squads mit zwei Duos (z.B. Grayzone Warfare).  
**Sekundär (Post-MVP):** Größere Gruppen mit 3+ Teams (z.B. Star Citizen Org-Operationen).  
**Nicht adressiert im MVP:** Broadcast-Streaming, Aufnahmen, Mobile-Clients.

---

## 3. Kommunikationsmodell

```
Duo A (P1 + P2):   always-on, hören sich permanent
Duo B (P3 + P4):   always-on, hören sich permanent

Leader-Net:        P1 drückt PTT  →  P3 hört ihn  (und umgekehrt)
                   Nur Leader, nicht alle Spieler
```

**Leader-Regeln:**
- Erster Spieler im Squad ist automatisch Leader
- Leadership per Klick in der App übertragbar
- Ein Squad hat genau einen Leader pro Duo

---

## 4. MVP-Umfang

### 4.1 Features (In Scope)

#### Squad-Management
- Squad erstellen via Matrix-Room
- Squad beitreten via Einladungslink / Room-ID
- Duo-Zuweisung (wer ist mit wem im Duo)
- Leader-Zuweisung mit Transfer-Funktion

#### Audio-Kanäle
- **Duo-Kanal:** always-on open mic zwischen den 2 Duo-Mitgliedern
- **Leader-Net:** PTT-Taste sendet an alle anderen Leader gleichzeitig
- Gleichzeitiges Mischen beider Kanäle ohne Artefakte

#### Audio-Stack
- Mikrofon-Capture und Wiedergabe via cpal
- WebRTC P2P Audio-Streams (Audio nie durch Server)
- STUN/TURN für NAT-Traversal (Internet-Gaming)
- Globaler PTT-Hotkey (auch wenn Spielfenster fokussiert)
- Konfigurierbare PTT-Taste

#### Signaling
- Matrix als Signaling- und Discovery-Backend
- Matrix-Account erforderlich (bestehende Accounts nutzbar)
- WebRTC Offer/Answer via Matrix-Events (MatrixRTC / MSC3401)

#### UI (minimal)
- Squad-Setup: erstellen / beitreten
- Mitgliederliste mit Duo-Zuordnung und Leader-Markierung
- Leader-Transfer per Klick
- PTT-Taste konfigurieren
- Danach: Tray-Icon, App läuft im Hintergrund

#### Plattformen
- **Windows** (primär – Gaming-Hauptplattform)
- **Linux** (sekundär)
- macOS explizit **nicht** im MVP

---

### 4.2 Features (Out of Scope – Post-MVP)

- Mehr als 2 Duos / 3+ Teams (Star Citizen Skalierung)
- Mobile App (iOS / Android)
- Aufnahme und Wiedergabe von Sessions
- Noise Suppression / Rauschunterdrückung (RNNoise o.ä.)
- LAN-Modus / mDNS Discovery
- Push-to-Talk per Joystick / HOTAS-Taste
- Eigener TURN-Server
- End-to-End-Verschlüsselung über Matrix-E2EE (WebRTC DTLS-SRTP ist ausreichend)

---

## 5. Technischer Stack (bestätigt)

| Komponente | Technologie | Begründung |
|---|---|---|
| Sprache | **Rust** | Performance, Sicherheit, Teamkompetenz |
| UI | **egui / eframe** | Reines Rust, kein WebView, minimal – passt zum Utility-Charakter |
| Tray + Hotkey | **tray-icon + global-hotkey** | Rust-nativ, kein Tauri nötig |
| Signaling | **Matrix** (matrix-rust-sdk) | Föderiert, kein eigener Server, Open Source |
| Audio I/O | **cpal** | Cross-platform, Rust-nativ |
| Audio P2P | **WebRTC** (str0m oder webrtc-rs, TBD via Spike) | P2P, DTLS-SRTP verschlüsselt |
| NAT-Traversal | **STUN/TURN** | Standard WebRTC, öffentliche Server nutzbar |
| Build | **Cargo Workspace** | Klare Crate-Grenzen, gute Testbarkeit |
| Lizenz | **MIT** | Maximale Offenheit, Community-freundlich |

---

## 6. Architektur-Prinzipien

1. **Audio verlässt nie einen Drittserver** – nur P2P via WebRTC
2. **Kein eigener Squelch-Server** – Matrix-Föderierung macht das überflüssig
3. **Cargo Workspace von Anfang an** – klare Crate-Grenzen, gute Testbarkeit
4. **Kein `.unwrap()` in Produktionscode** – `thiserror` / `anyhow`
5. **Transparent dokumentiert** – jede Architekturentscheidung in `docs/adr/`
6. **Reines Rust** – kein zweiter Tech-Stack, kein JavaScript

---

## 7. Geplante Crate-Struktur

```
squelch/
├── squelch-core/       # Gemeinsame Typen, Config, Fehlertypen, Squad-Modell
├── squelch-matrix/     # Matrix-Client, Signaling, Room-Management
├── squelch-webrtc/     # WebRTC Peer-Verbindungen, Audio-Streams
├── squelch-audio/      # Mikrofon-Capture, Mixer (Duo + Leader-Net), cpal
└── squelch-app/        # Entry Point, egui UI, Tray-Icon, globaler PTT-Hotkey
```

---

## 8. MVP-Erfolgskriterien

Das MVP gilt als abgeschlossen wenn folgende User Stories erfüllt sind:

1. **Als Spieler** kann ich einen Squad erstellen und drei Freunde einladen – ohne Account-Registrierung bei Squelch.
2. **Als Duo-Mitglied** höre ich meinen Teamkameraden permanent ohne eine Taste zu drücken.
3. **Als Squad-Leader** kann ich alle anderen Leader gleichzeitig erreichen indem ich eine konfigurierbare Taste gedrückt halte – auch während das Spiel im Fokus ist.
4. **Als normaler Spieler** habe ich keinen Zugang zum Leader-Net – Koordination bleibt bei den Leadern.
5. **Als Mitwirkender** kann ich das Projekt verstehen und Änderungen nachvollziehen weil alle Entscheidungen in `docs/adr/` dokumentiert sind.

---

## 9. Nicht-funktionale Anforderungen

- **Latenz:** < 50ms Ende-zu-Ende Audio
- **Startzeit:** < 3 Sekunden
- **Speicherverbrauch:** < 100MB RAM im Leerlauf
- **Keine Cloud-Abhängigkeit von Squelch** – Matrix-Server wird vom Nutzer gewählt

---

## 10. Offene Fragen (Spike-Phase)

- [ ] **WebRTC-Crate:** str0m vs webrtc-rs – Reifegrad, API, Wartungsstatus (Spike 001)
- [ ] **MatrixRTC:** Wie viel nimmt uns matrix-rust-sdk beim WebRTC-Signaling ab? (Spike 002)
- [ ] **Audio-Mixing:** Gleichzeitiges Mischen von Duo-Kanal + Leader-Net ohne Artefakte (Spike 003)
- [ ] **TURN-Server:** Welche öffentlichen TURN-Server sind für Gaming-Latenz geeignet?

# ADR-0003 – Matrix als Signaling-Backend
**Datum:** 2026-06-02  
**Status:** Angenommen  
**Autor:** Christian / SetScallywag

---

## Kontext

Squelch benötigt einen Weg über den sich Spieler gegenseitig finden und WebRTC-Verbindungen aushandeln können (Signaling). Die Lösung muss über das Internet funktionieren, soll keine eigene Server-Infrastruktur von Squelch erfordern, und zum Open-Source-Ansatz des Projekts passen.

---

## Entscheidung

Squelch nutzt das **Matrix-Protokoll** (via `matrix-rust-sdk`) ausschließlich als Signaling- und Discovery-Layer. Audio-Daten fließen direkt zwischen Peers via WebRTC – nie durch einen Matrix-Server.

---

## Begründung

Matrix ist föderiert: Nutzer können jeden öffentlichen Homeserver (z.B. matrix.org) nutzen oder einen eigenen betreiben. Das Projekt ist nicht abhängig von einer zentralen Squelch-Infrastruktur. Das Protokoll ist etabliert, gut dokumentiert, und `matrix-rust-sdk` ist production-ready. MatrixRTC (MSC3401) ist genau für WebRTC-Signaling über Matrix konzipiert.

---

## Betrachtete Alternativen

| Option | Warum verworfen |
|---|---|
| Eigener Axum-Signaling-Server | Erfordert permanente Squelch-Infrastruktur, Single Point of Failure |
| WebRTC via LAN/mDNS | Kein Internet-Support, NAT-Probleme |
| XMPP/Jingle | Kleineres Ökosystem, weniger aktive Rust-Bibliotheken |
| Peer-ID via QR-Code / manuell | Zu viel Reibung für den Nutzer |

---

## Konsequenzen

**Positiv:**
- Keine eigene Server-Infrastruktur notwendig
- Föderiert: kein Vendor Lock-in, kein Single Point of Failure
- Nutzer können eigenen Homeserver betreiben
- Audio-Daten verlassen nie den P2P-Kanal
- Gut zu Matrix-Community vermarktbar

**Negativ / Risiken:**
- Matrix-Account erforderlich (geringe Hürde, aber vorhanden)
- matrix-rust-sdk ist umfangreich – Lernaufwand
- MatrixRTC noch relativ jung, API könnte sich ändern

---

## Verwandte ADRs

- ADR-0001 – Cargo Workspace als Projektstruktur
- ADR-0004 – Open-Source-Lizenz

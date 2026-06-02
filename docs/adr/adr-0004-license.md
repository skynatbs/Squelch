# ADR-0004 – Open-Source-Lizenz (MIT)
**Datum:** 2026-06-02  
**Status:** Vorgeschlagen  
**Autor:** Christian / SetScallywag

---

## Kontext

Squelch soll als Open-Source-Projekt veröffentlicht werden. Die Lizenzwahl beeinflusst welche Abhängigkeiten genutzt werden dürfen, wie die Community beitragen kann, und ob kommerzielle Nutzung erlaubt ist.

---

## Entscheidung

Squelch wird unter der **MIT-Lizenz** veröffentlicht.

---

## Begründung

MIT ist die permissivste und am weitesten verbreitete Open-Source-Lizenz im Rust-Ökosystem. Sie erlaubt uneingeschränkte Nutzung, Modifikation und Weitergabe – auch in kommerziellen Kontexten. Das maximiert die potenzielle Nutzerbasis und senkt die Hürde für externe Beiträge. Alle geplanten Abhängigkeiten (matrix-rust-sdk: Apache-2.0, Tauri: MIT/Apache-2.0, WebRTC-Crates: MIT) sind mit MIT kompatibel.

---

## Betrachtete Alternativen

| Option | Warum verworfen |
|---|---|
| Apache-2.0 | Ebenfalls permissiv, aber komplexer – MIT reicht für diesen Use Case |
| GPL-3.0 | Copyleft erzwingt Open Source bei Derivaten – schränkt kommerzielle Nutzung ein |
| AGPL-3.0 | Zu restriktiv für eine App die auch in kommerziellen Gaming-Kontexten genutzt wird |
| Proprietär | Widerspricht dem Community-Gedanken des Projekts |

---

## Konsequenzen

**Positiv:**
- Maximale Offenheit für Nutzer und Mitwirkende
- Kompatibel mit allen geplanten Abhängigkeiten
- Einfach verständlich, keine Compliance-Komplexität

**Negativ / Risiken:**
- Niemand ist verpflichtet Verbesserungen zurückzugeben
- Kommerzielle Nutzung ohne Gegenleistung möglich (bewusst akzeptiert)

---

## Verwandte ADRs

- ADR-0003 – Matrix als Signaling-Backend

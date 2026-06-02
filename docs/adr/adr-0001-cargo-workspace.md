# ADR-0001 – Cargo Workspace als Projektstruktur
**Datum:** 2026-06-02  
**Status:** Angenommen  
**Autor:** Christian / SetScallywag

---

## Kontext

Squelch soll von Anfang an wartbar und für externe Mitwirkende verständlich sein. Eine monolithische `main.rs` würde Testbarkeit und Modularität von Beginn an erschweren. Die Erfahrung aus dem Torval-Projekt zeigt, dass ein Cargo Workspace klare API-Grenzen erzwingt und spätere Erweiterungen vereinfacht.

---

## Entscheidung

Squelch wird als **Cargo Workspace** mit mehreren eigenständigen Crates organisiert.

---

## Begründung

Jeder Crate hat eine einzige Verantwortlichkeit und kann unabhängig getestet werden. Neue Funktionsbereiche (z.B. weitere Signaling-Backends) können als eigene Crates ergänzt werden ohne bestehenden Code anzufassen. Ein gemeinsames `Cargo.lock` stellt konsistente Abhängigkeitsversionen sicher.

---

## Betrachtete Alternativen

| Option | Warum verworfen |
|---|---|
| Einzelnes Crate (Monolith) | Schlechte Testbarkeit, erschwert externe Beiträge |
| Separate Repositories pro Crate | Zu viel Overhead für ein frühes Projekt |

---

## Konsequenzen

**Positiv:**
- Klare Trennung von Verantwortlichkeiten
- Einzelne Crates unabhängig testbar
- Externe Mitwirkende finden sich schneller zurecht
- Erweiterungen (neue Audio-Backends, neue Signaling-Backends) einfach ergänzbar

**Negativ / Risiken:**
- Initialer Setup-Aufwand höher als ein einzelnes `cargo new`
- Abhängigkeiten zwischen Crates müssen explizit deklariert werden

---

## Verwandte ADRs

- ADR-0002 – Tauri v2 als Desktop-Shell
- ADR-0003 – Matrix als Signaling-Backend
- ADR-0004 – Open-Source-Lizenz

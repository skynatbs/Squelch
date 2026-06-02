# ADR-0005 – Kommunikationsmodell: Duo-Kanäle + Leader-Net
**Datum:** 2026-06-02  
**Status:** Angenommen  
**Autor:** Christian / SetScallywag

---

## Kontext

Ein Squad von 4+ Spielern braucht strukturierte Kommunikation. Ein einziger offener Kanal für alle führt zu Chaos – Spieler reden durcheinander, kein Signal-Rausch-Verhältnis. Discord löst das durch manuelle Channel-Wechsel, was während des Spiels zu viel Reibung verursacht. Ein "Alle-PTT"-Knopf der jedem Spieler zur Verfügung steht löst das Problem ebenfalls nicht, da Gamers – anders als Militär – selten die nötige Funkkdisziplin mitbringen.

Das Spiel *Squad* (Offworld Industries) hat dieses Problem durch einen mehrstufigen Kanal-Ansatz überzeugend gelöst: lokaler Kanal, Squad-Kanal und ein Command-Net nur für Squad-Leader.

---

## Entscheidung

Squelch implementiert **zwei Kanal-Ebenen**:

1. **Duo-Kanal (Team-Kanal):** Always-on open mic zwischen den 2 Spielern eines Duos. Kein Drücken, kein Denken.
2. **Leader-Net:** PTT-Kanal exklusiv für Squad-Leader. Ein Leader drückt PTT und erreicht alle anderen Leader gleichzeitig. Normale Spieler haben keinen Zugang zum Leader-Net.

**Leader-Zuweisung:** Der erste Spieler im Squad ist automatisch Leader. Leadership ist per Klick in der App an ein anderes Mitglied übertragbar.

---

## Begründung

Das Modell löst das Disziplin-Problem durch Mechanik statt durch Konvention: normale Spieler *können* gar nicht den Leader-Net belasten, unabhängig von ihrer Aufgeregtheit. Gleichzeitig bleibt die Kommunikation innerhalb des Duos natürlich und friktionslos. Das Leader-Net skaliert elegant: bei 3 Teams (Star Citizen) sprechen Leader A, B und C per PTT miteinander ohne dass die 6 anderen Spieler betroffen sind.

---

## Betrachtete Alternativen

| Option | Warum verworfen |
|---|---|
| Alle-PTT für jeden Spieler | Fehlende Spieler-Disziplin macht das chaotisch |
| Nur Team-Kanäle, kein übergreifender Kanal | Leader können sich nicht koordinieren |
| Alle-PTT nur für Leader (broadcast) | Besser, aber einseitig – Leader-Net erlaubt Dialog |
| Konfigurierbar durch Squad selbst | Zu viel Komplexität im MVP; sinnvolles Default reicht |

---

## Konsequenzen

**Positiv:**
- Disziplin ist strukturell erzwungen, nicht konventionsabhängig
- Duo-Kanal ist immer aktiv – keine Taste, kein Aufwand
- Leader-Net skaliert auf beliebig viele Teams (Star Citizen, größere Gruppen)
- Klares mentales Modell: "Ich spreche mit meinem Duo ODER mit den anderen Leadern"

**Negativ / Risiken:**
- Jede Gruppe braucht mindestens einen zugewiesenen Leader
- Ein vergessener Leader-Wechsel kann Koordination blockieren (Leadership-Transfer muss einfach sein)

---

## Verwandte ADRs

- ADR-0001 – Cargo Workspace als Projektstruktur
- ADR-0003 – Matrix als Signaling-Backend

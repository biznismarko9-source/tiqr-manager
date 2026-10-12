# TIQR Manager 2.91.0 — report

**Čo si poslal:** dva screenshoty toho istého grafu. Pri *Cashflow* je
prepínač vpravo na riadku s nadpisom; pri *Profit & Loss* padne pod nadpis
a odtlačí graf dole. „oprav to."

---

## Príčina bol nadpis, nie rozloženie

„Balance cashflow" sa vedľa prepínača zmestil. „Profit & Loss **over time**"
nie — a riadok sa zalomil.

Tie dve slová pritom nenesú nič: **karta má časovú os po celej spodnej
hrane.** Každý režim v tomto grafe je *over time*. Takže nadpis je teraz len
názov metriky.

| nadpis | zalomí sa pod šírkou karty |
|---|---|
| „Profit & Loss over time" *(pred)* | **622 px** |
| „Balance cashflow" | 567 px |
| „Profit & Loss" *(po)* | **562 px** |

Najdlhší nadpis je teraz kratší než ten, ktorý sa už predtým zmestil. Čiže
Profit & Loss sa zmestí všade, kde sa zmestí Cashflow.

---

## Čo som skúsil najprv a zahodil

Prvá verzia opravy bola „nech sa ľavý blok zmenšuje" (`min-w-0`) a „riadok
nech sa nezalamuje vôbec". **Vyrenderoval som to a bolo to horšie:** blok sa
zmenšil tak, že **peňažné číslo vyliezlo spod nadpisu a prekrylo prepínač.**

Preto je tam teraz **reálna podlaha** `min-w-[150px]`, nie nula. Zvyšné
zábrany, v tomto poradí:

- nadpis má `truncate` — je to tá polovica, ktorá má ustúpiť
- hodnota má `whitespace-nowrap`, lebo v 2.74.0 sa peňažné číslo už raz
  skrátilo na nulovú šírku
- `flex-wrap` na riadku **ostáva** ako skutočne poslednná možnosť, pre okno
  užšie než podlaha a prepínač dokopy

---

## Overenie

Vyrenderované na hranici zalomenia, nie „podľa oka":

| šírka | pred | po |
|---|---|---|
| 600 px | drží | drží |
| **580 px** | **zalomí sa** *(tvoj screenshot)* | **drží** |

Plus vyváženie zátvoriek a obe triedy komentárovej chyby — 0 nálezov.

**Nie je to kompilácia.** Node ani cargo na tomto Macu nie sú, CI je prvý
kompilátor.

---

## Verzia

2.90.0 → **2.91.0**, 9 výskytov v 7 súboroch plus `CURRENT_STATE.md`.

# TIQR Recap 2.24.0 — release report

Prémiová **prezentačná vrstva** nad dátami, ktoré appka už mala a overila.
Žiadny nový finance engine, žiadny reporting backend, **ani jeden nový
biznisový výpočet** — a pri stavaní sa ukázalo, že ani jeden nebol potrebný.

---

## 1. Ticket Recap

**Realized (za obdobie):** Tickets bought · Tickets sold · Total invested ·
Sales revenue · Realized profit · ROI (+ margin ako podtitul).

**Vizuál „Purchased vs Sold vs Remaining"** je jeden kompozičný pruh, nie
časový graf. Dôvod: existujúca agregácia hlási, koľko sa v ktorom koši
**predalo**, ale nie koľko sa **kúpilo** — časová krivka nákupov by sa musela
vymyslieť.

**Highlights:**
- **Best channel this period** — z `salesByPlatform`, podľa zisku. Period-scoped, existuje.
- **Best event · all time** — z `EventWithStats.stats`, a **na obrazovke je napísané, že je to all-time**, lebo to je scope, ktorý appka per event drží.

**Čo tam zámerne NIE JE:** biggest single sale, fastest selling event, best
tier. Ani jedno dnes neexistuje ako agregácia a dorobiť ju by znamenalo spraviť
z Recapu reporting engine. Recap to píše priamo na obrazovke, nie je to tichá
diera.

---

## 2. Finance Recap

**REALIZED:** Money in (tržba za obdobie) · Money out (nákup lístkov +
poplatky platforiem) · Realized profit · Received payouts (inkasované).

**PENDING:** Pending payouts (nezinkasované, aj s počtom predajov) ·
**Unpaid orders — len POČET**, a je to tam napísané: appka sleduje, *ktoré*
objednávky sú nezaplatené, ale nikdy nesčítava, koľko na nich dlhuješ, a
dorobiť ten súčet by bol nový výpočet · Pulls to transfer.

**CAPITAL / POTENTIAL:** Tickets still held · Capital tied up · Listing value ·
Potential profit (+ upozornenie, koľko lístkov nemá cenu, takže ide proti
kapitálu, ale k hodnote neprispieva).

---

## 3. Výber obdobia

This month · Last month · 3 months · 6 months · This year · Custom.

„This/Last month" sú **kalendárne**, „3/6 months" sú **posuvné okná končiace
dnes** — a pod hlavičkou je napísané ktoré je ktoré, takže sa to nemusí hádať.

**Otestované skutočným behom, nie úvahou:**

| deň | Last month |
|---|---|
| 31. marec 2026 | 1.–**28.** február |
| 15. marec 2024 (priestupný) | 1.–**29.** február |
| 5. január 2026 | 1.–31. **december 2025** |

Custom si drží presne to, čo zadáš.

---

## 4. Monthly summary

Tabuľka `Metric | This period | Previous period | Change` na konci Recapu.

Používa **existujúce** helpery appky: `computeTrend` pre peniaze a počty,
`computeTrendPoints` pre ROI a margin. Preto:

- Revenue: **+62,2 %**
- ROI 31,8 % vs 28,2 %: **+3,6 pp**

„Previous period" je **definícia appky, nezmenená** — rovnako dlhé okno tesne
pred aktuálnym — a je to napísané pod tabuľkou, nie ponechané na dohad. Keď
porovnanie neexistuje (All time, vlastný rozsah bez začiatku), tabuľka to
povie namiesto pomlčiek.

---

## 5. Realized / Pending / Potential

Tri pásy, každý s vlastnou farbou, nadpisom **a jednou vetou definície priamo
na obrazovke**. Tá veta je nosná, nie ozdoba: bez nej sa „pending" a
„potential" čítajú ako to isté číslo — a presne to má toto rozloženie
znemožniť.

Nikdy sa nemieša realized profit, potential profit, pending payout a hodnota
nepredaného skladu.

---

## 6. Grafy

`MetricChart` — vlastný SVG komponent appky, **nezmenený**. Žiadna druhá
grafová technológia pre tie isté dáta, žiadna nová závislosť.

Graf je jeden, s prepínačom metriky. Druhý „graf" je kompozičný pruh
Purchased/Sold/Remaining. Nič navyše len preto, aby tam bolo.

---

## 7. Shareable output

**PNG**, 1200×675, v 2× rozlíšení.

Karta je **ručne poskladané SVG** (nie screenshot appky — presne ako si
žiadal), ktoré sa rasterizuje na canvase. Žiadna screenshot knižnica; tento
projekt nemá žiadne UI závislosti a nemá ich prečo dostať kvôli jednému
obrázku.

Ukladá sa cez normálny save dialóg. Nový príkaz `save_png_file` dekóduje
base64, **overí PNG signatúru** (aby zle orezaný data URL nezapísal súbor,
ktorý sa neotvorí) a zapíše bajty. Žiadna logika, žiadna databáza, žiadny
cloud.

---

## 8. Navigácia

**Settings → Insights** → dve karty (🎟️ Ticket Recap, 💰 Finance Recap).
Žiadna položka v bočnom menu.

Z Recapu sa dá kliknúť na **event detail** (najlepší event). Žiadny nový
detailový systém nevznikol.

`Esc` zatvára, rovnako ako každý iný overlay v appke.

---

## 9. Zmenené súbory

| súbor | čo |
|---|---|
| `src/components/Recap.tsx` | **nový** — celý Recap + vstupné karty |
| `src-tauri/src/commands/share.rs` | **nový** — `save_png_file` (I/O, 3 testy) |
| `src-tauri/src/commands/mod.rs`, `lib.rs` | registrácia |
| `src/lib/api.ts` | `savePngFile` |
| `src/pages/Settings.tsx` | sekcia Insights |
| `CHANGELOG.md`, `PROJECT_STATE/*` | dokumentácia |

**Nedotknuté:** Orders, Tickets, Sales, Listings, Finance, Inventory,
Fulfillment, refund/resell, `batch_id`, peniaze, Calendar, Price Checker,
Google Sheets, Sync. Žiadna zmena schémy, žiadna migrácia (ďalšia je 029),
žiadna nová závislosť.

---

## 10. Testy

**Spustené tu:**
- **Dátumové rozsahy** — všetkých 5 typov období proti 4 hraničným dňom
  (koniec marca, priestupný rok, január, bežný deň) + custom. Všetko sedí.
- **180 príkazov** sedí medzi `api.ts` a `lib.rs` v oboch smeroch.
- Vyváženosť JSX a zátvoriek vo všetkých dotknutých súboroch.

**Napísané, spustí sa v CI:** 3 testy na `save_png_file` (platná signatúra
prejde; JPEG, prázdny vstup, orezaná signatúra a holý text sú odmietnuté a
súbor sa nezapíše; nevalidný base64 je zrozumiteľná chyba, nie panika).

**Pokryté existujúcimi testami, ktoré Recap iba číta:** `previous_period_bounds`,
`safe_ratio` (nulový menovateľ), refundy vylúčené z tržby, mixed-currency
pravidlá, prázdne obdobia — to všetko žije v `dashboard.rs` (52 testov) a
Recap si k tomu nepridáva vlastnú verziu.

**Nespustené tu:** `npx tsc -b`, `npm run build`, `cargo check --lib`,
`cargo test --lib` — na tomto Macu nie je Rust ani Node.

---

## 11. Limity — čo NEsľubujem

- **Biggest sale, fastest selling event, best tier** nie sú. Neexistuje k nim
  agregácia a vyrobiť ju by porušilo zadanie.
- **Best event je all-time**, nie za obdobie. Per-event per-period P&L
  neexistuje.
- **Unpaid orders sú len počet.** Suma, ktorú dlhuješ, sa nikde nedrží.
- **Money in / Money out sú z predajov a nákupov lístkov**, nie z Finance
  modulu. Miešať oboje by buď duplikovalo nákupy, alebo si vyžiadalo novú
  definíciu „money out".
- **Received payouts / capital tied up sú all-time**, nie period-scoped — taký
  je existujúci scope `cashflow` a `inventoryPotential`, a je označený.
- **Žiadne PDF.** Projekt nemá PDF infraštruktúru; pridávať ju kvôli jednému
  exportu by bola väčšia zmena než celý zvyšok tejto funkcie.

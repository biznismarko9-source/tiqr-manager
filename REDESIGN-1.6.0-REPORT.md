# TIQR Manager 1.6.0 — Audit, Bug Fixes, Dashboard Chart, Sales Redesign

Dátum: 2026-08-18
Rozsah: audit celej appky → oprava 6 HIGH bugov → Dashboard graf tržieb/profitu → Sales redesign (karty namiesto tabuľky) → New Sale/Sale Detail polish → verzia 1.6.0.

---

## 1. Zhrnutie

Nič z existujúcej architektúry sa nemenilo len kvôli dizajnu. Sales grouping (SaleGroup/batch_id/GROUP_BASE_SELECT), refund/resell logika, migrácie, finance.rs/money.rs, backup/restore, CSV import — všetko zostalo štrukturálne presne také, aké bolo. Zmeny tohto vydania sú buď (a) oprava reálneho bugu nájdeného auditom, (b) vizuálny redesign Sales stránky nad tými istými dátami, (c) nový Dashboard graf nad novým, samostatným SQL dotazom, alebo (d) drobný, bezpečný UX polish priamo vyplývajúci z auditu.

Žiadna nová DB migrácia. Presne 4 migrácie ako predtým.

## 2. Baseline pred zmenami

`cargo test --lib`: 83 passed / 0 failed / 3 ignored. `cargo clippy`: len 3 staré, nesúvisiace warningy. `tsc -b` + `npm run build`: čisté. Verzie (`package.json`/`tauri.conf.json`/`Cargo.toml`): 1.5.0 všade, konzistentné.

## 3. Audit — výsledky

Podrobný AUDIT RESULTS (kritické/high/medium-low/UX/already-correct, s root cause/súbor/dopad/fix/test pre každý HIGH bug) som poslal ako samostatnú správu pred implementáciou. Zhrnutie: **0 kritických bugov**, **6 HIGH bugov** (všetky opravené, pozri §4), 13 medium/low nálezov (zdokumentované, neopravované — nízke riziko/dopad), 12 UX nálezov (9 z nich priamo súvisí so Sales redesignom, vyriešené v §5-6).

## 4. Opravené HIGH bugy

Pre každý: root cause → súbor/funkcia → dopad → fix → test.

**H1 — EventDetail Margin/ROI leak pri Mixed mene** (opätovné potvrdenie starého nálezu)
Root cause: `finance::compute_summary` (`src-tauri/src/finance.rs`) počítal margin/ROI čisto z cents hodnôt cez `safe_ratio`, bez ohľadu na `currency` parameter — takže aj keď `events.rs`'s STATS_SQL správne poslalo `currency: None` pre mixed-currency event, margin/ROI stále vyšli ako reálne číslo. Dopad: `EventDetail.tsx` ukazoval konkrétne %, ktoré si protirečilo s vlastným "Mixed" bannerom nad ním.
Fix: `compute_summary` teraz vracia `margin`/`roi` ako `None` vždy, keď `currency.is_none()` — opravené centrálne v `finance.rs`, takže to platí pre KAŽDÉHO volajúceho (events.rs aj dashboard.rs), nie len pre jedno miesto. `EventDetail.tsx` prepnuté z `formatPercent` na `formatPercentOrMixed`.
Test: `finance::tests::compute_summary_forces_margin_and_roi_to_none_when_currency_is_mixed` (nový).

**H2 — Events.tsx (zoznam) — ten istý bug, horší (žiadny banner)**
Root cause: identický ako H1, len na `Events.tsx`'s zoznamovom riadku, kde navyše chýba akýkoľvek "Mixed" banner.
Fix: rovnaká centrálna oprava v `compute_summary` (H1) pokrýva aj toto; `Events.tsx` prepnuté na `formatPercentOrMixed`.
Test: pokrytý tým istým `finance.rs` testom (H1) — obe stránky idú cez `map_event_with_stats` → `compute_summary`.

**H3 — Dashboard: únik interného sentinel dátumu**
Root cause: `dashboard.rs`'s `period_bounds()` používa `"0001-01-01"`/`"9999-12-31"` ako "žiadna hranica" pre "All time"/čiastočne vyplnený Custom rozsah — `Dashboard.tsx` tieto hodnoty predtým vypisoval priamo ("Activity 0001-01-01 → 9999-12-31").
Fix: čisto frontend — `Dashboard.tsx` teraz rozpozná sentinel hodnoty a namiesto nich ukáže "All time" / "the beginning" / "today". Backend SQL logika (BETWEEN dotaz) je nedotknutá — sentinely tam fungujú správne, problém bol iba v zobrazení.
Test: frontend-only zmena (appka nemá frontend test framework, rovnako ako všetky predošlé BUG#1-7 frontendové opravy) — overené cez `tsc -b`/`npm run build` a manuálnou logickou kontrolou oboch vetiev (All time, Custom s jedným poľom).

**H4 — OrderDetail: tichý strop 5000 tiketov bez upozornenia**
Root cause: `tickets.rs`'s `LIST_CAP=5000` sa aplikuje aj pri dotaze na jednu objednávku, objednávka môže mať až 50 000 ks. Orders.tsx/Tickets.tsx/Sales.tsx majú banner na 5000+, `OrderDetail.tsx` nemal žiadny.
Fix: pridaný rovnaký amber banner na `OrderDetail.tsx`, aktivuje sa pri `tickets.length >= 5000`.
Test: frontend-only, overené cez build; logika je identická s už-existujúcim, overeným vzorom na ostatných 3 stránkach.

**H5 — Sales/SaleDetail: nesprávny currency pri refundovanej odlišnej mene**
Root cause: `sales.rs`'s `GROUP_BASE_SELECT` počítal `currency` cez `COUNT(DISTINCT s.currency)` cez VŠETKY riadky (aj refundované), zatiaľ čo revenue/fees/cost sa už správne počítali len z nerefundovaných. Zrkadlovo v `SaleDetail.tsx`'s `uniform(lines, ...)`.
Dopad: batch, kde JEDINÁ odlišná mena bola refundovaná, ukazoval "Mixed" namiesto reálneho, plne vypočítateľného čísla. Potvrdené ako bezpečný smer chyby (nikdy neukázal zle zmiešané číslo, len zbytočne skrýval správne).
Fix: `currency` sa teraz počíta z nerefundovaných riadkov, s fallbackom na všetky riadky len keď je CELÁ skupina refundovaná (aby fully-refundovaná skupina stále ukázala svoju menu namiesto prázdna). Rovnaká oprava v `sales.rs` (SQL CASE) aj `SaleDetail.tsx` (JS).
Testy (2 nové): `sales::tests::refunding_the_only_differently_currencied_line_reveals_the_real_single_currency_total`, `sales::tests::fully_refunded_mixed_currency_batch_falls_back_to_all_lines_for_currency`.

**H6 — CSV export: profit v exporte ignoruje refundy**
Root cause: `csv_export.rs`'s `export_sales_csv` počítal `profit = sale_price - cost - fees` bez ohľadu na `payment_status`, na rozdiel od každého iného peňažného súčtu v appke (realized-only pravidlo).
Fix: refundovaný riadok teraz exportuje `profit = 0` (sale_price/cost/fees ostávajú ako reálne historické hodnoty riadku, `payment_status` stĺpec ukazuje prečo je profit 0). Pri tejto oprave som rozdelil `export_sales_csv` na thin wrapper + testovateľný `export_sales_csv_impl` (rovnaký vzor, aký už appka používa pre `get_dashboard`/`list_sale_groups`), lebo predtým sa táto funkcia nedala vôbec unit-testovať.
Testy (3 nové): `csv_export::tests::active_sale_profit_matches_sale_price_minus_cost_minus_fees`, `refunded_sale_exports_zero_profit_instead_of_a_misleading_realized_number`, `mixed_active_and_refunded_export_sums_to_the_realized_only_total`.

## 5. Dashboard: graf tržieb a profitu v čase

Predtým appka nemala ŽIADNU time-series granularitu — všetko bolo jeden preagregovaný súčet za obdobie. Nový backend dotaz (`dashboard.rs::get_dashboard_impl`, nová sekcia "Revenue/Profit over time") berie presne rovnaký scope, aký už má `period_summary` (`period_from`/`period_to`, `primaryCurrency`, `event_id`/`platform_id` filter, vylúčené refundy) a rozbije ho po dátumových bucketoch namiesto jedného súčtu — takže graf a StatCards nad ním nikdy nemôžu ukázať iné číslo.

Šírka bucketu sa prispôsobuje dĺžke obdobia (`time_series_granularity()`): ≤31 dní → deň, ≤180 dní → týždeň, viac (vrátane "All time") → mesiac. Dôvod: "Last 7 days" nemá zmysel po dňoch rozbíjať na 7×nič-iné a viacročné "All time" nemá zmysel po dňoch (tisíce stĺpcov).

Frontend (`src/components/RevenueChart.tsx`) je vlastný, jednoduchý SVG bar chart — **žiadna nová npm závislosť** (appka doteraz nemala žiadnu UI knižnicu okrem React/Tailwind, pridávanie charting knižnice len kvôli 2 stĺpcom na bucket by zbytočne zväčšilo Windows build). Zobrazuje Revenue (brand farba) a Profit (zelená/červená podľa znamienka) vedľa seba za bucket, s hover tooltipom (natívny SVG `<title>`, žiadny extra JS), gridlines a osou. Vizuálne overené cez Playwright screenshot pri viacerých scenároch (bežné dáta, záporný profit, 1 bucket, prázdne obdobie, 14+ bucketov, dark mode) — viď §8.

Umiestnenie: hneď pod "Activity" StatCards (Revenue/Purchase cost/Profit/Margin/ROI/Tickets sold) pre zvolené obdobie, nad "Current inventory" sekciou — rozširuje existujúcu informáciu o časovú os, nie je to duplicitná/samostatná sekcia.

Testy (7 nových, `commands::dashboard::tests`): bucketing po dňoch a súčet späť na `period` total (cross-check proti driftu), vylúčenie refundov, zoskupenie rovnakého ISO týždňa, zoskupenie rovnakého mesiaca, rešpektovanie event filtra, 6 hraníc granularity vrátane "All time".

## 6. Sales: redesign na karty

Predtým: dáta už boli zoskupené (`SaleGroup`/`batch_id`/`GROUP_BASE_SELECT`), ale vizuálne to bola plochá `<table>` s jedným `<tr>` na skupinu. Zmenil sa **len render main listu** (`src/pages/Sales.tsx`) — filter bar, 5000-cap banner, totals summary riadok, aj celá dátová/filtrovacia logika (`listSaleGroups`) sú nedotknuté.

Nová `SaleGroupCard` komponenta: jedna karta na sale action (1 tiket alebo batch). Riadi sa presne rovnakým row-click vzorom, aký už má `Events.tsx` (celá karta klikateľná → Sale Detail, okrem kliku na vnorený `<a>` — link na event — ktorý naviguje sám osebe, bez dvojitej navigácie). Priamo rieši 3 UX nálezy z auditu:
- **vizuálne odlíšenie 1-tiketového predaja od veľkého batchu** — badge "N tickets" má brand farbu pri N>1, sivú pri N=1.
- **umiestnenie refund indikátora** — "N of M refunded" má teraz vlastnú amber farbu (rovnaký warning-konvent ako inde v appke) namiesto splývajúceho sivého textu.
- **nekonzistentné click affordance** — predtým bol klikateľný len kód, zvyšok riadku mal hover pozadie ale nefungoval; teraz je klikateľná celá karta.

Sale Detail (`SaleDetail.tsx`) **zostáva tabuľka s per-ticket rozpisom** presne podľa zadania ("Sale Detail keeps per-ticket breakdown") — nekonvertoval som ju na karty. Jediná zmena tam: oprava M9 nálezu (hlavička kódu používala `lines[0].batchId ?? lines[0].code`, čo mohlo zostať "stale" po zmazaní najnižšieho id v batchi — teraz vždy `lines[0].code`, presne podľa toho, čo robí aj backend).

New Sale flow (`SaleFormModal` v `Sales.tsx`) — 3 malé, bezpečné UX opravy priamo z auditu, žiadna z nich sa nedotýka `submit()`/skutočného zápisu do DB:
- **currency label pri Price/Fees** — predtým nebolo vidieť menu pri poliach, teraz je vždy viditeľný label (nie len placeholder, ktorý by zmizol po vyplnení).
- **profit preview pre mixed-currency batch** — predtým len text "different currencies", teraz rozpis Revenue/Profit per mena (`perCurrencyTotals`, čisto lokálny výpočet, rovnaká logika ako existujúci `totals`, len zoskupená po mene).
- **caption pri bulk-apply** — jasne hovorí, že "Apply to all" prepíše už vyplnené hodnoty.

Navyše (UX #7, "ticket → sale" prelinkovanie): `OrderDetail.tsx` pri predanom tikete pridáva link "View sale", ktorý naviguje na `/sales` a predvyplní hľadanie kódom tiketu — cez rovnaký `navigate(path, { state })` vzor, aký už `Orders.tsx` používa pre `presetEventId`, a existujúce ticket-code hľadanie (BUG #5). **Žiadna zmena backendu.**

**Vedome NEurobené** (z 9 UX nálezov, zvyšné 2 sú reálne nové features, nie oprava/redesign v rámci zadaného rozsahu):
- ticket picker v New Sale orezaný na 25 bez triedenia/stránkovania — vyžadovalo by novú backend query alebo "load more" mechaniku, reálna nová funkcionalita.
- bulk-select na Sales hlavnom zozname (hromadné akcie) — v appke dnes neexistuje žiadna bulk-action infraštruktúra na nadviazanie, bola by to celá nová feature.

## 7. Verzia

`package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`: 1.5.0 → **1.6.0**, všetky 3 konzistentne. Žiadna nová migrácia (presne 4 ako predtým). `release.ps1` a `1-CLICK-UPDATE.bat` tiež aktualizované na v1.6.0 (vrátane commit message, ktorá teraz opisuje reálne zmeny tohto vydania) — bez toho by ti tieto skripty ešte stále hovorili "v1.5.0" pri publikovaní 1.6.0 zdrojového kódu.

## 8. Testovanie a verifikácia

**Automatizované (spustené, nie len tvrdené):**
- `cargo check --lib`: čisté.
- `cargo test --lib`: **95 passed / 0 failed / 3 ignored** (83 pôvodných + 13 nových: 1× finance.rs, 2× sales.rs, 3× csv_export.rs, 7× dashboard.rs). 3 ignored sú tie isté dávno-existujúce perf testy (spúšťajú sa len manuálne).
- `cargo clippy --lib --all-targets`: presne tie isté 3 staré, nesúvisiace warningy ako pred týmto vydaním (finance.rs digit-grouping, dashboard.rs if_same_then_else, db.rs type_complexity) — žiadny nový warning.
- `tsc -b` (samostatne aj cez `npm run build`): čisté, 0 chýb.
- `npm run build` (plný vite build): úspešný, 60 modulov, veľkosť bundlu len mierne narástla (288KB→293KB) kvôli novej `RevenueChart` komponente — žiadna nová závislosť.

**Vizuálne (Playwright + headless Chromium, cez dočasný preview harness mimo Tauri, zmazaný po použití):**
- `RevenueChart`: bežné dáta, záporný profit (červený stĺpec pod nulovou čiarou), 1 bucket, prázdne obdobie, 14-bucketový dataset s orezávaním popiskov, dark mode — všetko vyzerá správne, žiadna console chyba.
- `SaleGroupCard`: 1-tiketový predaj, 12-tiketový batch, čiastočne refundovaný batch, mixed-events batch, mixed-currency batch, refundovaný predaj, dark mode — zarovnanie stĺpcov, farby, badge-y správne. Geometria flex layoutu overená aj priamym meraním (žiadna "diera" v layoute).
- New Sale currency label a per-currency profit preview — layout správny.

**Manuálna logická kontrola (bez GUI behu appky v tomto prostredí):**
- Refund/resell cyklus, migrácia 004, partial unique index — nedotknuté, testy prechádzajú.
- `finance.rs`/`money.rs` — jediné miesto peňažnej matematiky, potvrdené nezmenené (money.rs) / bezpečne rozšírené (finance.rs).
- Backup/Restore, safety backup, rollback, DB lokácia, device-local architektúra — nedotknuté (žiadny súbor v `backup.rs`/`db.rs` sa v tomto vydaní needitoval).
- CSV import (transactional all-or-nothing) — nedotknuté; CSV export dostal len cielenú H6 opravu.
- Existujúci `batch_id` model, order-grouped Tickets/Inventory model — nedotknuté štrukturálne, len 2 nové UI prvky v `OrderDetail.tsx` (cap banner, "View sale" link), oba čisto aditívne.
- BUG #1-7 a Custom date filter fix — testy pre všetky stále prechádzajú bezo zmeny v počte/mennách testov.

## 9. Zmenené súbory

Backend: `finance.rs`, `commands/dashboard.rs`, `commands/sales.rs`, `commands/csv_export.rs`, `models.rs`.
Frontend: `pages/Dashboard.tsx`, `pages/Events.tsx`, `pages/EventDetail.tsx`, `pages/OrderDetail.tsx`, `pages/Sales.tsx`, `pages/SaleDetail.tsx`, `lib/types.ts`, novy súbor `components/RevenueChart.tsx`.
Verzia/deploy: `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `release.ps1`, `1-CLICK-UPDATE.bat`.
Nedotknuté (podľa DO NOT TOUCH): `db.rs`, `money.rs`, `backup.rs`, `csv_import.rs`, všetky 4 migrácie, `codes.rs`, jadro `sales.rs`'s grouping (len 1 cielená oprava v SQL), `.github/workflows/build-windows.yml`.

## 10. Ako to nasadiť

Rovnaký postup ako predtým — v priečinku appky spusti `1-CLICK-UPDATE.bat` (dvojklik), ten zavolá `release.ps1`, ktorý overí, že všetky 3 verzie sedia na 1.6.0, commitne, vytvorí a pushne tag `v1.6.0`, čo spustí GitHub Actions signed build.

**Jedna vec na sledovanie:** naposledy (pri 1.5.0) sa riešil problém, že GitHub Actions niekedy vypľul installer stále pomenovaný podľa starej verzie napriek správnemu zdrojovému kódu. Do `build-windows.yml` pribudli 2 opravy (zmazanie starého GitHub Release pred publikovaním; `release.ps1` teraz vždy vytvorí nový commit) — ale nikdy sa nepotvrdilo, či to definitívne vyriešilo problém, lebo si medzitým prešiel na túto 1.6.0 úlohu. Ak pri sledovaní GitHub Actions po spustení `1-CLICK-UPDATE.bat` opäť uvidíš installer pomenovaný "...1.5.0..." namiesto "...1.6.0...", daj vedieť — je to jediné vlákno z minula, ktoré zostalo definitívne nepotvrdené.

---

Priložený zip obsahuje kompletný zdrojový kód s verziou 1.6.0, pripravený na spustenie `1-CLICK-UPDATE.bat`.

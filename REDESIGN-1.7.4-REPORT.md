# TIQR Manager 1.7.4 — Audit, Bug Fixes, Dashboard Chart, Sales Redesign, New Order polish, Refunded-sale deletion, Delete entire sale, Order-grouped New Sale, Currency picker, Supplier cleanup, Simpler revenue chart, Settings redesign

Dátum: 2026-08-19
Rozsah: audit celej appky → oprava 6 HIGH bugov → Dashboard graf tržieb/profitu → Sales redesign (tabuľka v štýle Tickets) → New Sale/Sale Detail polish → New Order form úpravy (§10) → mazanie refundovaných predajov (§11) → mazanie celého predaja naraz, New Sale zoskupenie podľa objednávky, viditeľný currency picker (§12) → **Supplier preč z New Order/Settings, jednoduchší Dashboard graf (len Revenue), modernejšie Settings (§13)** → verzia 1.7.4.

**Prečo 1.7.0 a nie 1.6.0:** toto vydanie prešlo dvomi kolami - prvý pokus (interne označený 1.6.0, nikdy nenasadený) mal Sales ako karty a graf ako jednoduché stĺpce. Po tvojom vysvetlení (8 tiketov predaných ako 4+2+2 → 3 riadky, presne v štýle, akým Tickets zoskupuje podľa objednávky) som Sales main list prerobil na tabuľku v identickom štýle ako `Tickets.tsx` a graf na moderný smooth area/line chart s interaktívnym hoverom - popísané v §5 a §6 nižšie.

**A teraz 1.7.1:** presne podľa dohody z minula - každá ďalšia zmena dostane vlastnú verziu a vlastný, jednoznačne stiahnuteľný report+zip. §10 nižšie bol jediný nový bod oproti 1.7.0 (New Order form: preč Ticket type, Purchase fees na per-unit, viac mien); §1-9 popisujú predchádzajúce kolo a platia bezo zmeny.

**A teraz 1.7.2:** posledná zo 4 vecí z tvojho zoznamu (mazanie sales/tickets/orders počas testovania) sa nedala vyriešiť bez rozhodnutia, ktoré je tvoje, nie moje - refundovaný sale bol doteraz **natrvalo** nemazateľný, úplne zámerne, presne preto, aby sa nedala stratiť história refundácie. Spýtal som sa ťa priamo (3 možnosti: Reset test dát tlačidlo / Povoliť mazanie refundovaných / Oboje) a vybral si **"Povoliť mazanie refundovaných"** - vedome, s tým, že som ťa vopred upozornil, že je to trvalá zmena správania aj pre budúce reálne dáta, nie len na testovanie. §11 nižšie popisuje presne čo sa zmenilo a prečo je to bezpečné aj v najnebezpečnejšom hraničnom prípade (tiket, ktorý bol refundovaný a odvtedy znova predaný).

**A teraz 1.7.3:** tri samostatné pripomienky z tvojej poslednej správy, každá nezávislá od ostatných. Prvá - na Sale Detail sa dal doteraz zmazať len jeden tiket/riadok naraz, pridal som tlačidlo na zmazanie celého predaja jedným krokom. Druhá - vo "+ New Sale" bol plochý zoznam všetkých tiketov naraz, po jednom; prerobil som to na výber podľa objednávky (najprv zoznam objednávok, klik na jednu ukáže jej tikety v malom okienku, tam vyberieš, ktoré chceš pridať) - keďže tvoje zadanie hovorilo o "celých sales", ktoré by si "otvoril", spýtal som sa ťa priamo, či myslíš zoskupenie podľa objednávky (rovnaký vzor, aký už majú Tickets/Inventory) alebo podľa niečoho iného - potvrdil si **"Podľa objednávky"**. Tretia - v Orders bolo pri mene vidieť len EUR, hoci mena sa dala zmeniť aj predtým; ukázalo sa, že to nebol chýbajúci zoznam (GBP aj USD tam už boli), ale neviditeľné pole.

**A teraz 1.7.4:** tri ďalšie, opäť navzájom nezávislé pripomienky. Prvá - v New Order sa pýta aj na Supplier, čo je pri rýchlom zakladaní objednávky zbytočné; preč z toho formulára aj zo Settings (§13.1) - podrobne nižšie o tom, čo som sa **vedome nedotkol**, aby sa nič reálne nestratilo. Druhá - Dashboard graf sa ti zdal príliš zložitý (dve krivky, farby, legenda); zjednodušený na jednu čistú Revenue krivku, presne podľa tvojich slov "stačí, aby to robilo čiary hore alebo dole" (§13.2). Tretia - Settings mali pôsobiť "priehľadnejšie" a modernejšie; prerobené z jednej plochej mriežky na prehľadné pomenované sekcie (§13.3).

---

## 1. Zhrnutie

Nič z existujúcej architektúry sa nemenilo len kvôli dizajnu. Sales grouping (SaleGroup/batch_id/GROUP_BASE_SELECT), refund/resell logika, migrácie, finance.rs/money.rs, backup/restore, CSV import — všetko zostalo štrukturálne presne také, aké bolo. Zmeny tohto vydania sú buď (a) oprava reálneho bugu nájdeného auditom, (b) vizuálny redesign Sales stránky nad tými istými dátami, (c) nový Dashboard graf nad novým, samostatným SQL dotazom, (d) drobný, bezpečný UX polish priamo vyplývajúci z auditu, alebo (e) vedomá, tebou schválená zmena existujúceho pravidla (refund permanence, §11, rozšírené v §12.1).

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
Fix: refundovaný riadok teraz exportuje `profit = 0` (sale_price/cost/fees ostávajú ako reálne historické hodnoty riadku, `payment_status` stĺpec ukazuje prečo je profit 0). Pri tejto oprave som rozdelil `export_sales_csv` na thin wrapper + testovateľný `export_sales_csv_impl` (rovnaký vzor, aký appka už používa pre `get_dashboard`/`list_sale_groups`), lebo predtým sa táto funkcia nedala vôbec unit-testovať.
Testy (3 nové): `csv_export::tests::active_sale_profit_matches_sale_price_minus_cost_minus_fees`, `refunded_sale_exports_zero_profit_instead_of_a_misleading_realized_number`, `mixed_active_and_refunded_export_sums_to_the_realized_only_total`.

## 5. Dashboard: graf tržieb v čase

Predtým appka nemala ŽIADNU time-series granularitu — všetko bolo jeden preagregovaný súčet za obdobie. Nový backend dotaz (`dashboard.rs::get_dashboard_impl`, sekcia "Revenue/Profit over time") berie presne rovnaký scope, aký už má `period_summary` (`period_from`/`period_to`, `primaryCurrency`, `event_id`/`platform_id` filter, vylúčené refundy) a rozbije ho po dátumových bucketoch namiesto jedného súčtu — takže graf a StatCards nad ním nikdy nemôžu ukázať iné číslo. Backend dodnes počíta aj `profitCents`/`cogsCents` pre každý bucket (nedotknuté, pozri §13.2) - len sa od 1.7.4 v samotnom grafe nepoužívajú.

Šírka bucketu sa prispôsobuje dĺžke obdobia (`time_series_granularity()`): ≤31 dní → deň, ≤180 dní → týždeň, viac (vrátane "All time") → mesiac. Dôvod: "Last 7 days" nemá zmysel po dňoch rozbíjať na 7×nič-iné a viacročné "All time" nemá zmysel po dňoch (tisíce stĺpcov).

Frontend (`src/components/RevenueChart.tsx`) je vlastný, dependency-free SVG graf — **žiadna nová npm závislosť** (appka doteraz nemala žiadnu UI knižnicu okrem React/Tailwind, pridávanie charting knižnice by zbytočne zväčšilo Windows build). Pôvodne (1.7.0-1.7.3) to bol smooth area/line graf s dvomi krivkami (Revenue + Profit); od 1.7.4 je to jedna čistá Revenue krivka - presný popis aktuálneho vzhľadu je v §13.2, tento oddiel opisuje časť, ktorá sa naprieč všetkými verziami nemenila (dátový zdroj, bucketing, umiestnenie na stránke).

Hover je interaktívny namiesto natívneho SVG tooltipu: pohyb myšou nad grafom prepne hlavičku nad grafom na live readout (dátum + presná Revenue hodnota pre ten bucket), plus vertikálna crosshair čiara a bodka na krivke presne v hoverovanom bode. Šírka grafu sa meria cez `ResizeObserver` (namiesto naťahovania fixného viewBoxu na celú šírku), takže súradnicový systém je vždy 1:1 s reálnymi pixelmi na oboch osiach — kruhová hover-bodka preto zostáva kruhová aj keď sa karta zmenší/zväčší, a pozícia myši sa dá prečítať priamo z bounding boxu bez extra prepočtu mierky.

Umiestnenie: hneď pod "Activity" StatCards (Revenue/Purchase cost/Profit/Margin/ROI/Tickets sold) pre zvolené obdobie, nad "Current inventory" sekciou — rozširuje existujúcu informáciu o časovú os, nie je to duplicitná/samostatná sekcia. Profit zostáva plne viditeľný ako StatCard v tomto riadku, aj keď od 1.7.4 už nemá vlastnú krivku v grafe pod ním.

Testy (7, `commands::dashboard::tests`, nezmenené v 1.7.4): bucketing po dňoch a súčet späť na `period` total (cross-check proti driftu), vylúčenie refundov, zoskupenie rovnakého ISO týždňa, zoskupenie rovnakého mesiaca, rešpektovanie event filtra, 6 hraníc granularity vrátane "All time".

## 6. Sales: redesign na tabuľku (rovnaký štýl ako Tickets)

Prvý pokus tohto vydania prerobil Sales main list z tabuľky na karty. Po tvojej spätnej väzbe a konkrétnom príklade (8 tiketov predaných ako 4+2+2 → 3 riadky, klik na riadok → vnútri presný rozpis tiketov k tomu predaju, presne v štýle akým Tickets zoskupuje podľa objednávky) som main list prerobil znova — tentokrát na **tabuľku v identickom vizuálnom štýle ako `Tickets.tsx`** (rovnaké `th`/`td` CSS triedy, rovnaký hover na riadku, rovnaký `overflow-x-auto rounded-xl border ... shadow-sm` wrapper).

Dáta a zoskupenie sa nemenili vôbec — to bolo a stále je správne (`SaleGroup`/`batch_id`/`GROUP_BASE_SELECT`, pozri DO NOT TOUCH). Zmenil sa **len render main listu** (`src/pages/Sales.tsx`) — filter bar, 5000-cap banner, totals summary riadok, aj celá dátová/filtrovacia logika (`listSaleGroups`) sú nedotknuté.

Stĺpce: Sale (kód, link → Sale Detail), Event (link → Event Detail, alebo "Mixed events" kurzívou), Platform, Sale date, Tickets (počet — presne toto číslo ukáže tvoj príklad ako riadky so 4/2/2), Revenue, Fees, Profit (farebne podľa znamienka), Margin/ROI (zlúčené do 1 stĺpca — rovnaká konvencia aká už je na Sale Detail's stat card), Status (Payment badge + amber "N of M refunded" pri čiastočnom refunde).

Klik funguje presne ako na Tickets: len konkrétne `<Link>` bunky (Sale kód, Event) navigujú, zvyšok riadku má len hover-highlight — nie celá karta klikateľná ako v predošlom pokuse. To je presne to, čo `Tickets.tsx` robí, žiadna vlastná interakcia navyše.

Čo sa stalo s 3 UX vylepšeniami z pôvodných kariet: refund indikátor s amber farbou zostal (je aj v tabuľke, pod Status badge). Badge "N tickets" zmizol — nahradilo ho jednoduché číslo v stĺpci Tickets, presne v duchu Tickets.tsx (ten tiež nemá badge na počty, len číslo v stĺpci). Klikateľná celá karta zmizla zámerne — to bol presne ten rozpor, ktorý si opravil svojou spätnou väzbou, keďže Tickets.tsx samotný túto vlastnosť nemá.

Sale Detail (`SaleDetail.tsx`) zostáva vizuálne rovnaké ako v 1.7.1 — tabuľka s per-ticket rozpisom presne podľa zadania, to je to, čo sa otvorí po kliku na riadok. Zmeny tam boli/sú: M9 oprava kódu v hlavičke (1.7.0/1.7.1, nezmenené), nová akcia pri refundovaných riadkoch (1.7.2, pozri §11), a nové tlačidlo "Delete entire sale" v hlavičke (1.7.3, pozri §12.1).

New Sale flow (`SaleFormModal` v `Sales.tsx`) — 3 malé, bezpečné UX opravy priamo z auditu, žiadna z nich sa nedotýka `submit()`/skutočného zápisu do DB:
- **currency label pri Price/Fees** — predtým nebolo vidieť menu pri poliach, teraz je vždy viditeľný label (nie len placeholder, ktorý by zmizol po vyplnení).
- **profit preview pre mixed-currency batch** — predtým len text "different currencies", teraz rozpis Revenue/Profit per mena (`perCurrencyTotals`, čisto lokálny výpočet, rovnaká logika ako existujúci `totals`, len zoskupená po mene).
- **caption pri bulk-apply** — jasne hovorí, že "Apply to all" prepíše už vyplnené hodnoty.

Navyše (UX #7, "ticket → sale" prelinkovanie): `OrderDetail.tsx` pri predanom tikete pridáva link "View sale", ktorý naviguje na `/sales` a predvyplní hľadanie kódom tiketu — cez rovnaký `navigate(path, { state })` vzor, aký už `Orders.tsx` používa pre `presetEventId`, a existujúce ticket-code hľadanie (BUG #5). **Žiadna zmena backendu.**

**Vedome NEurobené v 1.7.0** (z 9 UX nálezov, zvyšné 2 boli vtedy reálne nové features, nie oprava/redesign v rámci vtedajšieho zadaného rozsahu): ticket picker v New Sale orezaný na 25 bez triedenia/stránkovania; bulk-select na Sales hlavnom zozname. Prvý z týchto dvoch bodov už neplatí — presne to rieši §12.2 nižšie (New Sale teraz ide cez objednávky, nie cez plochý orezaný zoznam). Bulk-select na Sales zozname zostáva neurobený, appka dodnes nemá žiadnu bulk-action infraštruktúru na nadviazanie.

## 7. Verzia

`package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`: 1.5.0 → 1.7.0 → 1.7.1 → 1.7.2 → 1.7.3 → **1.7.4** (aktuálna), všetky 3 konzistentne pri každom kroku. Žiadna nová migrácia (presne 4 ako predtým). `release.ps1` a `1-CLICK-UPDATE.bat` tiež aktualizované pri každom kroku (vrátane commit message, ktorá vždy opisuje reálne zmeny toho konkrétneho vydania).

Medzikrok 1.6.0 sa nikdy nenasadil (žiadny tag, žiadny build) — bol to len report/zip, ktorý si dostal predtým, než si ma upozornil na chybu v Sales dizajne. Od 1.7.0 platí: **verzia sa zvýši po každej jednej zmene**, ktorú urobím a pošlem ti na stiahnutie (aj keby to bol len drobný patch) — presne to sa stalo aj teraz (1.7.3 → 1.7.4 pre §13).

## 8. Testovanie a verifikácia

**Automatizované (spustené, nie len tvrdené):**
- `cargo check --lib`: čisté. Log potvrdzuje `Compiling tiqr-manager v1.7.4`.
- `cargo test --lib`: **99 passed / 0 failed / 3 ignored** — nezmenené oproti 1.7.3, keďže 1.7.4 sa vôbec nedotkla backendu (žiadny súbor v `src-tauri/src` sa nezmenil). 3 ignored sú tie isté dávno-existujúce perf testy.
- `cargo clippy --lib --all-targets`: presne tie isté 3 staré, nesúvisiace warningy ako pred týmto vydaním (finance.rs digit-grouping, dashboard.rs if_same_then_else, db.rs type_complexity ×2) — žiadny nový warning, keďže backend sa nezmenil.
- `tsc -b` (samostatne aj cez `npm run build`): čisté, 0 chýb.
- `npm run build` (plný vite build): úspešný, 60 modulov, bundle **296.90 kB / gzip 82.42 kB** (bolo 300.25 kB / gzip 82.96 kB v 1.7.3 — **menšie**, keďže tento krát sa z RevenueChart/Settings odstránilo viac kódu, než sa pridalo). Build log potvrdzuje `tiqr-manager@1.7.4`.

**Vizuálne (Playwright + headless Chromium, cez dočasný preview harness mimo Tauri, zmazaný po použití):**
- Sales tabuľka, SaleDetail refundovaný riadok, Delete entire sale, New Sale order-grouped picker, Orders currency picker (nezmenené v 1.7.4, overené v predchádzajúcich kolách — pozri §5-6, §11, §12).
- **New Order bez Supplier (nové v 1.7.4):** harness s mockovanými objednávkovými dátami, `api.listSuppliers`/`api.createSupplier` zámerne NEmockované (aby prípadné volanie zlyhalo viditeľne). Potvrdené: nula výskytov textu "Supplier" kdekoľvek vo formulári; pole "Platform" prítomné a jeho `<select>` má rovnakú šírku ako pole "Event" nad ním (t.j. zaberá celý riadok namiesto polovičnej medzery po odstránenom Supplier). Nula console error/pageerror.
- **Settings redesign (nové v 1.7.4):** harness s mockovanými platformami, `api.listSuppliers` zámerne nemockované. Potvrdené: nula výskytov nadpisu "Suppliers" kdekoľvek na stránke; karta "Platforms" stále prítomná a naplnená mock dátami; všetky 4 sekčné nadpisy (Appearance, Lookups, Data, Software) prítomné. Nula console error/pageerror.
- **Dashboard graf - len Revenue (nové v 1.7.4):** harness s mock dátami obsahujúcimi vlniacu sa Revenue krivku (hore-dole-hore-dole) a nenulovým `profitCents` na každom bode (aby test dokázal, že sa profit naozaj nikde nevykreslí, nielen že chýba v mock dátach). Kontroly zámerne obmedzené len na kartu s grafom (nie na celú stránku - Dashboard má vlastné "Profit"/"Revenue" StatCards inde na stránke, ktoré by inak falošne prešli). Potvrdené: nadpis karty zmenený na "Revenue over time" (starý "Revenue & profit over time" nikde); nula výskytov textu "Profit" v karte grafu (hoverované aj nehoverované); nula viditeľných legend/swatchov pred hoverom (jedna séria = žiadna legenda, presne podľa dataviz pravidiel); presne 1 `<path>` element v grafe (jedna Revenue krivka, žiadna druhá Profit krivka); statické mriežkové čiary sú plné (nie prerušované), presne 3 (max/stred/nula); pri hoveri sa objaví presne 1 Revenue údaj a prerušovaná crosshair čiara. Nula console error/pageerror. Snímky obrazovky (nehoverovaný aj hoverovaný stav) vizuálne potvrdzujú čistú, jednoduchú krivku idúcu hore aj dole.

**Manuálna logická kontrola (bez GUI behu appky v tomto prostredí):**
- `orders.rs`/`models.rs` (Supplier backend), `csv_import.rs` (auto-vytvorenie supplier-a podľa mena), `csv_export.rs`, `backup.rs`, `db.rs`, `lookups.rs`, `sales.rs` — **nedotknuté**; 1.7.4 je čisto frontend zmena, nula backend súborov v diffe.
- `dashboard.rs`'s `revenue_time_series` — nedotknuté; stále počíta `profitCents`/`cogsCents` presne ako predtým, len `RevenueChart.tsx` ich už nečíta.
- `OrderDetail.tsx` (Edit Order), Orders zoznam (stĺpec Supplier), Tickets.tsx (filter Supplier), `LookupSelect.tsx`, `api.ts` — nedotknuté, Supplier tam funguje presne ako predtým (podrobnosti prečo v §13.1).
- Refund/resell cyklus, migrácia 004, partial unique index, `finance.rs`/`money.rs`, Backup/Restore, `batch_id` model — nedotknuté týmto vydaním.
- BUG #1-7, H1-H6, Custom date filter fix — testy pre všetky stále prechádzajú bezo zmeny v počte/mennách testov.

## 9. Zmenené súbory

Backend: **žiadny** — 1.7.4 je výhradne frontend zmena.
Frontend: `pages/Orders.tsx` (1.7.4: Supplier preč z New Order formulára), `pages/Settings.tsx` (1.7.4: karta Suppliers preč, prerobené na sekcie), `pages/Dashboard.tsx` (1.7.4: text nadpisu grafu), `components/RevenueChart.tsx` (1.7.4: zjednodušené na jednu Revenue krivku), `pages/Events.tsx`, `pages/EventDetail.tsx`, `pages/OrderDetail.tsx`, `pages/Sales.tsx`, `pages/SaleDetail.tsx`, `lib/types.ts`, `lib/api.ts`.
Verzia/deploy: `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `release.ps1`, `1-CLICK-UPDATE.bat`.
Nedotknuté (podľa DO NOT TOUCH): `db.rs`, `money.rs`, `backup.rs`, `csv_import.rs`, všetky 4 migrácie, `codes.rs`, `sales.rs` (celý súbor, vrátane `commands/sales.rs`), `orders.rs`, `models.rs`, `.github/workflows/build-windows.yml`.

## 10. Verzia 1.7.1: New Order form úpravy

Tri drobné, presne cielené zmeny, všetky len v `src/pages/Orders.tsx` (`OrderFormModal` - formulár "New order"). Nič iné sa nezmenilo: dátový model, migrácie, CSV import/export, ani samotný zoznam objednávok (`Orders`/`OrderDetail`).

**Preč pole "Ticket type"** — formulár mal 4 polia (Ticket type, Section, Row, Seats), teraz len 3 (Section, Row, Seats). Odstránený je len vstup pri vytváraní objednávky - `ticket_type` v dátovom modeli aj naďalej existuje a dá sa nastaviť neskôr priamo na tikete (Tickets → Edit), takže sa nič nestráca, len sa to pri zakladaní objednávky nepýta. Mriežka polí je prerobená (Section+Row vedľa seba, Seats na celú šírku), aby po odstránení jedného poľa nezostala prázdna diera.

**Purchase fees teraz per-unit, presne ako Unit purchase price** — pole sa premenovalo na "Unit purchase fees ({mena})" a už nehovorí "(total)"/"Split evenly across all tickets". Zadáš poplatok za JEDEN tiket, appka ho sama vynásobí počtom kusov (rovnako ako to už robí Unit purchase price). "Total cost (preview)" v spodnej časti formulára to počíta správne (overené: 4 tikety × 25.50 unit price + 4× 2.00 unit fees = €110.00).

Backend (`orders.rs`) sa **vôbec nemenil** — `OrderInput.feesCents` tam už dávno znamená "celkové fees za objednávku" a rozdeľuje ich rovnomerne cez existujúci, otestovaný `allocate_cents`. Namiesto prerábania backendu frontend teraz jednoducho pošle `zadaná_suma × počet_tiketov` ako predtým očakávaný total - keďže je to vždy presný násobok počtu kusov, `allocate_cents` každému tiketu pridelí presne tú istú sumu, čo si zadal, bez akéhokoľvek zaokrúhľovacieho zvyšku. CSV import a demo dáta idú cez ten istý backend nezmenené, takže sú touto úpravou úplne nedotknuté - zmenilo sa naozaj len to, čo vidíš vo formulári "New order".

**Viac mien v zozname** — `CURRENCIES` v `Orders.tsx` rozšírené z 10 na 13: pribudli **RON** (Rumunsko), **TRY** (Turecko), **BGN** (Bulharsko) - najbližšie väčšie trhy, ktoré ešte chýbali vedľa už existujúcich EUR/USD/GBP/CHF/CZK/PLN/HUF/SEK/NOK/DKK. (V 1.7.1 bola mena stále len voľný text s návrhmi, len bez viditeľnej šípky/dropdownu - to vyriešilo až 1.7.3, §12.3.)

Overené: `tsc -b` aj `npm run build` čisté, `cargo test --lib` nezmenené 95/0/3 (nulová backend zmena), a vizuálne cez Playwright - formulár otvorený, vyplnený (Quantity 4, Unit purchase price 25.50, Unit purchase fees 2.00), potvrdené že "Ticket type" pole v DOM neexistuje (count 0), potvrdený zoznam 13 mien, a total preview ukázal presne €110.00.

## 11. Verzia 1.7.2: Mazanie refundovaných predajov

**Kontext.** Tvoja 4. požiadavka z minulej správy bola možnosť vymazať sales, lebo si počas testovania nevedel zmazať ani tickets, ani orders. Preskúmal som to a zistil som, že to nie je jednoduchý bug, ale naráža to na **zámernú ochranu dát**: jednotlivý sale sa dal (a stále dá) zmazať cez Sale Detail's ikonku koša, ale **iba pokým nebol refundovaný** — refundovaný sale bol natrvalo nezmazateľný, presne aby história refundácií nikdy nezmizla. A keďže `delete_order_impl` blokuje mazanie objednávky, kým existuje čo i len jeden sales záznam (aj refundovaný) pre ktorýkoľvek jej tiket, práve refundované testovacie predaje ťa natrvalo blokovali aj pri mazaní objednávok. Namiesto tichej zmeny takéhoto pravidla som sa ťa spýtal priamo - a vybral si **"Povoliť mazanie refundovaných"**.

**Čo presne sa zmenilo (`sales.rs`, `delete_sale_impl`).** Predtým: pokus o zmazanie refundovaného sale vrátil chybu "This sale has been refunded and is kept as history - it can't be deleted." a nič sa nestalo. Teraz: refundovaný sale sa dá zmazať úplne rovnako ako aktívny (tou istou funkciou, tým istým tlačidlom).

**Prečo je to bezpečné aj v najhoršom hraničnom prípade.** Toto je jediná naozaj riziková časť tejto zmeny, tak som jej venoval extra pozornosť. Vďaka migrácii 004 (BUG #1 fix, existuje už dávno, nedotknutá) môže mať jeden tiket v históriu **viac ako jeden** sales záznam naraz - napríklad starý refundovaný + nový aktívny (ak si tiket refundoval a potom znova predal). Predtým mazanie sale vždy nastavilo tiket na "Available" - to je správne pre mazanie AKTÍVNEHO predaja (bol to jediný predaj, ktorý mohol tiket označiť ako predaný), ale bolo by to **katastrofálne nesprávne** pre mazanie starého REFUNDOVANÉHO záznamu, ak medzičasom vznikol nový aktívny predaj - naivná oprava by omylom "odpredala" tiket, ktorý je v tú chvíľu reálne predaný pod úplne iným, novším sale. Oprava preto rozlišuje: mazanie aktívneho sale aj naďalej nastaví tiket na Available (nezmenené), mazanie refundovaného sale sa **vôbec nedotkne** stavu tiketu - ten je už správne nastavený nezávisle od tohto historického záznamu. Presne tento scenár overuje nový test `deleting_an_old_refunded_sale_never_disturbs_a_newer_active_sale_on_the_same_ticket`.

**Orders (`orders.rs`) — žiadna zmena kódu, len presnejší komentár.** `delete_order_impl`'s kontrola "má táto objednávka nejaký sales záznam (aj refundovaný)?" je živý `COUNT(*)` dotaz, nie uložená hodnota - takže keď teraz zmažeš refundovaný sale cez Sale Detail, počet klesne sám a objednávka sa stane zmazateľnou bez toho, aby som musel meniť čokoľvek v `orders.rs`. Upravil som len sprievodný komentár, aby presne opisoval nové správanie, a text potvrdzovacieho dialógu v `OrderDetail.tsx` (teraz spomína, že refundované predaje sa dajú zmazať jednotlivo cez Sale Detail).

**Frontend (`SaleDetail.tsx`).** Refundovaný riadok predtým ukazoval len text "Locked" namiesto akcií. Teraz ukazuje ikonku koša (Edit/Refund ostávajú skryté - backend ich aj naďalej odmieta pre už-refundovaný sale, to sa nemenilo). Potvrdzovací dialóg má pre refundovaný riadok **iný text** než pre aktívny predaj - jasne hovorí, že sa maže len historický záznam, že samotný tiket to nijako neovplyvní (ten sa už vrátil na Available pri refundácii) a že po zmazaní už nebude žiadna stopa po tom, že bol tiket predaný a refundovaný. Aj toast správa po zmazaní je iná ("Refund record deleted" namiesto "Sale deleted, ticket is available again").

**Aktualizácia "na toto nesahaj" pravidla.** Toto vydanie vedome mení jeden bod z nášho zoznamu vecí, na ktoré sa nesiaha bez vysvetlenia - konkrétne "refund permanence". Nové pravidlo od 1.7.2 (rozšírené v 1.7.3 §12.1 aj na mazanie celého predaja naraz): refundovaný sale sa **dá** zmazať (jednotlivo cez Sale Detail, alebo od 1.7.3 aj ako súčasť mazania celého predaja), ale zmazanie **nikdy** neovplyvní stav tiketu. Zvyšok zoznamu (refund/resell logika, migrácia 004, partial unique index, `batch_id`, Sales/Tickets dátový model, `finance.rs`/`money.rs`, Restore/backup, atď.) zostáva presne taký, aký bol.

**Testy.** Starý test `refunded_sale_cannot_be_hard_deleted` (tvrdil presný opak novej požiadavky) nahradený dvomi novými - `refunded_sale_can_now_be_deleted_and_does_not_touch_an_already_available_ticket` (základný prípad) a `deleting_an_old_refunded_sale_never_disturbs_a_newer_active_sale_on_the_same_ticket` (kritický hraničný prípad, popísaný vyššie). `cargo test --lib`: 96/0/3 (bolo 95/0/3). `cargo clippy`: bez nového warningu. Vizuálne overené cez Playwright (mock dáta, plný kolobeh vrátane skutočného potvrdenia a zmazania) - podrobnosti v §8.

**Čo som sa vedome nedotkol.** Reset-all-test-data tlačidlo (možnosť "A"/"C" z otázky, ktorú si nevybral) som neimplementoval - ak by si to predsa len chcel neskôr (napr. ako rýchlejšiu cestu k čistému stavu popri tejto zmene), stačí povedať.

## 12. Verzia 1.7.3: Mazanie celého predaja, New Sale podľa objednávky, viditeľný currency picker

**Kontext.** Tri samostatné pripomienky z testovania, každá vo svojej vlastnej časti appky, žiadna z nich sa nedotýka databázovej štruktúry ani žiadneho z existujúcich pravidiel - okrem prirodzeného rozšírenia pravidla z §11 (refund permanence) na "zmaž celý predaj naraz" (§12.1).

### 12.1 Sale Detail: "Delete entire sale"

**Čo bolo predtým.** Na Sale Detail sa dal zmazať len jeden riadok (jeden tiket) naraz, cez ikonku koša pri danom riadku. Pri predaji s viacerými tiketmi (napr. 4+2+2 vzor zo Sales redesignu, §6) to znamenalo klikať na zmazanie opakovane, riadok po riadku.

**Čo je nové.** Pribudlo tlačidlo "Delete entire sale" priamo v hlavičke stránky (`SaleDetail.tsx`), vedľa čísla predaja. Zmaže **všetky** riadky patriace k danému predaju naraz - `delete_sale_group_impl` (`sales.rs`) najprv zistí `batch_id` daného riadku a podľa neho nájde celú skupinu presne tým istým spôsobom, akým to už robí `list_sales_by_group_impl` pre samotné zobrazenie stránky (ak riadok nemá `batch_id`, skupina je len ten jeden riadok) - takže "celý predaj", ktorý appka zmaže, je zaručene presne to isté, čo appka aj zobrazuje ako "tento predaj". Celá skupina sa maže v **jednej DB transakcii** - buď zmizne celá, alebo pri chybe nezmizne nič, nikdy nie "napoly".

**Bezpečnosť pri refundovaných riadkoch v skupine.** Skupina môže obsahovať zmes aktívnych aj refundovaných riadkov (napr. 3 tikety predané spolu, 1 z nich medzitým refundovaný). Pravidlo z §11 platí riadok po riadku, nie na celú skupinu naraz: aktívny (nerefundovaný) riadok pri zmazaní vráti svoj tiket na "Available", refundovaný riadok sa zmaže bez toho, aby sa čo i len dotkol stavu tiketu - presne preto, že ten tiket mohol byť odvtedy znova predaný pod úplne iným, novším predajom mimo tejto mazanej skupiny. Toto som overil samostatným, dedikovaným testom (`delete_sale_group_never_disturbs_a_different_newer_sale_on_a_resold_ticket`) - nespoliehal som sa na to, že nová skupinová funkcia automaticky zdedí bezpečnosť jednoriadkovej opravy z 1.7.2, overil som to nezávisle.

**Potvrdzovací dialóg** jasne hovorí, koľko tiketov sa zmaže, že aktívne sa vrátia na Available, že refundované zmiznú bez stopy, a že to nejde vrátiť späť. Po potvrdení appka ukáže toast s presným počtom ovplyvnených tiketov a presunie ťa späť na zoznam Sales (na rozdiel od mazania jedného riadku, tu už niet na čo v rámci stránky zostať ukotvený - celý predaj je preč).

**Testy (3 nové):** `delete_sale_group_removes_every_line_in_a_batch_and_resets_only_non_refunded_tickets`, `delete_sale_group_on_a_single_non_batch_sale_behaves_like_deleting_just_that_one_line`, `delete_sale_group_never_disturbs_a_different_newer_sale_on_a_resold_ticket`.

### 12.2 New Sale: výber tiketov podľa objednávky

**Čo bolo predtým.** V "+ New Sale" bol jeden plochý, vyhľadávateľný zoznam VŠETKÝCH dostupných tiketov naraz (orezaný na 25 podľa hľadaného textu) - pri väčšom počte objednávok/tiketov ťažko prehľadný, žiadna štruktúra.

**Ktoré zoskupenie?** Tvoje zadanie hovorilo o "celých sales, ktoré otvorím, v tom sa mi ukážu tikety" - keďže appka slovo "sale" používa aj pre samotný predaj (Sales/SaleDetail), no z kontextu (vyberáš si TIKETY NA predaj, ešte pred jeho vznikom) bolo jasnejšie, že myslíš zoskupenie zdrojových tiketov, nie existujúce predaje. To by mohlo znamenať podľa objednávky (rovnaký vzor ako Tickets/Inventory) alebo podľa eventu - opýtal som sa priamo, potvrdil si **"Podľa objednávky"**.

**Čo je nové.** Prvý krok New Sale formulára teraz ukazuje zoznam **objednávok** (kód, event, platforma, dátum nákupu, zelený badge "N available" = súčet available+listed tiketov tej objednávky), s vyhľadávaním nad nimi. Klik na objednávku otvorí malé okienko s presne jej tiketmi (dostupné/listed, zoradené od najnovšieho), kde vyberieš, ktoré konkrétne chceš pridať - tlačidlo "← Back to orders" sa vráti na zoznam objednávok. Vybrané tikety (sekcia "Selected (N)" s čipmi) zostávajú viditeľné a zachované aj pri prechode medzi rôznymi objednávkami - takže jeden predaj môže obsahovať tikety z viacerých objednávok naraz, presne ako predtým, len s prehľadnejšou cestou k výberu.

**Žiadna zmena backendu.** `list_orders_impl` aj `list_tickets` (obidva už existujúce, už otestované) mali presne tie parametre, ktoré táto redizajnovaná obrazovka potrebuje - `status: "available,listed"` (objednávky/tikety, ktoré majú ešte čo predať) a `search` (kód objednávky, event, platforma, dodávateľ, **aj kód tiketu** - toto pole už predtým prehľadávalo aj kódy tiketov kvôli BUG #5, takže keď niekto napíše presný kód tiketu do vyhľadávania objednávok, appka aj naďalej nájde správnu objednávku, hoci vyhľadávacie pole už nie je na úrovni tiketov). `OrderRecord.availableCount`/`listedCount` (už existujúce polia) sa použili priamo pre badge "N available", bez potreby nového SQL dotazu.

**Testy:** čisto frontend zmena (appka nemá frontend test framework, rovnako ako všetky predošlé frontendové redesigny) - overené `tsc -b`/`npm run build` a vizuálne cez Playwright (§8): zoznam objednávok namiesto plochých tiketov, otvorenie objednávky, výber z dvoch rôznych objednávok do jedného zoznamu, vyhľadávanie filtrujúce objednávky.

### 12.3 Orders: viditeľný currency picker

**Čo bolo predtým.** Pole "Currency" pri zakladaní novej objednávky bolo `<input list="currency-list">` s `<datalist>` - vizuálne nerozoznateľné od bežného textového poľa, predvyplnené na "EUR", bez akejkoľvek šípky/ikonky naznačujúcej, že existuje zoznam možností na výber. Mena sa dala zmeniť už predtým (prepísaním textu), len to nebolo vidieť.

**Skutočný root cause.** Preveril som `CURRENCIES` pole v `Orders.tsx` skôr, než som čokoľvek menil - **GBP aj USD tam už boli**, dokonca hneď za EUR na začiatku zoznamu (`["EUR", "USD", "GBP", "CHF", "CZK", "PLN", "HUF", "SEK", "NOK", "DKK", "RON", "TRY", "BGN"]`, nezmenené od 1.7.1 §10). Problém nebol chýbajúci zoznam mien, bol to čisto neviditeľný UI prvok - takže pridávanie ďalších kódov mien by bolo riešením niečoho iného, než čo si reálne narazil.

**Fix.** Pole je teraz skutočný, viditeľný `<Select>` (rovnaká komponenta, aká sa používa všade inde v appke) s 13 menami, EUR predvolené. Vedľa labelu pribudlo tlačidlo "Other..." (rovnaký vzor "+ New" prepínača, aký appka už používa v `LookupSelect.tsx` pre Event/Platform/Supplier) - klik naň prepne pole na voľný textový vstup pre menu, ktorá nie je v zozname (napr. AED), s automatickým veľkými písmenami. Prepnutie späť na "Choose from list" ukáže aktuálne zadanú vlastnú menu ako súčasť zoznamu (aj keby v `CURRENCIES` nebola), takže sa nikdy nestratí to, čo si už napísal.

**Testy:** čisto frontend zmena, overené `tsc -b`/`npm run build` a vizuálne cez Playwright (§8): skutočný `<select>` namiesto neviditeľného `<input list>`, 13 možností, prepnutie na vlastný vstup a späť, vlastná hodnota (AED) sa zachová a zobrazí v zozname.

**Čo som sa vedome nedotkol.** Samotné `CURRENCIES` pole som nerozširoval ani nepreusporadúval (žiadny dôvod - problém nebol v zozname). Ak by si chcel pridať/odobrať konkrétne meny zo zoznamu ("a potom pridame dalsie veci"), stačí povedať, je to jednoriadková zmena.

**Zhrnutie testov 1.7.3:** `cargo test --lib` 99/0/3 (bolo 96/0/3), `cargo clippy` bez nového warningu, `tsc -b`/`npm run build` čisté (300.25 kB / gzip 82.96 kB). Žiadna nová DB migrácia, žiadna nová npm/cargo závislosť.

## 13. Verzia 1.7.4: Supplier preč z New Order/Settings, jednoduchší Dashboard graf, modernejšie Settings

**Kontext.** Tri ďalšie, navzájom nezávislé pripomienky - žiadna z nich sa nedotýka databázy, migrácií, CSV importu/exportu ani žiadneho z doterajších pravidiel. Všetky tri sú čisto frontend.

### 13.1 New Order + Settings: Supplier preč z formulárov

**Čo si chcel.** V "New order" sa appka pýta aj na Supplier, čo je pri rýchlom zakladaní objednávky zbytočné - preč z toho formulára. A tiež preč zo Settings.

**Prečo som sa najprv pozrel, kade všade Supplier vedie.** Supplier nie je len jedno pole - je to plnohodnotná entita ako Platform, s vlastnou správou (Settings), vlastným stĺpcom v objednávkach (`orders.supplier_id`), a používa sa aj v CSV importe (`csv_import.rs` si podľa mena z CSV stĺpca "supplier" sám vytvorí nový supplier záznam, ak ešte neexistuje - `resolve_or_create_supplier`), v CSV exporte, v Tickets filtri, aj v Edit Order formulári. Preto som sa rozhodol zámerne **neodstrániť Supplier zo všetkého**, len presne z tých dvoch miest, ktoré si menoval:
- **New Order formulár** (`Orders.tsx`) - pole Supplier preč, LookupSelect pre Platform teraz zaberá celý riadok namiesto toho, aby ostala prázdna polovičná medzera (rovnaký princíp ako pri odstránení "Ticket type" v 1.7.1, §10).
- **Settings** (`Settings.tsx`) - celá karta "Suppliers" (pridávanie, zoznam, mazanie) preč.

**Čo som sa vedome NEdotkol** (a prečo to nič nerozbije):
- **Edit Order** (na Order Detail stránke) - Supplier pole tam **zostáva** presne také, aké bolo, vrátane vlastného "+ New" tlačidla na vytvorenie novej hodnoty priamo tam. Takže existujúcu/CSV-importovanú objednávku vieš aj naďalej podľa potreby označiť alebo opraviť dodávateľom - len sa na to appka nepýta hneď pri rýchlom zakladaní.
- **Zoznam Orders** (stĺpec Supplier) a **Order Detail** (read-only "Supplier" údaj) - obidva ďalej zobrazujú historické dáta, ak ich objednávka má. Nie je to editovateľné miesto, len zobrazenie skutočných dát, takže nebol dôvod ho skrývať.
- **Tickets** stránka - jej vlastný filter "Supplier" (dropdown na filtrovanie tiketov podľa dodávateľa) je nedotknutý.
- **Backend** (`orders.rs`, `models.rs`) - vôbec nezmenený. `supplierId` bol už predtým voliteľný (`Option<i64>` / v TS `supplierId?: number | null`), takže New Order formulár teraz jednoducho vždy posiela `supplierId: null` (explicitne, nie len vynechaním kľúča - jasnejšie pre budúceho čitateľa kódu).
- **CSV import** (`csv_import.rs`) - vôbec nezmenený, stále rozpoznáva stĺpec "supplier" a podľa mena si sám vytvorí supplier záznam. Popis stĺpcov na karte "Import orders from CSV" v Settings preto aj naďalej správne spomína "supplier" ako platný stĺpec.
- **`api.ts`**'s `listSuppliers`/`createSupplier`/`deleteSupplier` - ponechané, používa ich Edit Order formulár.

**Testy:** čisto frontend zmena, `tsc -b`/`npm run build` čisté, vizuálne overené cez Playwright (§8) - vo formulári New Order sa nikde nevyskytuje text "Supplier", pole Platform je rovnako široké ako pole Event nad ním.

### 13.2 Dashboard: jednoduchší graf (len Revenue)

**Čo si chcel.** Graf s dvomi krivkami (Revenue plocha + Profit čiara, ktorá menila farbu, plus legenda) ti prišiel príliš zložitý - stačí, aby to robilo čiary hore alebo dole, a stačí, aby počítalo Revenue.

**Najprv som si prečítal interný dataviz návod**, ktorý appka má k dispozícii pre akúkoľvek prácu s grafmi, predtým než som čokoľvek menil. Dve jeho pravidlá presne sedeli na tvoju požiadavku: (1) "jedna séria nepotrebuje legendu - názov karty už hovorí, čo sa zobrazuje" - potvrdilo, že úplné odstránenie stálej legendy je správne, nielen zjednodušenie kvôli zjednodušeniu. (2) mriežkové čiary majú byť vždy plné, nikdy prerušované - graf ich mal doteraz prerušované; drobná, dovtedy nepomenovaná chybička, opravená v rámci tejto úpravy.

**Čo sa zmenilo (`RevenueChart.tsx`).** Profit krivka aj vyplnená plocha pod Revenue krivkou sú preč - zostala jedna čistá 2px Revenue krivka (stále jemne vyhladená, rovnako ako predtým). Os Y sa zjednodušila z "môže ísť aj pod nulu" (kvôli Profitu, ktorý mohol byť záporný) na jednoduchú škálu 0→maximum, keďže Revenue samo osebe nikdy záporné nie je - vďaka tomu zmizla aj samostatná "nulová" čiara, lebo 0 je teraz prirodzene jedna z troch mriežkových hodnôt. Hover (najazdenie myšou) funguje rovnako ako predtým - crosshair čiara, bodka na krivke, dátum + suma v hlavičke - len bez druhého Profit riadku.

**Backend (`dashboard.rs`) som zámerne nedotkol.** `revenue_time_series` aj naďalej počíta `profitCents`/`cogsCents` pre každý bucket presne ako predtým (už funkčný, už otestovaný kód, netreba ho meniť) - graf ich odteraz len nečíta. Profit sám osebe z appky nezmizol - ostáva vlastný StatCard v riadku "Activity" nad grafom, presne ako predtým; zmenil sa len rozpis po jednotlivých obdobiach v grafe pod ním. Nadpis karty sa zmenil z "Revenue & profit over time" na "Revenue over time".

**Testy:** backend nezmenený, takže `cargo test`/`clippy` čísla sú rovnaké ako v 1.7.3 (§8). Frontend overené vizuálne cez Playwright, cielene len v rámci karty s grafom (nie na celej stránke - Dashboard má vlastné, nesúvisiace "Profit"/"Revenue" StatCards inde, ktoré by inak skreslili výsledok): nula výskytov "Profit" v karte (hoverovanej aj nie), nula legiend pred hoverom, presne 1 krivka v grafe, mriežka plná nie prerušovaná, hover ukazuje presne 1 Revenue údaj. Aj vizuálne (screenshot) potvrdené - krivka jasne ide hore aj dole na testovacích dátach.

### 13.3 Settings: prerobené na prehľadné sekcie

**Čo si chcel.** Aby Settings pôsobili "priehľadnejšie" a modernejšie. (Toto som pochopil ako "prehľadnejšie/zrozumiteľnejšie" - jasnejšie usporiadanie a vizuálna hierarchia - nie doslovnú vizuálnu priehľadnosť/sklo. Ak si myslel niečo iné, daj vedieť.)

**Čo bolo predtým.** Jedna plochá mriežka so 7 rovnako veľkými kartami (Appearance, Platforms, Suppliers, Import CSV, Export CSV, Backup & restore, Software updates) bez akéhokoľvek zoskupenia - len náhodné zalamovanie do 2 stĺpcov.

**Čo je nové.** Prerozdelené do 4 pomenovaných sekcií, rovnakým štýlom nadpisu (VEĽKÉ PÍSMENÁ, tenké, sivé), aký appka už používa napríklad pre "Activity" na Dashboarde - takže to pôsobí ako súčasť appky, nie ako nový, cudzí vzor:
- **Appearance** - prepínač témy (Light/System/Dark), nezmenené.
- **Lookups** - Platforms (jediný "lookup" karta, ktorá po odstránení Suppliers zostala - už sa nemusí vizuálne párovať s ničím).
- **Data** - Import CSV a Export CSV vedľa seba, Backup & restore na celú šírku pod nimi (má navyše informáciu o umiestnení databázového súboru, tak dostal viac priestoru).
- **Software** - Software updates.

Platforms a Software updates majú teraz obmedzenú maximálnu šírku (predtým sa naťahovali podľa toho, s čím boli spárované v mriežke) - keď sú samé vo svojej sekcii, nedáva zmysel naťahovať jednoduchý formulár/zoznam na celú šírku širokého okna.

Žiadne správanie sa nezmenilo - každé tlačidlo, každý handler robí presne to, čo predtým, ide výhradne o usporiadanie/vizuál.

**Testy:** čisto frontend zmena, `tsc -b`/`npm run build` čisté, vizuálne overené cez Playwright (§8) - nadpis "Suppliers" sa nikde na stránke nevyskytuje, karta Platforms je prítomná a funkčná, všetky 4 sekčné nadpisy prítomné.

**Zhrnutie testov 1.7.4:** `cargo test --lib` 99/0/3 (nezmenené - žiadna backend zmena), `cargo clippy` bez nového warningu, `tsc -b`/`npm run build` čisté (296.90 kB / gzip 82.42 kB - menej ako 300.25/82.96 v 1.7.3, keďže sa tento krát viac kódu odstránilo než pridalo). Žiadna nová DB migrácia, žiadna nová závislosť, žiadny backend súbor sa nedotkol. Podrobnosti k vizuálnym testom v §8.

## 14. Ako to nasadiť

Rovnaký postup ako predtým — v priečinku appky spusti `1-CLICK-UPDATE.bat` (dvojklik), ten zavolá `release.ps1`, ktorý overí, že všetky 3 verzie sedia na 1.7.4, commitne, vytvorí a pushne tag `v1.7.4`, čo spustí GitHub Actions signed build.

**Jedna vec na sledovanie:** naposledy (pri 1.5.0) sa riešil problém, že GitHub Actions niekedy vypľul installer stále pomenovaný podľa starej verzie napriek správnemu zdrojovému kódu. Do `build-windows.yml` pribudli 2 opravy (zmazanie starého GitHub Release pred publikovaním; `release.ps1` teraz vždy vytvorí nový commit) — ale nikdy sa nepotvrdilo, či to definitívne vyriešilo problém, lebo si medzitým prešiel na ďalšie úlohy. Ak pri sledovaní GitHub Actions po spustení `1-CLICK-UPDATE.bat` uvidíš installer pomenovaný podľa starej verzie namiesto "...1.7.4...", daj vedieť — je to jediné vlákno z minula, ktoré zostalo definitívne nepotvrdené.

---

Priložený zip obsahuje kompletný zdrojový kód s verziou 1.7.4, pripravený na spustenie `1-CLICK-UPDATE.bat`.

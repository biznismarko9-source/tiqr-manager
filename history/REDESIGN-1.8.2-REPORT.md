# TIQR Manager 1.8.2 — Sales & Settings UX Polish

Tento report nahrádza REDESIGN-1.8.1-REPORT.md a je štruktúrovaný presne podľa zadania pre 1.8.2 (13
sekcií). 1.8.0 (Sales Management / Sales 2.0 — search, filtre, sorting, bulk export) a 1.8.1 (oprava
balenia release zipu, kvôli ktorej sa nespúšťali GitHub Actions) sú hotové a stabilné — 1.8.2 na nich nič
nemení. Toto vydanie je **výhradne UX/layout** — Sales, Sale Detail a Settings. Žiadna zmena backendu,
databázovej schémy ani migrácií. Ani jeden `.rs` súbor nebol v tomto kole vôbec otvorený.

---

## 1. Prečo bol layout Sales problém

Tabuľka na `/sales` mala `<table className="w-full min-w-[1220px] ...">` v `overflow-x-auto` wrapperi.
To nie je len teoretický problém pri malom okne — je to problém **pri bežnej, predvolenej šírke okna**:

- Okno aplikácie sa spúšťa na `width: 1400` (tauri.conf.json).
- Sidebar má pevných 224px (`w-56` v `Layout.tsx`), obsahový wrapper má `px-6` padding (24px z každej
  strany = 48px).
- Reálne dostupná šírka pre obsah pri predvolenom okne: 1400 − 224 − 48 = **1128px**.
- Tabuľka si ale vynucovala `min-width: 1220px` → **92px cez dostupnú šírku aj pri predvolenom okne**,
  takže horizontálny scroll sa objavoval prakticky vždy, nielen pri zúženom okne.

Rovnaký typ problému mala aj `/sales/:id` (Sale Detail) tabuľka ticketov: `min-w-[1150px]` — tesnejšie
pod 1128px, ale stále len kúsok od problému a bez akejkoľvek rezervy.

## 2. Ako bol horizontálny scroll odstránený

Namiesto "zmenšiť font, kým sa to nezmestí" (čo bolo explicitne zakázané v zadaní) som použil
`table-layout: fixed` + `<colgroup>` s pevnou šírkou pre každý stĺpec **okrem jedného** (Event), ktorý
zostáva bez šírky a pohltí zvyšný priestor. Toto je matematická, nie vizuálna záruka: pokým súčet
pevných stĺpcov ostáva bezpečne pod minimálnou možnou šírkou obsahu, scroll sa **nemôže** objaviť, nech
je okno akokoľvek veľké — nie je to odhad vykresľovania, je to súčet čísel.

Najužšie možné okno aplikácie je `minWidth: 1080` (tauri.conf.json, OS-vynútené), čo dáva **808px**
dostupného obsahu (1080 − 224 − 48) ako absolútnu spodnú hranicu.

Stĺpce na `/sales` (12 stĺpcov vrátane checkboxu), pevná šírka každého okrem Event:

| Stĺpec | Šírka | Poznámka |
|---|---|---|
| checkbox | 32px | |
| Sale | 90px | kód predaja, truncate + tooltip |
| **Event** | *(flexibilný)* | pohlcuje zvyšok, truncate + tooltip |
| Platform | 70px | truncate + tooltip |
| Date | 76px | `formatDateCompact` — napr. "15 Aug 26" |
| Tix | 40px | počet ticketov, vpravo zarovnané |
| Revenue | 84px | vpravo zarovnané |
| Fees | 64px | vpravo zarovnané |
| Cost | 72px | vpravo zarovnané |
| Profit | 72px | vpravo zarovnané, farebné podľa znamienka |
| Margin/ROI | 68px | dvojriadková bunka (Margin nad ROI) |
| Status | 92px | badge + skrátený refund text "N/M refunded" |

Súčet pevných stĺpcov: **760px**. Aj pri absolútne najužšom okne (808px) ostáva Event stĺpcu 48px, ktoré
rastú s každým ďalším pixelom šírky okna. Pri predvolenom okne (1128px) má Event cez 360px.

Ďalšie zmeny, ktoré k tomu boli potrebné:

- Nové CSS triedy `.th-c` / `.td-c` v `src/index.css` — rovnaká veľkosť písma ako pôvodné `.th`/`.td`
  (nikdy nezmenšené na nečitateľnú veľkosť), len tesnejší padding. Sú to **nové, samostatné** triedy —
  `.th`/`.td` zostávajú nedotknuté a naďalej ich používajú Tickets/Orders/Events/Inventory presne ako
  predtým.
- Dlhé texty (Event, Platform, Sale kód, dátum) majú `truncate` + `title` (natívny tooltip) — nikdy sa
  nezobrazí orezaný text bez možnosti vidieť plnú hodnotu na hover.
- **Peňažné stĺpce (Revenue/Fees/Cost/Profit) nikdy nemajú `truncate`** — dáta sa nesmú skrývať. Ak by
  bolo číslo neočakávane dlhé, bunka sa zalomí na 2 riadky (čo zadanie výslovne povoľuje), nikdy sa
  neorezáva a nikdy nepretečie mimo tabuľku.
- Nový `formatDateCompact()` helper (`src/lib/format.ts`) — krátky formát dátumu, ktorý si zámerne
  ponecháva 2-ciferný rok (aplikácia sleduje predaje naprieč rokmi) a vypisuje mesiac ako krátke meno
  (nikdy holé číslo), aby nebola nejednoznačnosť DD/MM vs MM/DD.
- Margin/ROI je teraz dvojriadková bunka (Margin nad ROI) namiesto "12.3% / 8.1%" na jednom riadku.
- Refund text skrátený z "N of M refunded" na "N/M refunded" (plný text je stále v `title` tooltipe).
- `overflow-x-auto` na wrapperi zostáva ako **defenzívny fallback** — pri správnom súčte by sa nikdy
  nemal aktivovať, ale ak by sa niekedy v budúcnosti pridal ďalší stĺpec bez prepočítania budgetu, aspoň
  nedôjde k vizuálnemu pretečeniu.

## 3. Ako bol Sale Detail prerobený

**SUMMARY** (predtým 4 karty: Revenue / Selling fees / Profit / Margin+ROI spolu) je teraz **6 kariet**
presne podľa zadania: Revenue, Fees, Cost, Profit, Margin, ROI — každá samostatne. `Cost` sa predtým
nezobrazovala vôbec, hoci `header.costCents` bola už dávno počítaná v existujúcom `useMemo` — pridanie
karty je čisto zobrazovacia zmena, nič sa prepočítalo inak. Grid je `grid-cols-3 lg:grid-cols-6`, čo pri
minimálnej šírke okna tejto aplikácie (1080px, nad `lg:` breakpointom 1024px) vždy vykreslí jeden riadok
6 kariet.

**Tabuľka ticketov** — Section/Row/Seat (pôvodne 3 samostatné stĺpce) sú zlúčené do jedného stĺpca
**Seat** cez nový `formatSeatLocation()` helper: vynecháva časti, ktoré chýbajú (veľa ticketov nemá
sedenie), a namiesto holej pomlčky vypisuje "General admission", keď chýbajú všetky tri. Podkladové polia
(`s.section`/`s.rowLabel`/`s.seat`) sú nedotknuté — zmenilo sa len to, ako sa zobrazujú.

Stĺpce teraz presne podľa poradia zo zadania: **Ticket, Order, Seat, Sale price, Fees, Cost, Profit,
Status, Actions** (9 stĺpcov). "Purchase cost" premenované na "Cost" a "Payment" na "Status" — kvôli
konzistencii s tabuľkou na `/sales` (rovnaká terminológia), žiadna dátová zmena, len popisok.

Rovnaká `table-layout: fixed` + `<colgroup>` technika: 8 pevných stĺpcov = **660px**, Seat stĺpec
pohlcuje zvyšok (min. 148px pri najužšom okne). Actions stĺpec (Edit/Refund/mazací kôš) má navyše
`flex-wrap`, aby sa tlačidlá v krajnom prípade radšej zalomili na 2 riadky než pretiekli mimo bunku.

**Refund/delete/anchor logika nebola zmenená ani o riadok** — `load()`, `header` `useMemo`, `RefundDialog`,
`SaleEditModal`, obe `ConfirmDialog` (delete jedného riadku aj delete celého predaja) a logika
prepočítania anchor ID po zmazaní najnižšieho riadku (`newAnchorId = Math.min(...)`) sú identické ako
pred 1.8.2. Zmenilo sa výhradne JSX okolo nich — usporiadanie kariet a tabuľky.

## 4. Nový Settings Home

Po otvorení `/settings` používateľ teraz vidí **4 karty naraz, bez scrollovania**: Lookups, Data,
Appearance, Software — presne kategórie zo zadania. Každá karta je klikateľný `<Link>` (nie tlačidlo s
`onClick`+navigate — `Link` je jednoduchšie a prístupnejšie pre čistú navigáciu) s ikonou, názvom a
krátkym popisom. Grid `sm:grid-cols-2 lg:grid-cols-4` — pri reálnej minimálnej šírke okna tejto aplikácie
(nad `lg:` breakpointom) je to vždy jeden riadok 4 kariet.

Pribudli 2 nové ikony v `src/components/icons.tsx` (`IconTag` pre Lookups, `IconSun` pre Appearance) —
presne v rovnakom `Svg`-wrapper vzore ako všetkých 21 existujúcich ikon. Data a Software používajú
existujúce `IconDatabase` / `IconDownload`, žiadny nový vizuálny jazyk.

## 5. Settings sekcie a navigácia

Kliknutím na kartu sa otvorí `/settings/lookups`, `/settings/data`, `/settings/appearance` alebo
`/settings/software` — **nová route** `settings/:section` v `App.tsx`, popri existujúcej `settings`
route (obe smerujú na tú istú komponentu `Settings.tsx`). Aplikácia používa `HashRouter`, takže táto
route je stabilná aj pri refreshi úplne bez ďalšej konfigurácie — presne ako žiadalo zadanie ("ak
existujúci routing stačí, nepotrebujem nový router").

`Settings.tsx` číta `useParams().section` a vetví sa: bez `section` (alebo s neznámym/neplatným
`section`) zobrazí Settings Home, so známym `section` zobrazí len obsah tej jednej kategórie + link
"← Back to Settings" hore. Všetok existujúci state a všetky handlery (`reload`, `addPlatform`, `doExport`,
`doBackup`, `pickRestoreFile`, `doRestore`, `doCheckForUpdate`, `doInstallUpdate`) zostávajú v **jednej**
komponente — vedomé rozhodnutie, aby sa nič nemuselo duplikovať ani presúvať medzi viacerými súbormi
(čo by bol práve ten "veľký refaktor", ktorému sa zadanie chcelo vyhnúť). Oba `ConfirmDialog` (restore,
zmazanie platformy) aj `CsvImportModal` zostávajú vždy pripojené na konci komponenty bez ohľadu na to,
ktorá sekcia sa práve zobrazuje, presne ako predtým.

Sidebar navigácia (`Layout.tsx`) nepotrebovala žiadnu zmenu: položka "Settings" už predtým nemala `end`
prop na `NavLink`, takže zostáva zvýraznená aj na `/settings/data` a podobne — rovnaké správanie, aké už
mala napr. "Sales" položka na `/sales/:id`. Overené priamym prečítaním `Layout.tsx`, nie predpokladom.

## 6. Zachovaná existujúca funkcionalita

Nič z existujúceho nebolo zmazané — len preusporiadané:

- **Sales**: search, filtre (Event/Platform/Payment/Currency/Refund status/dátumový rozsah), sorting,
  bulk výber + "Export selected" — nič z toho som sa v tomto kole ani nedotkol, zmenila sa len tabuľka
  pod filter barom.
- **Sale Detail**: refund, edit, delete (jednotlivý riadok aj celý predaj) — identická logika, len iné
  usporiadanie kariet/tabuľky.
- **Settings**: pridávanie/mazanie platforiem, CSV import s náhľadom, 5× CSV export, backup, restore,
  kontrola aktualizácií a inštalácia, prepínač témy (Light/System/Dark) — všetko funguje identicky,
  len je to teraz rozdelené do 4 podstránok namiesto jednej dlhej.

## 7. Zmenené súbory

- `src/index.css` — pridané `.th-c` / `.td-c` (nič odobraté ani zmenené na existujúcich triedach)
- `src/components/icons.tsx` — pridané `IconTag`, `IconSun`
- `src/lib/format.ts` — pridané `formatDateCompact()`, `formatSeatLocation()`
- `src/pages/Sales.tsx` — tabuľka prerobená na `table-fixed` + `colgroup`, import `formatDateCompact`
- `src/pages/SaleDetail.tsx` — SUMMARY 4→6 kariet, tabuľka prerobená, import `formatSeatLocation`
- `src/pages/Settings.tsx` — rozdelené na Settings Home + sekcie podľa `useParams().section`
- `src/App.tsx` — pridaná route `settings/:section`
- `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`,
  `release.ps1`, `1-CLICK-UPDATE.bat` — verzia 1.8.2

**Žiadny `.rs` súbor, žiadna migrácia, žiadny SQL dotaz, žiadny Tauri command sa v tomto kole nezmenil.**

## 8. Testy

V repozitári neexistuje frontendový test framework (overené: `grep` po `playwright|vitest|jest|.test.` v
celom `src/` nenašiel nič relevantné) — v súlade so zadaním som žiadny nový nezaviedol.

Rustové testy (`cargo test --lib`) existujú z predchádzajúcich kôl, ale nedali sa v tomto sandboxe
spustiť (pozri sekciu 9) — keďže **žiadny `.rs` súbor nebol tento krát zmenený**, riziko regresie na
Rust strane je z princípu nulové: nespustiteľnosť testov tu nie je dôsledok mojich zmien, ale dôsledok
sieťového obmedzenia sandboxu, ktoré existuje nezávisle od toho, čo som upravoval.

## 9. Výsledky buildu

Poctivo, presne ako v 1.8.0/1.8.1 reporte — **nepredstieram úspešný build, ktorý som reálne nespustil:**

- `cargo check --lib`, `cargo test --lib`, `cargo clippy --lib --all-targets` — všetky tri zlyhali
  identicky: `error: failed to get 'anyhow' as a dependency ... Host not in allowlist: index.crates.io`
  (HTTP 403). Toto je rovnaké, už skôr zdokumentované sieťové obmedzenie sandboxu (žiadny prístup na
  crates.io), nie nová chyba.
- `npx tsc -b` a `npm run build` — zlyhali, pretože `node_modules/` je v tomto sandboxe prázdny (0
  balíkov — overené priamo, `ls node_modules | wc -l` = 0) a `npm install` nemá prístup na
  registry.npmjs.org. `tsc` konkrétne hlási chýbajúce typy pre `vite` a `@vitejs/plugin-react` vo
  `vite.config.ts` — presne to, čo by chýbalo bez nainštalovaných `devDependencies`.

Toto sieťové obmedzenie je od začiatku tejto spolupráce potvrdené ako trvalé vlastníctvo tohto konkrétneho
sandboxu, nie niečo, čo by 1.8.2 spôsobilo. Namiesto reálneho buildu som urobil, čo bolo možné bez neho:

- Opätovné prečítanie každého zmeneného súboru celý, po edite, kontrola vyváženosti JSX tagov/zátvoriek.
- `grep` prehľadávka na potvrdenie, že nezostal žiadny odkaz na staré `min-w-[1220px]` / `min-w-[1150px]`,
  že `formatDateCompact`/`formatSeatLocation` sa používajú presne tam, kde majú, a že premenované popisky
  ("Selling fees"→"Fees" a pod.) sa netýkajú iných miest v kóde (napr. popisky vo formulároch v
  `SaleEditModal`/`SaleFormModal` som si overil, že sú to iné, nedotknuté miesta).
- Ručné prepočítanie pixelových súčtov oboch tabuliek (sekcia 2 a 3) namiesto vizuálneho odhadu.
- Overenie typovej bezpečnosti (napr. `title={g.platformName ?? undefined}` namiesto `?? null`, keďže
  React-ove typy pre `title` neakceptujú `null`) manuálnym prejdením kódu, keďže `tsc` nešiel spustiť.

**Toto je best-effort statická/manuálna verifikácia, nie náhrada za reálny build.** Odporúčam po nasadení
skontrolovať, že `npm run build` prejde na tvojom počítači (kde `npm`/`cargo` majú normálny prístup na
internet) predtým, než sa spustí `1-CLICK-UPDATE.bat`.

## 10. Vizuálna QA

V tomto sandboxe nie je funkčný build ani prehliadač s touto appkou, takže **nemôžem urobiť reálny
screenshot ani kliknúť cez appku** — poviem to rovno, namiesto aby som to predstieral. Čo som overil
namiesto toho:

- Sales aj Sale Detail: pixelová matematika (sekcie 2 a 3) ukazuje rezervu 48px, resp. 148px aj pri
  najužšom možnom okne appky — `table-layout: fixed` garantuje, že sa to nikdy nepremení na horizontálny
  scroll (CSS vlastnosť, nie odhad).
- Dlhý názov eventu: keďže Event stĺpec je jediný flexibilný a má `truncate` + `title`, dlhý názov sa
  orezáva s "…" a plný text je na hover — nemôže "rozbiť" layout, lebo `table-layout: fixed` nedovolí
  žiadnemu stĺpcu prekročiť určenú šírku.
- Refund badge: text skrátený na "N/M refunded", plný text v tooltipe — čitateľné aj v 92px stĺpci.
- Profit/margin: farebné triedy (emerald/red podľa znamienka) prevzaté 1:1 z pôvodného kódu, nezmenené.
- Settings Home: 4 karty v `lg:grid-cols-4` — pri reálnej min. šírke okna appky je `lg:` vždy aktívny,
  takže je to vždy jeden riadok, nikdy nie je treba scrollovať, aby bola vidieť ďalšia kategória.

**Odporúčanie:** po inštalácii 1.8.2 skontroluj vizuálne aspoň Sales pri predvolenej šírke okna a Settings
Home — ak niečo nesedí, napíš mi presne čo (najlepšie screenshot), opravím to cielene.

## 11. Výsledky regresie

Keďže appku nemôžem reálne spustiť, nižšie je **overenie na úrovni kódu** (čo presne som sa dotkol/nedotkol),
nie klikacie testovanie:

- Zoskupenie predajov (SaleGroup), 4 tickety = 1 riadok, Sale Detail = 4 tickety — nedotknuté, `groups`/
  `lines` state a ich zdrojové API volania (`listSaleGroups`, `listSalesByGroup`) sú identické.
- Search podľa ticket/order kódu, Event/Platform/Payment/Currency/Refund filter, sorting — nedotknuté,
  celý filter bar nad tabuľkou v `Sales.tsx` som neupravoval, len tabuľku pod ním.
- Export selected — nedotknuté, `doExportSelected`/`exportSalesCsvSelected` nezmenené.
- Refund → resell, delete sale, delete refunded sale — nedotknuté, `RefundDialog`, oba `ConfirmDialog` v
  `SaleDetail.tsx` a `api.deleteSale`/`api.deleteSaleGroup`/`api.refundSale` volania sú identické.
- Mixed currency Margin/ROI — nedotknuté, `formatPercentOrMixed`/`formatMoneyOrMixed` sa volajú s
  rovnakými argumentmi (`header.margin/roi`, `header.currency`), len teraz v samostatných kartách.
- Dashboard graf a metriky — nedotknuté, `Dashboard.tsx` som v tomto kole vôbec neotvoril.
- Settings existujúce akcie — pozri sekciu 6, všetky handlery identické.
- CSV import/export, Backup, Restore — nedotknuté, presunuté do sekcie "Data" bez zmeny logiky.

## 12. Čo NEBOLO zmenené

Presne podľa zoznamu zo zadania, potvrdzujem, že tieto zostali nedotknuté: refund/resell logika,
migrácia 004, `batch_id` / `SaleGroup` / `GROUP_BASE_SELECT`, zoskupovanie Tickets/Orders/Event,
`finance.rs`, `money.rs` (celé peniaze zostávajú integer centy), Backup/Restore bezpečnosť, transakčný
CSV import, migration runner (stále presne 4 migrácie, žiadna nová), finančná logika Dashboardu, Sales
search/filter/sorting **backend** (Rust/SQL), sémantika delete sale / delete refund record. DB verzia sa
nezmenila, žiadna nová migrácia nepribudla. Toto je potvrdené aj mechanicky: **ani jeden súbor v
`src-tauri/src/` nebol v tomto kole otvorený ani editovaný** (jediné zmeny v `src-tauri/` sú verziovacie
polia v `Cargo.toml` a `Cargo.lock`).

## 13. Návrhy do budúcna

Toto sú len nápady na zváženie nabudúce, nič z toho som teraz nerobil (mimo rozsahu 1.8.2):

- Nastaviteľné šírky stĺpcov v tabuľkách (drag-to-resize), ak by sa časom ukázalo, že pevné šírky niekomu
  nesedia pri konkrétnych dátach.
- Klávesová skratka (napr. Esc) na "Back to Settings" z ktorejkoľvek sekcie.
- Ak by Settings časom pribudli ďalšie úrovne vnorenia, breadcrumbs namiesto jednoduchého "← Back" linku.
- Skutočný CI beh (mimo tohto sandboxu, kde `cargo`/`npm` majú normálny prístup na internet) na overenie,
  že build/testy reálne prejdú predtým, než sa spustí `1-CLICK-UPDATE.bat`.

## FOUND BUT NOT TOUCHED

Počas tohto kola som nenarazil na žiadny nový bug v existujúcom kóde — toto vydanie bolo čisto
reštrukturalizácia JSX/CSS bez toho, aby som musel meniť logiku, takže som ani nemal príležitosť na niečo
podozrivé naraziť. Ak by som niečo našiel, podľa zadania by som to sem zapísal a **neopravoval** — v
tomto kole je tento zoznam prázdny.

---

**Zastavujem sa po 1.8.2**, presne podľa zadania — žiadne Payments/Invoices/Cloud/Discord/Accounts/
Webhooks/integrácie/bulk delete/nový dashboard/nový Sales backend som nepridával. Čakám na tvoju spätnú
väzbu, hlavne vizuálnu (screenshot, ak niečo nesedí) — build/test nástroje v tomto sandboxe nemajú prístup
na internet, takže reálne funkčné overenie vie spraviť len build na tvojom počítači.

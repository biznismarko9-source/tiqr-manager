# TIQR Manager 1.8.3 — Sales + Workflow + UX Improvements

Tento report nahrádza REDESIGN-1.8.2-REPORT.md. 1.8.2 (Sales/Sale Detail bez horizontálneho scrollu,
Settings Home) je hotové a stabilné — 1.8.3 na tom nič neruší, len pridáva. Toto vydanie sa sústredí na
každodenné používanie Sales/Sale Detail/Order Detail: bezpečný hromadný (bulk) edit ticketov, menšie UX
opravy naprieč Sales/Order Detail/Tickets/Orders/Events, CSV import polish, malý Quick Actions blok a
Pending Sales viditeľnosť na Dashboarde, a Potential Profit + Remaining na Event Detail. **Žiadna zmena
schémy ani migrácií** — DB zostáva na migrácii 004.

---

## 1. Audit — čo som našiel na začiatku

Pred akoukoľvek zmenou som si prešiel `tickets.rs`, `sales.rs`, `dashboard.rs`, `csv_import.rs`,
`csv_export.rs`, `Sales.tsx`, `SaleDetail.tsx`, `OrderDetail.tsx`, `Tickets.tsx`, `Orders.tsx`,
`Events.tsx`, `EventDetail.tsx`, `Dashboard.tsx`, `Settings.tsx` a `App.tsx`. Zhrnutie nálezov:

- **`update_ticket` bol jediný veľký `#[tauri::command]`** bez oddelenej `impl` funkcie — nekonzistentné
  s konvenciou, ktorú používa zvyšok backendu (`list_tickets_impl`, `create_sales_batch_impl`,
  `get_dashboard_impl`...). Opravené (sekcia 3) — rozdelené na `update_ticket_impl` (testovateľná, plain
  `&Connection`) + tenký `#[tauri::command]` wrapper, bajtovo identická logika.
- **V Sales UI existuje presne jeden Export CSV** a je to presne "Select → Export selected" (`Sales.tsx`,
  tlačidlo sa zobrazí len keď `selected.size > 0`). `SaleDetail.tsx` nemá žiadny export vôbec. Žiadny
  redundantný export nenašiel — nie je čo odstrániť (sekcia 2 zadania bola už splnená pred týmto kolom).
- **Settings Home + `settings/:section` routing** (z 1.8.2) je štrukturálne v poriadku a sidebar
  zvýraznenie "Settings" funguje aj na podstránkach (`NavLink` bez `end` prop) — overené priamym čítaním
  `Layout.tsx`, nie predpokladom.
- **Dashboard Attention** už predtým odkazovalo na reálne existujúce routes (`/orders`, `/inventory`,
  per-event `/events/:id`) — požiadavka "attention položky musia byť klikateľné" bola už splnená.
- **Tickets.tsx a Orders.tsx nemali whole-row-click navigáciu**, hoci Events.tsx ju už mala (BUG #7 z
  predošlých kôl) a riadky mali hover highlight, ktorý sľuboval klikateľnosť — malá, ale reálna
  nekonzistentnosť.
- **`Events.tsx` tabuľka mala `min-w-[900px]`** — nad skutočnou minimálnou šírkou obsahu tejto appky
  (808px pri `minWidth:1080` okne), teda reálne riziko horizontálneho scrollu. `EventDetail.tsx` (2×
  `min-w-[700px]`) a Settings CSV preview (`min-w-[600px]`) sú pod touto hranicou — nikdy nemôžu pretiecť,
  netýka sa ich rovnaký bug (pozri FOUND BUT NOT TOUCHED).
- **CSV import nemá žiadny spoľahlivý signál na detekciu duplicít** — kódy objednávok/ticketov sa vždy
  generujú nanovo, nič sa neviaže na obsah CSV. Predstierať "Duplicates" počítadlo by bolo zavádzajúce.
- **Bulk update (nový kód tohto kola, pozri sekciu 3) pôvodne robil N samostatných SQL príkazov** (jeden
  SELECT na overenie + jeden UPDATE na zápis, pre každý vybraný ticket zvlášť) namiesto jedného
  hromadného `IN (...)` príkazu — nájdené a opravené počas performance auditu (sekcia 9), skôr než sa
  dostalo do finálnej verzie.

## 2. Bulk edit ticketov — Sale Detail aj Order Detail (hlavná featura)

Nový zdieľaný backend command `bulk_update_tickets` (`tickets.rs`) mení **jedno pole na jednu hodnotu**
naprieč viacerými ticketmi naraz, v jednej atomickej transakcii:

- Povolené polia: **Section, Row, Seat, Ticket type, Listing price**. Status je **zámerne vylúčený** — v
  `models.rs` je `BulkTicketField` uzavretý enum bez `Status` variantu, takže neexistuje žiadna cesta v
  kóde, ktorá by mohla skompilovať hromadný UPDATE proti `tickets.status`. Dôvod: naivná hromadná zmena
  statusu by mohla vytvoriť `status='sold'` ticket bez aktívneho predaja (alebo naopak) — presne ten typ
  poškodenia dát, pred ktorým už chráni `update_ticket_impl` pri jednotlivom edite, a nič iné v appke by
  to vedelo odhaliť ani opraviť.
- **Atomicita**: existencia každého ID sa overí PRED akýmkoľvek zápisom (jeden `SELECT ... WHERE id IN
  (...)` dotaz); ak čo i len jeden ID neexistuje, vráti sa chyba a **nič sa nezmení** — `rusqlite`
  transakcia sa automaticky rollbackne, keď `commit()` nie je nikdy zavolaný.
- **Duplicitné ID sa deduplikujú** (napr. dvojklik) — rovnaký ticket sa upraví raz, nie dvakrát.
- Zápis je jeden `UPDATE tickets SET pole = ?1 WHERE id IN (...)` príkaz pre celú dávku (nie cyklus) —
  pozri sekciu 9 (performance).
- Refetch upravených ticketov je jeden `SELECT ... WHERE id IN (...)` dotaz, nie jeden dotaz na ticket.

**Frontend**: nový zdieľaný komponent `BulkTicketEditBar.tsx`, použitý v `SaleDetail.tsx` aj
`OrderDetail.tsx`. Zobrazí sa len keď je niečo vybrané ("Selected: N" + "Bulk edit..." + "Clear
selection"). V modale vyberieš pole a novú hodnotu — **Apply je zámerne disabled, kým je hodnota
prázdna**, aby nebolo možné omylom hromadne vymazať pole naprieč N ticketmi (jednotlivý edit toto
umožňuje, bulk edit nie — riziko jedného zlého kliku je tu N-krát väčšie).

Rozdiel medzi Sale Detail a Order Detail vo výbere ticketov je zámerný:

- **Sale Detail**: vylúčené sú len **refundnuté riadky** (rovnako ako už predtým skrývajú svoje
  Edit/Refund tlačidlá) — nie podľa statusu podkladového ticketu.
- **Order Detail**: **žiadny ticket nie je vylúčený** — presne odzrkadľuje existujúci `TicketEditModal`,
  ktorý už teraz umožňuje editovať section/row/seat/type/listing price bez ohľadu na status (zamknutý je
  len samotný Status dropdown pri predanom tickete).

Tabuľka v `SaleDetail.tsx` dostala checkbox stĺpec (select-all v hlavičke, vynecháva refundnuté), rovnako
`OrderDetail.tsx` (select-all bez výnimky). Checkbox klik nikdy nenaviguje (žiadny whole-row-click na
týchto dvoch stránkach — pozri sekciu 6, prečo zámerne).

## 3. Menšia, ale dôležitá oprava: `update_ticket`

Pri príprave bulk editu som najprv rozdelil existujúci `update_ticket` (dovtedy jeden veľký
`#[tauri::command]`) na `update_ticket_impl` (testovateľná `impl` funkcia) + tenký wrapper — presne ten
istý vzor, aký už mali `list_tickets_impl`, `create_sales_batch_impl` atď. Validácia aj SQL sú **bajtovo
identické** ako predtým, len teraz sú priamo unit-testovateľné bez plného Tauri kontextu. Toto pripravilo
pôdu pre `bulk_update_tickets_impl`, ktorá je jeho súrodenec.

## 4. Order Detail — back navigation zachováva kontext

Predtým: klik na ticket z `/tickets` alebo `/inventory` do Order Detail a späť vždy skončil na
`/orders` s vyprázdnenými filtrami — stratený kontext ("kde som to bol/a"). Riešenie **bez veľkého router
refactoru**, tou istou technikou, akú už mal `Orders.tsx` (`presetEventId`) a `Sales.tsx`
(`presetSearch`):

- `Tickets.tsx` a `Orders.tsx` teraz pri linku do Order Detail posielajú `state: { from:
  location.pathname }`.
- `OrderDetail.tsx` prečíta `location.state.from`, overí ho proti zoznamu povolených cieľov (`/tickets`,
  `/inventory`, `/orders`) a použije ho (s dynamickým popiskom "Back to tickets"/"Back to inventory"/"Back
  to orders") — inak sa správa presne ako predtým (fallback na `/orders`).
- Navyše: `Tickets.tsx`/`Inventory.tsx` (zdieľajú jeden `TicketsView`) a `Orders.tsx` si teraz **pamätajú
  vlastné filtre** medzi návštevami (modulová premenná, len v pamäti počas behu appky, nikdy sa
  neukladá na disk) — presne ten istý vzor, aký už mal `Sales.tsx` od 1.8.0. Tickets/Inventory zdieľajú
  komponent, takže filter sa pamätá zvlášť pre každú z nich (kľúčované cez `location.pathname`), aby si
  navzájom neprepisovali stav.

Rovnaký `bulk_update_tickets`/`BulkTicketEditBar` ako v Sale Detail (sekcia 2) je teraz aj v Order Detail.

## 5. Sales / Sale Detail UX — čo bolo overené

Prešiel som celý zoznam zo zadania priamym čítaním `Sales.tsx`/`SaleDetail.tsx`:

- Checkboxy nenavigujú (žiadny whole-row-click na Sales/Sale Detail/Order Detail — zámerne, pozri sekciu
  6) — **bez zmeny, už fungovalo správne**.
- Sale/Event/Order linky fungujú, refund badge, payment status, mixed currency (`formatMoneyOrMixed`/
  `formatPercentOrMixed`), filtre a sorting — **bez zmeny, overené, fungovalo správne**.
- Sale Detail poradie je teraz **SALE HEADER → SUMMARY → BULK ACTIONS → TICKETS** (bulk toolbar z
  `BulkTicketEditBar` sedí medzi summary a tabuľku ticketov — logické miesto, hneď nad tým, čo ovplyvňuje).
- `CHECKBOX_CLASS` (predtým definovaná lokálne len v `Sales.tsx`) presunutá do `ui.tsx`, aby ju zdieľali
  Sales, Sale Detail aj Order Detail z jedného miesta namiesto kopírovania.

## 6. Tabuľkový UX audit (Events/Orders/Tickets/Inventory)

- **Events.tsx**: tabuľka prerobená z `min-w-[900px]` + `overflow-x-auto` na `table-layout:fixed` +
  `<colgroup>` (10 stĺpcov, Event stĺpec flexibilný, zvyšok pevný, súčet necháva rezervu aj pri
  najužšom 808px okne) — bola to skutočná chyba (900px > 808px floor), nie len štýl. Existujúci
  whole-row-click (BUG #7 fix) a jeho `closest("a")` guard zostali nedotknuté.
- **Orders.tsx a Tickets.tsx** (zdieľané `TicketsView`, teda platí aj pre Inventory): rovnaká
  `table-fixed`+`colgroup` konverzia, plus **nová whole-row-click navigácia** (rovnaký, už overený
  `closest("a")` guard z Events.tsx) — predtým mali len konkrétne `<Link>` bunky klikateľné, hoci celý
  riadok mal hover highlight sľubujúci klikateľnosť. Toto som **zámerne nepridal na Sales/Sale
  Detail/Order Detail**, lebo tam majú riadky bulk-selection checkboxy — whole-row-click by kolidoval s
  klikaním na checkbox (priamo v rozpore s požiadavkou "checkboxy nesmú navigovať").
- **EventDetail.tsx** (2× `min-w-[700px]`) a Settings CSV preview (`min-w-[600px]`) sú pod 808px floor —
  nemôžu nikdy pretiecť, nechané tak (pozri FOUND BUT NOT TOUCHED).

## 7. CSV import/export UX

- **Import preview teraz má Valid/Error zhrnutie ako klikateľné chipy** (`ImportSummaryChip`), ktoré
  filtrujú náhľadovú tabuľku nižšie (len náhľad — samotný import vždy spracuje všetky riadky, filter na
  to nemá vplyv). Toto je presne to, čo sa dá **spoľahlivo** určiť z existujúcich dát (`CsvPreviewRow.
  errors`).
- **"Warning" a "Duplicate" úrovne som nepridal** — v dátach neexistuje nič medzi "valid" a "error"
  (žiadne pole na "warning" v `CsvPreviewRow`), a duplicity sa nedajú spoľahlivo určiť (kódy sa vždy
  generujú nanovo, nič sa neviaže na obsah CSV riadku). Namiesto predstierania warningu/duplicít, ktoré
  appka reálne nevie podporiť, je v UI priamy text vysvetľujúci prečo.
- **Nový "Download template" v Settings → Data** (`export_orders_csv_template` command) — stiahne CSV s
  hlavičkou presne podľa stĺpcov, ktoré import rozpoznáva, plus jeden vzorový riadok. Existuje len jeden
  import typ (orders + tickets spolu), takže len jeden template — nie viac, ako appka reálne podporuje.
- Import samotný (`preview_orders_csv`/`import_orders_csv`, transakčný all-or-nothing) je **nezmenený**.

## 8. Dashboard — Quick Actions + Pending Sales

- **Nový, zámerne malý a druhoradý riadok tlačidiel** hneď pod hlavičkou: New Event, New Order, New Sale,
  Import CSV, Export CSV. Každé len naviguje na existujúcu route/otvorí existujúci modal (`navigate(path,
  { state: { openCreate: true } })`) — rovnaký vzor, aký už mal `Orders.tsx` (`presetEventId`) a
  `Sales.tsx` (`presetSearch`), teraz rozšírený o všeobecný `openCreate` flag v `Events.tsx`, `Orders.tsx`
  aj `Sales.tsx`. **Žiadny nový backend command, žiadna nová stránka.**
- **Pending Sales** — nová položka v Attention sekcii, symetrická k existujúcemu "Unpaid payments"
  (ktoré je o objednávkach — peniaze dlžné dodávateľovi). Pending Sales je o predajoch so
  `payment_status='pending'` — peniaze, ktoré ešte neprišli od kupujúceho. Nová dátová cesta:
  `DashboardAlerts.pending_sales_count` / `pending_sales_amount_cents` / `pending_sales_currency`
  (rovnaký "null = mixed, nikdy nemiešať meny" vzor ako `InventoryPotential.currency`), počítané jedným
  ďalším jednoduchým SQL dotazom v `get_dashboard_impl` — **žiadny nový command, žiadny scoring systém,
  žiadne transakcie/účty/faktúry** (presne v medziach zadania — len viditeľnosť).
- Attention grid rozšírený z `lg:grid-cols-3` na `lg:grid-cols-4` (4. karta), a "Nothing needs your
  attention" teraz zohľadňuje aj Pending Sales.

## 9. Event Detail — Remaining a Potential Profit

- **Nová karta "Remaining"** = `available + listed` spolu (predtým karta "Available" ukazovala len
  `availableTickets` a `listedTickets` bolo schované v pod-riadku — podceňovalo to skutočný počet
  nepredaných ticketov, keď časť z nich bola už listnutá). Oba pôvodné počty sú stále vidieť, len v
  pod-riadku ("X available, Y listed").
- **Nová "Potential Profit" zóna** (Inventory cost / Listing value / Potential profit), rovnaký vizuálny
  aj obsahový vzor ako existujúci Dashboard blok — počítané **klientsky** z už načítaných ticketov tohto
  eventu (žiadny nový backend command), presne zrkadlí `InventoryPotential` logiku (rozsah:
  `available`+`listed` tickety, currency-mixing-safe). "Purchased"/"Sold"/"Cost"/"Revenue"/"Profit"/
  "Margin"/"ROI" boli už predtým prítomné a nezmenené.

## 10. Performance audit

- **`bulk_update_tickets_impl` refaktorovaná z N samostatných SQL príkazov na 2 hromadné** — počas
  tohto auditu som si všimol, že môj vlastný pôvodný návrh (napísaný skôr v tomto kole) robil jeden
  `SELECT` na overenie existencie + jeden `UPDATE` na zápis **pre každý vybraný ticket zvlášť** (cyklus).
  Keďže všetky vybrané tickety dostávajú presne tú istú novú hodnotu, prerobil som to na **jeden**
  `SELECT ... WHERE id IN (...)` (existencia) + **jeden** `UPDATE ... WHERE id IN (...)` (zápis), rovnakou
  technikou (`rusqlite::params_from_iter` / `Vec<Box<dyn ToSql>>`), akú už používa `dashboard.rs` pre
  svoje dynamické dotazy a akú už používal refetch krok v tomto istom súbore. Presná chybová správa
  ("Ticket #N does not exist") je zachovaná — počíta sa v Rust kóde (množinový rozdiel), nie v SQL cykle.
  Správanie je identické, len rýchlejšie a bez zbytočných SQL príkazov naviac.
- **Refetch upravených ticketov** je (a bol už predtým) jeden `WHERE id IN (...)` dotaz, nie jeden dotaz
  na ticket.
- **Nová Pending Sales query** na Dashboarde je jeden ďalší jednoduchý agregát, rovnakého tvaru ako
  existujúce (`unpaid_orders_count` a pod.) — nepridáva žiadny cyklus ani N+1 vzor.
- **Potential Profit na Event Detail** je čisto klientsky výpočet nad už načítanými dátami — nulový
  dopad na počet SQL dotazov.
- Existujúce `LIST_CAP = 5000` (Tickets/Orders zoznamy) s banner upozornením zostáva nezmenené a
  postačujúce.
- Žiadny iný nový N+1 vzor nebol týmto kolom zavedený — filter caching (Tickets/Orders), whole-row-click,
  CSV summary chipy a `openCreate` navigácia sú všetko čisto klientske zmeny bez nových SQL dotazov.

## 11. Testy

Nové Rust unit testy (v repozitári, napísané a zahrnuté — pozri sekciu 12, prečo sa nedali v tomto
sandboxe reálne spustiť):

**`tickets.rs`** (`bulk_update_tickets_impl`):
- `bulk_update_tickets_impl_changes_selected_fields_and_ignores_status` — mení Section na dostupnom aj
  predanom tickete, overuje, že status a predaj za predaným ticketom ostávajú nedotknuté.
- `bulk_update_tickets_impl_only_changes_the_selected_tickets_out_of_four` — presne zadaný test scenár:
  4 tickety → vyberiem 3 → upravia sa len tie 3, 4. ostáva nezmenený.
- `bulk_update_tickets_impl_is_all_or_nothing` — 2 platné ID + 1 neexistujúce → chyba, **žiadny** z
  platných ticketov sa nezmení.
- `bulk_update_tickets_impl_rejects_negative_listing_price` — záporná cena → chyba, nič sa nezmení.
- `bulk_update_tickets_impl_rejects_empty_selection` — prázdny výber → chyba.
- `bulk_update_tickets_impl_dedupes_ids` — rovnaké ID 3× vo výbere → aplikuje sa raz.
- `bulk_update_tickets_impl_does_not_disturb_refund_history` — bulk edit nesúvisiaceho poľa (Row) na
  tickete s refundnutým + novým aktívnym predajom nesmie narušiť refund/resell históriu.

**`dashboard.rs`** (Pending Sales):
- `pending_sales_counts_and_sums_only_pending_payment_status` — počíta a sčítava len `payment_status=
  'pending'`, ignoruje `'paid'`.
- `pending_sales_excludes_refunded_even_if_it_was_pending_before_the_refund` — refund musí vyradiť
  predaj z Pending Sales.
- `pending_sales_amount_is_not_period_filtered` — Pending Sales je "right now" fakt, nie ovplyvnený
  Dashboard obdobím (rovnako ako `unpaid_orders_count`).
- Rozšírené existujúce `empty_inventory_gives_zeroed_potential_not_an_error` o 3 nové asserty (nulové
  hodnoty na prázdnej DB).

**Manuálne overené testovacie scenáre zo zadania** (kód-úrovňová kontrola, keďže appku nemôžem reálne
spustiť — pozri sekciu 12):
- 4 tickety → vyber 3 → zmenia sa len 3: pokryté testom vyššie.
- Zlyhaný bulk update → nič sa nezmení: pokryté testom vyššie.
- Refund → resell funguje: nedotknuté, existujúci `refund_sale_impl`/`create_sale_impl` a ich pôvodné
  regresné testy (`ticket_with_a_refunded_and_a_new_active_sale_appears_exactly_once`) sú nezmenené.
- Export selected funguje: nedotknuté, `doExportSelected`/`exportSalesCsvSelected` nezmenené.
- Settings export funguje: nedotknuté, `doExport`/5× export command nezmenené.
- CSV import zostáva transactional: nedotknuté, `import_orders_csv` nezmenené (len preview UI dostalo
  filter chipy).
- Mixed currency správne: nový Pending Sales/Potential Profit kód dôsledne nasleduje existujúci "null =
  mixed, nikdy nemiešaj" vzor.
- Sale/Order/Ticket navigácia funguje: nové `state.from`/`openCreate` cesty majú vždy platný fallback
  (`/orders`, žiadny modal sa neotvorí bez explicitného flagu).

## 12. Výsledky buildu

Poctivo, presne ako v každom predchádzajúcom kole — **nepredstieram úspešný build, ktorý som reálne
nespustil:**

- `cargo check --lib` (aj `test`/`clippy`) zlyhali identicky: `error: failed to get 'anyhow' ... Host not
  in allowlist: index.crates.io` (HTTP 403) — rovnaké, dlhodobo potvrdené sieťové obmedzenie sandboxu.
- `npx tsc -b` a `npm run build` zlyhali, pretože `node_modules/` je prázdny (0 balíkov, overené) a `npm
  install` nemá prístup na `registry.npmjs.org` (HTTP 403 na `registry.npmjs.org/yallist/...`).

Namiesto reálneho buildu som urobil, čo bolo možné bez neho:
- Opätovné prečítanie každého zmeneného súboru po edite, kontrola typovej logiky ručne (napr. presné
  odvodenie typov cez `Iterator::find`/`HashSet::contains`/`Box<dyn ToSql>` pri refaktore v sekcii 10,
  keďže sa to nedalo overiť kompilátorom).
- Automatizovaná kontrola vyváženosti `{}`/`()` na každom zmenenom súbore (Rust aj TS/TSX) — všetky
  vyšli presne vyvážené.
- Krížová kontrola, že nové polia v `DashboardAlerts` (Rust, `snake_case`) majú presne zodpovedajúce
  `camelCase` náprotivky v `types.ts` (serde `rename_all = "camelCase"` kontrakt).
- Ručné overenie existujúcich, už fungujúcich vzorov (`params_from_iter`, `Vec<Box<dyn ToSql>>`,
  `Option<T>: ToSql`) priamym porovnaním s kódom, ktorý už v repozitári existuje a musí byť funkčný.

**Toto je best-effort statická/manuálna verifikácia, nie náhrada za reálny build.** Odporúčam po nasadení
spustiť `npm run build` a `cargo test --lib` na tvojom počítači (kde majú normálny prístup na internet)
predtým, než pustíš `1-CLICK-UPDATE.bat`.

## 13. Zmenené súbory

**Backend (`src-tauri/src/`):**
- `models.rs` — `BulkTicketField`, `BulkTicketUpdateInput`, 3 nové polia na `DashboardAlerts` (Pending
  Sales)
- `commands/tickets.rs` — `update_ticket` rozdelený na impl+wrapper, nový `bulk_update_tickets`
  (impl+wrapper), 7 nových testov
- `commands/dashboard.rs` — Pending Sales query + wiring, 4 nové/rozšírené testy
- `commands/csv_export.rs` — `export_orders_csv_template`
- `lib.rs` — registrácia `bulk_update_tickets`, `export_orders_csv_template`

**Frontend (`src/`):**
- `lib/types.ts` — `BulkTicketField`, `BulkTicketUpdateInput`, 3 nové polia na `DashboardAlerts`
- `lib/api.ts` — `bulkUpdateTickets`, `exportOrdersCsvTemplate`
- `components/ui.tsx` — `CHECKBOX_CLASS` (presunuté zo `Sales.tsx`)
- `components/BulkTicketEditBar.tsx` — **nový súbor**, zdieľaný bulk-edit toolbar+modal
- `pages/Sales.tsx` — import `CHECKBOX_CLASS` z `ui.tsx`, `openCreate` do `location.state` efektu
- `pages/SaleDetail.tsx` — bulk selection + `BulkTicketEditBar`, poradie SALE HEADER→SUMMARY→BULK
  ACTIONS→TICKETS
- `pages/OrderDetail.tsx` — bulk selection + `BulkTicketEditBar`, back-navigation (`state.from`),
  tabuľka na `table-fixed`
- `pages/Tickets.tsx` (zdieľané s Inventory) — filter caching, whole-row-click, `table-fixed`, `state.from`
  pri linkoch do Order Detail
- `pages/Orders.tsx` — filter caching, whole-row-click, `table-fixed`, `state.from`, `openCreate`
- `pages/Events.tsx` — `table-fixed` (bola to skutočná chyba, nie len štýl), `openCreate`
- `pages/EventDetail.tsx` — "Remaining" karta, "Potential Profit" zóna
- `pages/Dashboard.tsx` — Quick Actions riadok, Pending Sales v Attention
- `pages/Settings.tsx` — Download template tlačidlo, Valid/Error summary chipy v CSV import náhľade
- `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`,
  `release.ps1`, `1-CLICK-UPDATE.bat` — verzia 1.8.3

**Žiadna migrácia, žiadna zmena schémy.**

## 14. Čo NEBOLO zmenené

Presne podľa zoznamu zo zadania, potvrdzujem nedotknuté: refund/resell logika, `batch_id` / `SaleGroup` /
`GROUP_BASE_SELECT`, zoskupovanie Tickets/Orders/Event, `finance.rs`, `money.rs` (celé peniaze zostávajú
integer centy), Backup/Restore, transakčný CSV import (samotný import, nie preview UI), migrácie 001–004
(stále presne 4, žiadna nová), finančná logika Dashboardu (`period`/`inventory` FinanceSummary bloky —
Pending Sales je nový, oddelený blok, nič neprepočítava), Sales search/filter/sorting, Export selected,
delete sale/refund sémantika, Settings routing (štruktúra zo 1.8.2 nedotknutá, len 2 malé UI doplnky v
sekcii "Data"). Žiadny nový Payments/Invoices/Cloud/Discord/Webhooks/Accounts/Marketplace systém.

## 15. FOUND BUT NOT TOUCHED

- `EventDetail.tsx` — 2× tabuľka s `min-w-[700px]` a Settings CSV preview tabuľka s `min-w-[600px]`: obe
  pod skutočnou minimálnou šírkou obsahu appky (808px), takže nemôžu nikdy spôsobiť horizontálny scroll.
  Na rozdiel od `Events.tsx` (900px, skutočná chyba, opravené v sekcii 6) tu nejde o bug, len o
  nekonzistentný štýl s ostatnými tabuľkami — vec vkusu, nechané tak, aby sa zbytočne nerozširoval rozsah
  tohto vydania.
- Ikona pre "Software" kategóriu v Settings Home je `IconDownload` (rovnaká ikona sa používa aj
  doslovne pre "download" akcie vo vnútri Data aj Software sekcií) — mierne sémantické prekrývanie, ale
  žiadna dostupná ikona v `icons.tsx` nesedí lepšie a Settings routing/štruktúra je explicitne mimo
  rozsahu tohto vydania — nechané tak.

## 16. Návrhy do budúcna

Len nápady na zváženie, nič z toho som teraz nerobil (mimo rozsahu 1.8.3):

- Nastaviteľné šírky stĺpcov (drag-to-resize), ak by pevné šírky časom niekomu nesedeli.
- Bulk edit rozšíriť aj na Tickets/Inventory zoznam (mimo Sale/Order Detail) — vyžadovalo by vlastnú
  úvahu o UX (výber naprieč viacerými stránkami/filtrami).
- Skutočný CI beh (mimo tohto sandboxu) na overenie buildu/testov pred `1-CLICK-UPDATE.bat`.
- Ak by časom pribudol reálny "warning" signál pri CSV importe (napr. neobvykle vysoká cena, chýbajúci
  ale nepovinný stĺpec), dá sa pridať ako 3. úroveň k existujúcim Valid/Error chipom.

---

**Zastavujem sa po 1.8.3**, presne podľa zadania — žiadny Payments/Invoices/Cloud/Discord/Accounts/
Webhooks/Marketplace systém som nepridával. Build/test nástroje v tomto sandboxe nemajú prístup na
internet (pozri sekciu 12) — odporúčam spustiť `npm run build` a `cargo test --lib` na tvojom počítači
pred `1-CLICK-UPDATE.bat`. Čakám na tvoju spätnú väzbu, hlavne vizuálnu.

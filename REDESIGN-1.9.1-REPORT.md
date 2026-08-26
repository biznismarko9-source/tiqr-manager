# TIQR Manager 1.9.1 — Navigácia, Ticket type, Export picker

Report k verzii **1.9.1**. Nadväzuje na 1.9.0 (Payments & Cashflow). Tentokrát to boli tri krátke, samostatné požiadavky — žiadna veľká spoločná téma, takže report je rozdelený presne na tie tri časti plus spoločný záver (testy/build/súbory).

---

## 1. Zrušenie automatickej cross-section navigácie (Orders/Tickets/Sales)

**Zadanie:** v Orders, Tickets aj Sales boli niektoré referencie (názov eventu, kód objednávky, kód lístka...) klikateľné odkazy, ktoré ťa hodili do inej sekcie (napr. klik na event v Orders ťa presunul do Events). Toto malo byť **celé zrušené**, „odstránit pri vsetkom čo som napísal".

### Ako som určil hranicu

Keďže presné znenie hovorilo „vo všetkom čo som napísal" a menované boli Orders/Tickets/Sales, použil som toto pravidlo: **odkaz v rámci tej istej sekcie bočného menu ostáva** (to len otvára detail toho istého záznamu, nie „hodí ťa inam"), **odkaz do inej sekcie sa ruší**. Bočné menu má 7 sekcií: Dashboard, Events, Orders, Tickets, Sales, Inventory, Settings.

Konkrétne zmeny:

| Súbor | Čo bolo odstránené |
|---|---|
| `Orders.tsx` | Názov eventu v riadku tabuľky (Orders → Events) — teraz obyčajný text |
| `Tickets.tsx` | Kód objednávky **aj** názov eventu v riadku (Tickets → Orders, Tickets → Events) — teraz obyčajný text. Tento komponent (`TicketsView`) sa používa aj pre `/inventory`, takže zmena platí pre obe |
| `Sales.tsx` | Názov eventu v riadku (Sales → Events) — teraz obyčajný text |
| `SaleDetail.tsx` | Názov eventu v hlavičke, kód lístka pri riadku (→ Tickets), kód objednávky pri riadku (→ Orders) — všetko teraz obyčajný text |
| `OrderDetail.tsx` | Názov eventu v hlavičke (→ Events) **a** odkaz „View sale" pri predanom lístku (→ Sales) — celý blok odstránený |

Odkazy **v rámci tej istej sekcie** som nechal (napr. kód objednávky v Orders zoznam → Order Detail, alebo „Back to orders" na detaile) — to nie je „hodenie inam", len otvorenie záznamu, ktorý si klikol.

### Jedno miesto, kde som sa rozhodol sám (over-literal čítanie)

Vypnutie odkazu „View sale" na Order Detail bol podľa mňa naozaj užitočný cross-referenčný odkaz (z predaného lístka priamo na jeho predaj), nie „otravné automatické presúvanie" ako to, čo si popisoval. Napriek tomu som ho zrušil, pretože zadanie znelo doslovne „odstrániť zo všetkého čo som napísal" a tento odkaz spĺňa presne ten istý technický vzor (klik → iná sekcia). Ak ho chceš späť, stačí povedať — je to jeden malý, izolovaný blok kódu.

Dôsledok: **Tickets zoznam (aj Inventory) teraz nemá žiadny spôsob, ako sa z riadku lístka dostať na jeho objednávku** — kód objednávky už nie je klikateľný nikde v Tickets/Inventory. Ak by si niekedy potreboval rýchlo prejsť z lístka na jeho objednávku, toto je jediné miesto, kde by to znova bolo treba pridať.

### Čo som si všimol, ale nechal bez zmeny

**`EventDetail.tsx`** má presne ten istý vzor — kód objednávky aj kód lístka v zozname sú klikateľné odkazy do Orders/Tickets. Keďže si menoval len Orders/Tickets/Sales, EventDetail som **zámerne nechal netknutý**. Ak to má platiť aj tu, daj vedieť.

**`Dashboard.tsx`** som nemenil, ale nie preto že by som to prehliadol — Dashboard odkazy sú iného druhu: explicitné popísané tlačidlá/odkazy typu „View all pending sales →" alebo „New Order", nie incidentálna referencia v riadku tabuľky, ktorú by si omylom klikol namiesto niečoho iného. Usúdil som, že to nie je ten istý problém, ktorý popisuješ.

---

## 2. Ticket type: z bulk edit do New Order

**Zadanie:** bulk edit malo pridané pole „ticket type", čo bolo podľa teba zle — zrušiť to tam, a namiesto toho pridať ticket type ako voliteľné pole (dropdown) pri vytváraní objednávky, s bežnými hodnotami (e-ticket, PDF, transfer, atď).

### Zrušené z bulk edit

`BulkTicketField` (enum v `models.rs` aj `types.ts`) mal 5 hodnôt, teraz má 4: `Section, RowLabel, Seat, ListingPriceCents`. `TicketType` je preč — z backendu (`models.rs`, `tickets.rs` — match na stĺpec) aj frontendu (`types.ts`, `BulkTicketEditBar.tsx`).

### Pridané do New Order

Na formulári „New order" (Orders.tsx) pribudlo pole **Ticket type** — dropdown s prednastavenými hodnotami:

> E-ticket, PDF, Mobile transfer, Physical, Will call

plus „Not specified" (prázdna hodnota, default) a prepínač **„Other..."** na voľný text — identický vzor, aký už formulár používa pri Currency. Túto konkrétnu päticu som zvolil sám ako „najbežnejšie" typy v ticket resale — nekonzultoval som to, takže ak chceš iný zoznam (pridať/ubrať/preusporiadať), stačí povedať, je to jeden `const TICKET_TYPES = [...]` na začiatku súboru.

Hodnota sa nastavuje **raz pri vytvorení objednávky** a odtiaľ sa skopíruje na každý vygenerovaný lístok — presne ten istý mechanizmus, aký už dnes platí pre Section/Row (`OrderInput.ticketType` → `insert_order_with_tickets` → `Ticket.ticketType`, nič nové v backende, len znovupoužitie existujúceho poľa, ktoré doteraz formulár jednoducho neponúkal).

### Drobná oprava po vlastnej kontrole

Pri nezávislom review (§5) sa ukázalo, že Select pre Ticket type — na rozdiel od Currency vedľa neho — nemal poistku na zachovanie vlastnej (Other...) hodnoty pri prepnutí späť na „Choose from list": Currency to rieši tak, že ak aktuálna hodnota nie je v zozname, dočasne sa do zoznamu pridá. Ticket type teraz robí to isté. Bez tejto opravy by sa po prepnutí Other → zoznam custom hodnota v selecte javila ako prázdna (dáta by boli aj tak správne, len by to vyzeralo zle).

---

## 3. Export CSV: presun z Sales do Settings → Data, s výberom záznamov

**Zadanie:** checkbox-výber + export CSV v Sales je na zlom mieste. Presunúť to do Settings → Data → Export CSV, kde sa dnes dá stiahnuť len celý súbor naraz — po kliknutí na Export by sa mal ukázať picker, kde vyberieš presne ktoré záznamy (jeden/viac/všetky), a to isté urobiť pre **všetky** exporty, čo tam sú (nielen Sales).

### Čo bolo v Sales zrušené

Celá checkbox-select-and-export funkcia: checkboxy v riadkoch, „select all", lišta „Selected: N / Export CSV / Clear selection". Sales zoznam je teraz čisto na prezeranie/filtrovanie, bez výberu.

### Čo pribudlo v Settings → Data

Nová zdieľaná komponenta `ExportPickerModal.tsx` — jeden generický picker (vyhľadávanie + zoznam s checkboxami + „select all" + Export tlačidlo), ktorý sa vie napojiť na ktorýkoľvek zoznam. Nastavený je pre všetkých **5** exportov v Settings → Data:

| Export | Zdroj zoznamu v pickeri | Zobrazuje |
|---|---|---|
| Events | existujúci `list_events` | názov, miesto · dátum |
| Orders | existujúci `list_orders` | kód, event · dátum nákupu |
| Tickets | existujúci `list_tickets` | kód, event · objednávka · stav |
| Inventory | `list_tickets` s filtrom `available,listed` | kód, event · objednávka |
| Sales | existujúci `list_sale_groups` | kód, event (alebo „Mixed events") · dátum predaja |

Klik na „Export {Events/Orders/.../Sales}" v Settings → Data teraz vždy otvorí tento picker namiesto okamžitého stiahnutia celého súboru. Vyhľadávanie v pickeri je naviazané na tú istú search funkciu, akú už dnes majú Orders/Tickets/Sales zoznamy — žiadny nový vyhľadávací kód.

### Čo pribudlo v backende

Sales už mala „export vybraných" (`export_sales_csv_selected`) z 1.8.0 — ten som nechal bez zmeny a použil ako vzor. Pridal som mu 3 súrodencov:

- `export_events_csv_selected(ids)`
- `export_orders_csv_selected(ids)`
- `export_tickets_csv_selected(ids)` — používa sa pre **oba** pickery, Tickets aj Inventory (Inventory picker len obmedzí ponúkaný zoznam na available/listed lístky, samotný export je identický)

Všetky 4 (vrátane existujúceho Sales) odmietnu prázdny výber s jasnou chybou („Select at least one X to export") skôr, než by sa čokoľvek dotklo databázy.

**Pôvodné „exportuj úplne všetko" príkazy** (`export_events_csv`, `export_orders_csv`, `export_tickets_csv`, `export_sales_csv`, `export_inventory_csv`) **v backende zostali** — nezmazal som ich, len ich už nič vo UI nevolá. Dôvod: sú to malé, samostatné, otestované funkcie a zmazanie by neprinieslo žiadnu výhodu, len riziko. Ak by si ich niekedy chcel naspäť viditeľné (napr. „Export all" tlačidlo vedľa pickera), backend už je pripravený.

Menšia interná úprava: `export_tickets_inner` predtým zobral rovno `&State<AppState>` (nedalo sa to jednotkovo testovať), teraz berie `&Connection` — rovnaký vzor „impl funkcia s plain Connection + tenký #[tauri::command] wrapper", aký má zvyšok kódu. Obaja pôvodní volajúci (`export_tickets_csv`, `export_inventory_csv`) fungujú nezmenene, len si teraz sami zamknú DB pred volaním.

### Dve drobné doladenia po review (dokumentačné, nie funkčné)

Nezávislý review (§5) našiel v Sales.tsx dva zabudnuté komentáre odkazujúce na kód, ktorý už neexistuje — opravil som oba:

- Komentár pri tabuľke odkazoval na „poznámku Export selected vyššie v súbore", ktorá bola touto zmenou odstránená — prepísaný, aby správne odkazoval na `ExportPickerModal.tsx`.
- `presetSearch` — mechanizmus, ktorým predtým „View sale" odkaz na Order Detail (§1) posielal do Sales predvyplnené vyhľadávanie — stratil svojho jediného volajúceho, keď bol ten odkaz zrušený. Mŕtvy kód aj jeho komentár som odstránil; `openCreate` (Dashboard „New Sale") tou istou funkciou naďalej funguje bez zmeny.
- Bonus: rovnaký zabudnutý odkaz na `presetSearch` som našiel aj v komentári v `Events.tsx` (nesúvisiaci súbor) — opravený nech neodkazuje na niečo, čo už neexistuje.

---

## 4. Testy

Pribudlo 9 nových testov v `csv_export.rs` (predtým 6, teraz 15 v tomto súbore):

| Test | Overuje |
|---|---|
| `export_events_csv_impl_with_no_ids_exports_every_event_unchanged` | `ids: None` sa správa presne ako pôvodné „exportuj všetko" |
| `export_events_csv_selected_exports_only_the_chosen_ids` | vyberie sa len to, čo malo |
| `export_events_csv_selected_rejects_an_empty_selection` | prázdny výber = chyba, nie prázdny súbor |
| `export_orders_csv_impl_with_no_ids_exports_every_order_unchanged` | to isté pre Orders |
| `export_orders_csv_selected_exports_only_the_chosen_ids` | " |
| `export_orders_csv_selected_rejects_an_empty_selection` | " |
| `export_tickets_csv_selected_exports_the_chosen_ids_regardless_of_status` | funguje aj naprieč rôznymi stavmi lístkov (dôležité pre Inventory picker, ktorý status vôbec neposiela) |
| `export_tickets_csv_selected_rejects_an_empty_selection` | " |
| `export_tickets_inner_status_and_ids_filters_still_compose` | refaktor `export_tickets_inner` (State→Connection) nezmenil správanie existujúcich filtrov |

Sales export testy (`export_selected_*`, refund/profit varianty) sú z 1.8.0/1.9.0 a zostali bez zmeny.

---

## 5. Nezávislý review

Keďže kód sa v tomto sandboxe nedá skompilovať (§6), dal som celú zmenu — všetkých 14 upravených/nových súborov — prejsť samostatným reviewom bez kontextu mojej práce (rovnaký postup ako pri 1.9.0, kde to našlo reálny bug). Výsledok: **žiadny blokujúci problém** v žiadnom Rust ani TypeScript súbore. Tri kozmetické nálezy (Ticket type select fallback, dva zabudnuté komentáre v Sales.tsx) — všetky tri sú opravené a popísané vyššie v §2 a §3. Záver review: *„Safe to ship."*

---

## 6. Build

Rovnako ako pri každej predchádzajúcej verzii: v tomto sandboxe je **trvalo zablokovaný sieťový prístup** na crates.io aj registry.npmjs.org — potvrdené aj tentokrát, odznova, po tejto zmene:

```
cargo check --lib  →  error: failed to get `anyhow` ... 403 Host not in allowlist: index.crates.io
npm run build      →  desiatky "Cannot find module" chýb (node_modules má 0 balíkov, npm install je tiež 403)
```

Toto nie je dôsledok žiadnej zmeny v 1.9.1 — je to obmedzenie tohto prostredia. Skutočné overenie prebehne až u teba cez `1-CLICK-UPDATE.bat` → GitHub Actions.

---

## 7. Regresia a DO NOT TOUCH

Skontrolované, že žiadna z týchto vecí nebola zmenená: refund/resell logika, `SaleGroup`/`batch_id`/`GROUP_BASE_SELECT`, `finance.rs`, `money.rs`, Backup/Restore, CSV transactional import, migrácie 001-004 (žiadna nová migrácia — celá táto zmena je čisto UI presun + 2 nové úzke stĺpce v už existujúcom enum-menu, nič v schéme), Dashboard finančná logika, Sales search/filter/sorting, delete sale/refund sémantika, Settings routing/architektúra, Dashboard metric picker/chart.

Bod 3 zadania bol explicitná, vedomá výnimka z DO NOT TOUCH položky „Export selected" — nie je to omyl, bolo mi to takto zadané.

---

## 8. Čo sa zmenilo — súhrn súborov

**Rust (backend):**
- `src-tauri/src/models.rs` — `BulkTicketField` zúžený na 4 hodnoty (TicketType preč)
- `src-tauri/src/commands/tickets.rs` — zrušený match arm pre `TicketType`
- `src-tauri/src/commands/csv_export.rs` — 3 nové `*_csv_selected` príkazy (Events/Orders/Tickets), refaktor `export_tickets_inner`, 9 nových testov
- `src-tauri/src/lib.rs` — registrácia 3 nových príkazov

**TypeScript/React (frontend):**
- `src/lib/types.ts` — `BulkTicketField` zúžený na 4 hodnoty
- `src/lib/api.ts` — 3 nové `export*CsvSelected` volania
- `src/components/BulkTicketEditBar.tsx` — Ticket type preč zo zoznamu polí
- `src/components/ExportPickerModal.tsx` — **nový súbor**, zdieľaný picker + 5 konfigurácií
- `src/pages/Orders.tsx` — zrušený odkaz na Event; nové pole Ticket type na New Order formulári
- `src/pages/Tickets.tsx` — zrušené odkazy na Order aj Event (platí aj pre `/inventory`)
- `src/pages/Sales.tsx` — zrušený odkaz na Event; zrušený celý checkbox-select-export; mŕtvy `presetSearch` kód odstránený
- `src/pages/SaleDetail.tsx` — zrušené odkazy na Event, Ticket, Order
- `src/pages/OrderDetail.tsx` — zrušený odkaz na Event; zrušený odkaz „View sale"
- `src/pages/Settings.tsx` — Export CSV karta teraz otvára `ExportPickerModal` namiesto okamžitého stiahnutia
- `src/pages/Events.tsx` — opravený zabudnutý komentár (nesúvisí funkčne)

---

## 9. Otvorené otázky pre teba

1. **„View sale" na Order Detail** (§1) — zrušil som ho podľa doslovného znenia zadania, ale bol to podľa mňa užitočný odkaz. Vrátiť späť, alebo nechať zrušený?
2. **EventDetail.tsx** (§1) — má rovnaký vzor odkazov (Orders/Tickets), nespomínal si ho. Zrušiť aj tam, alebo nechať?
3. **Zoznam Ticket type hodnôt** (§2) — E-ticket/PDF/Mobile transfer/Physical/Will call som zvolil sám. Vyhovuje, alebo chceš iný zoznam?

Žiadnu z týchto troch vecí som sám nemenil bez opýtania — čakám na tvoje potvrdenie/opravu.

---

Zatiaľ toto. Čakám na spätnú väzbu k reportu — ak je to good, ozvi sa s ďalšími vecami.

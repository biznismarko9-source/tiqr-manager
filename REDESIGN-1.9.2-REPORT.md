# TIQR Manager 1.9.2 — Workflow, Dashboard, New Order a Export UX

Report k verzii **1.9.2**. Nadväzuje na 1.9.1. Toto bolo 18-bodové zadanie na kontrolovaný UX/flow cleanup — postupoval som presne podľa tvojho AUDIT → PLAN → IMPLEMENT → TEST → VERIFY, celé samostatne až po výsledok, presne ako si chcel („chcem vidieť najprv výsledok 1.9.2"). Report je rozdelený na jednotlivé časti zadania + spoločný záver (súbory/testy/build/regresia).

---

## 1. Audit pred implementáciou

Pred akoukoľvek zmenou som si prešiel aktuálny stav 1.9.1: starý `BulkTicketEditBar` v Sale Detail (zdieľaný s Order Detail), štruktúru `Dashboard.tsx` (StatCards, Revenue chart, Cashflow, Inventory & Potential Profit, Attention, 3× Recent karta, Quick Actions), formulár New Order v `Orders.tsx`, `TicketsView` komponent zdieľaný medzi Tickets a Inventory, `ExportPickerModal.tsx` (z 1.9.1) a text CSV import karty v `Settings.tsx`. Až po tomto auditu som začal plánovať jednotlivé zmeny nižšie — nikde som nezačal písať kód „naslepo".

---

## 2. Payment status — nový bulk edit v Sale Detail (bod 3–4)

**Zadanie:** Sale Detail mal doteraz ten istý všeobecný Bulk Ticket Edit bar ako Order Detail (Section/Row/Seat/Listing price). To malo byť nahradené úzkou akciou, ktorá vie zmeniť **len** `payment_status` predaja — Mark as Paid / Mark as Pending.

### Backend

Nová funkcia `bulk_update_sale_payment_status_impl(conn, sale_ids, payment_status)` v `sales.rs`, presne podľa vzoru `bulk_update_tickets_impl` (validate-all-then-write-all, jedna transakcia):

- prázdny výber → chyba, nič sa nezavolá na DB
- cieľový stav smie byť len `"pending"` alebo `"paid"` — `"refunded"` je explicitne odmietnuté s hláškou, že refund ide len cez dedikovanú akciu (`refund_sale_impl`, jednosmerná, nezávislá od tohto)
- ak dávka obsahuje čo i len jeden refundovaný predaj alebo neexistujúce id → **celá dávka sa zamietne**, nič sa nezmení (rovnaká all-or-nothing garancia ako pri tiketoch)
- duplicitné id v poli sa spracujú raz
- jeden `SELECT ... WHERE id IN (...)` na validáciu + jeden `UPDATE ... WHERE id IN (...)` na zápis, v jednej transakcii — žiadny N+1

Nový príkaz `bulk_update_sale_payment_status` je zaregistrovaný v `lib.rs`. Nový typ `BulkSalePaymentStatusInput` v `models.rs` (a `types.ts` na frontende).

### Frontend

V `SaleDetail.tsx` som `BulkTicketEditBar` úplne odstránil a nahradil novou lokálnou komponentou `SalePaymentStatusBar` — lišta sa zobrazí len keď je vybraný aspoň jeden riadok, ukazuje počet vybraných, tlačidlá **Mark as Paid** / **Mark as Pending** a **Clear selection**. Mapovanie vybraných `ticketId` → `saleId` (výber v UI je podľa lístka, API potrebuje id predaja) je priame a otestované cez review (§14).

Order Detail som sa **nedotkol** — tam ostáva pôvodný plnohodnotný Bulk Ticket Edit (Section/Row/Seat/Listing price), presne podľa zadania.

---

## 3. Zrušenie starého bulk edit v Sale Detail

`BulkTicketField` / `BulkTicketUpdateInput` / `BulkTicketEditBar` / `bulk_update_tickets_impl` som nechal **úplne nezmenené** — je to zdieľaný mechanizmus a Order Detail ho ďalej používa presne ako doteraz. Zmenilo sa len to, že Sale Detail už naň neukazuje — žiadny kód v tomto zdieľanom module nebol upravený ani zmazaný.

---

## 4. Dashboard — personalizácia (Customize panel, bod 5)

Nové tlačidlo **Customize** v hlavičke Dashboardu (vedľa prepínača obdobia — vždy viditeľné, nikdy nezávisí od toho, čo je práve zapnuté/vypnuté, inak by si sa mohol sám zamknúť mimo vlastného nastavenia). Otvára modal so zoznamom **10 checkboxov** (žiadny drag-and-drop, presne podľa zadania):

Quick actions · Overview / Activity · Revenue chart · Cashflow · Current inventory · Inventory & potential profit · Attention · Recent events · Recent orders · Recent sales

Zmena sa ukladá **okamžite** pri každom kliku (žiadne samostatné Save/Cancel) do existujúceho `app_settings` key/value mechanizmu pod kľúčom `"dashboardWidgets"` ako JSON — presne ten istý vzor ako `useTheme()` hook (load-on-mount + persist-on-change). Žiadny nový backend príkaz, žiadna migrácia. Ak niekedy pribudne 11. widget, chýbajúci kľúč v už uloženom nastavení sa berie ako `true` (merge-with-defaults), takže existujúcemu používateľovi sa novučičká sekcia automaticky neschová.

Ak si niekto vypne úplne všetko, Dashboard nezostane prázdny — zobrazí sa `EmptyState` s odkazom naspäť na Customize.

**Rozhodnutie, ktoré som urobil sám:** tvoj zoznam widgetov menoval „Overview/KPI" a „Activity" ako dve oddelené položky, ale v kóde existuje len **jedna** zodpovedajúca sekcia (riadok StatCards) — žiadna samostatná KPI sekcia inde neexistuje. Zlúčil som ich preto do jedného prepínača `overview`. Revenue chart je naopak v kóde skutočne samostatná karta, tak má svoj vlastný prepínač `revenueChart`, nezávislý od `overview`. (Pri písaní tohto reportu som si všimol, že dokumentačný komentár k `DashboardWidgets` v `types.ts` túto vec pôvodne popisoval opačne, nekonzistentne so skutočným kódom — opravil som ho, nešlo o funkčnú chybu, len o zavádzajúci komentár.)

---

## 5. Dashboard — menšie čistenie

Tri karty „Recent" (Events/Orders/Sales) mali doteraz pevný `grid-cols-3`. Teraz sa počet stĺpcov prispôsobuje tomu, koľko z tých troch kariet je práve zapnutých (1, 2 alebo 3) — implementované cez statický `RECENT_GRID_COLS` lookup objekt namiesto skladania Tailwind triedy z premennej (`` `lg:grid-cols-${n}` `` by Tailwindov build-time scanner nevidel a v produkčnom CSS by triedu vystrihol — toto je bežná Tailwind pasca, vyhol som sa jej).

Pri implementácii som narazil na `RecentCard` komponentu, ktorá používala `React.ReactNode` v type signatúre bez toho, aby súbor `React` menný priestor vôbec importoval — to by v reálnom builde spadlo na `TS2503: Cannot find namespace 'React'`. Nesúvisí to priamo so zadaním (túto komponentu som nemenil, len som okolo nej pridával `widgets.*` podmienky), ale keďže odosielam celý súbor, opravil som to na bežný vzor tohto projektu (`import { ..., type ReactNode } from "react"` + `ReactNode` bez `React.` prefixu). Viac v §14.

---

## 6. New Order — redesign formulára (bod 6–7)

Formulár „New order" (Orders.tsx) bol predtým jeden dlhý zoznam polí. Teraz je rozdelený do 4 vizuálnych skupín (nová lokálna komponenta `FormGroup`, oddelenie čiarou + voliteľný nadpis):

| Skupina | Polia |
|---|---|
| **Event** | Event, Purchase date |
| **Tickets** | Quantity, Ticket type (teraz vedľa Quantity), Section, Row, Seats |
| **Purchase** | Platform, Unit price, Currency, Unit fees, Other costs |
| *(bez nadpisu)* | Payment status, Notes |

Pod formulárom pribudol živý rozpad nákladov namiesto pôvodnej jednej sumy — „N tickets · Purchase: X · Fees: X · Other costs: X · **Total: X**" (Other costs sa zobrazí len keď je nenulový), prepočítava sa pri každej zmene množstva/ceny/poplatkov. Matematika je algebraicky rovnaká ako predtým (`qty × unit price` + `qty × unit fees` + other costs), len rozpísaná — nič sa v tom, čo sa reálne uloží do objednávky, nezmenilo.

**Rozhodnutie, ktoré som urobil sám:** Platform som zaradil do skupiny Purchase (odpovedá na „kde/ako bolo kúpené"), nie do Tickets ani Event — logicky patrí k nákupným údajom rovnako ako cena a mena.

---

## 7. Supplier zmizol zo zoznamu Orders (bod 2)

Stĺpec Supplier je preč **len** z tabuľky v zozname Orders. Supplier ostáva úplne nedotknutý v: dátovom modeli, `supplier_id` v DB (žiadna migrácia, presne podľa DO-NOT-TOUCH), CSV exporte aj v Edit Order formulári na detaile objednávky. Uvoľnené miesto v tabuľke som rozpočítal medzi zvyšné stĺpce (pevné stĺpce 648px → 556px súčet, Event dostal zvyšok — floor 160px → 252px), tabuľka je stále nad appkiným 808px minimom obsahu, takže nehrozí horizontálny scroll.

---

## 8. Inventory — ponechané krížové odkazy (bod 1)

V 1.9.1 sa zrušili všetky krížové navigačné odkazy (klik na kód objednávky/názov eventu v riadku, ktorý ťa hodí do inej sekcie) naprieč Orders/Tickets/Sales/Inventory. Tento raz si chcel explicitnú výnimku: **Inventory** (jediná zo štyroch) má tieto odkazy ponechané — kód objednávky aj názov eventu v riadku Inventory zoznamu sú znova klikateľné do Order Detail / Event Detail.

Implementované ako nový voliteľný prop `allowCrossLinks` na zdieľanej `TicketsView` komponente (predvolene `false`). `Inventory.tsx` ho zapína, `Tickets.tsx` (ostrý zoznam Tickets) ho nezapína — zdieľaný kód je teda jeden, správanie sa líši len jedným propom. Keď je `allowCrossLinks` vypnutý, výstup je byte-identický tomu, čo bolo v 1.9.1 (overené aj nezávislým reviewom, §14).

---

## 9. CSV import — skrátený popis (bod 8)

Karta „Import orders from CSV" v Settings mala jeden hustý odsek textu. Teraz sú to 3 krátke riadky: **Required format:** + zoznam 15 stĺpcov, poznámka k `seats` (čiarkami oddelený zoznam, dá sa vynechať a import prebehne bez čísel sedadiel), a „Import is all-or-nothing." Samotná logika importu (`CsvImportModal`, transakčnosť) je nedotknutá — zmenil sa len popisný text na Settings stránke.

---

## 10. Export picker — Tickets zoskupené podľa objednávky (bod 9)

Picker pre Tickets export (Settings → Data) bol doteraz plochý zoznam lístkov. Teraz je to rozbaľovací strom **Objednávka → jej lístky**, ale bez druhého groupovacieho enginu na backende — presne podľa DO-NOT-TOUCH bodu 14:

- Dáta stále idú cez existujúci plochý `list_tickets` príkaz, bezo zmeny.
- Zoskupenie podľa `orderId` sa robí **na frontende**, cez `useMemo`, čisto z už načítaného poľa.
- Vyhľadávanie je tá istá existujúca funkcia, akú Tickets zoznam už má dnes — a keďže táto funkcia hľadá aj v kóde objednávky a názve eventu, hľadanie podľa kódu objednávky prirodzene vráti **všetky** lístky tej objednávky (cela skupina „matchne"), zatiaľ čo hľadanie konkrétneho lístka vráti len jeden riadok v jednej skupine. Presne to správanie „zvýrazni buď lístok, alebo celú objednávku, podľa toho čo sedí" vyšlo samo, bez akéhokoľvek nového kódu na zvýrazňovanie.
- Skupina sa pri hľadaní automaticky rozbalí; keď sa hľadanie vymaže, stromy sa opäť zbalia.
- Checkbox na úrovni objednávky je **tri-stavový** (žiadny / niektoré / všetky lístky vybrané) — nová malá `TriStateCheckbox` komponenta, `.indeterminate` sa nastavuje imperatívne cez ref, keďže HTML to inak nevie.
- Hlavička pickera teraz v grupovanom režime ukazuje „Select all (N tickets in M orders)" / „Selected: N tickets / M orders".

Samotný export (čo sa reálne stiahne) je bezo zmeny — mení sa len to, ako picker vyzerá a ako sa v ňom vyberá.

---

## 11. Export picker — Inventory (bod 10)

Rovnaký mechanizmus ako Tickets vyššie (rovnaká `groupBy` konfigurácia, rovnaká komponenta), len so vstupným zoznamom obmedzeným na `available,listed` lístky — presne tak, ako to Inventory picker robil aj predtým v plochom režime. Events, Orders a Sales pickery ostávajú plochý zoznam bez zmeny, presne podľa zadania (bod 11 — Sales stránka samotná zostáva jednoduchá, žiadne exportové UI priamo tam).

Jedna kozmetická vec, ktorú našiel review (§14) a nechal som ju tak: údaj „N tickets / M orders" v hlavičke sa počíta len zo skupín **aktuálneho** hľadania, zatiaľ čo samotný výber lístkov prežíva zmenu hľadaného textu. Ak vyberieš lístky pri jednom hľadaní a potom zmeníš text hľadania, číslo „M orders" môže na chvíľu nesedieť s reálnym výberom — ide čisto o zobrazenie tohto jedného čísla, nie o to, čo sa reálne vyexportuje (to je vždy presné). Viac v §18.

---

## 12. Zmenené súbory

**Rust (backend):**
- `src-tauri/src/models.rs` — nový `BulkSalePaymentStatusInput`
- `src-tauri/src/commands/sales.rs` — nová `bulk_update_sale_payment_status_impl` + `#[tauri::command]` wrapper, 8 nových testov (§13)
- `src-tauri/src/lib.rs` — registrácia nového príkazu

**TypeScript/React (frontend):**
- `src/lib/types.ts` — `BulkSalePaymentStatusInput`, `DashboardWidgets`
- `src/lib/api.ts` — `bulkUpdateSalePaymentStatus`
- `src/pages/SaleDetail.tsx` — nová `SalePaymentStatusBar` namiesto `BulkTicketEditBar`
- `src/pages/Tickets.tsx` — nový `allowCrossLinks` prop na `TicketsView`
- `src/pages/Inventory.tsx` — zapína `allowCrossLinks`
- `src/pages/Orders.tsx` — Supplier preč zo zoznamu; New Order formulár prerobený na `FormGroup` skupiny + živý súhrn
- `src/pages/Dashboard.tsx` — Customize panel, `useDashboardWidgets` hook, gating všetkých sekcií, oprava `RecentCard` typu
- `src/pages/Settings.tsx` — kratší text CSV import karty
- `src/components/ExportPickerModal.tsx` — voliteľné zoskupovanie podľa objednávky, `TriStateCheckbox`

Plus štandardný balík 6 súborov pre bump verzie (§15 nižšie — `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1`, `1-CLICK-UPDATE.bat`).

---

## 13. Testy

Do `sales.rs` pribudlo **8 nových testov** (predtým end-to-end pokryté len cez `bulk_update_tickets_impl`, teraz má aj sale payment-status svoju vlastnú sadu):

| Test | Overuje |
|---|---|
| `bulk_update_sale_payment_status_only_changes_the_selected_sales_out_of_four` | zmenia sa presne vybrané predaje, ostatné tri netknuté |
| `bulk_update_sale_payment_status_rejects_a_refunded_sale_and_changes_nothing` | dávka s 1 refundovaným predajom → celá zamietnutá, nič sa nezmení |
| `bulk_update_sale_payment_status_is_all_or_nothing_with_a_missing_id` | neexistujúce id v dávke → celá zamietnutá |
| `bulk_update_sale_payment_status_rejects_refunded_as_a_target_status` | `"refunded"` ako cieľ cez tento endpoint → zamietnuté (refund len cez dedikovanú akciu) |
| `bulk_update_sale_payment_status_rejects_empty_selection` | prázdny výber → chyba |
| `bulk_update_sale_payment_status_dedupes_ids` | duplicitné id v poli sa spracujú raz |
| `bulk_update_sale_payment_status_can_move_paid_sales_back_to_pending` | zmena funguje aj opačným smerom (paid → pending) |
| `bulk_update_sale_payment_status_does_not_disturb_ticket_status` | **nový, z reviewu (§14)** — stav lístka (`sold`) prežije zmenu platobného stavu predaja úplne nedotknutý; zrkadlí analogický test pri `bulk_update_tickets_impl` |

---

## 14. Nezávislý review

Keďže kód sa v tomto sandboxe nedá skompilovať (§15), dal som — rovnako ako pri 1.9.0/1.9.1 — celú zmenu prejsť dvomi nezávislými reviewmi bez kontextu mojej práce: jeden na Rust súbory (`models.rs`, `sales.rs`, `lib.rs`), druhý na TypeScript/React súbory (všetkých 9 z §12).

- **Backend review:** *„Safe to ship as-is."* Žiadny blokujúci nález. Dve drobné štylistické poznámky (plne kvalifikovaný `std::collections::HashMap` namiesto importu; chýbajúci test na prežitie stavu lístka pri bulk zmene platobného stavu) — druhú som doplnil, je to §13 posledný test.
- **Frontend review:** *„Not safe to ship as-is"* — kvôli presne jednému nálezu: `RecentCard` v `Dashboard.tsx` používal `React.ReactNode` bez importu `React` (§5). Opravené.

Pri písaní tohto reportu som pri spätnej kontrole vlastnej práce navyše sám našiel a opravil nekonzistentný dokumentačný komentár pri `DashboardWidgets` v `types.ts` (§4) — nešlo o funkčnú chybu (TypeScript sa nestará o obsah komentárov), len o zavádzajúci popis vlastného rozhodnutia, opravený nech sedí so skutočným kódom.

Po týchto dvoch opravách (jedna funkčná, jedna dokumentačná) a jednom doplnenom teste považujem 1.9.2 za pripravené na tvoj build.

---

## 15. Build

Rovnako ako pri každej predchádzajúcej verzii je v tomto sandboxe **trvalo zablokovaný sieťový prístup** na `crates.io` aj `registry.npmjs.org` — potvrdené aj tentokrát, s jedným presnejším zistením oproti minulým reportom:

```
cargo check --offline   →  chrono sa nedá vyriešiť bez siete
cargo check (so sieťou) →  403 Host not in allowlist: index.crates.io
npm install --dry-run   →  NEČAKANE PREŠIEL (203 balíkov vyriešených cez metadata registry)
npm install (skutočný)  →  403 Forbidden — ale až na sťahovaní .tgz balíčkov, nie na metadátach
```

Inak povedané: metadátové/verzové endpointy `registry.npmjs.org` sú z tohto sandboxu dostupné, samotné sťahovanie balíkov (`.tgz`) nie je. Toto je presnejšie zistenie než doterajšie „npm je tiež blokované" — overil som, že zlyhaný pokus o `npm install` nič nepokazil: v `node_modules` nepribudol ani jeden skutočný balík (len npm-ova vlastná metadátová cache z toho pokusu), `package-lock.json` má nezmenený čas úpravy aj obsah.

Verzia je bumpnutá vo všetkých **6 miestach** (`package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` — vrátane prepísaného commit-message textu, `1-CLICK-UPDATE.bat`), skontrolované po zmene aj `grep`om. `1-CLICK-UPDATE.bat` som menil cez binárne-bezpečnú náhradu (nie textový editor), aby sa nedotkla CRLF konca riadkov — overené, súbor je stále „DOS batch file, ASCII text, with CRLF line terminators".

Skutočné overenie buildu prebehne až u teba cez `1-CLICK-UPDATE.bat` → GitHub Actions.

---

## 16. Regresia a DO NOT TOUCH

Namiesto len vizuálnej kontroly som si tento raz priamo overil na súborovom systéme, ktoré súbory boli dnes vôbec dotknuté (`find` podľa času úpravy) — výsledných **18 súborov** presne zodpovedá 12 zdrojovým súborom z §12 plus 6 verzovacím súborom z §15, nič naviac:

- **Migrácie:** `src-tauri/migrations/001..004` — stále presne 4 súbory, žiadny nový, časy úprav nezmenené (žiadna migrácia sa dnes nedotkla).
- **`finance.rs` / `money.rs`:** časy úprav nezmenené — dnes vôbec neotvorené, nieto ešte upravené.
- **`GROUP_KEY_EXPR` (riadok 34) a `GROUP_BASE_SELECT` (riadok 198) v `sales.rs`:** nová funkcia bola vložená až na riadku 798, medzi `update_sale` a `refund_sale_impl` — čisto prídavná zmena, obe konštanty aj všetko okolo nich nedotknuté.
- **`refund_sale_impl`:** nedotknutá, nový bulk endpoint ju explicitne odmieta obísť (§2).
- **Backup/Restore, transakčný CSV import (samotná logika), Settings routing/architektúra:** žiadny z týchto súborov nie je v zozname 18 dnes dotknutých súborov.
- **`supplier_id`:** nezmazané z DB, žiadna nová migrácia (§7).
- **Export picker:** žiadny druhý groupovací engine na backende — potvrdené v §10 (frontend `useMemo` nad existujúcim `list_tickets`).

Formálny 30-bodový regresný zoznam zo zadania (bod 15) som takto pokryl podľa kategórií vyššie — každá stojí na overiteľnom fakte (čas úpravy súboru / číslo riadku), nie len na tom, že som si to „pamätal".

---

## 17. Čo NEBOLO zmenené

Refund/resell logika, `batch_id`/`SaleGroup`/`GROUP_BASE_SELECT`/`GROUP_KEY_EXPR`, zoskupovanie Tickets/Orders/Event, `finance.rs`, `money.rs` (celé peniaze zostávajú integer centy), Backup/Restore, transakčný CSV import (logika, nie popisný text), migrácie 001–004, finančná logika Dashboardu, existujúce Sales filtrovanie/vyhľadávanie/triedenie na backende, delete sale/refund sémantika, Settings routing/architektúra, `supplier_id` v DB. Žiadny nový Payments/Invoices/Cloud/Discord/Webhooks/Accounts/Marketplace systém.

---

## 18. FOUND BUT NOT TOUCHED

- **`selectedGroupCount` v `ExportPickerModal.tsx`** (§11) — počíta sa len zo skupín aktuálneho hľadania, kým samotný výber prežíva zmenu textu hľadania. Zobrazované číslo „M orders" sa tak môže na chvíľu rozísť so skutočným výberom (export samotný je vždy presný). Nechané tak — oprava by vyžadovala držať zoskupenie nad *celým* doteraz videným výberom, nie len nad aktuálnym výsledkom hľadania, čo je väčší zásah než si toto vydanie žiadalo pre čisto kozmetický detail.
- **Fully-qualified `std::collections::HashMap`** v novej funkcii v `sales.rs` (namiesto `use` importu) — štylistická poznámka z reviewu, nič funkčné, nechané tak.

---

## 19. Návrhy do budúcna

Len nápady na zváženie, nič z toho som teraz nerobil:

- Poradie widgetov na Dashboarde (nielen zapnuté/vypnuté) — vyžadovalo by drag-and-drop, čo si pre toto vydanie explicitne nechcel.
- Rovnaký Mark as Paid/Pending bulk nástroj priamo v Sales zozname (nielen na Sale Detail), ak by sa ukázalo užitočné.
- Vyriešiť kozmetickú nezrovnalosť `selectedGroupCount` z §18, ak by v praxi vadila.
- Zoznam hodnôt Ticket type (E-ticket/PDF/Mobile transfer/Physical/Will call z 1.9.1) — stále otvorená otázka z minulého vydania, tento raz som sa jej nedotýkal.

---

## STOP

Podľa bodu 18 zadania týmto **končím po 1.9.2** a čakám na tvoju spätnú väzbu. Nezačínam žiadny Invoices/Cloud/Accounts/Discord/Webhooks/Marketplace systém ani ďalší veľký redesign.

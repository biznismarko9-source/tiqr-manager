# TIQR Manager 1.9.4 — Supplier cleanup, Platform filter scoping, Sales/Orders layout

Report k verzii **1.9.4**. Nadväzuje na 1.9.3 (po oprave chýbajúceho importu, ktorá sa reálne dostala do
zipu - pozri REDESIGN-1.9.3-REPORT.md, sekcia 0b). Tentokrát išlo o 5 konkrétnych vecí z tvojej spätnej
väzby k štyrom screenshotom (Inventory, Sales, Orders, Dashboard). Šiestu vec zo správy - presun Dashboard
"New Event/Order/Sale" riadku niekam inam - som **zámerne nechal netknutú**, presne ako si napísal
("zatiaľ oprav toto"); čaká na konkrétnejšie zadanie kam presne. **Žiadna zmena v `src-tauri/` - toto
vydanie sa celé odohralo vo frontende**, overené aj priamo na súborovom systéme (sekcia 8).

---

## 1. Supplier preč z Order Detail

**Čo si napísal:** keď editneš existujúcu objednávku, formulár ti ponúka aj Supplier - keďže sme ho
odstránili, nemá tam čo robiť.

Odstránené z `OrderDetail.tsx`:
- Read-only riadok "Supplier" v hornej info karte (predtým vedľa Platform/Currency).
- Celé `LookupSelect` pole "Supplier" v Edit Order formulári, vrátane jeho `suppliers` zoznamu a
  `api.listSuppliers()`/`api.createSupplier()` volaní.

**Dôležité - dáta sa nič nemenia:** `supplierId` stav v komponente **som nechal** a v `submit()` sa
naďalej posiela presne taká hodnota, akú mala objednávka pri otvorení editu (`order.supplierId`,
nezmenená). Keby som toto pole jednoducho vynechal z odosielaného objektu, `update_order` v Rust-e ho
nastaví na `NULL` bez pýtania (robí priamy `UPDATE orders SET supplier_id=?1, ...` s hocičím, čo príde -
overil som si to priamo v `orders.rs`) - to by ticho vymazalo supplier na každej objednávke, ktorú by si
odteraz upravil. Namiesto toho sa supplier jednoducho už nedá **vidieť ani zmeniť** cez túto appku, ale
existujúca hodnota prežije akýkoľvek ďalší edit nedotknutá.

**Jedna vec, ktorú som si sám overil a opravil vo vlastnom komentári:** pôvodne som si myslel a napísal do
kódu, že Edit Order bol "posledné miesto, kde sa dal supplier nastaviť" - nie je to presné. CSV import
(`csv_import.rs`, `resolve_or_create_supplier`) vie supplier podľa mena nájsť alebo založiť a nastaviť ho
na importovanej objednávke, a CSV export ho stále exportuje - obidve som si priamo prečítal v kóde, nie
len predpokladal. Opravil som si vlastný komentár, aby netvrdil niečo, čo som si neoveril.

**Layout formulára:** Platform stratil svojho párového suseda (predtým Supplier+Platform, Purchase
date+Currency), tak teraz zaberá celú šírku sám a Purchase date + Currency (čo aj tak logicky patria k
sebe) tvoria pár pod ním - namiesto toho, aby po Supplier ostala prázdna medzera v mriežke.

---

## 2. Supplier filter preč z Tickets/Inventory

**Čo si napísal (a fotka to potvrdzuje):** v Inventory je filter Supplier, ktorý tam nemá čo robiť.

`Tickets.tsx`/`TicketsView` je zdieľaná medzi stránkami Tickets a Inventory, takže odstránenie sa
prejavilo na oboch naraz - rovnaký princíp, akým v 1.9.3 zmizol Supplier **stĺpec** z tej istej tabuľky.
Tento raz šlo o **filter**, nie stĺpec: odstránil som celý `suppliers` zoznam/fetch, `supplierId` stav,
jeho perzistenciu medzi návštevami stránky (`lastTicketsFilters`) aj jeho parameter v `listOrders()`
volaní - kompletné vyčistenie, nie len skrytie UI. Backend (`list_orders`/`list_orders_impl`) filtrovanie
podľa `supplier_id` naďalej podporuje, len ho už z tejto stránky nič nevolá.

---

## 3. Platform filter v Tickets/Inventory zúžený na purchase/both

**Čo si napísal:** v tom istom filtri je aj Platform, ale tam môžu byť len platformy, z ktorých sa
kupovalo, nie tie, kde sa predávalo.

Presne tvoja vlastná formulácia dôvodu - Tickets/Inventory je o nakúpenom tovare, takže platforma, z
ktorej sa niečo len predávalo, by tu nikdy nemohla sedieť so žiadnym skutočným lístkom. Filter teraz
zobrazuje len `kind === "purchase" || kind === "both"` - presne ten istý vzor, aký už majú Platform
pickery na New/Edit Order (od 1.9.3). Sales zoznam svoj Platform filter **nemá** zúžený - to je vedomá,
už z 1.9.3 zdokumentovaná výnimka (vyhľadáva naprieč všetkými predajmi bez ohľadu na to, aké platformy
dnes existujú v zoznamoch), a keďže sa teraz mohlo zdať nekonzistentné, prečo je to inak, doplnil som ku
Sales.tsx-u komentár, ktorý na túto novú Tickets/Inventory logiku priamo odkazuje - a naopak - aby bolo
jasné, že ide o zámer, nie prehliadnutie.

---

## 4. Sales — From/To zoskupené

**Čo si napísal:** search/event/from/to/more filters treba prerobiť, aby to bolo krajšie a dávalo zmysel -
From a To by mali byť vedľa seba, nie ako teraz.

Root cause: celý riadok filtrov je `flex flex-wrap` - keď sa nezmestí všetko na jeden riadok, prebytok sa
zalomí. From a To boli dva **samostatné** položky v tomto riadku, takže sa mohli (a na tvojom screenshote
aj reálne) zalomiť oddelene - From ostalo na prvom riadku, To skočilo na druhý, vedľa "More filters".

Oprava: From a To sú teraz zabalené do jedného vnoreného `flex` kontajnera (tesnejší `gap-2` namiesto
vonkajšieho `gap-3`, nech vizuálne pôsobia ako pár). Keďže je to teraz **jedna** položka vo vonkajšom
zalamovanom riadku, zalamuje sa ako celok - buď obidva ostanú na riadku s ostatnými filtrami, alebo
obidva spolu skočia nižšie. Nikdy sa už nerozdelia. Zvyšok riadku (Search/Event/Platform/Payment/
Currency/More filters) som nechal netknutý - konkrétna požiadavka bola o From/To, nie o celkovom počte
alebo poradí filtrov.

---

## 5. Orders — širší Platform stĺpec

**Čo si napísal:** pri dlhšom názve platformy (napr. "Fnac Spectacles") sa to orezáva, hoci je v tabuľke
dosť miesta na presunutie.

Pôvodné pevné stĺpce (Order 92 + Date 84 + Platform 92 + Qty 48 + Sold 64 + Total cost 88 + Payment 88)
dávali súčet 556px, Event (jediný neurčený `<col>`) dostával zvyšok nad 808px podlahou tejto appky - teda
aspoň 252px. Platform som rozšíril **92px → 160px** (+68px, dosť na ~20 znakov pri tomto fonte/paddingu -
pokryje aj "Fnac Spectacles"). Nové pevné stĺpce dávajú 624px, Event má teraz floor 184px namiesto 252px -
menej ako predtým, ale stále pohodlne dosť pre bežné názvy eventov, a pre tú zriedkavú výnimku má bunka
Event aj naďalej `title` tooltip s celým názvom (nezmenené).

---

## 6. Zmenené súbory

**Frontend (`src/`) - jediné funkčné zmeny tohto vydania:**
- `pages/OrderDetail.tsx` - Supplier preč (zobrazenie aj edit), formulár preskupený
- `pages/Tickets.tsx` (zdieľané s Inventory) - Supplier filter preč, Platform filter zúžený na
  purchase/both, opravený zastaraný komentár nad tabuľkou z 1.9.3
- `pages/Sales.tsx` - From/To zoskupené, doplnený komentár k zámerne nezúženému Platform filtru
- `pages/Orders.tsx` - Platform stĺpec 92px → 160px

**Verzia (6 súborov, ako vždy):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`,
`src-tauri/Cargo.lock` (len vlastný `tiqr-manager` balík - v `Cargo.lock` je zhodou okolností aj
nesúvisiaci balík `indexmap` vo verzii `1.9.3`, ten som sa uistil, že som **nezmenil**), `release.ps1`
(verzia + prepísaný commit-message text, aby opisoval toto kolo, nie 1.9.3), `1-CLICK-UPDATE.bat` (CRLF
overené `file` príkazom aj po úprave).

**`src-tauri/` - nezmenené ani raz.** Overené priamo (sekcia 8), nie len tvrdené.

---

## 7. Testy

Žiadne nové Rust testy - keďže sa nezmenil ani jeden `.rs` súbor, existujúca sada (166 `#[test]` funkcií,
3 ignored) je týmto vydaním úplne nedotknutá, nie len "pravdepodobne v poriadku". Appka nemá frontend
test framework (nezmenené, existujúce obmedzenie).

---

## 8. Overenie

Bez nezávislého reviewera tento raz (v tomto rozhraní nemám k dispozícii samostatného agenta na review
bez kontextu, ako pri 1.9.0-1.9.3) - namiesto toho som spravil, čo sa dalo, sám a metodicky, presne v
duchu toho, čo tie reviewy predtým odhaľovali:

- **Skutočný TypeScript syntaktický parser** (`ts.createSourceFile`, rovnaký balík ako v 1.9.3) nad
  všetkými 5 dotknutými/súvisiacimi súbormi (4 upravené + `Inventory.tsx`, ktorý zdedí zmenu cez zdieľaný
  komponent) - **0 syntaktických chýb**.
- Kontrola párovania `{}`/`()` na všetkých 4 upravených súboroch aj na `release.ps1` - všade vyvážené.
- `grep` sweep po každej zmene, že nezostala žiadna osirotená referencia na zmazaný `suppliers`/
  `setSuppliers` stav.
- `find -newermt` na celý zdrojový strom **pred** aj **po** balení zipu, aby zoznam zmenených súborov v
  tomto reporte (sekcia 6) bol overiteľný faktom, nie spomienkou.
- Priama kontrola `orders.rs`, aby som si bol istý, že vynechanie `supplierId` z odosielaného objektu by
  ho tichoNULLovalo - toto ma priamo viedlo k rozhodnutiu poslať pôvodnú hodnotu nezmenenú (sekcia 1).
- Priama kontrola `csv_import.rs`/`csv_export.rs`, ktorá odhalila, že môj vlastný prvý draft komentára
  ("Edit Order bolo posledné miesto...") bol nepresný - opravil som ho skôr, než sa dostal do finálneho
  kódu (sekcia 1).

---

## 9. Build

Rovnaké, dlhodobo potvrdené obmedzenie sandboxu (žiadny sieťový prístup na `crates.io`/balíky z
`registry.npmjs.org`) - tento raz to ale **nie je vôbec relevantné pre samotný kód**, keďže sa nezmenil
ani jeden `.rs` súbor a `cargo test`/`cargo check` by teda aj tak nemali čo nanovo overiť. TypeScript
build (`tsc -b`/`npm run build`) sa stále nedá reálne spustiť (`node_modules` prázdny) - sekcia 8 vyššie
je najbližšie k tomu, čo sa dá overiť bez neho. Skutočné overenie prebehne u teba cez
`1-CLICK-UPDATE.bat` → GitHub Actions.

---

## 10. Regresia a DO NOT TOUCH

`refund/resell`, `SaleGroup`/`batch_id`/`GROUP_BASE_SELECT`/`GROUP_KEY_EXPR`, `finance.rs`, `money.rs`,
Backup/Restore, transakčný CSV import (logika), migrácie 001-004, `bulk_update_ticket_status_impl`/
`bulk_update_sale_payment_status_impl`, Dashboard finančná logika a taby, `supplier_id` v DB - nič z toho
sa dnes ani len neotvorilo (potvrdené `find -newermt` v sekcii 8, nie len spomienkou).

---

## 11. Čo NEBOLO zmenené

Dashboard (taby, Cashflow, Attention, Quick Actions blok - viď sekcia 12 nižšie prečo), Sale Detail,
Event Detail, Settings, CSV import/export logika, Tickets/Inventory tabuľka samotná (len jej filter
riadok), Sales tabuľka/sorting/search, Supplier v Settings → Lookups (naďalej sa tam dá spravovať) ani
supplier_id kdekoľvek v databáze alebo CSV.

---

## 12. FOUND BUT NOT TOUCHED

- **Dashboard "New Event/New Order/New Sale/Import/Export" riadok** - spomenul si, že by si ho niekam
  presunul (pod niečo, alebo inde), ale bez konkrétneho miesta a s výslovným "zatiaľ oprav toto" pri
  zvyšku správy. Nechané presne tak, ako je - čaká na tvoje konkrétnejšie zadanie kam presne.

---

## 13. Návrhy do budúcna

Len nápady, nič z toho som teraz nerobil:

- Ak by niekedy vadilo, že Sales Platform filter zostáva nezúžený (sekcia 3), dá sa to premyslieť znova -
  zatiaľ je to vedomá, zdokumentovaná výnimka.
- Rovnaké zoskupenie ako pri Sales From/To (sekcia 4) by sa dalo aplikovať aj na Tickets/Inventory From/To,
  ak by sa tam niekedy ukázal ten istý problém so zalamovaním - teraz to nebolo súčasťou zadania.

---

## STOP

Toto boli všetky body z tvojej správy okrem Dashboard riadku (sekcia 12, zámerne nedotknuté). Čakám na
tvoju spätnú väzbu, hlavne vizuálnu, a na konkrétnejšie zadanie k Dashboardu, ak k nemu chceš pokračovať.

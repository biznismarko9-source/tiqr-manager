# TIQR Manager 1.9.6 — Order Detail reframed for Tickets/Inventory, Dashboard Quick Actions moved

Report k verzii **1.9.6**. Nadväzuje na 1.9.5, konkrétne na tvoje odpovede k dvom otázkam z toho reportu.
**Žiadna zmena v `src-tauri/` - opäť len frontend**, overené na súborovom systéme (sekcia 4).

---

## 1. Tickets/Inventory → Order Detail: teraz "Ticket detail"/"Inventory detail"

**Kde som sa v 1.9.5 pomýlil:** myslel som si, že stačí, aby Order kód spoľahlivo fungoval ako odkaz.
Tvoja odpoveď to opravila - vysvetlil si, že Event/Order/Sale na svojich vlastných stránkach dávajú "viac
info o danom objekte", nie pocit, že ťa to niekam hodilo inde. Problém teda nebol v tom, či Order odkaz
funguje, ale v tom, že stránka, na ktorú vedie, **stále vyzerá a pôsobí ako "Order Detail"** - aj keď si
prišiel z Tickets alebo Inventory.

**Prečo som nespravil úplne samostatnú Tickets Detail / Inventory Detail stránku:** Tickets aj Inventory
sú zoskupené podľa objednávky - jeden riadok = jedna objednávka. Dáta, ktoré by taká samostatná stránka
zobrazovala (zoznam lístkov tej objednávky), sú **presne tie isté dáta**, čo dnes zobrazuje Order Detail -
nie je čo duplikovať, žiadna iná informácia neexistuje. Namiesto vytvárania druhej, takmer identickej
stránky (viac kódu na údržbu, riziko, že sa časom rozídu) som pretvoril **rovnakú** stránku tak, aby sa
podľa toho, odkiaľ prišla, aj tak cítila:

- Appka si už od 1.8.3 pamätá, odkiaľ si prišiel (`location.state.from`, používané na "Back to
  tickets/inventory/orders" odkaz). Táto informácia teraz navyše ovplyvňuje aj **nadpis stránky**.
- Nový malý popisok nad kódom objednávky: **"Ticket detail"** (z Tickets), **"Inventory detail"** (z
  Inventory), alebo **"Order detail"** (z Orders, alebo priamy odkaz/refresh - bez zmeny oproti doteraz).
- Kód objednávky (`ORD-0000...`) ostáva ako hlavný nadpis vo všetkých troch prípadoch - je to jediný
  naozaj jedinečný identifikátor toho, čo je na stránke, a stále sa hodí.
- Edit/Delete tlačidlá aj zvyšok stránky (platba, lístky, bulk akcie) sa nemenia - stále ide o správu tej
  istej objednávky, bez ohľadu na to, odkiaľ si prišiel.

Mimochodom, pri tejto úprave som si všimol a opravil aj zastaraný komentár v `OrderDetail.tsx`, ktorý
tvrdil, že "Orders je teraz jediná stránka, odkiaľ sa sem dá prísť" - to už neplatí od 1.9.2/1.9.3 (a už
vôbec nie od 1.9.5, keď sa Order odkaz stal vždy funkčným).

---

## 2. Dashboard — Quick Actions presunuté nižšie

**Čo si zvolil:** nižšie na Dashboarde.

Riadok New Event/New Order/New Sale/Import CSV/Export CSV bol doteraz úplne prvá vec pod hlavičkou
Overview tabu, nad prepínačom obdobia. Presunul som ho **na koniec** Overview tabu - za graf Revenue/
Profit/Sales, teda naozaj nižšie, nie len o jeden riadok. Samotné tlačidlá a ich správanie sú nezmenené
(rovnaké `navigate(path, { state })` volania ako doteraz).

**Jeden reálny vedľajší efekt, ktorý chcem priznať otvorene:** riadok bol predtým mimo časti, čo čaká na
načítanie dát (`loading || !data`) - teda sa zobrazil okamžite. Keďže je teraz na konci stránky, za grafom,
je aj on vnútri tejto načítavacej vetvy - čiže sa krátko nezobrazí počas úvodného načítania. Pri lokálnej
SQLite appke to je zlomok sekundy, nie reálna medzera, ale je to skutočná zmena správania, nie len
kozmetika, tak to radšej hovorím rovno.

---

## 3. Zmenené súbory

**Frontend (`src/`) - jediné zmeny tohto vydania:**
- `pages/OrderDetail.tsx` - nový kontextový popisok "Ticket detail"/"Inventory detail"/"Order detail",
  opravený zastaraný komentár o tom, odkiaľ sa sem dá prísť
- `pages/Tickets.tsx` - doplnený komentár vysvetľujúci, že 1.9.5 bola len čiastočná odpoveď, dokončená
  týmto vydaním (odkazuje na `OrderDetail.tsx`)
- `pages/Dashboard.tsx` - Quick Actions riadok presunutý z vrchu Overview tabu na koniec (za graf)

**Verzia (6 súborov, ako vždy):** rovnaký postup, `Cargo.lock` opäť len vlastný `tiqr-manager` balík,
`release.ps1` commit-message prepísaný na toto kolo, `1-CLICK-UPDATE.bat` CRLF overené po úprave.

**`src-tauri/` - nezmenené ani raz** (potvrdené `find -newermt`, sekcia 4).

---

## 4. Testy, build, regresia

Žiadny `.rs` súbor sa nedotkol - existujúca sada (166 testov, 3 ignored) nedotknutá. TypeScript build sa
stále nedá reálne spustiť (`node_modules` prázdny) - `ts.createSourceFile` nad všetkými 3 dotknutými
súbormi ukázal **0 syntaktických chýb**, párovanie `{}`/`()` vyšlo vyvážené. Presun v `Dashboard.tsx` bol
štrukturálne najnáročnejší zo všetkých úprav v tomto kole (presúval JSX naprieč hranicami fragmentov/
podmienok), tak som si po ňom stránku ešte raz celú vizuálne prečítal priamo v zdrojáku, nie len spoľahol
na to, že parser nehlási chybu. `find -newermt` potvrdzuje presne tie 3 súbory zo sekcie 3 - nič v
`src-tauri/`, `finance.rs`, `money.rs`, migráciách ani `refund_sale_impl` sa dnes ani len neotvorilo.

---

## 5. Čo NEBOLO zmenené

Samotná URL/routa Order Detailu (stále `/orders/:id`, žiadna nová routa pre "Ticket Detail"), Edit/Delete
akcie a zvyšok obsahu Order Detailu, `TicketStatusBar`, Financials/Activity Dashboard taby, Quick Actions
tlačidlá samotné (len ich pozícia), Event odkaz na Tickets/Inventory (stále pod `allowCrossLinks`).

---

## STOP

Toto boli obe veci z tvojich odpovedí. Čakám na spätnú väzbu, hlavne či "Ticket detail"/"Inventory detail"
popisok naozaj rieši ten pocit "hodilo ma to inde", ktorý si opísal.

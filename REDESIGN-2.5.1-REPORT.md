# TIQR Manager - Report k verzii 2.5.1

Priamy nadväzujúci krok na 2.5.0 - tvoja spätná väzba po tom vydaní, tri
konkrétne body. Žiadna zmena v databáze/migráciách, žiadny nový Rust kód -
celé je to frontend/UX.

---

## 1. Sidebar - poradie + Ticket Center mimo Finance

Poradie v sidebar je teraz presne podľa teba: **Dashboard, Tickets, Price
Checker, Pulls, Finance, Ticket Center, Calendar.** Predtým bol Calendar
hneď pod Dashboard - teraz je posledný. Ticket Center bol len jednu verziu
(2.4.4) tab pod Finance - je späť ako samostatná položka v sidebar, hneď za
Finance, s vlastnou routou `/ticket-center`.

## 2. Ticket Center - prerobený na objednávky

Toto bola najväčšia zmena. Staré `TicketControlCenter.tsx` (2.4.3) a
`FulfillmentCenter.tsx` (2.2.12) ukazovali riadok za **lístok** (Control
Center) alebo za **predajovú dávku** (Fulfillment) - presne to, čo si
povedal, že nechceš. Obe stránky aj tab-shell (`finance/TicketCenter.tsx`)
sú zmazané, nová `src/pages/TicketCenter.tsx` je postavená úplne odznova.

**Ako to funguje teraz:**

- Zoznam ukazuje **objednávky**, nie lístky - jeden riadok = jedna
  objednávka (Event, kód objednávky, sklad Available/Listed/Sold, platba od
  kupujúceho, doručenie, celkový stav "Completed").
- 4 rýchle filtre nad zoznamom: **Potrebuje pozornosť** (všetko
  nedokončené), **Potrebuje listing** (má lístky, ktoré ešte nie sú
  vystavené na predaj), **Potrebuje platbu** (predané lístky, kde kupujúci
  ešte nezaplatil), **Potrebuje doručenie** (predané lístky, ktoré sa ešte
  nedoručili).
- Klik na riadok otvorí **existujúcu Order Detail stránku** (`/orders/:id`)
  - tá už dnes ukazuje každý lístok v objednávke so stavom/doručením/
    platbou upraviteľnými priamo v riadku. Presne to je "vidieť, čo treba
    urobiť s ktorými lístkami" - nestaval som žiadnu novú detailovú
    obrazovku, len som na ňu Ticket Center napojil ako štvrtý vstupný bod
    (vedľa Orders/Tickets/Inventory, ktoré tam už viedli).
- Nič z toho nie je nový backend kód - zoznam objednávok aj číselné
  hodnoty (Available/Listed/Sold/Paid/Delivered) sú presne tie isté dáta,
  aké už dnes číta stránka Orders (`api.listOrders`); aj odznak "Completed"
  je znovupoužitý priamo z `Orders.tsx` (`orderCompletionChecks`), takže
  Ticket Center nemôže nikdy ukázať iný výsledok ako Orders/Order Detail o
  tom, čo je "hotové".

**Čo som vedome vynechal** (a prečo): filtre na tier/section/row/
marketplace zo starého Control Center - na úrovni objednávky (jedna
objednávka môže mať rôzne sektory/rady) by nedávali zmysel; stĺpec s
nákupným (purchase) payment statusom objednávky - to je vidno na
Orders/Order Detail, tu som nechal len predajnú stránku veci (sklad/platba
od kupujúceho/doručenie), aby zoznam zostal prehľadný a nie znova preplnený
12 filtrami ako predtým.

Backend príkaz starého Control Center (`ticket_control_center.rs`,
`list_control_center_tickets`) som nezmazal, len zdokumentoval ako nateraz
nepoužívaný (frontend naň už nikde neukazuje) - rovnaký prístup, aký má
tento projekt aj pri iných "mŕtvych" veciach (napr. tabuľka `payments`).
Dá sa to kedykoľvek naozaj odstrániť ako samostatná úloha, ak budeš chcieť.

## 3. Calendar - modernejší, prehľadnejší vzhľad

Čisto vizuálna úprava, žiadna zmena dát, requestov ani navigácie:

- Každý typ záznamu (Event/Order/Sale/Pull/Attention) má teraz svoju
  vlastnú farbu - rovnakú vo filtroch (ktoré teraz slúžia aj ako legenda
  farieb), v mriežke Month/Week, v Day Detail okne aj v súhrne "Today &
  next 7 days".
- Naliehavosť (critical/attention) sa predtým ukazovala iba jednou sivou/
  červenou/oranžovou bodkou. Teraz je to samostatný druhý signál navyše -
  červené/oranžové orámovanie na položke v mriežke, tučné/farebné písmo v
  zoznamoch - takže vieš na prvý pohľad rozlíšiť aj "čo to je" (farba) aj
  "ako naliehavé to je" (orámovanie/písmo) naraz.
- Víkendové stĺpce a dnešný deň majú jemné podfarbenie celej bunky (nie len
  krúžok okolo čísla dňa, ako predtým).
- Day Detail okno teraz v nadpise ukazuje aj deň v týždni (napr.
  "Streda, 17. aug 2026").

## Rozhodnutia, ktoré som urobil sám (na zváženie)

- Presný obsah a hranice 4 filtrov v Ticket Center (napr. "Potrebuje
  platbu" ráta len predané lístky, nie celú objednávku) - zdôvodnenie je
  priamo v komentári na vrchu `TicketCenter.tsx`.
- Nezabalil som k tomu žiadny build/zip hneď po predošlej správe, keďže si
  napísal, že budeme pokračovať - teraz, na tvoje "zabal to", je zabalené.
- `ticket_control_center.rs` som nechal v kóde (nepoužívaný, zdokumentovaný)
  namiesto úplného zmazania - pozri vyššie.

## Overenie

`npx tsc -b` aj `npm run build` čisté. `cargo check --lib` prešiel čisto
(žiadny Rust súbor sa v tomto kole nemenil, len verzia v `Cargo.toml`) -
jediné hlásenie je to isté, už predtým existujúce upozornenie na nepoužitú
funkciu v `sales.rs`, ktoré s týmto kolom nesúvisí. Klikacie prechody
Ticket Center -> Order Detail -> "Späť" (späť na Ticket Center, nie na
Orders) som overil priamo v kóde (`OrderDetail.tsx`'s `backTo`).

## Balík

`tiqr-manager-2.5.1.zip` - 360 súborov, verzia zjednotená vo všetkých 5
miestach (`package.json`, `tauri.conf.json`, `Cargo.toml`, `release.ps1`,
`1-CLICK-UPDATE.bat`) + oboch lockfile-och (`Cargo.lock`,
`package-lock.json`), integrita zip súboru overená. Staré, už doručené
zipy (2.4.2, 2.4.3, 2.5.0) som zo staging priečinka vymazal, aby sa
šetrilo miesto na disku (stále dosť napnuté - cca 2.2 GB voľných).

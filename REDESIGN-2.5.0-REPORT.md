# TIQR Manager - Report k verzii 2.5.0

Táto dodávka spája dve samostatné zadania, ktoré si poslal v jednej správe:
**Časť A** (UI/UX úpravy, pôvodne označené ako 2.4.4) a **Časť B** (TIQR
Operations Calendar, 2.5.0). Keďže si výslovne povedal, že mám po Časti A
pokračovať rovno do Časti B bez ďalšieho pýtania sa, a keďže Časť A sama o
sebe nikdy nešla cez `release.ps1`/neinštalovala sa samostatne, obe časti
dodávam spolu ako **jednu verziu 2.5.0** - samostatný build/zip pre 2.4.4
sa nerobil. Ak by si radšej chcel mať 2.4.4 ako samostatný krok v histórii,
daj vedieť a viem to pri ďalšej úprave rozdeliť.

---

## Časť A - UI/UX úpravy (zhrnutie)

1. **Control Center - Orders klikateľné.** Bunka "Order" má teraz vlastný
   klik nezávislý od zvyšku riadku (rovnaké správanie ako pri New Sale) -
   otvorí `/orders/:id` aj keď riadok ako celok vedie inam (napr. na Sale
   Detail pri predanom lístku).
2. **Control Center + Fulfillment Center zlúčené.** Obe stránky teraz žijú
   pod Finance ako jeden tab "Ticket Center" s dvomi podtabmi (Control
   Center, Fulfillment) - nový `src/pages/finance/TicketCenter.tsx`. Obe
   komponenty sú znovupoužité bez zmeny vnútornej logiky (okrem opráv nižšie).
3. **Umiestnenie - Finance.** Keďže si dal na výber Finance alebo Dashboard,
   spýtal som sa a potvrdil si Finance - `/control-center` a `/fulfillment`
   ako samostatné routy aj položky v sidebar sú preč.
4. **Sticky header oprava.** Príčina "zavadzania" textu pri scrollovaní bola
   priehľadné pozadie hlavičky (`dark:bg-slate-800/60`, 60% priehľadnosť) -
   skopírované z bežného (nie sticky) `<thead>` v tejto appke, kde to nevadí.
   Teraz plne nepriehľadné pozadie.
5. **"Ticket / Seats" -> "Seats".** Stĺpec teraz zobrazuje iba miesto na
   sedenie; kód lístka je stále dostupný ako tooltip pri prejdení myšou.
6. *(Alternatíva "Dashboard" sa nepoužila - pozri bod 3.)*
7. **Dashboard - "mini scroll" preč.** Karta "Sales by platform" použila
   vlastný vnútorný scrollbar - nahradené rovnakým vzorom "zobraz prvých N +
   Show more", aký už majú 3 karty na Activity tabe.
8. **Sidebar prestrukturovaný.** Dashboard, potom "Tickets" (rozbaľovacia
   skupina: Events/Orders/Tickets/Sales/Inventory), potom Pulls, Price
   Checker, Finance. Toto som prečítal ako harmonika nad existujúcimi 5
   routami (nie ako zlúčenie 5 stránok do jednej) - keby si mal na mysli to
   druhé, daj vedieť, je to výrazne väčšia úloha.
9. **Appearance presunuté.** Prepínač svetlý/tmavý režim je teraz jedným
   klikom nad profilovým widgetom v sidebar; zo Settings zmizol (presunuté,
   nie duplikované) - používa ten istý `useTheme()` hook ako predtým.

Overené: `npx tsc -b` a `npm run build` čisté, žiadny Rust kód sa v tejto
časti nemenil.

---

## Časť B - TIQR Operations Calendar (2.5.0)

Pred písaním kódu som najprv preskúmal `CURRENT_STATE.md`/
`PROTECTED_AREAS.md` a existujúci kód Events/Orders/Sales/Fulfillment/
Finance/Pulls/Attention Center, aby som zistil, ktoré z tvojich 8
navrhnutých kategórií majú naozaj existujúci, spoľahlivý dátum - presne, ako
si žiadal ("Ak nejaký dátum v databáze neexistuje, NEVYMÝŠĽAJ ho").

### 1. Aké typy udalostí sú skutočne podporené

Presne **5**: **Events**, **Orders/Purchases**, **Sales**, **Pulls**,
**Attention**. Ostatné 3 z tvojho pôvodného zoznamu (**Payouts**,
**Payments**, **Fulfillment**) v kalendári nie sú a nič sa pre ne
nevymýšľalo - dôvod pri každej nižšie a v bode 11.

### 2. Aké existujúce dáta sú za každým typom

- **Events** - `events.event_date` (rovnaký stĺpec, aký používa Events aj
  Event Workspace).
- **Orders/Purchases** - `orders.purchase_date`.
- **Sales** - `sales.sale_date`, zoskupené presne tak, ako to už robí
  hlavný zoznam Sales aj Dashboard "Recent sales" (`sales::GROUP_KEY_EXPR`
  - jeden riadok na predajovú akciu/dávku, nikdy jeden na lístok).
- **Pulls** - `pulls.event_date`. **Nie** starý stĺpec
  `pulls.transfer_deadline` - ten je od verzie 1.9.8 mŕtvy (nič ho už
  nezapisuje, nahradilo ho klient-side upozornenie "N dní pred eventom"
  počítané z `event_date` priamo v `Pulls.tsx`). Presne tento druh pasce si
  spomínal - overil som si to cez doc-comment v `models.rs`, nie len podľa
  názvu stĺpca.
- **Attention** - existujúci `AttentionCenterItem.eventDate` (Dashboardova
  globálna Attention Center), znovupoužitý priamo, nie prepočítaný nanovo.

### 3. Ako funguje Month/Week view

Month zobrazuje vždy celé týždne (pondelok-nedeľa), doplnené dňami z
predchádzajúceho/nasledujúceho mesiaca, aby mriežka nemala prázdne bunky.
Week zobrazuje presne 7 dní. Today/Previous/Next prepína zobrazený rozsah;
prepnutie Month<->Week alebo posun na iný mesiac/týždeň vyvolá **iba jeden**
nový dotaz na presne zobrazovaný rozsah - nikdy reload celej stránky ani
celej appky.

### 4. Day Detail

Klik na číslo dňa (ak má aspoň jednu položku) alebo na "+X more" otvorí
modálne okno (znovupoužitý existujúci `Modal` komponent) so zoznamom
všetkých položiek daného dňa - názov, popis, farebná bodka podľa
naliehavosti, suma (ak existuje) a klik naň rovno naviguje na cieľovú
stránku. Kliknutie priamo na položku v mriežke (nie na "+X more") naviguje
rovno, bez otvárania Day Detail.

### 5. Filters

Riadok prepínačov iba pre tých **5 reálnych** kategórií (Events/Orders/
Sales/Pulls/Attention) + tlačidlo "Reset". Žiadne tlačidlo pre Payouts/
Payments/Fulfillment neexistuje - nedávalo by zmysel ponúkať filter na
kategóriu, ktorá nikdy nemá žiadnu položku. Filtrovanie beží čisto na
frontende (dáta pre zobrazený rozsah sú už stiahnuté), takže prepínanie
filtrov nevyvoláva žiadny ďalší dotaz na server.

### 6. Navigation

Žiadna nová detailová obrazovka sa nevytvárala - všetko vedie na existujúce
stránky, presne tak, ako to už dnes robí Attention Center/Ticket Control
Center/Fulfillment Center pre tie isté záznamy:

- Event -> `/events/:id`
- Order -> `/orders/:id`
- Sale (dávka/skupina) -> `/sales/:id`, kde `:id` je reprezentatívne id
  skupiny (najmenšie id v dávke) - presne to isté id, na aké dnes vedú
  odkazy zo Sales, Event Workspace aj Fulfillment Center.
- Pull -> zoznam `/pulls` (Pulls nemá vlastnú detailovú routu ani dnes -
  editácia sa otvára rovno zo zoznamu; ak by si chcel `/pulls/:id`, je to
  samostatná úloha).
- Attention -> presne tá istá logika, akú Attention Center už dnes používa:
  ak položka má priradenú objednávku, ide sa na `/orders/:id`, inak na
  `/events/:id`.

### 7. Performance prístup

Každý dopyt je orezaný na presný zobrazovaný rozsah dátumov (`WHERE date >=
? AND date <= ?`), žiadne "načítaj celú tabuľku". Events/Orders/Sales majú
tento stĺpec indexovaný už od prvej migrácie (`idx_events_date`,
`idx_orders_date`, `idx_sales_date`) - kalendár tieto indexy len
znovupoužíva. `pulls.event_date` index nemá, ale nemá ho ani dnešný,
existujúci filter dátumu v Pulls - nový index som nepridával, keďže na to
nebol nový dôvod (tvoja vlastná inštrukcia "nepridávaj index bez dôvodu").
Attention Center dáta sa počítajú raz za request a znovupoužijú aj pre
farbu "critical" pri Events aj pre kategóriu Attention - nie dvakrát.
Prepnutie Month/Week nikdy nereloaduje celú appku, iba dáta kalendára.

### 8. Zmenené/nové súbory

Backend: `src-tauri/src/commands/calendar.rs` (nový), `src-tauri/src/models.rs`
(pridané `CalendarFilters`/`CalendarEntry`), `src-tauri/src/commands/mod.rs`,
`src-tauri/src/lib.rs` (registrácia príkazu).
Frontend: `src/pages/Calendar.tsx` (nový), `src/lib/types.ts`, `src/lib/api.ts`,
`src/App.tsx` (nová routa), `src/components/Layout.tsx` (nová položka v
sidebar), `src/components/icons.tsx` (2 nové ikony - šípky vľavo/vpravo pre
Previous/Next, rovnaký vzor ako existujúce ikony).
Dokumentácia: `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/PROTECTED_AREAS.md`,
`CHANGELOG.md`.

### 9. Zmeny v DB/migráciách

**Žiadne.** Žiadna nová migrácia, žiadna nová tabuľka, žiadny nový index.
Kalendár je čisto read-only agregácia nad existujúcimi dátami - presne ako
Attention Center a Inventory Intelligence.

### 10. Výsledky testov

14 nových Rust testov v `commands/calendar.rs`: rozsah dátumov (vrátane
oboch hraníc), lístky bez dátumu sa nikdy nezobrazia, farba "critical" pri
Events sa zhoduje s Attention Center vlastnou kategóriou "event_soon",
zoskupenie predajovej dávky (nie jeden riadok na lístok), dávka naprieč
dvomi eventmi ukazuje "Mixed events", dávka v dvoch menách nikdy nemieša
sumy, farba naliehavosti pri Pulls presne podľa `Pulls.tsx` vlastného
varovného okna (aj že sa vypne po označení "transferred"), navigácia
Attention položiek (na objednávku aj na event), viac typov v ten istý deň,
prázdna databáza vracia prázdny zoznam (nie chybu). Celá sada:
**1052 testov, 0 zlyhaní** (predtým 1038 - žiadny existujúci test sa
nezmenil ani nerozbil). `npx tsc -b` aj `npm run build` čisté.

### 11. Čo bolo zámerne NEimplementované (a prečo)

- **Payouts** - v appke neexistuje žiadna samostatná entita ani dátum
  "payout". Slovo "Payout" existuje IBA ako názov stĺpca v Google Sheets
  synchronizácii ("Payout Per Ticket"/"Payout status"), čo je len iný názov
  pre už existujúce `sale_price_cents`/`payment_status` pri Sales. Nebolo
  čo pridať bez vymyslenia niečoho, čo v appke nie je.
- **Payments** - tabuľka `payments` v databáze síce existuje (migrácia 007),
  ale žiadny Rust kód ju nikde nečíta ani nezapisuje - súbor `payments.rs`,
  na ktorý odkazuje komentár v migrácii, sa nikdy nenapísal. Je to
  schéma bez funkčnosti, rovnaký prípad ako staré, dnes nepoužívané tabuľky
  z verzie 2.4.1 (Market Monitor).
  **Odporúčanie**: ak by si chcel Payments do kalendára pridať naozaj, treba
  najprv postaviť funkciu, ktorá tú tabuľku reálne používa - to je
  samostatná úloha, nie niečo, čo sa dá "dorobiť" do kalendára.
- **Fulfillment** - `delivery_status` je iba text (Delivered/Not delivered/
  ...), bez akéhokoľvek dátumu (žiadny "delivery_date"/deadline stĺpec
  nikde neexistuje). Bez dátumu nemá kalendárová položka kam patriť.

Nič z týchto troch nebolo vymyslené ani odhadnuté - ak sa niektorá z týchto
dátových medzier v budúcnosti zaplní (napr. pribudne reálny dátum doručenia),
kalendár sa dá rozšíriť o štvrtú/piatu skutočnú kategóriu rovnakým spôsobom,
akým sú postavené tieto tri.

---

## Rozhodnutia, ktoré som urobil sám (na zváženie)

- Časť A aj Časť B idú von ako jedna spoločná verzia 2.5.0 (pozri úvod).
- Sidebar "Tickets" som prečítal ako harmonika nad existujúcimi routami, nie
  ako zlúčenie 5 stránok do jednej (pozri bod 8 v Časti A).
- Suma pri Sale položkách v kalendári je zjednodušená verzia toho, čo už
  robí Sales stránka (rovnaké pravidlo "nikdy nemiešaj meny", ale bez
  výpočtu zisku/marže/ROI, ktoré tu nedávajú zmysel).
- Farba naliehavosti pri Pulls (3 dni pred eventom) je v Rust kóde
  zámerne duplikovaná (nie importovaná) z `Pulls.tsx` - rovnaký spôsob, aký
  už appka používa aj inde, keď treba to isté pravidlo na dvoch miestach.

## Balík

`tiqr-manager-2.5.0.zip` - 366 súborov, verzia zjednotená vo všetkých 5
miestach + oboch lockfile-och, integrita zip súboru overená. Zostavené
priamo z toho istého stromu súborov, na ktorom bežali všetky testy vyššie
(nie zo samostatnej kópie) - `node_modules`/`dist`/`src-tauri/target`/
`src-tauri/gen` vylúčené rovnako ako doteraz.

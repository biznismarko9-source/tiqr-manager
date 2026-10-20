# TIQR Manager 2.0.55 — čísla, čo appka zapisuje do Sheetu, teraz s čiarkou

## Čo si mi napísal

*"vsetko co sa po update zmeni nesmie davat nikde . medzi cisla v tabulkach, musi tam ist čiarka ,"*

- spolu so screenshotom tvojho Google Sheetu, kde bolo vidieť presne to - niektoré riadky "337,12" (čiarka, správne), iné "50.00" (bodka, zle) v tom istom stĺpci.

## Čo som našiel

Appka pri zápise čísel priamo do tvojho Google Sheetu vždy používala bodku ako desatinnú čiarku ("50.00"). Tvoj Sheet je ale nastavený na slovenskú/európsku lokalizáciu (desatinná čiarka) - keď appka pošle "50.00" cez Sheets API, Google Sheets to niekedy neprečíta ako skutočné číslo 50, len to uloží ako obyčajný text presne tak, ako to prišlo. Výsledok presne sedí s tvojím screenshotom: riadky, čo appka nikdy nezapisovala (buď si ich zadal ty sám, alebo boli v Sheete predtým), majú správnu čiarku; riadky, čo appka zapísala alebo opravila, majú bodku a vyzerajú inak než zvyšok tabuľky.

**Dôležité - toto nie je nová chyba z môjho posledného kola.** Siaha až k pôvodnej Sheets synchronizácii a k oprave cien z verzie 2.0.42 - moja vlastná práca na prevode mien (2.0.53/2.0.54) len prevzala ten istý (chybný) spôsob zápisu, keďže som sa pri písaní držal existujúceho vzoru v appke. Netýka sa to appky samotnej (Dashboard, Sales, Orders v appke ukazujú čísla správne, tvojím lokálnym formátom) - len textu, čo appka posiela do Sheetu.

## Čo je nové

Všade, kde appka zapisuje sumu do Google Sheetu, teraz použije čiarku namiesto bodky. Konkrétne:

- Nová objednávka, čo appka sama založí v Sheete (Total Purchase Price, Price Per Ticket)
- Automatická oprava cien pri drobnom nesúlade (2.0.42 - keď sa Total Purchase Price a Number × Price Per Ticket líšia len o pár centov)
- Prevod na EUR (2.0.53/2.0.54 - Currency/Price Per Ticket/Total Purchase Price)
- Predaj zapísaný späť do Sheetu (Payout Per Ticket)
- Pull fee (koľko si zaplatil tomu, čo ti tiket ťahal) - na oboch miestach, kde sa píše (Orders & Sales aj samotný Pulls Sheet)

Appka samotná (čo vidíš na obrazovke v Dashboard/Sales/Orders) sa vôbec nemenila - toto je len o tom, čo appka posiela von, do tvojho Sheetu.

## Ako som to overoval

Rovnaké obmedzenie ako pri 2.0.52-2.0.54 - žiadny `rustc`/`cargo` k dispozícii. Napísal som novú funkciu (`format_cents_for_sheet`) s vlastným testom, a keďže táto zmena mení SKUTOČNÝ výstup viacerých už existujúcich funkcií, musel som prejsť a opraviť aj **existujúce testy appky**, čo predtým očakávali starý formát s bodkou (inak by po tejto zmene pri reálnom `cargo test` popadali - nie preto, že by bol kód zlý, ale preto, že testy by kontrolovali starú, teraz už nesprávnu hodnotu). Pri jednom z nich som si všimol, že by bez opravy testu appka nesprávne nahlásila aj úplne správny riadok v Sheete ako "treba prepísať" - opravené priamo v teste, nie v appke (appka bola v poriadku, len testovacie dáta boli v starom formáte).

Vlastný skript na kontrolu spárovania zátvoriek prešiel čisto na všetkých troch upravených súboroch.

## Čo teraz urobiť

1. Nainštaluj 2.0.55.
2. Skús Push Orders/Push Sales (Settings → Integrations) na objednávkach, kde si predtým videl bodku namiesto čiarky - mali by sa teraz opraviť.
3. Vytvor alebo synchronizuj novú objednávku a skontroluj priamo v Sheete, že Total Purchase Price/Price Per Ticket majú čiarku.
4. Skús znova Convert to EUR na objednávke v inej mene a skontroluj to isté.

## Zmenené súbory

**Backend (Rust, 3 súbory):**
- `src-tauri/src/money.rs` - nová `format_cents_for_sheet` (čiarka namiesto bodky) + test
- `src-tauri/src/commands/orders_sheet_sync.rs` - 9 miest prepnutých na `format_cents_for_sheet`, oprava 7 existujúcich testov, ktoré by inak spadli
- `src-tauri/src/commands/pulls_sheet_sync.rs` - 1 miesto prepnuté, oprava 2 existujúcich testov

Žiadny frontend súbor, žiadna migrácia.

**Verzia (8 miest):** ako vždy, všetkých na `2.0.55`.

## STOP

2.0.55 hotové. Skontroluj podľa krokov vyššie - najmä existujúce riadky v Sheete po Push Orders/Push Sales. Ak niekde stále uvidíš bodku namiesto čiarky, pošli mi prosím presne to miesto (názov stĺpca) - mohol som prehliadnuť ešte nejaké miesto zápisu.

Keď toto potvrdíš, vraciam sa k automatickej kategorizácii eventov (bod, čo ešte stále čaká).

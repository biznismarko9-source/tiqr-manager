# TIQR Manager 2.0.19 — Automatická štruktúra hárku (dropdowny + Revenue/Profit vzorce)

Presne to, čo si chcel: hárok **Orders & Sales** sa teraz sám udržiava v správnom stave — dropdowny na výber aj živé vzorce pre Revenue/Profit — automaticky, zakaždým, keď appka s hárkom pracuje (Order sync, Sales sync, Push orders alebo Push sales). Netreba na to žiadne nové tlačidlo — nech klikneš ktorékoľvek z týchto štyroch, appka si po ceste skontroluje a doplní celú štruktúru.

## ⚠️ Prečítaj si najprv toto — prvé spustenie prepíše Revenue/Profit vo VŠETKÝCH riadkoch

Ako si vybral (potvrdené cez moju otázku): appka nastavuje dropdowny aj vzorce na **úplne všetkých riadkoch hárku**, nielen na tých, čo appka pozná. To znamená, že hneď pri prvom spustení ktoréhokoľvek zo 4 tlačidiel po tomto update appka **prepíše obsah stĺpcov Revenue a Profit na každom riadku so vzorcom** — aj keby si tam predtým mal niečo napísané ručne. Presne to si chcel (živý vzorec namiesto ručného čísla), ale keďže ide o zásah do celého hárku naraz, odporúčam si pred prvým spustením spraviť kópiu hárku (File → Make a copy v Google Sheets), pre istotu.

Dropdowny (Ticket Type/Site Listed/Status/Delivery status/Payout status/pull) sú v tomto zmysle bezpečné — nemenia obsah žiadnej bunky, len obmedzujú, čo sa dá vybrať pri kliknutí.

## 1. Šesť stĺpcov teraz má skutočný dropdown v hárku

Presne stĺpce z tvojich obrázkov + Status/Delivery status/Payout status/pull:

- **Ticket Type** — rovnaké možnosti ako appka vždy ponúkala (E-ticket, PDF, Mobile transfer, Physical, Will call)
- **Site Listed** — všetky platformy, čo máš v appke označené na predaj (Seatiks, Viagogo, Whatsapp, ...)
- **Status** — presne Listed / Unlisted / Sold
- **Delivery status** — presne Delivered / Not delivered
- **Payout status** — presne Pending / Paid
- **pull** — presne Yes / No

Dôležité: dropdown **nikdy nič neodmieta** ("show warning", nie "reject input") — ak do bunky napíšeš alebo appka zosynchronizuje hodnotu, čo ešte nie je v zozname, appka to pokojne prijme, len ti Google Sheets ukáže malú výstrahu (žltý trojuholník). Presne to si chcel: nová hodnota sa nikdy nezablokuje.

### Ticket Type a Site Listed rastú samé

Toto sú tie dve, čo si chcel "rozširovateľné" — keď niekto pridá novú hodnotu **buď v hárku, alebo v appke** ("Other..." pri vytváraní objednávky, alebo "+ New" pri platforme), tá hodnota sa objaví v zozname na výber úplne všade od ďalšieho sync-u/push-u. Netreba na to žiadnu novú tabuľku ani nastavenie — appka si jednoducho vždy pozrie, čo je aktuálne použité na reálnych lístkoch/platformách.

Status/Delivery status/Payout status/pull naopak **zostávajú presne také, ako si zadal** — pevný zoznam, nič sa tam nepridáva samo.

## 2. Revenue a Profit sú teraz živé vzorce

Presne ako si vybral (potvrdené cez moju otázku): appka do buniek vpisuje **skutočný vzorec Google Sheets**, nie číslo:

- **Revenue** = `Payout Per Ticket × Number of Tickets`
- **Profit** = `Revenue − Total Purchase Price`

Keďže je to skutočný vzorec, prepočíta sa **okamžite** pri akejkoľvek zmene v tom riadku — aj keď si niečo upravíš ručne priamo v hárku bez appky, aj keď appka o tom sync-i ešte nevie. Appka tieto dva stĺpce naďalej nikdy nečíta (Dashboard si Revenue/Profit počíta úplne nezávisle, ako doteraz) — len ich zapisuje.

Vzorec appka vie postaviť len vtedy, keď má v hárku k dispozícii všetky potrebné stĺpce (Payout Per Ticket + Number of Tickets pre Revenue; k tomu ešte Total Purchase Price pre Profit) — ak niektorý chýba, ten jeden vzorec sa jednoducho vynechá, zvyšok hárku sa nedotkne.

## 3. Nový riadok pridaný ručne v hárku

Ak si nový riadok pridáš ty sám priamo v Google Sheets (nie cez appku): dropdown naň bude fungovať hneď (appka nastavuje dropdowny s veľkou rezervou dopredu, min. 500 riadkov). Revenue/Profit vzorec sa na ten riadok doplní **pri najbližšom spustení** ktoréhokoľvek zo 4 tlačidiel (Order sync/Sales sync/Push orders/Push sales) — nie okamžite v momente, keď riadok napíšeš, keďže appka sa s hárkom rozpráva len vtedy, keď niektoré z tlačidiel spustíš.

## 4. Aj appka sama teraz drží presne tieto isté možnosti

- **Resale status** a **Delivery status** na lístku (Tickets → Edit) boli doteraz obyčajné textové polia — teraz sú to presne tie isté dropdowny ako v hárku (Listed/Unlisted/Sold a Delivered/Not delivered). Stará hodnota z pred tejto verzie (ak by bola iná) zostane viditeľná a vybraná, len sa nezobrazí v zozname možností, kým ju nezmeníš.
- **Ticket type** vo formulári "New order" teraz ukazuje presne ten istý rastúci zoznam ako hárok, namiesto pevného zoznamu ako doteraz — "Other..." funguje rovnako ako predtým, len sa tá nová hodnota od teraz aj zapamätá pre budúce objednávky.

## 5. Ak niečo z toho zlyhá, zvyšok syncu/push-u to nezastaví

Ak by appka z nejakého dôvodu nevedela dropdowny/vzorce obnoviť (napr. dočasný výpadok siete), nahlási to ako jeden riadok upozornenia navyše vo výsledku — ale samotný sync/push, ktorý si spustil, prebehne normálne ďalej. Táto nová časť je vylepšenie navyše, nikdy nie dôvod na zastavenie tej hlavnej práce.

## 6. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 428 passed, 0 failed (bolo 404 - pribudlo 24 nových testov)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.19 build" v hlavičke)
```

24 nových testov pokrýva presne rozhodnutia vyššie: že Status/Delivery status/Payout status/pull majú vždy presne tie isté pevné možnosti, že Ticket Type začína piatimi pôvodnými možnosťami a rastie o každú reálne použitú hodnotu (case-insensitive, aby "e-ticket" a "E-ticket" neboli dve rôzne položky), že Site Listed ukazuje len platformy označené na predaj (nie nákupné), že sa dropdown pre Site Listed vynechá, keď zatiaľ nemáš žiadnu predajnú platformu, že chýbajúci stĺpec sa jednoducho preskočí bez chyby, že Revenue/Profit vzorec vždy trafí presne tie správne stĺpce hárku (aj keď sú v inom poradí), že Profit sa vynechá samostatne, keď mu chýba len jeho vlastný stĺpec (Revenue pritom ide ďalej), a že pri 0 riadkoch dát appka nevytvorí žiadny vzorec.

## 7. Zmenené súbory

**Zmenené:**
- `src-tauri/src/google_sheets.rs` — nové `get_sheet_numeric_id`, `batch_update`, `update_values_as_formulas`, `set_data_validation_request`
- `src-tauri/src/commands/tickets.rs` — nová `known_ticket_type_names` + príkaz `list_ticket_types`
- `src-tauri/src/commands/orders_sheet_sync.rs` — nová `plan_sheet_structure_updates` + `ensure_orders_sheet_structure`, zapojené do všetkých 4 príkazov (sync_orders/sync_sales/push_orders/push_sales)
- `src-tauri/src/lib.rs` — zaregistrovaný `list_ticket_types`
- `src/lib/api.ts` — nová `listTicketTypes`
- `src/pages/Orders.tsx` — Ticket type teraz rastúci zoznam namiesto pevného poľa
- `src/pages/Tickets.tsx` — Resale status/Delivery status teraz dropdowny namiesto textových polí

**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.19`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.19 hotové a overené (428/428 testov, čisté `tsc`/`build`). **Priorita:** pred prvým spustením si over bod "⚠️ Prečítaj si najprv toto" vyššie (kópia hárku pre istotu). Potom spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. V hárku klikni na bunku v stĺpci Ticket Type/Site Listed/Status/Delivery status/Payout status/pull na hociktorom riadku — mal by sa objaviť dropdown so šípkou.
2. Skontroluj pár riadkov so stĺpcami Revenue/Profit — mali by tam byť skutočné vzorce (klikni na bunku, hore vo vzorcovom riadku uvidíš `=...`), nie len čísla.
3. Skús v hárku ručne zmeniť Payout Per Ticket na existujúcom riadku — Revenue/Profit by sa mali prepočítať okamžite, bez toho, aby si čokoľvek spúšťal v appke.
4. V appke skús Tickets → Edit na nejakom lístku — Resale status aj Delivery status by mali byť dropdowny s presne tými istými možnosťami.
5. V appke skús Orders → New order — Ticket type by mal fungovať rovnako ako doteraz (vrátane "Other...").

Napíš mi, či to takto sedí, alebo či niečo chceš inak (napr. iné presné názvy pre Status/Delivery status možnosti, alebo väčšiu/menšiu rezervu riadkov pre dropdowny).

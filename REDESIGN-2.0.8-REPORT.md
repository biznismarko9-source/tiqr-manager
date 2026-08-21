# TIQR Manager 2.0.8 — Orders & Tickets sync z Google Sheets (fáza 1)

Toto je väčší report, lebo ide o väčšiu vec: appka sa vie po prvý raz napojiť na tvoju skutočnú kombinovanú "buy + sell" tabuľku a z jej **prvej časti** (tá, čo vypĺňaš pri nákupe) si sama vytvoriť Orders a Tickets. Druhá časť (predaj - Sales) je zámerne mimo tohto reportu, ide na ňu hneď ďalší krok.

## 1. Čo presne appka teraz vie

V Settings -> Integrations pribudla druhá karta, "Orders & Tickets", vedľa tej pre Pulls. Funguje rovnako - vložíš URL/ID tabuľky a presný názov hárku, pripojíš, klikneš "Sync now".

Appka si z pripojeného hárku prečíta **len túto prvú časť** stĺpcov (presne tie, čo si spomínal, že vypĺňaš ako prvé):

| Stĺpec v tabuľke | Čo z neho appka spraví |
|---|---|
| `Event Name` | Nájde Event podľa mena, alebo ak neexistuje, **sama ho vytvorí** (s dátumom z tohto riadku) |
| `Date (DD/MM/YYYY)` | Dátum eventu (len pri prvom vytvorení) AJ dátum nákupu (Order.purchase_date) - tvoja tabuľka nemá samostatný "dátum nákupu" v tejto prvej časti, takže tento jeden stĺpec slúži na oboje |
| `platform` | Nájde platformu podľa mena, alebo ju vytvorí |
| `Section`, `Row` | Uložia sa na každý vytvorený lístok |
| `Seats` | Zoznam oddelený čiarkou (napr. "11,12,13") - musí presne sedieť s počtom kusov, alebo nechaj prázdne |
| `Order ID` | Tvoje vlastné číslo objednávky - uloží sa do appky ako referencia (appka má aj svoj vlastný kód, napr. ORD-000123, ten ostáva) |
| `Total Purchase Price` | **Nepoužíva sa priamo** - appka si ho len skontroluje oproti "Number of Tickets × Price Per Ticket" (viď bod 3 nižšie) |
| `Number of Tickets` | Počet kusov |
| `Price Per Ticket` | Cena za kus |
| `currency` | Mena tohto riadku - ak je prázdna, použije sa mena, čo si nastavil pri pripojení hárku |
| `Email (used)` | Uloží sa do poznámky objednávky ako "Email used: ..." (appka nemá samostatné pole na email, takže poznámka je najbližšie miesto) |
| `Ticket Type` | Uloží sa na každý lístok |

Povinné stĺpce (bez nich appka sync vôbec nespustí a napíše presne, čo chýba): **Event Name, Date, Number of Tickets, Price Per Ticket**. Všetko ostatné môže chýbať - riadok sa vytvorí aj bez toho.

Zvyšné stĺpce z tvojej tabuľky (Site Listed, Payout Per Ticket, Revenue, Profit, Status, Delivery status, Payout status, date of purchase, paid by, pull, who pulled, how much pull) **táto fáza úplne ignoruje** - nespôsobia žiadnu chybu, len sa zatiaľ nič s nimi nedeje. Na tie príde rad v ďalšom kroku (Sales sync).

## 2. Ako appka pozná, ktorý riadok už spracovala

Presne tak, ako pri Pulls: appka si do tabuľky sama pridá nový stĺpec **"TIQR ID"** a doňho pri prvom syncu zapíše kód vytvorenej objednávky (napr. ORD-000045). Tento stĺpec nikdy nepíš rukou.

Toto je dôležité pre budúcnosť: presne tento stĺpec bude appka neskôr používať na to, aby vedela, ku ktorej objednávke/lístkom patrí druhá časť riadku, keď príde na Sales sync.

## 3. Fáza 1 = len vytváranie, nie úpravy (a prečo)

Toto je najdôležitejšia vec, čo si treba zapamätať: **keď má riadok už vyplnené "TIQR ID", appka ho pri ďalšom syncu úplne preskočí** - vôbec sa nepozerá, či si medzičasom zmenil počet kusov, cenu, event, čokoľvek. Nič sa neaktualizuje.

Prečo takto: keď už objednávka existuje aj s vytvorenými lístkami, zmena napríklad ceny by znamenala prepočítavať náklady rozpočítané po jednotlivých lístkoch - a to je presne tá citlivá finančná logika, ktorú som sľúbil nikdy neupravovať bez toho, aby som sa ťa najprv opýtal. Tak isto to spravila aj prvá verzia Pulls syncu - toto je rovnaké rozhodnutie, len teraz pre Orders.

Čo to prakticky znamená: nový riadok (bez TIQR ID) → vytvorí sa objednávka. Starý riadok (má TIQR ID) → nič sa nedeje, ani keby si v ňom čokoľvek prepísal. Ak potrebuješ opraviť už vytvorenú objednávku, uprav ju priamo v appke (Orders -> otvoriť objednávku).

## 4. Kontrola Total Purchase Price

Keď je stĺpec "Total Purchase Price" vyplnený, appka ho porovná s "Number of Tickets × Price Per Ticket". Ak to presne nesedí, celý riadok odmietne a napíše prečo (namiesto toho, aby hádala, ktoré z tých dvoch čísel je to správne).

Príklad: 2 kusy × 50.00 = 100.00 - ak by si do Total Purchase Price napísal napr. 95, appka ten riadok preskočí s chybou, kým to neopravíš. Ak stĺpec Total Purchase Price necháš prázdny, táto kontrola sa jednoducho nerobí a riadok prejde normálne.

## 5. Event a Platform sa vytvárajú automaticky

Na rozdiel od CSV importu (kde musí Event už v appke existovať) tento sync si chýbajúci Event aj Platformu **sám vytvorí podľa mena** - presne tak, ako to už appka robí pri Pulls syncu. Dôvod: sync je určený na to, aby si ho spúšťal opakovane nad tabuľkou, ktorú priebežne vypĺňaš, takže vyžadovať si najprv ručne založiť každý event v appke by celú vec spomalilo.

Keď appka Event podľa mena už raz nájde (alebo vytvorí), pri ďalších riadkoch s tým istým menom ho len znovu použije - jeho dátum sa už nikdy neprepíše, aj keby mal iný riadok iný dátum.

## 6. Čo urob teraz

1. Spusti `1-CLICK-UPDATE.bat` (teraz na v2.0.8), počkaj na zelený build.
2. V appke choď do Settings -> Integrations, pripoj svoju skutočnú tabuľku na karte "Orders & Tickets" (rovnaký postup ako pri Pulls - vlož URL/ID + presný názov hárku, over si menu).
3. Klikni "Sync now" a skontroluj výsledok - koľko sa vytvorilo, či sa niečo preskočilo a prečo.
4. Skontroluj v appke (Orders / Tickets), že sa vytvorené objednávky a lístky zhodujú s tým, čo čakáš.

## 7. Čo príde ďalej

Fáza 2: sync **Sales** z druhej časti tých istých riadkov (Site Listed, Payout Per Ticket, Revenue, Profit, Status, Delivery status, Payout status, date of purchase, paid by, pull, who pulled, how much pull) - napojený na presne tie lístky, čo vytvorí táto fáza cez ten istý "TIQR ID" stĺpec. Idem na to hneď po tom, ako potvrdíš, že táto časť funguje na tvojich reálnych dátach.

## 8. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 302 passed, 0 failed (276 + 26 nových)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.8 build" v hlavičke)
```

26 nových testov pokrýva presne rozhodnutia z bodov 2-5 vyššie: že nový riadok bez TIQR ID vytvorí objednávku aj lístky so správnymi hodnotami, že Seats musí sedieť s počtom kusov, že Total Purchase Price sa skontroluje (a čo sa stane, keď nesedí / keď je prázdny), že mena z riadku má prednosť pred menou z pripojenia (a že prázdna bunka spadne na menu z pripojenia), že Platform aj Event sa vytvoria automaticky a druhýkrát sa už len znovu použijú (Event si nikdy neprepíše dátum), že "Order ID" sa uloží, že riadok s TIQR ID sa naozaj úplne preskočí - vrátane toho, že ani nevytvorí platformu/event, ktoré by inak z jeho hodnôt vznikli.

## 9. Zmenené a nové súbory

**Nové:** `src-tauri/src/commands/orders_sheet_sync.rs` (celá logika syncu), `src-tauri/migrations/009_orders_external_reference.sql` (nový stĺpec `orders.external_reference` na "Order ID")
**Zmenené:** `src-tauri/src/commands/pulls_sheet_sync.rs` (jedna funkcia na prevod dátumu sprístupnená aj pre nový modul), `src-tauri/src/models.rs` (výsledok syncu premenovaný z `PullsSyncResult` na `SheetSyncResult` - bol vždy všeobecný, teraz ho používajú obe synchronizácie), `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/db.rs` (registrácia nového modulu/príkazu/migrácie), `src/lib/types.ts`, `src/lib/api.ts` (rovnaké premenovanie + nová `syncOrders`), `src/pages/Settings.tsx` (karta pre Pulls a nová karta pre Orders & Tickets teraz zdieľajú jeden komponent)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.8`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.8 hotové a overené (302/302 backend testov vrátane 26 nových presne na Orders/Tickets sync, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat`, pripoj svoju reálnu tabuľku na novej karte "Orders & Tickets" a skús "Sync now" - napíš mi, koľko sa vytvorilo a či niečo prekvapilo (najmä keby appka niečo odmietla kvôli Total Purchase Price - to je zámer, nie chyba, viď bod 4). Keď potvrdíš, že sedí, idem rovno na Sales sync (druhá časť riadkov).

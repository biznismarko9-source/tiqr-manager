# TIQR Manager 2.0.10 — Sales sync (fáza 2)

Toto je druhá polovica tej istej tabuľky: appka teraz vie prečítať aj časť "predaj" a napojiť ju na presne tie objednávky/lístky, čo vytvorila fáza 1 (Order sync). Obe fázy bežia nad **tou istou jednou tabuľkou** - nič sa nepripája druhýkrát.

Keďže si na moje dve doplňujúce otázky (aké presné hodnoty píšeš do "Status"/"Delivery status") odpovedal "pokračuj", pokračoval som s najbezpečnejšou voľbou, ktorú som navrhol predtým: **obyčajné textové políčka**, nie pevný zoznam možností - takže appka nikdy nič neodmietne kvôli neznámej hodnote. Ak by si to radšej chcel ako dropdown s presne danými voľbami, napíš mi zoznam a prerobím to.

## 1. Ako appku teraz pripájaš a spúšťaš

Nič sa nemení na pripájaní - stále len jedna karta "Orders & Tickets" v Settings -> Integrations, stále len jedno URL/ID + názov hárku. Pribudlo len druhé tlačidlo: namiesto jedného "Sync now" tam teraz sú **"Order sync"** (presne to, čo appka robila doteraz) a **"Sales sync"** (nové). Obe čítajú z tej istej pripojenej tabuľky.

## 2. Čo presne appka teraz vie (Sales sync)

| Stĺpec v tabuľke | Čo z neho appka spraví |
|---|---|
| `Site Listed` | Nájde platformu podľa mena (alebo ju vytvorí) - **inú** od platformy z nákupu, appka si ich vie odlíšiť |
| `Payout Per Ticket` | Cena, za ktorú sa lístok predal - **toto je zároveň signál appke, že riadok je pripravený na predaj** (bez tejto hodnoty appka riadok len prejde bez zmeny) |
| `Status` | Uloží sa do nového políčka **Resale status** na lístku (pozri bod 4) |
| `Delivery status` | Uloží sa do nového políčka **Delivery status** na lístku |
| `Payout status` | pending/paid - premietne sa priamo do appky. Prázdne = pending |
| dátum predaja (pozri bod 3 nižšie - **over si tento stĺpec, nie som si istý presným názvom**) | Dátum predaja |
| `paid by` | Uloží sa ako referencia kupujúceho |
| `pull`, `who pulled`, `how much pull` | Uložia sa ako text do poznámky k predaju (tvoja voľba - Pull funkcia zostáva úplne samostatná, žiadne prepojenie) |
| `Revenue`, `Profit` | **appka ich vôbec nečíta** (tvoja voľba) - appka si ich aj tak vždy sama dopočíta z Payout Per Ticket a nákladov na lístok, takže ukladať ešte raz to isté číslo z tabuľky by mohlo časom začať nesedieť s tým, čo appka reálne zobrazuje |

Keď má riadok viac lístkov (napr. 2 kusy v jednej objednávke), appka ich predá **naraz, ako jeden predaj** - presne tak, ako keby si v appke použil "New sale" na viac lístkov naraz. Uvidíš ich v appke zoskupené ako jeden predaj, presne ako je to zoskupené aj v tvojom riadku.

## 3. Dôležité - over si názov stĺpca s dátumom predaja

Nebol som si istý, ako presne sa u teba volá stĺpec s dátumom predaja (v tvojom pôvodnom popise to vyzeralo ako "date of purchase", čo je mätúce, lebo dátum nákupu už appka rieši v prvej časti). Radšej som **nehádal** a nepoužil napríklad dátum nákupu namiesto neho - to by mohlo pokaziť mesačné súčty v appke, keby si lístky predal neskôr, než si ich kúpil.

Appka teraz skúša tieto názvy (nezáleží na veľkých/malých písmenách): **Date sold, Sale date, Date of sale, Date of purchase, Sold date, Payout date**. Ak tvoj stĺpec znie inak, Sales sync ti to napíše ako chybu pri každom riadku ("missing a recognized sale-date column...") - stačí mi poslať presný názov a pridám ho.

## 4. Nové políčka na lístku - Resale status a Delivery status

Keďže si chcel skutočné nové políčka (nie len text v poznámke), appka teraz má na každom lístku dve nové políčka: **Resale status** a **Delivery status**. Sú to obyčajné textové políčka (nie dropdown), vidíš ich a vieš ich upraviť aj ručne v appke (Tickets -> otvoriť lístok -> Edit) - presne vedľa existujúceho poľa "Status".

Sú zámerne oddelené od existujúceho poľa "Status" na lístku (available/listed/sold/cancelled) - to appka aj tak sama prepne na "sold" v momente, keď sa predaj vytvorí, takže tvoje vlastné "Status" z tabuľky je niečo iné a appka ho nikam nemieša.

## 5. Sales sync sa spúšťa len raz za lístok (rovnako ako Order sync)

Presne, ako si vybral: keď už má lístok aktívny predaj, ďalší sync sa ho vôbec nedotkne - ani jeho ceny, ani platformy, ani Resale/Delivery status. Ak potrebuješ niečo opraviť, uprav to priamo v appke.

Ak je v objednávke viac lístkov a niektoré už predané (napr. si jeden predal ručne v appke pred spustením sync-u), appka predá len tie zvyšné - nepokazí to, čo už existuje.

Zrušený (cancelled) lístok v objednávke appka jednoducho vynechá z predaja - nezablokuje tým predaj ostatných lístkov v tej istej objednávke.

## 6. Čo urob teraz

1. Spusti `1-CLICK-UPDATE.bat` (teraz na v2.0.10), počkaj na zelený build.
2. V appke choď do Settings -> Integrations, na karte "Orders & Tickets" klikni **"Sales sync"**.
3. Skontroluj výsledok - najmä ak appka nahlási chybu o chýbajúcom dátume (pozri bod 3 vyššie), napíš mi presný názov tvojho stĺpca.
4. Skontroluj v appke pár predaných lístkov (Sales, alebo Tickets -> Edit), či Resale status/Delivery status a cena sedia s tým, čo čakáš.

## 7. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 330 passed, 0 failed (310 + 20 nových)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.10 build" v hlavičke)
```

20 nových testov pokrýva presne rozhodnutia z bodov 2-5 vyššie: že riadok bez "TIQR ID" sa ticho preskočí, že riadok s "TIQR ID" ale bez Payout Per Ticket sa spočíta ako "unchanged", že sa vytvorí predaj so správnou cenou/menou/stavom, že viac lístkov z jednej objednávky sa predá naraz ako jeden zoskupený predaj, že už raz predaný lístok sa pri ďalšom syncu vôbec nedotkne (ani jeho Resale/Delivery status), že záporná/nečíselná cena, chýbajúci dátum a neplatný Payout status sa odmietnu so zrozumiteľným dôvodom, že "refunded" sa nedá nastaviť cez sync, že platforma zo "Site Listed" sa vytvorí správne odlíšená od nákupnej platformy (a existujúca nákupná platforma s rovnakým menom sa označí, že sa používa aj na predaj), že Resale/Delivery status sa uloží na každý lístok objednávky, že pull/who pulled/how much pull sa uložia do poznámky, že chýbajúci stĺpec "Payout Per Ticket" celý sync rovno zastaví so zrozumiteľnou správou, a že zrušený lístok sa vynechá bez toho, aby zablokoval ostatné.

## 8. Zmenené a nové súbory

**Nové:** `src-tauri/migrations/010_ticket_resale_delivery_status.sql` (nové stĺpce `tickets.resale_status`, `tickets.delivery_status`)
**Zmenené:** `src-tauri/src/commands/orders_sheet_sync.rs` (celá logika Sales sync-u - `apply_sales_rows`, `sync_sales`), `src-tauri/src/commands/tickets.rs` a `src-tauri/src/models.rs` (nové políčka na lístku, aj v úprave lístka), `src-tauri/src/db.rs` (registrácia novej migrácie), `src/lib/types.ts`, `src/lib/api.ts` (nová `syncSales`), `src/pages/Tickets.tsx` (dve nové polia v úprave lístka), `src/pages/Settings.tsx` (karta Orders & Tickets teraz má dve tlačidlá na jednom pripojení)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.10`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.10 hotové a overené (330/330 backend testov vrátane 20 nových, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat`, skús "Sales sync" na karte "Orders & Tickets" a napíš mi výsledok - **najdôležitejšie je, či appka nahlási chybu o chýbajúcom dátume predaja** (bod 3) - ak áno, pošli mi presný názov toho stĺpca z tvojej tabuľky a opravím alias na prvý pokus. Keď potvrdíš, že sedí, Orders/Sales sync je hotový celý.

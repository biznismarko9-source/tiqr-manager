# TIQR Manager 2.0.20 — Oprava chyby z 2.0.19 (`sheetId`) + nové tlačidlo "Update sheet"

## 1. Najprv ospravedlnenie — toto bola moja chyba v 2.0.19

Presne tá chyba, čo si nahlásil (`missing field sheetId`), je regresia, ktorú som spôsobil ja v predošlej verzii. Keď som v 2.0.19 pridával dropdowny do hárku Orders & Sales, potreboval som od Google Sheets aj interné číselné ID hárku (`sheetId`) — to je iná vec ako názov záložky ("Pulls", "Orders"), Google Sheets ho používa len interne pri nastavovaní dropdownov. Na to som pridal nový dopyt na Google, ktorý si popri bežných veciach pýta aj toto číslo.

Problém je, že appka už dávno predtým (od 2.0.14) používala **iný, starší dopyt** na úplne to isté miesto v Google Sheets — ten, čo beží zakaždým, keď vložíš/pridáš URL hárku, aby appka vedela ponúknuť skutočné záložky na výber namiesto ručného písania názvu. Ten starší dopyt sa Google Sheets nikdy nepýtal na `sheetId`, takže Google mu ho v odpovedi ani neposiela (Google Sheets API funguje tak, že keď o pole nepožiadaš, jednoducho ho v odpovedi vôbec nemá — nepošle "prázdne", proste tam nie je).

Ja som ale omylom naprogramoval appku tak, že **obidva** tieto dopyty (ten starý aj ten nový) čítajú odpoveď od Google cez ten istý spoločný "tvar dát", na ktorom som `sheetId` označil ako **povinné**. Výsledok: každý pokus pridať/vložiť hárok — čo interne beží presne cez ten starý dopyt — začal padať presne s hláškou, čo si poslal. Toto vôbec nesúvisí s tým, či je hárok prázdny alebo nie; padalo to úplne vždy, pri akomkoľvek pridávaní hárku, od chvíle, čo si nainštaloval 2.0.19.

**Oprava:** `sheetId` je teraz označené ako nepovinné pole (appka počíta s tým, že tam občas jednoducho nebude). Pridal som aj nový test, ktorý presne reprodukuje tvoju hlášku slovo od slova (rovnaké telo odpovede, aké si poslal ty), aby sa niečo takéto už nemohlo znova nepozorovane vrátiť.

## 2. Nové tlačidlo "Update sheet"

Presne to, čo si chcel: "ked manualne si tam das tabulku tak vies si dat update tlacitko a ten sheet ti vytvorí presne tak ako ma byt keby nahodou mu tam posles prazny".

V Settings → Integrations pribudlo pri oboch kartách (Pulls aj Orders & Sales) nové tlačidlo **"Update sheet"**, hneď vedľa existujúcich tlačidiel (Sync now / Push to sheet). Je to niečo iné ako "Create a new sheet for me" — to tlačidlo vždy vytvorí úplne nový hárok. "Update sheet" naopak pracuje s hárkom, ktorý už máš pripojený (vložil si jeho URL/ID ručne) — a keď zistí, že v ňom ešte vôbec nie je hlavička (prvý riadok so stĺpcami), sám ju tam napíše presne tak, ako má appka rada.

Dôležité správanie:

- Ak hárok **už hlavičku má** (nech je akákoľvek), appka sa jej vôbec nedotkne — nič neprepíše, nezmení poradie stĺpcov, nič. Kliknutie na "Update sheet" v tom prípade len oznámi, že už je všetko v poriadku. Bezpečné kliknúť kedykoľvek, opakovane, bez rizika.
- Ak hárok **hlavičku nemá** (presne tvoj prípad — "keby nahodou mu tam posles prazny"), appka napíše prvý riadok presne v tom istom tvare, ako keby si klikol "Create a new sheet for me" — len do hárku, ktorý si už sám vybral, namiesto vytvárania nového.
- Pri karte **Orders & Sales** tlačidlo urobí ešte jednu vec navyše: hneď potom, ako je hlavička na mieste, appka **rovno nastaví aj dropdowny a Revenue/Profit vzorce** (presne tú istú automatiku z 2.0.19) — nemusíš čakať na najbližší Order sync/Sales sync/Push orders/Push sales, kým sa to samo doplní. Pri karte Pulls táto časť netreba — Pulls dropdowny/vzorce nemá, tam ide len o hlavičku.

## 3. Testy a build

```
cargo test --lib -> 433 passed, 0 failed (bolo 428 - pribudli 4 nové testy + 1 nový v google_sheets.rs pri oprave chyby = 5 celkovo)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.20 build" v hlavičke)
```

Nový test v `google_sheets.rs` presne reprodukuje tvoju hlášku (rovnaké telo JSON) a overuje, že sa teraz spracuje bez chyby. Ďalšie 4 nové testy (2 pre Pulls, 2 pre Orders & Sales) overujú, že "Update sheet" jasne a rovnako ako Sync/Push zlyhá, keď zatiaľ nič nie je pripojené, a že s naozaj pripojeným hárkom appka v teste bez skutočného Google prístupu zlyhá čisto na správnom mieste (na prihlásení, nie na niečom inom/páde appky) — skutočný zápis do hárku (keď je prázdny/nie je) sa v tomto testovacom prostredí nedá overiť naostro, keďže appka tu nemá pripojenie na skutočný internet/Google účet (rovnaký dôvod, prečo to isté platí pre Sync now/Push/Create a new sheet aj doteraz).

## 4. Zmenené súbory

**Zmenené:**
- `src-tauri/src/google_sheets.rs` — `sheet_id` v `SheetMetadataProperties` je teraz `Option<i64>` namiesto povinného `i64` (samotná oprava chyby), nový regresný test
- `src-tauri/src/commands/pulls_sheet_sync.rs` — nová `setup_pulls_sheet_impl` + príkaz `setup_pulls_sheet` ("Update sheet" pre Pulls)
- `src-tauri/src/commands/orders_sheet_sync.rs` — nová `setup_orders_sheet_impl` + príkaz `setup_orders_sheet` ("Update sheet" pre Orders & Sales, vrátane rovno spusteného `ensure_orders_sheet_structure`)
- `src-tauri/src/lib.rs` — zaregistrované obidva nové príkazy
- `src/lib/api.ts` — nové `setupPullsSheet`, `setupOrdersSheet`
- `src/pages/Settings.tsx` — nové tlačidlo "Update sheet" na oboch kartách + zobrazenie výsledku

**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.20`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.20 hotové a overené (433/433 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. V Settings → Integrations skús znova vložiť/pridať ten istý hárok Pulls, čo ti predtým padal s `missing field sheetId` — teraz by malo prejsť bez chyby, appka by mala rovno ponúknuť skutočné záložky na výber.
2. Skús si niekam (pokojne do nového, úplne prázdneho testovacieho hárku) vložiť URL a pripojiť ho ako Pulls alebo Orders & Sales bez toho, aby si tam čokoľvek písal ručne.
3. Klikni "Update sheet" — hárok by mal dostať správnu hlavičku (over si to priamo v Google Sheets).
4. Pri Orders & Sales skús "Update sheet" aj na hárku, kde už dáta máš — hlavička sa nedotkne, ale dropdowny/Revenue/Profit vzorce by sa mali objaviť/obnoviť hneď, nie až pri ďalšom sync-i.
5. Klikni "Update sheet" ešte raz na hárku, čo je už v poriadku — nemalo by sa vôbec nič zmeniť, len hláška, že je všetko OK.

Napíš mi, či sa to takto pri tebe naozaj správa, hlavne bod 1 (samotná oprava) — to je priorita.

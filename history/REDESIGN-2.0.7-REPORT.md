# TIQR Manager 2.0.7 — Oprava: Sync now padal na bežných číslach

Ďalší malý, cielený report. Reaguje na presne to, čo si poslal - chybu hneď pri prvom "Sync now" na skutočnej tabuľke.

## 1. Čo bolo zle

Chyba, čo si videl (`invalid type: integer 46291, expected a string`), mala jednu konkrétnu príčinu: appka si od Google Sheets pýta bunky v režime, kde Google posiela **skutočný typ** toho, čo je v bunke - číslo ako JSON číslo, nie ako text. Keď si do stĺpca "event date" napísal skutočný dátum (nie text "25.09.2026", ale ozajstný dátum, ktorý Sheets pozná ako dátum), Google ho poslal späť ako svoje interné poradové číslo dňa (46291), nie ako text. To isté sa deje pri akomkoľvek čísle napísanom ako číslo - "Ks", "Section", "Row", "Price". Appka predtým čakala, že úplne každá bunka bude text, a keď dostala čokoľvek iné, celý sync spadol naraz - aj keby bol problém len v jednej jedinej bunke.

Toto je dôležité: nebol to okrajový prípad. Presne takto väčšina ľudí prirodzene píše do tabuľky - dátum ako dátum, číslo ako číslo. Sync now bol touto chybou v podstate nepoužiteľný pre bežne vyplnenú tabuľku.

## 2. Čo som opravil

Dve súvisiace opravy:

1. **`google_sheets.rs`** - appka teraz prijme z Google akýkoľvek typ bunky (text, číslo, áno/nie), a **sama** si ho hneď premení na presný text - čísla presne také, aké boli napísané (žiadne zaokrúhľovacie chyby desatinných čísel, čo je dôležité najmä pri "Price"), nie cez žiadne priblížené desatinné číslo. Toto je ten istý princíp, čo appka už dodržiava všade inde (peniaze, CSV import) - nikdy neveriť tomu, ako niečo "vyzerá", vždy si to overiť/spracovať sama.
2. **`pulls_sheet_sync.rs`** - stĺpec "event date" teraz vie rozpoznať aj to poradové číslo dňa (ako to 46291) a spoľahlivo ho premeniť na skutočný dátum, popri doterajšom rozpoznávaní textu v tvare DD.MM.RRRR.

Nič iné sa nezmenilo - žiadna finančná logika (`money.rs`, prepočet cien) sa neupravovala, len to, čo appka dostane predtým, než čokoľvek počíta.

## 3. Čo teraz spraviť

Spusti `1-CLICK-UPDATE.bat` (teraz na v2.0.7), počkaj na zelený build, a skús "Sync now" na tej istej tabuľke znova - presne ten istý riadok, čo predtým spadol, by teraz mal prejsť bez problémov.

## 4. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 276 passed, 0 failed (271 + 5 nových)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.7 build" v hlavičke)
```

5 nových testov: že appka správne premení každý typ Google bunky na text (vrátane toho, že desatinné číslo ostane presne také, aké bolo, nie s zaokrúhľovacou chybou), že sa dá zo skutočnej Google odpovede (presne tvar, aký si nám poslal v chybe) načítať bez pádu, že poradové číslo dňa 46291 sa správne premení na 26.9.2026 (over si dátum zápasu - ak sedí, je to dôkaz, že prevod funguje presne), že úplne nezmyselné malé číslo v dátume appka odmietne namiesto tichého prijatia zlého dátumu, a jeden test, čo presne napodobňuje tvoj riadok (sojky/England vs Spain) od začiatku do konca.

## 5. Zmenené súbory a verzia

**Zmenené:** `google_sheets.rs` (Sheets bunky sa teraz prijmú v akomkoľvek type), `commands/pulls_sheet_sync.rs` (dátum ako poradové číslo dňa)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.7`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.7 hotové a overené (276/276 backend testov vrátane 5 nových presne na tento prípad, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a skús Sync now znova na tej istej tabuľke - napíš mi, či prešiel aj so zvyškom riadkov, čo si dovtedy stihol vyplniť.

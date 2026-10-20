# TIQR Manager 2.0.14 — Sheet/tab name sa už nezadáva ručne, appka ho sama zistí

Mal si pravdu, že to takto nefungovalo. 2.0.13 ti síce už presne povedala, aký názov tabu má byť správny - ale stále si ho musel sám prepísať do políčka, a presne to bol ten krok, čo nefungoval. Takže v 2.0.14 som zmenil samotný princíp: appka sa teraz na skutočné názvy tabov spýta Google priamo, a ty si len vyberieš z ponuky namiesto písania.

## 1. Ako to teraz funguje

Keď vyplníš pole **"Spreadsheet URL or ID"** a klikneš/tabneš mimo neho (teda pole stratí focus), appka na pozadí hneď zistí, aké taby tá tabuľka naozaj má - presne tou istou cestou, akou si to predtým len vypisovala do chybovej hlášky. Ak sa to podarí, pole **"Sheet/tab name"** sa samo zmení z písacieho poľa na **rozbaľovací zoznam (dropdown)** so skutočnými názvami tabov tej tabuľky - už nemáš ako preklepnúť alebo si pomýliť názov súboru s názvom tabu, lebo si vyberáš z toho, čo appka reálne našla.

To isté sa deje aj automaticky, keď otvoríš Settings a nejaké pripojenie už existuje - appka hneď na pozadí over, aký tab je uložený, a:
- ak je uložený názov skutočne medzi reálnymi tabmi, necháva ho tak, ako bol,
- ak uložený názov medzi reálnymi tabmi NIE JE (presne tvoj prípad - "TIQR Manager - Pulls" namiesto "Pulls"), appka **rovno predvyplní prvý skutočný tab** namiesto toho zlého názvu. Stačí potom kliknúť Save a je to opravené - žiadne prepisovanie.

Ak appka taby zistiť nevie (tabuľka ešte nie je zdieľaná, adresa/URL ešte nie je celá zadaná, a podobne), pole ostáva presne také, ako doteraz - obyčajné písacie pole - a pod ním appka napíše prečo (napr. že to ešte nevyzerá ako platná adresa, alebo že k tabuľke zatiaľ nemá prístup). Pod dropdownom je aj malý odkaz **"Type it in manually instead"** pre prípad, že by si chcel/potreboval napísať názov ručne (napr. tab, ktorý ešte len plánuješ vytvoriť) - appka ťa teda nikdy nenechá v slepej uličke.

## 2. Prečo je to takto spoľahlivejšie

Predtým appka len OPISOVALA problém ("skontroluj názov tabu, môže byť iný ako názov súboru") a neskôr aj priamo VYPÍSALA správnu odpoveď do textu chyby - ale oba prístupy stále vyžadovali, aby si niečo ručne prepisoval do poľa, čo je presne krok, kde to podľa teba prestávalo fungovať. Teraz appka namiesto opisovania alebo vypisovania rovno **zmení, čo to pole je** - z voľného textu na výber z reálnych možností - takže preklep alebo zámena "názov súboru vs. názov tabu" už nie je možná tam, kde appka vie taby zistiť.

## 3. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 345 passed, 0 failed (343 + 2 nové)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.14 build" v hlavičke)
```

Nové testy pokrývajú, že zisťovanie tabov vždy vráti zrozumiteľný dôvod namiesto pádu appky, keď adresa/ID ešte nie je platné alebo appka nemá k tabuľke prístup (rovnaký princíp ako doteraz pri "Test connection").

## 4. Zmenené súbory

**Zmenené:** `src-tauri/src/models.rs` (nový typ výsledku zisťovania tabov), `src-tauri/src/commands/sheets_sync.rs` (nový príkaz `detect_spreadsheet_tabs`, znovu použil existujúcu funkciu na zistenie skutočných názvov tabov z 2.0.13), `src-tauri/src/lib.rs` (registrácia nového príkazu), `src/lib/types.ts`, `src/lib/api.ts` (zrkadlenie nového typu/príkazu do frontendu), `src/pages/Settings.tsx` (pole "Sheet/tab name" sa teraz vie samé zmeniť na rozbaľovací zoznam)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.14`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.14 hotové a overené (345/345 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a vyskúšaj presne to, čo predtým nefungovalo:

1. Choď do **Settings -> Integrations**, na Pulls alebo Orders & Sales tam, kde už máš ručne pripojenú tabuľku so zlým názvom tabu.
2. Bez toho, aby si čokoľvek prepisoval, over, či sa pole "Sheet/tab name" samo zmenilo na zoznam so skutočnými názvami (napr. "Pulls"/"Orders") a či je už rovno vybraný ten správny.
3. Klikni **Save** a skontroluj, že tentoraz prejde aj samotný test pripojenia (žiadna chyba "Unable to parse range").
4. Skús to aj nanovo - vlož URL úplne novej/inej tabuľky do "Spreadsheet URL or ID", klikni mimo poľa a over, že sa dropdown "Sheet/tab name" sám naplní tabmi tej novej tabuľky.

Napíš mi presne, čo uvidíš - či sa taby zobrazili správne, či sa to už dá uložiť bez chyby, a ak niečo z toho stále robí problém, pošli mi presné znenie toho, čo appka ukáže.

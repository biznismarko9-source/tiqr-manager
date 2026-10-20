# TIQR Manager 2.0.12 — 5 opráv z tvojho testovania

Postupne všetkých päť vecí, čo si napísal.

## 1. "Sign in with Google" mrzlo, keď si sa vrátil bez dokončenia

Keď si klikol "Sign in with Google" a potom sa chcel vrátiť bez dokončenia (zavrieť tú kartu v prehliadači, alebo vybrať iný účet a nedokončiť to), appka čakala **až 5 minút**, kým to sama vzdá - a počas toho nebolo v appke ako sa z toho dostať von, čo pôsobilo ako zamrznutie a nútilo ťa appku reštartovať.

Pridal som skutočné tlačidlo **"Cancel"**, ktoré sa objaví presne počas tohto čakania (vedľa "Waiting for you to finish in your browser..."). Klikneš naň a appka sa spamäti vráti do normálu do cca 0,2 sekundy - žiadny reštart. Otestoval som to aj automatizovaným testom, ktorý dokazuje, že zrušenie appku uvoľní behom 2 sekúnd namiesto plných 300 sekúnd.

## 2. Premenované "Orders & Tickets" -> "Orders & Sales"

Karta v Settings -> Integrations sa teraz volá **"Orders & Sales"**. Nič iné sa nemení - rovnaké pripojenie, rovnaký hárok, rovnaké tlačidlá.

## 3. Currency - vysvetlenie (bez zmeny funkčnosti, presne ako si chcel)

Toto už appka v skutočnosti robí: pri Orders sync appka **vždy prednostne číta menu priamo z riadku** (stĺpec `currency` - EUR/USD/GBP), ak je vyplnený. Dropdown "Currency" v appke je len **záloha pre riadok, kde je táto bunka prázdna** - nie hlavný ovládač. Pri Pulls to inak byť nemôže, lebo tvoj Pulls hárok stĺpec s menou vôbec nemá.

Podľa tvojej voľby som nechal funkčnosť presne ako je, len som prepísal text pod poľom "Currency" na karte Orders & Sales, aby bolo jasnejšie, že ide len o zálohu, nie o hlavné nastavenie.

## 4. Email (used) - už bez "Email used:" na začiatku

Order sync teraz zapíše do poznámky objednávky presne to, čo je v bunke stĺpca "Email (used)" - napr. `sikmy@gmail.com` - bez pridávania "Email used: " na začiatok.

## 5. "Unable to parse range" pri ručne pripojenom hárku

Toto bola najväčšia vec, tak vysvetlím podrobnejšie, čo sa deje a čo som opravil.

**Čo sa reálne deje:** Tlačidlo "Save"/"Connect" doteraz kontrolovalo len to, či zadaný text *vyzerá* ako platné URL/ID a či je vyplnené meno hárku/tabu - **vôbec sa nespojilo s Google**, takže vždy nahlásilo úspech, aj keby zadaný názov tabu v skutočnosti v tabuľke neexistoval. Chyba "Unable to parse range" sa potom objavila až neskôr, pri Sync/Test - matúce, lebo Save predtým povedalo, že je všetko OK.

**Prečo sa to najčastejšie stáva:** Google touto istou hláškou hlási dve úplne rôzne veci - naozaj zle zapísaný rozsah (to už appka rieši od verzie 2.0.9 - úvodzovky okolo mena tabu) AJ prípad, keď meno tabu proste **v danej tabuľke neexistuje**. To druhé sa ľahko stane, lebo "sheet" je v Google zavádzajúce - môže znamenať celý súbor (čo vidíš ako názov v Google Drive) alebo konkrétny tab/kartu naspodku tabuľky. Ak appka vytvorila hárok sama, tab sa vždy volá jednoducho "Pulls"/"Orders" - ale pri ručne pripojenom hárku je veľmi ľahké omylom napísať do poľa "Sheet/tab name" názov **súboru** namiesto názvu **tabu**.

**Čo som opravil:**
- **Save teraz hneď spustí to isté overenie, čo robí "Test connection"** - takže ak je niečo zle, zistíš to okamžite pri uložení, nie až pri ďalšom kroku.
- Pole "Sheet/tab name" má teraz vysvetľujúci text priamo pod sebou (že ide o tab naspodku hárku, nie o názov súboru).
- Samotná chybová hláška "Unable to parse range" teraz dostane doplnenie: že najčastejšie ide o to, že tab s daným menom v tabuľke neexistuje, a nech skontroluješ presné meno naspodku hárku.

**Čo urob ty:** Keď appku zaktualizuješ, skús znova pripojiť ten ručne zadaný hárok (Pulls aj Orders & Sales) - teraz uvidíš hneď pri "Save", či to naozaj funguje. Ak nahlási chybu, skontroluj presné meno tabu naspodku daného Google Sheets súboru (klikni na kartu úplne dole) a priraď presne to isté meno (vrátane veľkých/malých písmen a medzier) do poľa "Sheet/tab name" v appke.

## 6. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 338 passed, 0 failed (330 + 8 nových/upravených)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.12 build" v hlavičke)
```

8 nových/upravených testov: že zrušenie čakania na Google prihlásenie skutočne preruší aj 300-sekundové čakanie do 2 sekúnd (nie len teoreticky), že zrušenie mimo prebiehajúceho prihlásenia nič nerobí (bezpečný no-op) a nedotkne sa cudzieho pokusu o prihlásenie, že chybová hláška "Unable to parse range" dostane vysvetľujúci dodatok a iná chyba (napr. bez oprávnenia) ho nedostane, a upravený test na "Email (used)" bez prefixu.

## 7. Zmenené súbory

**Zmenené (backend):** `src-tauri/src/db.rs` (nové pole pre zrušenie prihlásenia), `src-tauri/src/lib.rs` (registrácia nového príkazu), `src-tauri/src/google_oauth.rs` (zrušiteľné čakanie na Google redirect), `src-tauri/src/commands/google_auth.rs` (nový príkaz `cancel_google_sign_in`), `src-tauri/src/google_sheets.rs` (jasnejšia chybová hláška), `src-tauri/src/commands/orders_sheet_sync.rs` (Email bez prefixu, premenované komentáre)
**Zmenené (frontend):** `src/lib/api.ts` (nová `cancelGoogleSignIn`), `src/pages/Settings.tsx` (tlačidlo Cancel, premenovaná karta, Save teraz hneď testuje, nové vysvetľujúce texty)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.12`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.12 hotové a overené (338/338 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat`. Najdôležitejšie na vyskúšanie: skús znova "Save" na ručne pripojenom hárku (bod 5) a napíš mi presne, čo teraz appka nahlási - ak to bude sťažovať sa na meno tabu, over si ho naspodku daného Google Sheets súboru a priraď presne to isté meno do appky. A skús aj "Sign in with Google" + "Cancel" (bod 1), nech vidím, že sa appka naozaj hneď vráti do normálu.

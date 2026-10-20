# TIQR Manager 2.0.15 — chybové hlášky pri Google Sheets sú teraz stručné

Presne ako si chcel - chyby na kartách Pulls a Orders & Sales (Test connection, Save, aj nové zisťovanie tabov z 2.0.14) už nevypisujú surový JSON od Googlu. Namiesto toho appka teraz ukáže krátky, jasný riadok a pod ním jeden konkrétny krok, čo s tým urobiť.

## 1. Ako to teraz vyzerá

Predtým (napr. pri nezdieľanej tabuľke):

> Google Sheets rejected the request (403 Forbidden): { "error": { "code": 403, "message": "The caller does not have permission", "status": "PERMISSION_DENIED" } } - this usually means the Google identity this request used does not have access to that spreadsheet yet. Check Settings -> Integrations for the exact e-mail to share it with (it differs depending on whether you're signed in with your own Google account or using the app's shared one), then share the spreadsheet with that address (Editor access) in Google Sheets itself.

Teraz:

> **Can't access this spreadsheet yet.**
> Share it with tiqr-sync@tiqr-manager-sync.iam.gserviceaccount.com (Editor access) in Google Sheets, or sign in with Google above since it's your own sheet.

Rovnaký princíp platí aj pre chýbajúci/zlý názov tabu (keď appka náhodou nevie taby zistiť automaticky cez 2.0.14 dropdown) - krátka hláška hore, pod ňou presný zoznam skutočných tabov.

## 2. Jedna dôležitá vec, ktorú appka teraz rozlišuje

Keď si **prihlásený cez Google** a napriek tomu appka dostane "nemám prístup", appka ti už nepovie "prihlás sa" (to by nedávalo zmysel, veď už si prihlásený) - namiesto toho ti povie, že tvoje prihlásenie je v poriadku, len samotná tabuľka nie je zdieľaná s tým konkrétnym Google účtom, čo je práve prihlásený (alebo je prihlásený iný účet, než ktorému tabuľka patrí). Keď prihlásený nie si, appka ti rovno napíše presnú emailovú adresu appkinho účtu na zdieľanie - žiadne "pozri do Settings", rovno tú adresu v texte chyby.

Pre chybu, ktorú appka nevie rozpoznať (niečo iné než tieto dva bežné prípady), necháva pôvodnú plnú hlášku - tam je totiž lepšie vidieť presne, čo sa deje, než dostať krátky, ale nič nehovoriaci text.

## 3. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 348 passed, 0 failed (345 + 5 nových/upravených, 2 staré zrušené)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.15 build" v hlavičke)
```

Nové testy okrem iného priamo overujú, že appka nikdy neponúkne prihlásenému človeku kruhovú radu "prihlás sa", a že skrátená hláška nikdy neobsahuje zvyšky surového JSON (`{`).

## 4. Zmenené súbory

**Zmenené:** `src-tauri/src/models.rs` (výsledky testu/detekcie majú teraz aj pole `hint` popri krátkej `message`), `src-tauri/src/commands/sheets_sync.rs` (nová logika na krátke, konkrétne hlášky pre "chýba prístup" a "zlý názov tabu"), `src/lib/types.ts`, `src/pages/Settings.tsx` (zobrazenie chyby ako dvoch riadkov - krátka hláška hore, konkrétny krok pod ňou)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.15`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.15 hotové a overené (348/348 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a skús presne to, čo si mi poslal predtým - Test connection alebo Save na nezdieľanej tabuľke - a over, že namiesto dlhého bloku s JSON teraz uvidíš krátky riadok a pod ním jednu konkrétnu vetu s adresou na zdieľanie.

Napíš mi, či to takto vyzerá dobre, alebo by si to chcel ešte inak.

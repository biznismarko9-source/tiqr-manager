# TIQR Manager 2.0.13 — prečo 2.0.12 nepomohlo, a skutočná oprava

Máš pravdu, že to nefungovalo. Obe veci som teraz opravil poriadne - nižšie vysvetľujem presne prečo to predtým nezabralo, nech to nie je len "skús znova a dúfaj".

## 1. "Sign in with Google" - skutočná príčina, a prečo Cancel tlačidlo nič nerobilo

Zistil som, prečo tlačidlo Cancel z 2.0.12 nepomohlo - a je to jednoduché vysvetlenie: appka (presnejšie Tauri, na ktorom appka beží) spúšťa bežné (nie "async") príkazy **na tom istom vlákne, čo obsluhuje celé okno appky**. To znamená, že predchádzajúca verzia "Sign in with Google" doslova **zablokovala celé okno appky** na tých 5 minút - a keďže to isté vlákno má na starosti aj spracovanie KAŽDÉHO iného kliknutia vrátane "Cancel", appka sa tvoje kliknutie na Cancel ani nemala šancu dozvedieť, kým by neuplynulo tých 5 minút samo od seba. Presne preto to vyzeralo ako zamrznutie - lebo appka bola v tú chvíľu doslova zamrznutá, nie len ten jeden dialóg.

**Skutočná oprava:** Prerobil som "Sign in with Google" tak, aby bežalo na **vlastnom, oddelenom vlákne** - appka aj počas čakania na teba zostáva úplne živá a reaguje na všetko ostatné, vrátane Cancel tlačidla. Toto som si tentokrát aj overil priamo v oficiálnej dokumentácii Tauri (nie len predpokladal), takže si touto opravou som si podstatne istejší než minule.

## 2. "Unable to parse range" - appka ti teraz priamo povie správny názov tabu

Predtým appka len hádala, PREČO to zlyháva ("asi to bude zlý názov tabu") - čo ti ale nepovedalo, aký názov je ten správny. Teraz appka pri neúspešnom teste/save **priamo zistí a vypíše skutočné názvy všetkých tabov v tej tabuľke** - takže namiesto hádania uvidíš presne, čo napísať do poľa "Sheet/tab name". Príklad, ako bude vyzerať chyba teraz:

> ...Unable to parse range: 'TIQR Manager - Orders'!A1:A1 ... **The tabs that actually exist in this spreadsheet are: "Objednávky", "Sheet1". Update "Sheet/tab name" to match one of these exactly.**

Skús to znova (Save aj na Pulls aj na Orders & Sales) a mal by si teraz hneď vidieť presný zoznam - stačí ten názov skopírovať do poľa.

## 3. Chyba 403 "The caller does not have permission" - vysvetlenie

Toto je iná vec ako body 1-2 - nie chyba appky, ale znamená, že daná tabuľka **nie je zdieľaná s tým Google účtom, ktorý appka práve používa**. Appka teraz k tejto chybe pridá vysvetlenie priamo v hláške. Dve možnosti, ako to vyriešiť:

- **Najjednoduchšie:** zostaň prihlásený cez "Sign in with Google" (hore na tej istej stránke) - keďže tabuľky sú tvoje vlastné, appka ich vtedy vidí rovno, bez potreby čokoľvek zdieľať.
- Alebo, ak chceš appku používať bez prihlásenia: v Google Sheets klikni "Share" na danej tabuľke a pridaj presne tú emailovú adresu, čo appka ukazuje v Settings -> Integrations pod poľami (Editor prístup).

## 4. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 343 passed, 0 failed (338 + 5 nových)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.13 build" v hlavičke)
```

Nový test priamo dokazuje, že zrušenie prihlásenia funguje aj pri behu appky ako celku (nielen v izolácii) - a ďalšie testy pokrývajú, že sa reálne názvy tabov pridajú do chyby len vtedy, keď to dáva zmysel (nie pri inej chybe, napr. 403), a že chýbajúce/prázdne názvy sa nikdy nepridajú ako prázdny/matúci text.

## 5. Zmenené súbory

**Zmenené:** `src-tauri/src/commands/google_auth.rs` (Sign in with Google teraz skutočne beží mimo hlavného vlákna appky), `src-tauri/src/google_sheets.rs` (nová funkcia na zistenie skutočných názvov tabov, jasnejšia 403 hláška), `src-tauri/src/commands/sheets_sync.rs` (Test connection teraz pridáva skutočné názvy tabov do chyby)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.13`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.13 hotové a overené (343/343 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a vyskúšaj oboje:

1. **Sign in with Google** - klikni, a kým to čaká na teba, skús kliknúť kdekoľvek inde v appke (napr. prepnúť stránku) - appka musí zostať plne použiteľná, nie len tlačidlo Cancel. Potom vyskúšaj aj samotné Cancel.
2. **Save na Pulls aj Orders & Sales** (ručne pripojené hárky) - napíš mi, či teraz appka vypísala skutočné názvy tabov, a či to zodpovedá tomu, čo v tabuľke naozaj máš.

Napíš mi presne, čo uvidíš pri oboch - ak sa niečo z toho ešte stále správa zle, pošli mi presné znenie hlášky.

# TIQR Manager 2.0.30 — Pulls: zmestí sa to celé, bez posúvania

## Čo je nové

Presne to, čo si chcel: Pulls (Given aj Received) sa teraz zmestí do okna appky **bez vodorovného posúvania**, aj s celým Date/Platform/Warning viditeľným. 2.0.29 tie tri stĺpce opravilo, ale za cenu posuvníka - toto je druhé kolo, ktoré miesto namiesto pridávania miesto naopak získava späť:

- **Date** je teraz kratší formát ("13 Aug 26" namiesto "Aug 13, 2026") - stále celý dátum, žiadna skratka informácie, len kratší zápis. Presne ten istý formát appka už roky používa v Sales. Po prejdení myšou nad dátumom uvidíš aj ten dlhší tvar.
- Ostatné stĺpce (Pull, For, More info, Fee, Warning) som zmenšil presne na toľko, koľko reálne potrebujú - nie odhadom, ale odmeraním skutočnej šírky v prehliadači s tvojimi vlastnými dátami ("fnac spetacles", "Slovakia vbs Spain" a podobne).
- **Platform** ostal rovnako široký ako v 2.0.29 - tvoje reálne dáta ("fnac spetacles") to potrebujú, je to dlhšie než "Ticketmaster", na čo som to pôvodne počítal.
- Postranné menu vľavo (Dashboard/Events/.../Settings) je teraz užšie, presne ako si navrhol - o čosi užšie, ale stále je v ňom všetko pekne čitateľné, a získava sa tým miesto navyše na každej stránke, nielen v Pulls.

Jediné miesto, kde je stále vidieť drobné orezanie, je **Seats** pri dlhších viacciferných číslach sedadiel (napr. "Seat 46082") - toto nebolo súčasťou toho, čo si nahlásil, má to stále aj tak náhľad po prejdení myšou, a rozšíriť to by znamenalo vziať späť miesto, čo som práve získal inde. Ak by ti to prekážalo, napíš mi.

## Ako presne to funguje pod kapotou

Toto som tentoraz overoval inak, než len vizuálne: dočasný Playwright skript nešiel len odfotiť obrazovku, ale priamo si v prehliadači odmeral, či sa tabuľka vojde do okna bez posúvania, a to pri viacerých bežných šírkach okna (1366, 1440, 1536, 1600, 1920px) - nie len na jednej náhodnej veľkosti. Zistil som pri tom, že veľa priestoru sa dá získať späť bez straty čitateľnosti - napríklad stĺpec "More info" mal dosť voľného miesta navyše, čo som mu ubral.

Frontendová zmena, dva súbory (`src/pages/Pulls.tsx`, `src/components/Layout.tsx`) - žiadna zmena v databáze, žiadna zmena v Rust kóde.

## Testy a build

```
cargo test --lib -> 491 passed, 0 failed, 3 ignored (bez zmeny - táto oprava sa Rust kódu vôbec netýka)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.30 build" v hlavičke)
```

Vizuálne aj číselne overené cez dočasný Playwright preview harness (mimo appky, po overení zmazaný) - obe záložky, svetlý aj tmavý režim, s tvojimi vlastnými reálnymi dátami zo screenshotu (soijak / Slovakia vbs Spain / fnac spetacles) aj s dlhšími testovacími hodnotami, pri piatich rôznych bežných šírkach okna, plus že výber pri hromadnom mazaní (z 2.0.28) stále vyzerá správne.

## Zmenené súbory

**Frontend:**
- `src/pages/Pulls.tsx` - kompaktnejší formát dátumu, zmenšené stĺpce Pull/For/Seats/More info/Fee/Warning, nový `min-width` prepočítaný na novú šírku
- `src/components/Layout.tsx` - užšie postranné menu (`w-56` → `w-48`)

**Verzia (7 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` - všetkých 7 na `2.0.30`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.30 hotové a overené (491/491 testov, čisté `tsc`/`build`, overené aj vizuálne aj odmeraním v prehliadači). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. Pulls (obe záložky) - na tvojom bežnom okne appky by sa už teraz nemal objaviť žiadny vodorovný posuvník, a Date/Platform/Warning by mali byť celé vidno.
2. Ak by sa ti aj tak niekde objavil posuvník (napr. veľmi úzke okno), napíš mi presne akú šírku okna appka má - viem to doladiť ešte presnejšie.
3. Skontroluj, či ti nový formát dátumu ("13 Aug 26") vyhovuje - ak by si radšej videl späť dlhší tvar, aj za cenu širšieho stĺpca, napíš mi.
4. Skontroluj, či ti užšie postranné menu vľavo vyhovuje na všetkých stránkach, nielen v Pulls.

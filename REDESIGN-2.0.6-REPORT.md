# TIQR Manager 2.0.6 — Oprava: CI build nezobral nové OAuth secrety

Malý, cielený report. Reaguje na tvoj screenshot ("Google sign-in isn't available in this build") potom, čo si už vybuildil 2.0.5. Príčina nebola v appke - bola v tom, čo appku buildí na GitHub Actions.

## 1. Čo bolo zle

Appka od 2.0.5 vie čítať `GOOGLE_OAUTH_CLIENT_ID`/`GOOGLE_OAUTH_CLIENT_SECRET` (presne ako `GOOGLE_SERVICE_ACCOUNT_JSON` od 2.0.2) - ale len vtedy, keď jej ich build na GitHub Actions naozaj **pošle**. Súbor, ktorý riadi ten build (`.github/workflows/build-windows.yml`), som pri 2.0.5 nechal odovzdávať `GOOGLE_SERVICE_ACCOUNT_JSON`, ale zabudol som doňho pridať odovzdanie tých dvoch nových. Výsledok: aj keby si presne podľa reportu k 2.0.5 pridal `GOOGLE_OAUTH_CLIENT_ID` a `GOOGLE_OAUTH_CLIENT_SECRET` do GitHub secrets, build by ich aj tak ignoroval - appka by ich nikdy nedostala a karta "Sign in with Google" by navždy hlásila "not available", presne ako na tvojom screenshote. Ospravedlňujem sa za tento nedopatrenie - mal to obsahovať už 2.0.5.

## 2. Čo som opravil

`.github/workflows/build-windows.yml`: obe miesta, kde appku appka reálne buildí (rýchly testovací build aj podpísaný release), teraz popri `GOOGLE_SERVICE_ACCOUNT_JSON` odovzdávajú aj `GOOGLE_OAUTH_CLIENT_ID` a `GOOGLE_OAUTH_CLIENT_SECRET`. Nič iné sa nemenilo - appka (Rust ani frontend kód) je presne taká istá ako v 2.0.5, len teraz build skutočne dostane to, čo mu ty dáš do secrets.

## 3. Čo teraz spraviť

Toto **nepotrebuje žiadny nový krok navyše** oproti tomu, čo už robíš - presne tie dva secrety, čo si mi poslal Client ID a Client Secret, len ich teraz pridaj do GitHub (ak si to ešte nestihol):

GitHub → repozitár `tiqr-manager` → Settings → Secrets and variables → Actions → New repository secret, dvakrát: meno `GOOGLE_OAUTH_CLIENT_ID` a meno `GOOGLE_OAUTH_CLIENT_SECRET`, hodnoty presne tie, čo si mi poslal v chate.

Potom spusti `1-CLICK-UPDATE.bat` (teraz publikuje v2.0.6, nie v2.0.5 - dôležité, lebo starý v2.0.5 build by mal stále tú istú chýbajúcu opravu). Po zelenom builde skús "Sign in with Google" znova - mal by sa už naozaj otvoriť Google prihlasovací formulár. Nezabudni, že sa vieš prihlásiť len tým e-mailom, ktorý si pridal do Test users v Cloud Console (report 2.0.5, sekcia 3, krok 2) - iný e-mail Google zatiaľ odmietne, kým appka neprejde schválením.

## 4. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 271 passed, 0 failed (nezmenené - žiadny Rust kód sa nemenil)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.6 build" v hlavičke)
```

Naviac tento krát: opravený `.github/workflows/build-windows.yml` som overil aj ako platný YAML súbor (načíta sa bez chyby) - keďže je to jediný zmenený súbor a nespadá pod `cargo`/`tsc`, chcel som mať istotu, že v ňom nie je preklep v odsadení, ktorý by celý build na GitHub Actions pokazil úplne inak, než je tento report.

## 5. Zmenené súbory a verzia

**Zmenené:** `.github/workflows/build-windows.yml` (odovzdanie dvoch nových secrets do oboch build krokov + aktualizovaný komentár na začiatku súboru)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.6`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.6 hotové a overené (271/271 backend testov - nezmenené, čisté `tsc`/`build`, YAML overený, že sa dá načítať). Toto je čisto oprava CI buildu, appka samotná sa nezmenila.

**Ďalší krok je na tebe:** pridaj oba secrety (`GOOGLE_OAUTH_CLIENT_ID`, `GOOGLE_OAUTH_CLIENT_SECRET`) do GitHub, spusti `1-CLICK-UPDATE.bat`, a skús sa naozaj prihlásiť. Daj mi vedieť, ako to dopadlo - ideálne či sa ti otvorilo Google okno, či prihlásenie prešlo, a či sa appka potom správne ukázala ako "Signed in as ...".

# TIQR Manager 2.1.1 — PRICE CHECKER: AUTO-CHECK

Toto je doplnok k Price Checkeru z 2.0.81/2.0.82 (manuálne zadávanie + "paste from listings page"). Poslal si
mi reálny kód (`tiqr-manager-2_1_0.zip`) s pokynom nič nemeniť a len toto pridať. Postavil som presne na
existujúcom kóde — žiadna nová appka, žiadny nový systém, len jedno nové tlačidlo vedľa "Check Prices".

## Audit existujúceho Price Checkera (odkiaľ som staval)

Pred napísaním čo i len riadku som si prešiel `commands/price_checker.rs`, `migrations/014_price_checker.sql`,
`models.rs`, `PriceChecker.tsx` a `priceParse.ts`. Kľúčové zistenie, ktoré určilo celý dizajn: Price Checker
je **zámerne manuálny/paste-based** — a 2.0.82 už rieši presne ten problém, ktorý som ja predtým riešil
zložito: `extractPricesFromText` vytiahne ceny z textu, ktorý si sám skopíruješ zo stránky. Toto pravidlo
("žiadne API, žiadny bypass") som nemenil — Auto-check ho neobchádza, len automatizuje ten JEDEN krok, ktorý
si predtým robil ty sám: otvorenie uloženého linku a prečítanie cien z neho.

## Čo Auto-check robí

Vedľa "Check Prices" pribudlo tlačidlo **"Auto-check"**. Po kliknutí appka otvorí uloženú (alebo práve
zadanú, ešte neuloženú) URL vo vlastnom skrytom WebView okne — presne ten istý engine, akým appka kreslí
svoje vlastné UI (WebView2/WKWebView/WebKitGTK podľa OS), nič sa nesťahuje ani nebalí navyše. Počká (nie
pevný `sleep`, ale opakovaný test každých 400ms až 9 sekúnd — "je už niečo na extrahovanie?"), prečíta ceny
tromi spôsobmi (JSON-LD štruktúrované dáta, HTML tabuľka v tvare Section/Row/Price, `og:price:amount` meta
tag — funguje bez ohľadu na to, ktorý marketplace to je, keďže `marketplaces` je tvoj vlastný spravovaný
zoznam) a výsledok **pošle do toho istého paste-boxu**, aký už poznáš z 2.0.82. Nič sa neuloží samo — otvorí
sa presne ten istý formulár ako pri "Check Prices", len predvyplnený, s tou istou hláškou ("Found 5 prices in
USD — filled in below, double-check before saving"), a ty klikneš Save presne ako doteraz.

Ak stránka vráti anti-bot/verification výzvu, appka to **nerieši ani neobchádza** — vráti `blocked` a
formulár sa otvorí prázdny, presne ako pri obyčajnom "Check Prices". Rovnako ak sa nič nenájde
(`unable_to_read`) alebo nastane technická chyba — vždy skončíš presne tam, kde si dnes: pri manuálnom
zadaní/paste, nič viac.

## Čo som reálne zistil o StubHub/Vivid Seats/Ticombo (prieskum, nie odhad)

- **StubHub**: reálne stiahnutá event stránka (dnešný dátum, živý zápas) neobsahuje žiadne extrahovateľné
  ceny bez spustenia JavaScriptu na strane klienta. Auto-check tu realisticky skončí ako `unable_to_read`.
- **Vivid Seats**: prekvapivo — reálna, živá event stránka obsahuje HTML tabuľku Section/Row/Price priamo,
  bez potreby JS. Presne túto štruktúru extrakcia cieli.
- **Ticombo**: `og:price:amount`/`og:price:currency` meta tagy existujú ako vzor v hlavičke stránky, ale
  nepodarilo sa mi nájsť aktuálnu (neexpirovanú) event stránku na priame overenie vyplnenej hodnoty — táto
  cesta je najmenej overená z troch.

## Čo som ZÁMERNE nezmenil (žiadna migrácia, žiadna nová tabuľka)

Zvažoval som pridať `source` stĺpec do `price_checks` (automatic vs. manual), ale rozhodol som sa proti tomu:
Auto-check je z pohľadu appky "urob za mňa ten copy-paste", nie nový druh záznamu. Výsledok sa ukladá presne
cez existujúci, nezmenený `save_price_check` — po tvojej kontrole a kliknutí Save. Žiadna migrácia,
`price_checks`/`event_marketplace_links`/`marketplaces` schéma bez zmeny, `save_price_check_impl` aj
`get_price_checker_summary_impl` bez zmeny, všetky existujúce testy v `price_checker.rs` bez úpravy.

## Zmenené / nové súbory

**Backend (Rust) — nové súbory:**
- `src-tauri/src/commands/price_checker_auto.rs` (hlavná logika, WebView + polling + parsovanie)
- `src-tauri/src/commands/price_checker_auto_readiness.js`
- `src-tauri/src/commands/price_checker_auto_extract.js`

**Backend — upravené súbory (len pridané riadky, nič vymazané okrem 1 chyby, ktorú som si sám všimol a
opravil — pozri nižšie):**
- `src-tauri/src/models.rs` — pridaný `AutoCheckResult` typ
- `src-tauri/src/commands/mod.rs` — `pub mod price_checker_auto;`
- `src-tauri/src/lib.rs` — registrácia `auto_check_price` v `invoke_handler`

**Frontend — upravené súbory:**
- `src/lib/types.ts` — pridaný `AutoCheckResult` typ
- `src/lib/api.ts` — pridané `autoCheckPrice`
- `src/pages/PriceChecker.tsx` — nové tlačidlo, prepojenie na existujúci paste-pipeline

**Verzia:** 4 miesta (`package.json`, `package-lock.json`, `src-tauri/Cargo.toml`,
`src-tauri/tauri.conf.json`): `2.1.0` → `2.1.1`. Appka číta verziu dynamicky z Cargo.toml
(`app.package_info().version`), takže žiadne ďalšie miesto netreba.

**Kompletný diff je priložený ako `PRICE-CHECKER-AUTO-CHECK.diff`** — 10 súborov, presne toľko, koľko je
vyššie vymenované.

## Chyba, ktorú som spravil a opravil (transparentne)

Pri úprave `models.rs` som pri prvom pokuse omylom zmazal 2 riadky existujúceho komentára
(`RECOMMENDED_PRICE_UNDERCUT_PCT` vysvetlenie). Všimol som si to hneď pri kontrole a opravil späť na presné
pôvodné znenie — over si to v diffe: hunk pre `models.rs` ukazuje **len `+40 / -0`**, žiadne mínusy, čiže
finálny stav je čisto aditívny.

## Testy — čo reálne prebehlo

```
cd pc-auto-check-test && cargo test    -> 8 testov, 0 zlyhaní (izolovaná kópia parse_auto_check_json)
node run.js (jsdom)                     -> 5/5 scenárov (JSON-LD, Vivid Seats tabuľka, Ticombo meta,
                                            Cloudflare-style blocked, prázdna StubHub-like stránka)
node run-readiness.js (jsdom)           -> 3/3 scenáre
npx tsc -b                              -> 0 chýb (CELÝ projekt, nie len nové súbory)
npm run build                           -> OK ("tiqr-manager@2.1.1 build", vite build prešiel)
```

**Skutočný bug nájdený a opravený počas testovania:** prvá verzia JS extrakcie používala `.innerText`,
ktoré jsdom (a potenciálne aj niektoré WebView implementácie) nemusí spoľahlivo podporovať —
`Cannot read properties of undefined (reading 'toLowerCase')`. Prepísané na `.textContent` (vždy dostupné,
štandardné), potom všetky testy prešli.

## Čo REÁLNE prebehnúť nemohlo (rovnaké obmedzenie ako google_oauth.rs/google_sheets.rs)

`cargo check`/`cargo test` pre celý `src-tauri` projekt (vrátane môjho nového kódu) sa v tomto sandboxi
nedá spustiť — **potvrdené aj s vaším presným `Cargo.lock`** (nie len fresh resolve): `dlopen2_derive`
vyžaduje Rust 1.85+ (edition2024), sandbox má len 1.75, `rustup` je mimo network allowlistu. Toto je presne
tá istá kategória obmedzenia, akú si `google_oauth.rs`/`google_sheets.rs` už sami dokumentujú vo svojich
vlastných modulových komentároch (nedostupné `accounts.google.com`/`oauth2.googleapis.com` z rovnakého
sandboxu) — nie nový problém, rovnaký vzor. Váš skutočný build stroj (kde bežalo `cargo test --lib -> 836
testov`) toto obmedzenie mať nebude.

Živé spustenie skutočného WebView (skryté okno, `eval_with_callback` na živej StubHub/Vivid Seats stránke)
som preto nemohol overiť v TOMTO projekte. Čo bolo overené: rovnaký mechanizmus (WebKitGTK, ten istý engine
ako Tauri používa na Linuxe) v predošlej, izolovanej fáze tejto konverzácie — reálna navigácia, reálna
detekcia Cloudflare ochrany, reálny JS eval round-trip. API použité tu (`WebviewUrl::External`,
`eval_with_callback`, `tauri::Url`) je overené priamo cez aktuálnu dokumentáciu `tauri` 2.11.5 (rovnaká
verzia, akú má váš `Cargo.lock`), nie z pamäte.

## Ako to sám vyskúšať

1. `npm install && npm run build` — malo by prejsť bez chýb (u mňa prešlo).
2. `cd src-tauri && cargo check` — u vás (na stroji s Rust 1.85+) by malo prejsť; ak nie, pošli mi chybu a
   opravím.
3. V appke: Price Checker → vyber event → zadaj/ulož marketplace URL → klikni **Auto-check** namiesto
   Check Prices → sleduj, čo sa stane (podľa vyššie: Vivid Seats má šancu nájsť reálne ceny, StubHub
   pravdepodobne skončí "unable to read", formulár sa otvorí prázdny na doplnenie).
4. Skús aj URL, ktorá vôbec neexistuje/je nezmyselná — over, že sa appka nezasekne a vráti čistú chybu.

## Známe obmedzenia

- Ticombo extrakcia je najmenej overená (pozri vyššie).
- Vivid Seats môže vrátiť len výber najlepších ponúk, nie celý inventár (rovnaké obmedzenie ako keby si
  to sám kopíroval zo stránky bez scrollovania celého zoznamu).
- Auto-check môže trvať až ~10 sekúnd (polling do MAX_WAIT) — tlačidlo počas toho ukazuje spinner a je
  disabled, appka sa ale nezasekne (beží na vlastnom vlákne, nie hlavnom UI vlákne).
- Žiadny nový štýl/farba/komponent — použité výhradne existujúce `Button`/`Spinner`/`Card` z
  `components/ui`.

## Čo som NEmenil

Presne ako pri Finance 2.1: žiadna zmena `price_checker.rs`, `models.rs` (okrem pridania), migrácií (žiadna
nová), Sales/Orders/Tickets/Inventory, refund/resell, SaleGroup, `batch_id`, `finance.rs`, `money.rs`,
Backup/Restore, CSV import, Google Sheets sync. Tieto súbory som ani neotváral na úpravu.

## STOP

Auto-check hotový, otestovaný v rámci toho, čo tento sandbox dovolí, a zabalený. Skontroluj:

1. `cargo check` na svojom stroji (Rust 1.85+) — toto ja overiť nemôžem, ty áno.
2. V appke skús Auto-check na Vivid Seats evente, na ktorom vieš, že sú listingy — over, či sa formulár
   naplní.
3. Skús Auto-check na StubHub evente — over, či dostaneš čistú "unable to read" správu a prázdny formulár,
   nič rozbité.
4. Skontroluj `PRICE-CHECKER-AUTO-CHECK.diff` riadok po riadku, hlavne `models.rs` hunk (mal by byť
   čisto `+40/-0`).

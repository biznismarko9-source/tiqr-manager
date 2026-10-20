# TIQR Manager 2.0.1 — Revert Payments 2.0 (Payment Ledger)

Report k verzii **2.0.1**. Nadväzuje na 2.0.0 (Payments 2.0 / Payment Ledger) - po vyskúšaní si vlastník appky funkciu rozmyslel a požiadal o návrat k správaniu z 1.9.10. Toto NIE JE obyčajný `git revert` (appka tu nemá `.git`, dostávam len zdrojový zip) - je to ručne vykonaný, riadok-po-riadku overený revert presne tých zmien, ktoré 2.0.0 report (sekcia 17) vymenoval ako zmenené súbory. Číslo verzie sa počas tohto kola menilo dvakrát (najprv 1.9.11, potom 2.0.1) - dôvod je v sekcii 9, obe predchádzajúce chyby aj ich opravy sú zdokumentované nižšie, nič sa nezamietlo pod koberec.

## 1. Čo sa stalo a prečo to nie je len "vymazať priečinok"

Medzitým, čo appka bežala na 2.0.0, si mohol do nového Payment Ledgera (`payments` tabuľka, migrácia 007) zapísať **reálne platby**. Preto som pred akoukoľvek úpravou najprv:

- Prečítal `db.rs` migration runner - je **forward-only** (žiadne "down" migrácie, žiadny mechanizmus na spätné vymazanie migrácie zo zoznamu bez rizika).
- Spýtal sa ťa priamo, či už máš 2.0.0 nainštalovanú a či v nej máš reálne platby - potvrdil si, že áno.

Záver z toho: **migrácia 007 a tabuľka `payments` musia v appke zostať navždy** (presne tak, ako sa nikdy nemažú/neupravujú migrácie 001-006) - inak riskujem rozbitie tvojej reálnej, už nainštalovanej databázy pri ďalšom spustení appky. Revert je preto urobený **na úrovni aplikácie** (UI, commands, business logika), nie na úrovni schémy databázy.

**Tvoje reálne platby sú v bezpečí bez ohľadu na čokoľvek ďalšie v tomto reporte:** tabuľka `payments` a jej riadky sa touto verziou vôbec nedotýkajú - len ich appka od tejto verzie nikde nezobrazuje ani nezapisuje. Ak by si ich niekedy chcel/a mať aj mimo appky (napr. CSV kópiu pre vlastnú evidenciu), daj vedieť a pripravím na to samostatný nástroj.

## 2. Čo bolo odstránené

- **`src-tauri/src/commands/payments.rs`** - celý súbor zmazaný (create/update/delete/get_payment_summary commands, `apply_paid_shortcut_*`/`revert_paid_shortcut_*`, aj jeho vlastná sada testov).
- **`src/components/PaymentsSection.tsx`** - celý súbor zmazaný.
- Registrácie v `commands/mod.rs` a `lib.rs` (5 Tauri commands) - odstránené.
- `models.rs`: `Payment`/`PaymentInput`/`PaymentSummary` structy - odstránené.
- `lib/types.ts`: `Payment`/`PaymentInput`/`PaymentSummary`/`PaymentStatus`/`PAYMENT_METHODS` - odstránené.
- `lib/api.ts`: `getPaymentSummaryForSale/Order`, `createPayment`, `updatePayment`, `deletePayment` - odstránené.
- `OrderDetail.tsx` a `SaleDetail.tsx`: `<PaymentsSection>` (import aj použitie) odstránené. Ich vlastné, staršie Paid/Outstanding karty (počítané priamo z `payment_status`, existovali už od 1.9.0) sú **nedotknuté** - fungujú presne ako v 1.9.10, keďže s Payment Ledgerom nikdy nezdieľali kód.
- `orders.rs`: `create_order_impl`/`update_order_impl` vrátené na jednoduchý zápis bez shortcut-payment logiky; `validate_creatable_payment_status` (obmedzenie na len Unpaid/Paid) zmazaná.
- `sales.rs`: `bulk_update_sale_payment_status_impl` vrátená na jednoduchý `UPDATE ... payment_status` bez počítania sale-group súm a bez volania shortcut funkcií.
- `dashboard.rs`: Cashflow (`paid_cents`/`outstanding_cents`) vrátený na priamy `SUM(sale_price_cents) WHERE payment_status = 'paid'/'pending'` namiesto `SUM` nad `payments` tabuľkou.
- `Orders.tsx` a `OrderDetail.tsx`: dropdown "Payment status" má opäť **Unpaid / Partial / Paid** (Partial bolo v 2.0.0 odstránené, keďže sa malo počítať len odvodene z ledgeru).

## 3. Čo NEBOLO odstránené (a prečo)

- **Migrácia `007_payments.sql` a tabuľka `payments`** - zostáva v `db.rs`'s `MIGRATIONS` poli navždy. Forward-only pravidlo appky (rovnaké ako pri 001-006) hovorí, že sa migrácie nikdy nesťahujú späť - najmä nie taká, ktorú tvoja reálna, už nainštalovaná databáza mohla použiť. Nová/preinštalovaná appka si tak vytvorí prázdnu, appkou nepoužívanú tabuľku - neškodné.
- **`db::migration_007_tests`** - ponechaný, ale zjednodušený (viac nevolá zmazané `payments.rs` funkcie, testuje len že migrácia 007 aplikovaná na reálne staršie dáta - presne v duchu existujúceho `migration_004_tests` - nič nepokazí a že tabuľka ostáva použiteľná cez čisté SQL). Toto som pridal ako vlastnú, zámernú výnimku - migrácia 007 sama o sebe totiž zostáva aktívnou, navždy platnou súčasťou schémy, aj keď appka nad ňou už nič nestavia, tak isto ako každá iná migrácia si zaslúži regresné pokrytie.
- **`CashflowSummary` a `PaymentSummary`-nezávislé časti `models.rs`/`types.ts`** - `CashflowSummary` (revenue/profit/paid/outstanding/currency) je presne ten istý tvar pred aj po 2.0.0 (menil sa len výpočet v `dashboard.rs`, nie tvar dát) - nedotknuté.
- **Auto-update mechanizmus** (`tauri-plugin-updater`, `src/lib/updater.ts`, `tauri.conf.json` -> `plugins.updater`) - úplne nedotknutý týmto revertom (nemá s Payments nič spoločné) - pozri sekciu 9 pre presný dôvod, prečo sa napriek tomu menilo číslo verzie.
- **CSV import/export, `finance.rs`, `money.rs`, refund/resell, `batch_id`/`SaleGroup`, Backup/Restore, migrácie 001-006, Pulls** - nič z toho sa 2.0.0 ani tohto revertu nedotklo.

## 4. Testy

`cargo test --lib` (skutočne spustený v tomto sandboxe, opakovane po každej zmene verzie):

```
test result: ok. 196 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out
```

196 namiesto pôvodných 223 (2.0.0) - rozdiel je zo zmazania `payments.rs`-ovej vlastnej sady testov spolu s ním; naopak `migration_007_tests` (2 testy) som zámerne ponechal, hoci pri 1.9.10 ešte neexistoval (migrácia 007 vtedy ešte nebola). Existujúce testy (Dashboard cashflow, `bulk_update_sale_payment_status`, `orders`) som vrátil na ich pôvodný, jednoduchší tvar (bez `seed_payment_for_sale` pomocníka) - všetky prešli.

## 5. Build

Aj toto sa dalo tentokrát spustiť naozaj (predchádzajúce kolá to museli robiť len ručnou kontrolou syntaxe, keďže `node_modules` a Tauri build nástroje v sandboxe chýbali):

```
cargo test --lib -> 196 passed, 0 failed (Cargo.lock sa zosynchronizoval automaticky)
npm install       -> OK (package-lock.json sa zosynchronizoval automaticky)
npx tsc -b        -> 0 chýb
npm run build     -> OK (vite build, dist/ vygenerovaný čisto, "tiqr-manager@2.0.1 build" v hlavičke - potvrdzuje že npm skutočne vidí správnu verziu)
```

Vizuálne som to neoveroval cez Playwright preview harness - úpravy v UI (odstránenie `<PaymentsSection>` z dvoch detailových stránok, pridanie jednej `<option>` do existujúceho dropdownu na dvoch miestach) sú čisto mechanické a izolované, žiadne prepočítanie layoutu okolo nich, a skutočný `tsc`/`vite build` prešiel čisto. Ak chceš, viem spraviť aj vizuálny prípad - daj vedieť.

## 6. Regresia

`finance.rs`, `money.rs`, refund/resell (vrátane 1.7.2 pravidla o mazaní refundovaného predaja), `SaleGroup`/`batch_id`, Backup/Restore, CSV import/export, migrácie 001-006, Pulls, Dashboard (mimo Cashflow výpočtu), auto-update - nedotknuté, potvrdené aj testami vyššie.

## 7. Zmenené súbory

**Zmazané:** `src-tauri/src/commands/payments.rs`, `src/components/PaymentsSection.tsx`
**Backend upravené:** `models.rs`, `db.rs` (test modul), `commands/mod.rs`, `lib.rs`, `commands/orders.rs`, `commands/sales.rs`, `commands/dashboard.rs`
**Frontend upravené:** `lib/types.ts`, `lib/api.ts`, `pages/OrderDetail.tsx`, `pages/SaleDetail.tsx`, `pages/Orders.tsx`
**Verzia (6 súborov - všetky miesta, ktoré appka aj release skript overujú):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` ($Version + commit message), `1-CLICK-UPDATE.bat` (title/echo text). `package-lock.json` sa synchronizuje sám cez `npm install`, needitoval som ho ručne.

## 8. Dodatok č. 1 - `release.ps1` mal zabudnutú starú verziu (prvá chyba, ktorú si narazil)

**Root cause:** V zozname "verzia, ktorú vždy meníš" som pri prvom kole mal len `package.json`/`tauri.conf.json`/`Cargo.toml`/`Cargo.lock` - úplne som zabudol, že aj `release.ps1` má na riadku 21 vlastnú natvrdo zapísanú `$Version`, ktorú si nastavila predchádzajúca session pri vydávaní 2.0.0 (`v2.0.0`), a ktorú skript používa na overenie ("majú tieto 3 súbory naozaj novú verziu?") ešte predtým, než niečo commitne/pushne. Keďže som ju nezmenil, skript čakal `2.0.0`, ale zdrojové súbory (vtedy správne) hovorili `1.9.11` - to je presne to "STOPPED: this clone does not actually have 2.0.0 everywhere", ktoré si videl. Skriptová rada "re-extrahuj zip nanovo" by toto nevyriešila - chyba nebola v tvojej kópii ani v kopírovaní, bola v mojom zdrojovom kóde `release.ps1`.

**Fix:** `release.ps1` riadok 21 → `$Version` nastavená na aktuálnu verziu; commit message (riadok 106) prepísaná, aby popisovala REVERT (predtým ešte opisovala pridanie Payment Ledgera z 2.0.0 - úplne nesprávny text pre toto vydanie). `1-CLICK-UPDATE.bat` mal tiež "v2.0.0" v title/echo texte (kozmetické, nemenilo to logiku, ale bolo by to zavádzajúce) - opravené, CRLF overené.

**Test:** Prešiel som celý zdrojový strom (`grep -rl "2.0.0"`, mimo `node_modules`/`target`/`dist`/historických reportov) - jediný zostávajúci výskyt bol zámerný, v komentári `db.rs` vysvetľujúcom pôvod migrácie 007. Nový zip som znova rozbalil cez `unzip -p` a overil `$Version`/title text priamo v zabalenom súbore, nie len v zdrojovom priečinku.

## 9. Dodatok č. 2 - prečo sa verzia zmenila z 1.9.11 na 2.0.1 (druhá chyba, ktorú si narazil)

**Root cause:** Po oprave `release.ps1` si nahlásil, že appka (bežiaca 2.0.0) ti aj tak ponúkala update. Overil som si to priamo v oficiálnej Tauri v2 dokumentácii k updater pluginu (`v2.tauri.app/plugin/updater`): Tauri **predvolene porovnáva verzie ako semver a update ponúkne len vtedy, keď je nová verzia väčšia než aktuálne nainštalovaná** ("Tauri checks if the update version is greater than the current app version"). `1.9.11` je podľa semver **menšie** číslo než `2.0.0` (major 1 < major 2) - appka teda úplne správne (z pohľadu updater pluginu) vyhodnotila "žiadny update nie je k dispozícii" a nikdy ho neponúkla. Tvoj odhad v predošlej správe ("mozno to musi byt vacsi update ako 2.0.0") bol presne správny.

**Zvažovaná alternatíva:** Tauri ponúka aj spôsob, ako toto obmedzenie obísť natrvalo - vlastný `version_comparator` v `updater_builder()` (Rust), napr. `|current, update| update.version != current`, ktorý by povolil aj downgrade. Zámerne som to takto **nespravil** - znamenalo by to trvalo zmeniť správanie auto-update mechanizmu pre všetky budúce verzie (appka by odvtedy vedela ponúknuť aj downgrade omylom), len kvôli jednorazovej chybe v číslovaní. Bezpečnejšia a štandardná oprava je jednoducho udržať čísla verzií vždy rastúce - presne ako to robí prakticky každý iný auto-update systém.

**Fix:** Premenoval som verziu z `1.9.11` na **`2.0.1`** vo všetkých 6 miestach zo sekcie 7 (znova spustený `cargo test --lib` aj `npm run build` po premenovaní - oba čisté, `Cargo.lock`/`package-lock.json` sa zosynchronizovali automaticky). Appka sa správaním rovná 1.9.10 (Payments úplne preč) - `2.0.1` je len číslo buildu/vydania, nič z 2.0.0 sa touto zmenou nevrátilo.

**Test:** `2.0.1 > 2.0.0` podľa semver, takže Tauri updater tento build teraz correctly ponúkne ako novšiu verziu tvojej nainštalovanej appke. (Toto som mohol overiť len logikou/dokumentáciou, nie reálnym behom updatera - v tomto sandboxe nemám nainštalovanú appku 2.0.0, voči ktorej by som update reálne vyskúšal. Ak by sa aj `2.0.1` z nejakého iného dôvodu neponúkol, daj vedieť presne čo appka/`release.ps1` hlási.)

## 10. Prečo 2.0.1, nie iné číslo

Tvoje pravidlo: žiadna zmena sa neposiela pod verziou, ktorá už raz odišla. `1.9.10` a `1.9.11` (ten prvý pokus) sú tak mimo hry. Zo zvyšku bolo `2.0.1` jediná voľba, ktorá zároveň (a) je väčšia než `2.0.0` (nutné pre auto-update, sekcia 9) a (b) je najmenší možný krok nad `2.0.0` - žiadne ďalšie skoky v čísle, len presne toľko, koľko treba na to, aby ju updater uznal za novšiu.

## STOP

2.0.1 hotové - appka sa správaním rovná 1.9.10, migrácia 007 a tvoje reálne platby ostávajú v databáze netknuté navždy, a auto-update by ju teraz mal korektne ponúknuť z nainštalovanej 2.0.0. Nezačínam nič ďalšie.

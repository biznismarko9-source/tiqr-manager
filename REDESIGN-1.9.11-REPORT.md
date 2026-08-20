# TIQR Manager 1.9.11 — Revert Payments 2.0 (Payment Ledger)

Report k verzii **1.9.11**. Nadväzuje na 2.0.0 (Payments 2.0 / Payment Ledger) - po vyskúšaní si vlastník appky funkciu rozmyslel a požiadal o návrat k správaniu z 1.9.10. Toto NIE JE obyčajný `git revert` (appka tu nemá `.git`, dostávam len zdrojový zip) - je to ručne vykonaný, riadok-po-riadku overený revert presne tých zmien, ktoré 2.0.0 report (sekcia 17) vymenoval ako zmenené súbory.

## 1. Čo sa stalo a prečo to nie je len "vymazať priečinok"

Medzitým, čo appka bežala na 2.0.0, si mohol do nového Payment Ledgera (`payments` tabuľka, migrácia 007) zapísať **reálne platby**. Preto som pred akoukoľvek úpravou najprv:

- Prečítal `db.rs` migration runner - je **forward-only** (žiadne "down" migrácie, žiadny mechanizmus na spätné vymazanie migrácie zo zoznamu bez rizika).
- Spýtal sa ťa priamo, či už máš 2.0.0 nainštalovanú a či v nej máš reálne platby - potvrdil si, že áno.

Záver z toho: **migrácia 007 a tabuľka `payments` musia v appke zostať navždy** (presne tak, ako sa nikdy nemažú/neupravujú migrácie 001-006) - inak riskujem rozbitie tvojej reálnej, už nainštalovanej databázy pri ďalšom spustení appky. Revert je preto urobený **na úrovni aplikácie** (UI, commands, business logika), nie na úrovni schémy databázy.

**Tvoje reálne platby sú v bezpečí bez ohľadu na čokoľvek ďalšie v tomto reporte:** tabuľka `payments` a jej riadky sa touto verziou vôbec nedotýkajú - len ich appka od 1.9.11 nikde nezobrazuje ani nezapisuje. Navyše: appka, ktorú máš teraz nainštalovanú (2.0.0), je stále 2.0.0, kým sám nespustíš `1-CLICK-UPDATE.bat` pre toto vydanie - takže si ich môžeš kedykoľvek pred aktualizáciou ešte pozrieť v Sale Detail / Order Detail presne tak ako doteraz. Ak by si si ich chcel/a niekedy pozrieť znova aj po aktualizácii na 1.9.11, appka z 2.0.0 (alebo jej opätovná inštalácia) by ich okamžite znova ukázala bez straty jediného riadku, keďže dáta v súbore ostávajú nedotknuté. Ak by si napriek tomu chcel/a mať tie platby aj mimo appky (napr. CSV kópiu pre vlastnú evidenciu), daj vedieť a pripravím ti na to samostatný nástroj - zámerne som ho teraz nebudoval naslepo bez istoty, aké nástroje (Python/sqlite3/...) máš k dispozícii na svojom počítači.

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
- **`db::migration_007_tests`** - ponechaný, ale zjednodušený (viac nevolá zmazané `payments.rs` funkcie, testuje len že migrácia 007 aplikovaná na reálne staršie dáta (presne v duchu existujúceho `migration_004_tests`) nič nepokazí a že tabuľka ostáva použiteľná cez čisté SQL). Toto som pridal ako vlastnú, zámernú výnimku - migrácia 007 sama o sebe totiž zostáva aktívnou, navždy platnou súčasťou schémy, aj keď appka nad ňou už nič nestavia, tak isto ako každá iná migrácia si zaslúži regresné pokrytie.
- **`CashflowSummary` a `PaymentSummary`-nezávislé časti `models.rs`/`types.ts`** - `CashflowSummary` (revenue/profit/paid/outstanding/currency) je presne ten istý tvar pred aj po 2.0.0 (menil sa len výpočet v `dashboard.rs`, nie tvar dát) - nedotknuté.
- **CSV import/export, `finance.rs`, `money.rs`, refund/resell, `batch_id`/`SaleGroup`, Backup/Restore, migrácie 001-006, Pulls** - nič z toho sa 2.0.0 ani tohto revertu nedotklo.

## 4. Testy

`cargo test --lib` (skutočne spustený, nie len manuálna kontrola - tento sandbox mal tentokrát cargo aj potrebné GTK/WebKit dev knižnice, tak som ich doinštaloval a spustil naostro):

```
test result: ok. 196 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out
```

196 namiesto pôvodných 223 (2.0.0) - rozdiel je zo zmazania `payments.rs`-ovej vlastnej sady testov spolu s ním; naopak `migration_007_tests` (2 testy) som zámerne ponechal, hoci pri 1.9.10 ešte neexistoval (migrácia 007 vtedy ešte nebola). Existujúce testy (Dashboard cashflow, `bulk_update_sale_payment_status`, `orders`) som vrátil na ich pôvodný, jednoduchší tvar (bez `seed_payment_for_sale` pomocníka) - všetky prešli.

## 5. Build

Aj toto sa dalo tentokrát spustiť naozaj (predchádzajúce kolá to museli robiť len ručnou kontrolou syntaxe, keďže `node_modules` a Tauri GUI knižnice v sandboxe chýbali):

```
npm install        -> OK
npx tsc -b          -> 0 chýb
npm run build       -> OK (vite build, dist/ vygenerovaný čisto)
```

Vizuálne som to tentokrát neoveroval cez Playwright preview harness - obe úpravy v UI (odstránenie `<PaymentsSection>` z dvoch detailových stránok, pridanie jednej `<option>` do existujúceho dropdownu na dvoch miestach) sú čisto mechanické a izolované, žiadne prepočítanie layoutu okolo nich, a skutočný `tsc`/`vite build` prešiel čisto. Ak chceš, viem spraviť aj vizuálny prípad - daj vedieť.

## 6. Regresia

`finance.rs`, `money.rs`, refund/resell (vrátane 1.7.2 pravidla o mazaní refundovaného predaja), `SaleGroup`/`batch_id`, Backup/Restore, CSV import/export, migrácie 001-006, Pulls, Dashboard (mimo Cashflow výpočtu) - nedotknuté, potvrdené aj testami vyššie.

## 7. Zmenené súbory

**Zmazané:** `src-tauri/src/commands/payments.rs`, `src/components/PaymentsSection.tsx`
**Backend upravené:** `models.rs`, `db.rs` (test modul), `commands/mod.rs`, `lib.rs`, `commands/orders.rs`, `commands/sales.rs`, `commands/dashboard.rs`
**Frontend upravené:** `lib/types.ts`, `lib/api.ts`, `pages/OrderDetail.tsx`, `pages/SaleDetail.tsx`, `pages/Orders.tsx`
**Verzia (4 súbory - `Cargo.lock` má tentokrát tiež svoj vlastný `tiqr-manager` riadok, keďže sa menila major/minor verzia):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`. `1-CLICK-UPDATE.bat` som needitoval - CRLF overené, je nedotknutý a už bol v poriadku.

## 8. Prečo 1.9.11, nie 1.9.10 alebo 2.0.1

Tvoje vlastné pravidlo: žiadna zmena sa neposiela pod verziou, ktorá už raz odišla - `1.9.10` teda nemôžem použiť znova. Ponúkol som ti `1.9.11` (pokračovanie v 1.9.x rade - appka sa správaním aj kódom vracia presne na úroveň 1.9.10, len ako nový build) oproti `2.0.1` (ostal by formálne v 2.x rade napriek tomu, že presne tá funkcia, kvôli ktorej appka skočila na 2.0.0, je preč) - vybral si `1.9.11`.

## STOP

1.9.11 hotové - appka sa správaním rovná 1.9.10, migrácia 007 a tvoje reálne platby ostávajú v databáze netknuté navždy. Nezačínam nič ďalšie.

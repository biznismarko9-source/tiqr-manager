# TIQR Manager 2.0.9 — Oprava chyby s názvom hárku + vlastný sheet pre Orders

Tento report rieši presne to, čo si nahlásil: chybu pri pripájaní tvojej skutočnej tabuľky, a k tomu možnosť appke nechať vytvoriť vlastný hárok pre Orders & Tickets, rovnako ako to už appka vie pri Pulls.

## 1. Čo sa stalo a prečo

Keď si sa appku pokúsil napojiť na svoj skutočný hárok s názvom **"Tiqr manager event + order"**, Google Sheets požiadavku odmietol s chybou "Unable to parse range: Tiqr manager event + order!A1:Z".

Príčina: appka pri skladaní adresy rozsahu (napr. `NázovHárku!A1:Z`) posielala názov hárku do Googlu "surový", bez úvodzoviek. To je platná syntax len vtedy, keď je názov hárku jedno jediné slovo bez medzier a znamienok (napr. "Pulls" alebo "Orders"). Akonáhle názov obsahuje medzeru alebo znamienko ako `+` - presne tvoj prípad - Google Sheets to odmietne rovno, skôr než sa vôbec appka dostane k čítaniu dát.

Toto sa netýkalo len Orders. Rovnaký spôsob skladania adresy sa používal na štyroch miestach naraz: pri synchronizácii Pulls, pri synchronizácii Orders, aj pri tlačidle "over pripojenie". Všetky tri by narazili na presne tento istý problém pri hárku s podobným názvom - takže táto oprava ich rieši všetky naraz, jedným spoločným miestom v kóde.

## 2. Oprava

Appka teraz vždy obalí názov hárku do jednoduchých úvodzoviek (`'Tiqr manager event + order'!A1:Z`) - presne tak, ako to vyžaduje Google aj Excel pri názvoch, čo nie sú jedno slovo. Robí to vždy, nielen keď to appka "uzná za potrebné" - takže sa nikdy nemôže stať, že by nejaký iný nezvyčajný názov hárku (medzera, pomlčka, čokoľvek) spôsobil rovnaký problém nabudúce.

Tvoju tabuľku teraz stačí pripojiť presne tak, ako si to skúšal predtým - žiadne premenovávanie hárku ani obchádzky nie sú potrebné.

## 3. Nová možnosť: appka ti vytvorí vlastný hárok pre Orders & Tickets

Presne, ako si žiadal - na karte "Orders & Tickets" v Settings -> Integrations teraz pribudlo aj tlačidlo "Create a new sheet for me", rovnaké ako pri Pulls.

Toto sa hodí, ak by si niekedy chcel začať s čistým hárkom namiesto pripájania toho existujúceho (napríklad pre test, alebo pre niekoho ďalšieho, kto appku začína používať od nuly). Appka vytvorí novú tabuľku s presne tými hlavičkami stĺpcov, ktoré Orders sync vie prečítať (Event Name, Date, platform, Section, Row, Seats, Order ID, Total Purchase Price, Number of Tickets, Price Per Ticket, currency, Email (used), Ticket Type), a rovno ju aj pripojí.

Ak si prihlásený cez Google (Sign in with Google), nový hárok rovno patrí tvojmu účtu a appka ho nemusí nikomu zvlášť zdieľať. Ak nie si prihlásený, appka hárok vytvorí cez zdieľaný účet appky a zdieľa ho na email, čo zadáš - rovnaké správanie ako pri Pulls.

Keďže tvoju skutočnú tabuľku už máš a po dnešnej oprave sa už dá pripojiť normálne, toto tlačidlo pre teba osobne asi nie je potrebné - je tu hlavne pre konzistenciu s Pulls a pre budúcnosť.

## 4. Čo urob teraz

1. Spusti `1-CLICK-UPDATE.bat` (teraz na v2.0.9), počkaj na zelený build.
2. V appke choď do Settings -> Integrations, na karte "Orders & Tickets" skús znova pripojiť svoju skutočnú tabuľku s hárkom "Tiqr manager event + order" - presne tak, ako predtým.
3. Klikni "Sync now" a napíš mi, či sa teraz problém s "Unable to parse range" už neobjavuje a koľko sa vytvorilo.

## 5. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 310 passed, 0 failed (302 + 8 nových)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.9 build" v hlavičke)
```

8 nových testov: 3 na `a1_range` (obaľuje názov s medzerou a znamienkom do úvodzoviek, obaľuje aj jednoduché jednoslovné názvy, zdvojí prípadnú úvodzovku vnútri názvu), a 5 na nové "Create a new sheet for me" pre Orders (hlavičky nového hárku spĺňajú vlastnú kontrolu povinných stĺpcov appky, hlavičky sedia presne s testovacími dátami tohto modulu, zlá emailová adresa aj zlá mena sa odmietnu ešte pred akýmkoľvek pokusom o sieť, a platný vstup zlyhá čisto a zrozumiteľne, keď appka v tomto testovacom prostredí nemá k dispozícii service account).

## 6. Zmenené a nové súbory

**Zmenené:** `src-tauri/src/google_sheets.rs` (nová funkcia `a1_range` - vždy obalí názov hárku do úvodzoviek), `src-tauri/src/commands/sheets_sync.rs`, `src-tauri/src/commands/pulls_sheet_sync.rs`, `src-tauri/src/commands/orders_sheet_sync.rs` (všetky miesta, čo skladali adresu rozsahu, teraz používajú `a1_range`; `pulls_sheet_sync.rs` navyše sprístupňuje `validate_share_email`/`validate_currency` aj pre nový modul), `src/lib/api.ts` (nová `createOrdersSheet`), `src/pages/Settings.tsx` (karta Orders & Tickets má teraz aj tlačidlo "Create a new sheet for me")
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.9`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.9 hotové a overené (310/310 backend testov vrátane 8 nových, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a skús znova pripojiť svoju skutočnú tabuľku na karte "Orders & Tickets" - napíš mi, či sa "Unable to parse range" už neobjavuje a čo sync vytvoril. Keď potvrdíš, že Orders sync na tvojich reálnych dátach sedí, idem rovno na Sales sync (druhá časť riadkov, viď bod 7 v 2.0.8 reporte) - presne, ako sme sa dohodli minule.

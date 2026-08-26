# TIQR Manager 2.0.3 — Google Sheets sync, krok 2 (Pulls: Sync now)

Report k verzii **2.0.3**. Nadväzuje na 2.0.2 (pripojenie tabuľky). Táto verzia pridáva to hlavné, čo si žiadal: tlačidlo **Sync now**, ktoré reálne prečíta tvoju Pulls tabuľku a založí/aktualizuje pulls priamo v appke. Smer je zatiaľ **len tabuľka → appka** — zápis zmien z appky späť do tabuľky je samostatný, neskorší krok (dôvod je v sekcii 5).

## 1. Ako to funguje

Klikneš **Sync now** v Settings → Integrations → Pulls. Appka:

1. Prečíta celý riadok hlavičiek + všetky dátové riadky tvojej tabuľky.
2. Ku každému riadku buď založí nový pull (ak ho ešte nepozná), alebo aktualizuje existujúci (ak sa niečo zmenilo od minulého syncu), alebo ho nechá tak (ak sa nič nezmenilo).
3. Do tabuľky si sama pridá nový stĺpec **"TIQR ID"** (na koniec, za tvoje existujúce stĺpce) a do neho pri založení zapíše vlastný kód pullu (napr. `PULL-000015`). Vďaka tomu vie pri ďalšom synchronizovaní presne spárovať riadok tabuľky s konkrétnym pullom v appke — bez rizika duplicít. Tento stĺpec je čisto pre appku, nič doňho ručne nepíš.

Po dokončení appka ukáže: koľko sa založilo, koľko aktualizovalo, koľko ostalo nezmenených, a — ak nastane problém — presne pri ktorom riadku a prečo (viď sekcia 4).

## 2. Mapovanie stĺpcov (finálne, podľa tvojho potvrdenia)

| Tvoj stĺpec | Pole v appke |
|---|---|
| `pull` | Kto (buyer) |
| `Event name` | Názov podujatia |
| `event date` | Dátum podujatia — appka rozumie `15.05.2026` aj `26.jan.2026` |
| `Ks` | Počet kusov — `2x` aj `2` fungujú |
| `Platform` | Platforma — appka ju nájde podľa mena alebo si ju sama založí, keď ešte neexistuje (rovnako ako pri CSV importe) |
| `More info` | Poznámka |
| **`Section`** (nový stĺpec) | Sektor |
| **`Row`** (nový stĺpec) | Rad |
| `Seats` | Sedadlo |
| `Transfer` | Áno/Nie → hotovo/nehotovo |
| `Price` | Tvoja odmena — mena sa berie z pripojenia (EUR/USD/GBP), nie z tabuľky |
| `date` | ignorované, presne ako si potvrdil |
| (prázdny stĺpec) | ignorované |

**Predtým, než prvýkrát klikneš Sync now, pridaj do svojej reálnej tabuľky dva nové stĺpce — "Section" a "Row"** (kdekoľvek, appka ich nájde podľa názvu, nie podľa pozície) — presne ako si navrhol. Ak ich tam ešte nemáš, appka jednoducho sedadlo/rad/sektor nechá prázdne, nič sa nepokazí, len o tie údaje prídeš.

**Dôležité pre tie pôvodné 3 riadky, čo si posielal ako prvé** (Fred Again/Eurovision/Chris Stapleton): ak sú v tvojej živej tabuľke stále v stĺpci "Seats" tie hodnoty čo vyzerajú ako heslá (`SlabeRuky22.`, `Markiboss1111.`), sync ich pri prvom spustení zoberie presne tak, ako tam sú, do poľa "Sedadlo" — appka nemá ako vedieť, že to nie je skutočné sedadlo. Ak to tak nechceš mať v appke, priprav si tie bunky (vymaž/uprav) pred prvým sync, alebo to pokojne oprav neskôr priamo v appke po importe.

## 3. Mena

Pridal som výber meny (EUR/USD/GBP) priamo do formulára pripojenia v Integrations, presne ako si chcel. Platí pre všetky riadky z danej tabuľky — ak by si niekedy pulloval naraz vo viacerých menách v jednej tabuľke, daj vedieť, zatiaľ appka predpokladá jednu menu na celú tabuľku.

## 4. Čo sa stane, keď niečo nesedí

Na rozdiel od CSV importu (kde je to "všetko alebo nič") sync ide **riadok po riadku nezávisle** — jeden pokazený riadok nezastaví import ostatných, lebo sync budeš spúšťať opakovane nad živou tabuľkou, nie raz. Presne rozlišuje tri prípady:

- **Chyba** (napr. nesprávny formát dátumu, chýbajúca cena) — appka ten riadok vynechá, ostatné riadky importuje, a presne ti povie pri ktorom riadku a čo bolo zle. Opravíš v tabuľke a spustíš sync znova.
- **Konflikt** — ak od posledného syncu zmeníš niečo v appke A ZÁROVEŇ sa zmení aj rovnaký riadok v tabuľke, appka to **nikdy sama nerozhodne** — ani jednu stranu neprepíše, len ti to nahlási, aby si to vyriešil ručne.
- **Neznáme ID** — ak by niekto omylom niečo napísal do stĺpca "TIQR ID" ručne, appka to nebude tíško brať ako nový pull (aby nevznikol duplicitný pull) — nahlási to namiesto toho.

## 5. Čo táto verzia stále nerobí (zámerne)

- **Appka nezapisuje zmeny z appky späť do tabuľky** (okrem toho jedného "TIQR ID" stĺpca). Ak niečo upravíš v appke, do tabuľky sa to samo nedostane — to je ďalší krok.
- **Tickets tabuľka** stále nemá svoju kartu v Integrations — backend je na to pripravený (rovnaké commands fungujú pre ľubovoľný zdroj dát), ale UI aj mapovanie stĺpcov pre Tickets pribudnú v samostatnom kroku.

## 6. Testy

```
cargo test --lib -> 244 passed; 0 failed; 3 ignored
```

244 = 217 (z 2.0.2) + 27 nových: 2 v `sheets_sync::tests` (validácia meny) a 25 v novom `commands::pulls_sheet_sync::tests` — parsovanie dátumu/počtu kusov/Transfer, mapovanie stĺpcov, zakladanie nových pullov, rozpoznanie nezmenených/zmenených riadkov, detekcia konfliktu, ochrana pri zmazanom prepojenom pulle, ochrana pri neznámom ID, a že opakovaný sync nad tými istými dátami nič nezduplikuje.

Rovnaké obmedzenie sandboxu ako v 2.0.2 platí aj tu — časť, ktorá skutočne komunikuje s Google Sheets (`sync_pulls_impl`), som nemohol odskúšať naživo (`googleapis.com` je tu nedostupné). Preto som logiku rozdelil na dve časti: `apply_pull_rows` (spracovanie riadkov, zakladanie/aktualizácia pullov, porovnávanie so snapshotom — celé plne otestované offline, 25 testov vyššie) a tenký "sieťový" obal okolo nej (`sync_pulls_impl` — načíta tabuľku, zavolá `apply_pull_rows`, zapíše značky späť), ktorý som nemohol reálne spustiť, len ho pozorne skontrolovať. Prvý reálny sync preto over cez appku priamo u seba — ak sa niečo nesprávalo presne podľa reportu, daj vedieť presne čo appka hlásila.

## 7. Build

```
cargo check --lib -> čisto, 0 warningov
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.3 build" v hlavičke)
```

## 8. Zmenené/nové súbory a verzia

**Nové (backend):** `src-tauri/src/commands/pulls_sheet_sync.rs`
**Backend upravené:** `models.rs` (menu pre pripojenie, výsledok syncu), `commands/sheets_sync.rs` (mena v pripojení, verejné pomocné funkcie pre nový modul), `commands/pulls.rs` (`fetch_one` sprístupnené novému modulu), `commands/csv_import.rs` (`resolve_or_create_platform` zdieľané s novým modulom), `commands/mod.rs`, `lib.rs`, `google_sheets.rs` (`update_values` sa už reálne používa)
**Frontend upravené:** `lib/types.ts`, `lib/api.ts`, `pages/Settings.tsx` (výber meny, tlačidlo Sync now, výsledkový panel)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.3`, `package-lock.json` sa zosynchronizoval sám cez `npm install`.

## STOP

2.0.3 hotové a overené (244/244 backend testov, čisté `tsc`/`build`). Sync now pre Pulls je funkčný smerom tabuľka → appka, s ochranou proti duplicitám aj konfliktom. Než to skúsiš naostro: pridaj do svojej tabuľky stĺpce "Section" a "Row" (sekcia 2), a skontroluj tie 3 staré riadky so "Seats" hodnotami čo vyzerajú ako heslá. Napíš mi, ako prvý reálny sync dopadol — najmä ak appka nahlási niečo iné, než si čakal. Nezačínam nič ďalšie (Tickets kartu ani zápis appka→tabuľka), kým to nepotvrdíš.

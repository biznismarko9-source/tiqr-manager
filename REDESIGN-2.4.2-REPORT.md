# TIQR Manager 2.4.2 — Live Market Monitor odstránený, Price Checker späť na manuálny nástroj

Toto je report k cleanup tasku: odstránil som celú funkciu Live Market
Monitoring / Live Event Intelligence, presne ako si žiadal —
*"TÚTO FUNKCIU NECHCEM V APLIKÁCII VÔBEC"*. Price Checker je opäť čisto
manuálny nástroj: žiadny background monitor, žiadny live tracker, žiadny
online monitoring systém.

Predtým než som čokoľvek menil, prešiel som si `PROJECT_STATE/CURRENT_STATE.md`
a `PROJECT_STATE/PROTECTED_AREAS.md` a identifikoval som rozsah bez full repo
scanu. Prvé zistenie: **"Live Event Intelligence" už netreba nič robiť** —
tento smer bol kompletne zrušený a odstránený ešte pred tým, než sa 2.4.1
vôbec vydalo (vidno to v `PROTECTED_AREAS.md`, sekcia "2.4.0 (pre-release
direction)" — tabuľka, migrácia, backend modul aj UI blok boli vtedy
odstránené a v appke dnes nič z toho neexistuje). Celý reálny rozsah tejto
úlohy bol teda len 2.4.1 "Price Checker Live Market Monitor" — presne 10
súborov v kóde (6 backend, 4 frontend) plus 1 migračný súbor, ktorý som
zámerne nechal netknutý (dôvod nižšie, bod 3).

## 1. Čo všetko bolo odstránené

- **Backend modul `commands/price_checker_monitor.rs`** — zmazaný celý,
  vrátane oboch `#[tauri::command]` príkazov (`get_market_monitor_summary`,
  `list_market_snapshots`), zrušená registrácia v `commands/mod.rs` a
  `lib.rs`.
- **Oba háčiky do tohto modulu v `price_checker_scanner.rs`** — jeden pri
  úspešnom/čiastočnom scane, jeden pri zlyhanom — obidva volania
  `record_scan_attempt_impl(...)` aj ich pomocné premenné odstránené. Scan
  samotný (čo vidíš v okne) sa tým vôbec nezmenil.
- **Auto Monitor (ON/OFF + interval 15m/30m/1h/3h/6h)** — celý stav, efekty
  aj `window.setInterval` odstránené z `PriceChecker.tsx`. Appka po štarte
  už nič sama od seba neskenuje ani nekontroluje.
- **"Scan All"** — tlačidlo aj funkcia odstránené (existovalo výhradne ako
  doplnok k Auto Monitoru, bez neho stráca zmysel — pozri bod 6).
- **Scheduled/background scanning a monitoring interval settings** — boli to
  presne tie isté dve veci vyššie (Auto Monitor), nič ďalšie na túto tému v
  appke nebolo.
- **Automatic market change detection + alerty MARKET DROP / MARKET RISE /
  NEW SUPPLY / SUPPLY DROP** — celá `detect_and_record_changes` logika
  zmizla spolu s modulom.
- **Monitoring-špecifický cache/storage a snapshoty/história** — appka už
  tieto dáta nikde nezapisuje ani nečíta; UI ("Market History" modal, panel
  s poslednými alertami) je preč. Podkladové tabuľky zostávajú v DB, ale
  nepoužité — pozri bod 3, prečo.
- **`market_alert` kategória v Attention Centri** — 6. box "LIVE MARKET
  ALERTS" na Dashboarde odstránený, mriežka späť na `lg:grid-cols-5`.
  `attention_center.rs`'s `push_item` vrátený na pôvodný 2-tvarový kľúč
  (bez `marketplace_id`/`marketplace_name`), 5 pôvodných volaní `push_item`
  zbavených dvoch nadbytočných `None, None` argumentov.
- **Nové UI prvky a navigation položky určené iba pre monitoring** — Live
  Market Monitor panel na každej marketplace karte, Market History modal,
  Auto Monitor ovládanie, "Scan All" tlačidlo, Dashboard 6. box — všetko
  preč. Žiadna samostatná navigácia pre monitoring nikdy neexistovala mimo
  tohto.
- **Nové DB tabuľky/migrácie určené iba pre monitoring** — nebolo možné ich
  bezpečne zmazať (2.4.1 sa už reálne vydalo) — pozri bod 3 pre presné
  vysvetlenie a čo som spravil namiesto toho.

Overil som gitrepom nezávislým gréčom po celom `src/` aj `src-tauri/src/`
po dokončení: **nula zvyšných referencií** na `price_checker_monitor`,
`MarketMonitor*`, `MarketSnapshot*`, `MarketAlert`, `MarketSourceStatus`,
`market_alert`, `autoMonitor`/`AUTO_MONITOR`, alebo `scanAll` kdekoľvek v
kóde.

## 2. Čo v Price Checkeri zostalo

Nezmenené, presne ako si žiadal:

- **Event selection** — výber eventu na začiatku Price Checkera.
- **Marketplace URLs/source handling** — presne to, čo už bolo súčasťou
  pôvodného Price Checkera pred monitoringom (`price_checker.rs`'s
  `list_marketplaces`/`create_marketplace`/`delete_marketplace`/
  `save_event_marketplace_link`).
- **Manual scan + visible browser/WebView scanner** — celý
  `price_checker_scanner.rs` (Visible Scanner, 2.1.9) beží presne ako
  predtým, jediná zmena bolo odstránenie dvoch monitoring-háčikov (bod 1),
  nič v samotnom skenovaní.
- **Existujúce marketplace readers** — `price_checker_scan.js` (injected
  extraction script) nedotknutý.
- **Existujúca Market Analysis** — `price_checker_analysis.rs` nedotknutý
  (jedna kozmetická poznámka k viditeľnosti jednej funkcie je v bode 6).
- **Existujúce tier/level zoskupenie** — `group_by_tier` funguje presne ako
  pred monitoringom; Section/Row/Seat sú stále len popisné údaje, nikdy nie
  cenový faktor.
- **Existujúca price history** — `price_checker.rs`'s vlastný
  `save_price_check`/uložená história kontrol (CRUD, existovala pred
  monitoringom) je nedotknutá. Toto je iný mechanizmus než odstránený
  "Market History" modal (ten patril výhradne k monitoringu a je preč).
- **Your Tickets comparison** — nedotknuté.
- **Všetka ostatná existujúca manuálna funkcionalita Price Checkera** —
  nedotknutá. Žiadny redesign.

## 3. Či bola potrebná DB migrácia/cleanup

**Migráciu ani jej tabuľky nebolo možné bezpečne zmazať — a nezmazal som
ich.** Presne podľa tvojej inštrukcie som najprv overil: migrácia
`026_price_checker_market_monitor.sql` (4 tabuľky: `market_snapshots`,
`market_snapshot_tiers`, `market_source_status`, `market_alerts`) **už bola
vydaná** — bola súčasťou reálneho 2.4.1 buildu, takže tvoja vlastná
nainštalovaná appka ju už spustila a tabuľky môžu obsahovať reálne dáta
(uložené snapshoty/alerty z tvojho vlastného používania).

Tento projekt má prísne pravidlo "forward-only migrations" — už aplikovaná
migrácia sa nikdy nemaže ani neprečísluje, bez ohľadu na to, či ju appka
ešte používa. Preto som **nevymýšľal žiadny rollback**. Namiesto toho:

- Migračný súbor aj všetky 4 tabuľky **zostávajú v schéme presne tak, ako
  boli** — nezmenené, nezmazané.
- Odstránil som len aplikačný kód, ktorý tieto tabuľky čítal/zapisoval
  (celý `price_checker_monitor.rs` a oba háčiky v `price_checker_scanner.rs`
  — bod 1). Appka od 2.4.2 tieto tabuľky vôbec nevyužíva.
- **Žiadne existujúce používateľské dáta neboli zmazané.**
- Zapísal som nové pravidlo do `PROJECT_STATE/PROTECTED_AREAS.md` (sekcia
  "2.4.2") aj `CURRENT_STATE.md`: tieto 4 tabuľky sú od teraz natrvalo
  "osirelé" (orphaned), číslo migrácie "026" sa už nikdy nesmie znovu
  použiť, a **ďalšia nová migrácia musí byť 027**.

## 4. Ktoré súbory sa zmenili

**Zmazané súbory:**
- `src-tauri/src/commands/price_checker_monitor.rs`

**Backend (Rust) — upravené:**
- `src-tauri/src/commands/price_checker_scanner.rs` — odstránené oba háčiky
  do zmazaného modulu (úspešný aj zlyhaný scan).
- `src-tauri/src/commands/attention_center.rs` (1237 → 1048 riadkov) —
  odstránená 6. kategória `market_alert`, `push_item` vrátený na 2-tvarový
  kľúč, `AttentionCenterItem` bez `marketplace_id`/`marketplace_name`,
  odstránený celý testovací blok pre `market_alert` (5 testov + pomocné
  funkcie).
- `src-tauri/src/models.rs` (2926 → ~2776 riadkov) — zmazaná celá sekcia
  "Price Checker Live Market Monitor (2.4.1)" (`MarketSnapshotTier`,
  `MarketSnapshot`, `MarketAlert`, `MarketMonitorMarketplaceView`,
  `MarketMonitorSummary`), odstránené 2 polia z `AttentionCenterItem`.
- `src-tauri/src/commands/mod.rs` — odstránený `pub mod
  price_checker_monitor;`.
- `src-tauri/src/lib.rs` — odstránené 2 príkazy z `invoke_handler!`.

**Backend — zámerne nedotknuté** (pozri aj bod 6):
- `src-tauri/src/commands/price_checker_analysis.rs`
- `src-tauri/src/commands/database.rs`
- `src-tauri/migrations/026_price_checker_market_monitor.sql`

**Frontend — upravené:**
- `src/lib/types.ts` (2006 → 1996 riadkov) — zmazaná sekcia "Price Checker
  Live Market Monitor (2.4.1)" (5 typov), odstránené 2 polia +
  `"market_alert"` z `AttentionCenterItem`.
- `src/lib/api.ts` — odstránené 2 wrappery
  (`getMarketMonitorSummary`/`listMarketSnapshots`) a ich typové importy.
- `src/pages/Dashboard.tsx` — odstránený 6. box, mriežka späť na
  `lg:grid-cols-5`, `AttentionCenterRow` zjednodušený (bez
  `marketplaceName`/`linkState`).
- `src/pages/PriceChecker.tsx` (najviac upravovaný súbor) — odstránené: 5
  typových importov, konštanty pre Live Market Monitor, Auto Monitor +
  Market History stav/efekty, celý renderovaný panel Live Market Monitor aj
  Market History modal, `scanAll`, "Scan All" tlačidlo, `monitor` stav a
  jeho načítanie na úrovni stránky, `highlightMarketplaceId` a
  scroll/zvýraznenie, 2 nepoužité importy (`CHECKBOX_CLASS`, `IconRefresh`).

**Vydávacie súbory (verzia zvýšená na 2.4.2, všetkých 9 miest/7 súborov):**
- `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`.
- `src-tauri/Cargo.lock` — regenerovaný cez `cargo check` (nie ručne).
- `package-lock.json` — regenerovaný cez `npm install --package-lock-only`
  (nie ručne).
- `release.ps1` — `$Version` na `"v2.4.2"`, `$CommitMsg` prepísaný na popis
  tohto odstránenia.
- `1-CLICK-UPDATE.bat` — titulok aj echo text na `v2.4.2`.

**Dokumentácia:**
- `PROJECT_STATE/CURRENT_STATE.md` — nová "## Version" poznámka pre 2.4.2,
  nový vrchný záznam v "Current focus", zoznamy modulov/stránok upravené
  tak, aby už nespomínali monitoring ako aktuálnu funkciu.
- `PROJECT_STATE/PROTECTED_AREAS.md` — nová sekcia "2.4.2" (dôvod pozri bod
  3), stará sekcia "2.4.1" skrátená na históriu (kód, ktorý opisovala, už
  neexistuje).
- `CHANGELOG.md` — nový záznam "2.4.2" navrchu (append-only, pôvodný
  "2.4.1" záznam nedotknutý).

## 5. Test results

```
cargo check --lib          -> OK (tiqr-manager v2.4.2, 1 pred-existujúce
                               upozornenie nesúvisiace s touto úlohou:
                               sales.rs's fetch_recent, nepoužitá mimo testov)
cargo test --lib           -> 1026 passed, 0 failed, 3 ignored
                               (1058 pred touto úlohou − 32 testov zmazaných
                               spolu s odstránenou funkciou, 0 zlyhaní)
cargo clippy --lib --tests -> bez nových upozornení
npx tsc -b                 -> 0 chýb
npm run build              -> OK ("tiqr-manager@2.4.2 build", vite build
                               prešiel bez chyby)
```

Testy pokrývajú presne to, čo si žiadal: manuálny scan a viditeľný scanner
(nezmenené, prechádzajú), Market Analysis a tier zoskupenie (nezmenené,
prechádzajú), Your Tickets comparison (nezmenené, prechádza), Attention
Center bez `market_alert` (5 pôvodných kategórií prechádzajú presne ako
predtým, žiadna zmena ich vlastnej logiky), a regresie
Orders/Tickets/Sales/Listings/Finance/Fulfillment — celý zvyšok balíka
(1026 testov) prešiel bez jedinej zmeny správania mimo toho, čo bolo
zámerom odstrániť.

## 6. Čo bolo zámerne ponechané nedotknuté

- **Migrácia `026_price_checker_market_monitor.sql` a jej 4 tabuľky** —
  nezmazané, dôvod v bode 3. Toto nie je prehliadnutie, je to explicitné
  rozhodnutie podľa tvojho pravidla o forward-only migráciách.
- **`database.rs`'s test počtu migrácií** (`assert_eq!(migration_count, 26,
  ...)`) — stále presný, keďže migrácia 026 v schéme zostáva, len sa už
  nepoužíva. Nedotknuté.
- **`price_checker_analysis.rs`'s `group_by_tier` viditeľnosť** — v 2.4.1
  bola zvýšená z `private` na `pub(crate)`, aby ju mohol používať zmazaný
  monitoring modul. Nechal som ju tak (nevrátil na `private`) — je to
  neškodný zvyšok bez funkčného dopadu a nechcel som kvôli čisto
  kozmetickej zmene zasahovať do tohto chráneného Market Analysis súboru.
  Ak by si to chcel prísne vrátiť na `private`, daj vedieť, je to
  jednoriadková zmena.
- **Refund/resell, `batch_id`, money/integer cents a ostatné core systémy**
  — nedotknuté, žiadna z týchto úprav sa ich ani len nepriblížila.
- **Zvyšok Price Checkera aj celej appky** — Orders, Tickets, Sales,
  Listings, Finance, Fulfillment, zvyšných 5 kategórií Attention Centra —
  nedotknuté, potvrdené aj testami v bode 5.

---

Verzia zvýšená **2.4.1 → 2.4.2** (nie naspäť na 2.3.5, hoci výsledný kód je
funkčne presne to, čo appka mala pred 2.4.0/2.4.1) — rovnaký precedens ako
pri zrušení "Event Lifecycle" (2.3.0 → vydané ako 2.3.1): odstránenie
funkcie stále posúva verziu dopredu, nikdy nie späť na číslo, ktoré appka
už raz mala. Celé odôvodnenie je aj v `PROJECT_STATE/CURRENT_STATE.md`'s
"## Version" sekcii.

Dve veci som musel rozhodnúť sám, keďže zadanie ich priamo nešpecifikovalo
(vypísané aj v bode 6 vyššie a v `PROTECTED_AREAS.md`): odstránenie "Scan
All" tlačidla a ponechanie `group_by_tier` na `pub(crate)`. Všetko ostatné
je presne podľa tvojho zadania. STOP — žiadne nové features.

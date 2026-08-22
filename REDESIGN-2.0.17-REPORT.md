# TIQR Manager 2.0.17 — Pulls received (nová funkcia)

## 1. Čo je nové

V sekcii **Pulls** je teraz hore prepínač **Given / Received**:

- **Given** — presne to, čo tam bolo doteraz (pully, ktoré si TY urobil iným ľuďom). Nič sa na tom nezmenilo, len teraz má vlastnú záložku.
- **Received** — úplne nová záložka: pully, ktoré si TY zobral od iných ľudí. Niekto iný ti vypuloval lístky, ty si mu za to zaplatil fee, a keďže tie lístky sa potom stanú tvojím vlastným tovarom (idú cez tvoje Orders/Tickets/Sales ako zvyčajne), dá sa Received pull aj napojiť na konkrétny Order.

## 2. Ručný zápis

V Received záložke je tlačidlo **"New received pull"** — formulár vyzerá podobne ako pri Given pulls, len s poľami, ktoré dávajú zmysel pre tento smer:

- **From** — kto pre teba pulloval (voľný text)
- **Quantity**, **Event name**, **Event date**
- **Fee you paid** + mena (presne ten istý picker ako všade inde v appke)
- **More info** (voliteľné)
- **Linked order** — voliteľné pole, kde vieš cez vyhľadávanie (píšeš kód objednávky alebo názov eventu) napojiť tento pull na konkrétny Order. Nemusíš — pokojne to nechaj samostatné, presne ako si chcel ("Aj samostatne").

Keď je pull napojený na Order, v zozname aj vo formulári je naň klikateľný odkaz priamo na ten Order.

## 3. Automatické napojenie z Orders & Sales hárku

Toto je tá časť, čo si chcel najviac: keď sa synchronizuje riadok z tvojho Orders & Sales hárku a stĺpec **"pull"** v ňom je presne **"yes"** (a **"who pulled"** nie je prázdne), appka teraz namiesto starého správania (kde sa to len ticho vložilo ako text do poznámky pri predaji) vytvorí **skutočný riadok v Received pulls**:

- **From** = hodnota zo stĺpca "who pulled"
- **Fee** = hodnota zo stĺpca "how much pull" (ak je prázdna alebo sa nedá prečítať ako číslo, jednoducho sa dá 0 — nikdy to nezablokuje samotný predaj)
- **Event / Event date / mena** — prevezme sa priamo z objednávky, ku ktorej ten riadok patrí (žiadne dopisovanie druhýkrát)
- **Linked order** — automaticky napojený na presne tú objednávku

Toto je **jednorazové na objednávku** — keď sa ten istý riadok/objednávka niekedy zosynchronizuje znova (napríklad keď sa časť lístkov predala neskôr), appka to nevytvorí druhýkrát. Je to poistené dvakrát: raz v appke (kontrola pred vytvorením) a raz priamo v databáze (aby sa to nedalo pokaziť ani teoreticky).

Stará "poznámka v predaji" s textom "Pull: yes; Who pulled: ...; How much pull: ..." už neexistuje — presne toto si chcel nahradiť skutočným prepojeným záznamom, aby si mal dobrý prehľad.

## 4. Peniaze - len informatívne

Presne ako si vybral: **fee pri Received pulls nikde neovplyvňuje Profit/Revenue** — ani na Dashboarde, ani v štatistikách eventu. Je to čisto informačný záznam, rovnako ako to funguje pri Given pulls (tvoj fee tam tiež nikde nevstupuje do financií). Ak by si to niekedy chcel zmeniť, je to samostatná zmena, čo sa dá pokojne dorobiť neskôr.

## 5. Testy a build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 376 passed, 0 failed (bolo 348 - pribudlo 28 nových testov)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.17 build" v hlavičke)
```

## 6. Zmenené súbory

**Nové:**
- `src-tauri/migrations/011_pulls_received.sql` — nová tabuľka `pulls_received`
- `src-tauri/src/commands/pulls_received.rs` — CRUD (list/get/create/update/delete) + testy

**Zmenené:**
- `src-tauri/src/models.rs` — nové typy `PullReceived`/`PullReceivedInput`/`PullReceivedEditInput`
- `src-tauri/src/commands/orders_sheet_sync.rs` — automatické napojenie pri sync-i (bod 3 vyššie), stará "poznámka" logika odstránená, nové testy
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/db.rs` — zaregistrovanie novej funkcie
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy a volania pre frontend
- `src/pages/Pulls.tsx` — Given/Received prepínač, nový zoznam a formulár pre Received (vrátane vyhľadávania objednávky na napojenie)

**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.17`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.17 hotové a overené (376/376 testov, čisté `tsc`/`build`). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. V Pulls je hore prepínač Given / Received.
2. V Received vieš ručne pridať nový pull, aj ho napojiť na existujúcu objednávku (alebo nechať samostatný).
3. Po ďalšom Sales sync-i sa riadok s "pull" = yes objaví ako nový záznam v Received, napojený na správnu objednávku.
4. Skús to zosynchronizovať dvakrát za sebou (napr. Sync now znova) — v Received by nemal pribudnúť druhý duplicitný záznam pre tú istú objednávku.

Napíš mi, či to takto sedí, alebo či niečo chceš inak (napr. iné názvy stĺpcov, iné polia vo formulári).

# TIQR Manager 2.114.1 — pri prijatých pulloch suma, za ktorú sa predali

Tvoja požiadavka po 2.114.0:

- *„pri pulls by som aj chcel vidieť pri tých, čo som predal iným, tak aká
  čiastka"*

---

## Čo je nové

V **Pulls → Received** je nový stĺpec **Sold for**, hneď za Fee:

| Prípad | Čo ukáže |
|---|---|
| všetky lístky predané | **€450.00** |
| predaná len časť | **€200.00 · 1/4** (1 lístok zo 4) |
| predaje v dvoch menách | **£360.00 + €120.00** (nesčítava sa, nič sa neprepočítava) |
| prepojené, zatiaľ nič nepredané | - (bublina pri myši: *Nothing sold yet*) |
| bez objednávky | - (bublina pri myši: *Not linked to an order*) |

- Suma sú predaje lístkov z prepojenej objednávky, ktoré nie sú
  refundované, teda presne tie, ktoré počíta *Revenue* na detaile
  objednávky.
- Počíta sa pri každom otvorení a nikam sa neukladá. Po novom predaji,
  refunde alebo oprave ceny teda sedí hneď.
- V okne úpravy pullu je pod prepojenou objednávkou riadok, napríklad
  *Sold for €200.00 · 1 of 4 tickets sold*. Keď v okne zmeníš prepojenú
  objednávku, riadok zmizne, kým zmenu neuložíš, lebo patrí k pôvodnej
  objednávke.
- Predaje v dvoch menách sa radšej zalomia na dva riadky, než by sa suma
  odrezala.
- V úzkej tabuľke (okno užšie ako 1649 px) je len suma; „1/4" povie
  bublina pri myši.

Záložka **Given** (pully robené pre iných) sa nemení - tam je suma Fee,
ktorý v tabuľke už je.

## Ako to funguje

`src-tauri/src/commands/pulls_received.rs`, funkcia `attach_sold`:

- jeden zoskupený dopyt na celý zoznam, nie jeden na riadok: lístky
  prepojených objednávok → ich predaje bez refundovaných → súčet podľa
  objednávky a meny, najväčšia suma prvá;
- platí pre zoznam Received pulls, pre jeden pull (aj hneď po uložení) aj
  pre pully na detaile objednávky;
- žiadna migrácia, žiadna nová tabuľka ani stĺpec v databáze (ďalšia
  migrácia je 047).

Nový stĺpec dostal miesto zo stĺpca Event (9 % v širokej, 10 % v úzkej
tabuľke), takže súčty šírok ostali presne ako predtým.

## Testy

- `cargo test --locked`: 1579 prešlo, 0 zlyhalo (4 ignorované, ako doteraz).
- Nové Rust testy:
  - objednávka so zaplateným, čakajúcim, refundovaným a nepredaným
    lístkom ukáže €450.00 za 2 lístky;
  - dve objednávky vedľa seba sa nemiešajú;
  - pull bez objednávky nemá nič;
  - dve meny ostávajú oddelené, väčšia suma prvá.
- `npm test`: 39 prešlo (nový test pre text stĺpca a bubliny).
- `npm run build` prešiel.
- Obrazovka v prehliadači s napodobeným backendom: okno 1080 px s bočným
  panelom (najmenšie, aké appka dovolí) a 1700 px. Bez vodorovného
  posúvania, žiadna suma odrezaná.

## Čo sa pri kontrole ukázalo navyše

V širokej tabuľke sa pri synchronizovanom pulli kód objednávky skráti na
„ORD-0..." vedľa štítku *Synced*. Bolo to tak aj predtým (šírka stĺpca
Order sa nemenila). Bublina pri myši kód ukáže celý. Opraviť sa to dá
širším stĺpcom Order, ak chceš.

## Súbory

- `src-tauri/src/commands/pulls_received.rs`, `src-tauri/src/models.rs`
- `src/pages/Pulls.tsx`, `src/lib/pullStatus.ts` (s testom),
  `src/lib/types.ts`
- `CHANGELOG.md`, `PROJECT_STATE/*`, verzia v piatich súboroch.

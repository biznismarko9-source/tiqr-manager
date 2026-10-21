# TIQR Manager 2.114.2 — Given pully: celkový zárobok

Tvoja odpoveď na 2.114.1:

- *„nie, skôr som to myslel tak, ako celkovo som na tom zarobil"*
- na otázku, ktorý zárobok: **Given: súčet Fee**

---

## Čo je nové

V **Pulls → Given** je nad tabuľkou riadok so súčtom:

> **Earned: €165.00** from 6 pulls · **Paid: €115.00** · **Outstanding: €50.00**

- **Earned** = súčet Fee všetkých pullov, zaplatených aj nezaplatených.
- **Paid** = fee, ktoré kupujúci už zaplatili (zaškrtnuté *Paid*).
- **Outstanding** = fee, ktoré ešte nezaplatili.
- „from 6 pulls" hovorí, z koľkých pullov je súčet.

### Čo sa započíta

- Všetky pully, ktoré vyberá **vyhľadávanie, platforma a dátumy Od/Do**,
  na **Active aj Completed spolu**, v každom stave prenosu.
- Prepnutie záložky Active/Completed ani filter **Status** súčet nemenia,
  menia len tabuľku. Rovnako to robia Paid/Outstanding v Sales, aby sa
  súčet „celkovo" nemenil podľa toho, na ktorej záložke práve si.
- Užitočné kombinácie:
  - dátumy Od/Do → zárobok za mesiac alebo rok;
  - meno kupujúceho vo vyhľadávaní → zárobok od jedného kupujúceho;
  - platforma → zárobok z pullov na Ticketmasteri a podobne.
- Meny sa nesčítavajú ani neprepočítavajú: pully v librách sa ukážu
  zvlášť, napríklad „€165.00 + £25.00".

### Čo sa zmenilo pod kapotou

Filter **Status** (Not transferred yet / Transferred, not paid) sa
predtým posielal do databázy, takže by súčet potichu sledoval aj jeho.
Teraz sa uplatní v appke nad už načítaným zoznamom. Tabuľka ukazuje
presne to isté ako predtým; súčet nad ňou vždy rátá všetko, čo vyberajú
vyhľadávanie, platforma a dátumy.

### Received pully

Stĺpec **Sold for** z 2.114.1 (za koľko sa predali lístky z prijatých
pullov) ostáva. Ak ho nechceš, daj vedieť a odstránim ho.

## Testy

- `cargo test --locked`: 1579 prešlo, 0 zlyhalo (4 ignorované, ako doteraz).
- `npm test`: 40 prešlo. Nový test pre súčet: zaplatené, nezaplatené, fee
  0, dve meny (každá zvlášť, väčšia prvá), nič zaplatené → Paid €0.00,
  všetko zaplatené → Outstanding €0.00, žiadny pull → riadok sa neukáže.
- `npm run build` prešiel.
- Obrazovka v prehliadači s napodobeným backendom, 6 pullov v rôznych
  stavoch a jeden v librách: okno 1080 px s bočným panelom aj 1700 px,
  tmavá aj svetlá téma. Súčet ostal rovnaký na Active, na Completed aj
  pri každej voľbe filtra Status, kým sa tabuľka menila správne.

## Súbory

- `src/pages/Pulls.tsx`, `src/lib/pullStatus.ts` (s testom)
- `CHANGELOG.md`, `PROJECT_STATE/*`, verzia v piatich súboroch.

Žiadna zmena v databáze ani v Ruste okrem verzie (ďalšia migrácia je 047).

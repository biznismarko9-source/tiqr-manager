# TIQR Manager 1.9.9 — Pulls: Seats a More info ako samostatné stĺpce

Report k verzii **1.9.9**. Priama, rýchla oprava k tomu, čo si napísal hneď po tom, čo si videl 1.9.8
naživo.

---

## 1. Čo si napísal

> "takto to urcite nechcem, necchem aby to bolo v EVENT ale vedla toho boli vytvorene dalsie 2 bolicka
> seats a more info a tam to bolo, rychla prerabka"

V 1.9.8 som Seats (miesto) a More info dal ako ďalšie riadky **vnútri** bunky Event - pod menom eventu a
dátumom. Bol to môj vlastný výklad tvojej pôvodnej požiadavky ("tu napiseme seats a taktiez pridame to
more info"), ale nebolo to, čo si chcel. Chcel si to presne opačne: **dva úplne samostatné stĺpce** vedľa
Event, nie text napchatý do jednej bunky.

---

## 2. Čo som zmenil

Stĺpec Event sa vrátil späť na to, čo bol pôvodne - len meno eventu a dátum, nič iné. Pribudli dva nové,
samostatné stĺpce hneď za ním:

Pull | For | Event *(meno · dátum)* | **Seats** *(nové)* | **More info** *(nové)* | Ks | Platform | Fee | Warning | Done

- **Seats** - miesto (Section/Row/Seat), presne v tom istom formáte ako v Order/Sale Detail. Ak nie je
  vyplnené nič, ukáže sa "General admission" (rovnaké správanie ako inde v appke).
- **More info** - tvoj voľný text (najčastejšie mail, na ktorý idú lístky), skrátený s "..." ak je dlhší,
  celý text uvidíš po nabehnutí myšou. Ak je prázdny, ukáže sa "-".

Formulár na pridanie/úpravu pullu (Section/Row/Seat, More info) sa nemenil - tam už bolo presne to, čo si
chcel v predchádzajúcom kole. Menil sa len zoznam.

---

## 3. Rozsah tejto zmeny

Toto bola čisto frontendová úprava rozloženia tabuľky - žiadna zmena v databáze, v Rust backende, ani v
tvare dát (`Pull`/`PullInput`/`PullEditInput` z 1.9.8 ostávajú presne také, aké boli). Nepridával sa nový
stĺpec do databázy ani sa nič neprečisľovalo v SQL - len sa v `Pulls.tsx` zobrazujú už existujúce polia
`section`/`rowLabel`/`seat`/`moreInfo` v dvoch nových bunkách tabuľky namiesto jednej spoločnej.

**Zmenené súbory:**
- `src/pages/Pulls.tsx` - tabuľka prerobená z 8 na 10 stĺpcov (`colgroup`/hlavička/riadky), Event bunka
  späť na meno + dátum, Seats a More info ako vlastné bunky. Odstránená aj pomocná premenná `hasSeatInfo`,
  ktorá slúžila len na potláčanie textu v starom prístupe a už nie je potrebná.

Nič iné sa nemenilo - `types.ts`, `api.ts`, backend (`pulls.rs`, `models.rs`, migrácie) ostali presne také,
aké boli v 1.9.8.

---

## 4. Testy, build - úprimne

Keďže ide o čisto frontendovú úpravu rozloženia (žiadne SQL, žiadny nový typ dát), riziko je oveľa menšie
ako pri predchádzajúcich dvoch kolách - hlavne som si dal záležať, aby po odstránení starého kódu
(zobrazovanie v Event bunke) neostalo niečo nepoužité alebo rozbité:

- Ručne som prešiel celý prepísaný súbor riadok po riadku - počet stĺpcov v `colgroup` (10), v hlavičke
  (10) aj v každom riadku tabuľky (10) sedí presne.
  - Skontroloval som, že `hasSeatInfo` už nikde nie je použité (bolo úplne odstránené, nie len
    "zabudnuté").
  - Skontroloval som, že import `centsToDecimalString` (potrebný vo formulári pri úprave) ostal na mieste.
- TypeScript syntax check (rovnaký spôsob ako v predošlých kolách) - **0 chýb** na `Pulls.tsx`, `types.ts`,
  `format.ts`, `api.ts`.
- Skutočný `cargo check` a `npm run build` som znova skúsil spustiť - stále rovnaké obmedzenie prostredia
  ako v každom predošlom kole (`cargo` narazí na `403 Host not in allowlist: index.crates.io`, `npm`
  nemá stiahnuté balíčky - `node_modules` je prázdny). Neviem to tu reálne skompilovať, len ručne
  a syntakticky overiť.

**Regresia:** nič v databáze, v `finance.rs`/`money.rs`, Backup/Restore, CSV importe, migráciách 001-006,
Dashboarde, ani v Sales/Orders/Tickets logike sa dnes nedotklo.

---

## 5. Čo NEBOLO zmenené

Refund/resell logika, `batch_id`/`SaleGroup`, zoskupovanie Tickets/Orders/Event, `finance.rs`/`money.rs`,
Backup/Restore, CSV import, migrácie 001-006, Dashboard finančná logika ani layout, existujúce Sales
filtrovanie/hľadanie/radenie, Settings routing, `supplier_id`. Financie z Pullov sú stále úplne mimo
Dashboardu.

---

## STOP

Toto by mala byť presne tá "rýchla prerábka", čo si chcel. Zároveň mám z minulého reportu (1.9.8) ešte
dve otvorené otázky, na ktoré si zatiaľ neodpovedal - dávam ich sem znova, nech sa nestratia:

1. **3 dni** ako práh na to, kedy sa začne objavovať upozornenie (Warning stĺpec) - vyhovuje, alebo chceš
   iný počet dní?
2. Po prejdení dátumu eventu upozornenie zosilnie na červenú a počíta, koľko dní je to už po termíne
   ("3d overdue") - je to tak, ako si chcel, alebo si predstavoval, že po evente už appka upozorňovať
   prestane úplne (keďže lístky už boli odovzdané)?

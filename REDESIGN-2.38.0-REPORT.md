# TIQR Manager 2.38.0 — riadok filtrov, vlastný kalendár, tenšie Settings

Sedem vecí, ktoré si napísal počas jedného tasku. Všetko je **prezentácia** —
žiadna zmena schémy, žiadna migrácia, žiadny Rust, žiadna matematika peňazí.

---

## 1. Prepínač kariet ide napravo do riadku filtrov

`Upcoming / Completed` (a v Sales `Pending / Completed`) už nesedí na vlastnom
riadku nad filtrami. Je **vpravo v tom istom riadku, kde je Search, Category,
Platform, Od/Do a Sort** — presne tak, ako to má Pulls, na ktoré si ukázal.

Platí na **Events, Orders, Inventory aj Sales**.

Jedna technická vec, o ktorej treba vedieť: `TabSwitcher` si doteraz nosil
vlastný spodný odstup (`mb-4`). Vnútri riadku filtrov je ten odstup zlý, mimo
neho správny — takže si ho komponent prestal nosiť a nosia si ho **dve miesta,
ktoré stoja samostatne** (obe v detaile eventu). Ak sa niekedy pridá nový
samostatný prepínač, musí si `mb-4` napísať tiež.

## 2. Sales

**Currency filter je preč.** Dátumový rozsah **Od / Do** sa posunul doslova na
jeho miesto — nie je to nový ovládač, je to ten istý pár, ktorý sa iba posunul
o jednu pozíciu vľavo.

**Refund status je preč úplne.** Bola to jediná vec pod tlačidlom
„More filters", takže zmizlo aj to tlačidlo a celý druhý riadok filtrov.

**Jeden status na riadok.** Badge `Paid / Pending` je preč, **guličky zostali**
(Sold · Deliv. · Paid). Riadok hovoril to isté dvakrát vedľa seba — tretia
gulička JE „paid".

Riadok `N/M refunded` sa presunul **pod guličky**, nezmizol. Je to jediné
miesto, kde zoznam vôbec povie, že sa niečo vrátilo, a to nie je duplicita,
to je informácia.

Stĺpcov je teraz **desať namiesto jedenástich** a obe `colgroup` sú prepočítané
na presne 100 %: tých 8 % po Statuse dostali štyri stĺpce, ktoré nesú text a
nie pevne široké číslo — Event, Platform, Seats a guličky (ich hlavička je
najširší popis v riadku).

## 3. Inventory

**Stĺpec Sold je preč**, zostáva **Total** a **Available**.

Nič sa neprepočítavalo: Total je stále počet kusov objednávky, Available je
stále „available + listed", takže predané sa dá stále prečítať ako **rozdiel
medzi nimi**. `soldCount` je stále v dátach a stále je vidieť na stránke
objednávky.

Obe `colgroup` sú prepočítané — **8 stĺpcov naširoko, 6 nasúzko** — a **Event
pustil kus zo svojich 43,5 %**, aby sa Seats a Purchase date zmestili celé a
neorezávali sa. To je presne to, čo si pýtal, keď si ten zoznam stĺpcov
nastavoval.

## 4. Vlastný kalendár — všade

Ten biely kalendár zo screenshotu **nie je našej appky**. `<input type="date">`
otvára kalendár **prehliadača** — vlastné písmo, vlastné rozloženie, o tmavom
režime nevie nič. Je to chrome prehliadača, CSS sa k nemu nedostane, takže
jediná cesta bola nakresliť si vlastný.

Čo má:

- mesiac a rok s šípkami dozadu/dopredu,
- **pondelok prvý**,
- **vždy šesť riadkov**, takže panel nemení výšku, keď listuješ mesiacmi,
- dni zo susedných mesiacov sú šedé, ale kliknuteľné,
- **dnešok** zvýraznený farbou akcentu,
- dole **Today** a **Clear**,
- otvára sa hore alebo dole a zarovná sa doprava podľa toho, koľko je miesta.

**Žiadna nová knižnica** — appka je offline-first a nepribudla ani jedna
závislosť.

A hlavné: `Input` posiela `type="date"` do nového kalendára **sám**, takže sa
**nemenilo ani jedno z ~30 miest**, kde sa v appke zadáva dátum. Stále posielajú
`YYYY-MM-DD` a stále čítajú `e.target.value`. Tri miesta, ktoré si `<input>`
kreslili ručne (Dashboard ×2 a detail objednávky), idú teraz cez `Input`, aby
vyzerali rovnako ako zvyšok.

Escape v kalendári je zastavený tak, aby zavrel **len kalendár** a nie formulár
pod ním.

## 5. Settings — Insights preč

**Sekcia Insights je preč úplne.** Bola to jediná cesta k Recapu, takže
**z appky odchádza aj Recap**. Súbor `components/Recap.tsx` zostáva na disku
nedotknutý, len ho už nič neimportuje — zmazať ho je samostatné rozhodnutie a
tvoje.

Dve veci, ktoré to ťahalo za sebou a sú vyriešené:

- **Posledný krok prehliadky** mieril na `/settings/insights`. Neznáma sekcia
  ticho spadne na prvú, takže by prehliadka skončila na Lookups a rozprávala o
  recapoch, ktoré už neexistujú. Teraz končí na Support, odkiaľ sa spúšťa.
- Graf (`MetricChart`) má stále svojho druhého používateľa — Dashboard — takže
  ten je nedotknutý.

## 6. Settings — jednoduchšie

**Import CSV + Export CSV = jedna karta „CSV"** s dvoma označenými riadkami.
Boli to dve karty vedľa seba, ktoré hovorili to isté slovo dvakrát. Handlery sú
do písmena tie isté.

## 7. Guide — lepší, profesionálnejší, stále jednoduchý

Gradientový hero je preč. Bol to najhlasnejší prvok v celých Settings a hovoril
najmenej.

Zostalo to isté, čo niesol, len čitateľnejšie:

- **tlačidlo na prehliadku** s tým, koľko to trvá,
- **tri veci, ktoré si nový používateľ pomýli**, ako tri krátke bloky vedľa
  seba namiesto jednej dlhej vety: event je prvý, objednávka si vyrobí lístky
  sama, predaj potrebuje poplatok platformy.

Počet krokov v texte sa berie **z prehliadky samotnej**, nie z čísla napísaného
v texte — nemôže sa rozísť s realitou.

---

## Na čo si treba dať pozor

**Refundy sú teraz v UI neviditeľné.** V 2.35.0 zmizlo tlačidlo Refund (to v
detaile predaja bolo jediné) a v tejto verzii zmizol filter Refund status.
**Momentálne neexistuje v appke cesta ani refund spraviť, ani si refundované
predaje vyfiltrovať.**

Dáta a logika sú úplne nedotknuté — refundované kusy sa stále počítajú v riadku
predaja (`N/M refunded`) aj v súhrne dole (`Refunded`). Píšem to sem, aby to
nebolo prekvapenie, keď to budeš niekedy potrebovať.

---

## Čo som overil

Na tomto Macu nie je Node ani Rust, takže `npx tsc -b`, `npm run build` ani
`cargo check` sa spustiť nedali — **prvý skutočný preklad je CI**. Overené bolo
toto:

- **Matematika kalendára** — preportovaná do Pythonu a prebehnutá na rokoch
  2019–2034, každý mesiac: pondelok je vždy prvá bunka, 42 dní ide bez medzery
  za sebou, každý deň mesiaca je presne raz, posun o mesiac/rok sa vracia späť
  na to isté. Aj kontrola, že šesť riadkov stačí vždy (najhorší mesiac
  1970–2099 potrebuje 37 buniek zo 42).
- **Párnosť stĺpcov** — Sales 10 `<col>` = 10 `<th>` = 10 `<td>` (+ checkbox),
  Inventory 8/8/8. Súčty `colgroup` sú presne 100 %.
- **Vyváženosť značiek a zátvoriek** vo všetkých 10 zmenených súboroch oproti
  balíku 2.37.0 — žiadny nový nepár.
- **Žiadne visiace odkazy** na odstránené veci (`refundStatus`,
  `showMoreFilters`, `currencyOptions`, `REFUND_STATUS_LABELS`, `InsightsCards`,
  `soldCount` v Inventory) a žiadny pokazený import.
- Tri nepoužité importy, ktoré nástroj našiel (`formatPercentOrMixed` v Sales,
  `formatPercent` v EventDetail, `Button` v Dashboard), sú **z 2.37.0**, nie
  z tejto verzie — nechal som ich tak.

**Čo overiť nedokážem:** ako vyzerá kalendár naživo (pozícia panela pri okraji
okna, v modálke, na Windows) a či sa niekde prepínač v riadku filtrov nezalomí
na úzkom okne. To uvidíš ty na prvom builde.

---

**Verzia:** 2.38.0 (9 miest v 7 súboroch vrátane `Cargo.lock`).
**Migrácie:** žiadne nové, ďalšia voľná je 031.

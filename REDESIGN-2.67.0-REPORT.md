# TIQR 2.67.0 — restriction pri novej objednávke

Pýtal si si tri veci. **Dve sú hotové, tretiu som nespravil** a chcem ti
povedať prečo.

---

## 1. Restriction v novej objednávke — HOTOVÉ

V mriežke novej objednávky je nový stĺpec **Restrictions**: osem prepínačov
po jednom písmene. Celý slovenský názov vyskočí v bublinke.

```
V  8  6  D   R  M  S  B
```

Štyri vľavo sú tie, **ktoré stoja peniaze, keď sa prehliadnu** — obmedzený
výhľad, 18+, 16+, doklad. Nie je to moje poradie, takto ich radí samotná apka
v `lib/restrictions.ts`.

**Ukladajú sa len na lístky, nie na objednávku.** To je dôležité a je to
zámerné. Migrácia 037 ich dala na sedadlo s odôvodnením, že *jedna objednávka
môžu byť pokojne dve sedadlá s čistým výhľadom a dve za pilierom*. Druhá kópia
na objednávke, ktorú by čítal nejaký filter alebo súčet, by bol **presne ten
istý tvar chyby ako riadky s `€`** — reprezentácia, ktorá vyzerá autoritatívne
a nie je.

Použil som vzor, ktorý v apke už existuje: `tier` z 2.2.7. Nastaví sa raz pri
vytváraní, skopíruje sa na každý vygenerovaný lístok, a potom sa dá meniť po
jednom v editore lístka.

**Dve veci navyše, ktoré z toho vyplynuli:**

Obmedzenia sú súčasťou **tvaru riadku**, takže dva riadky, ktoré sa líšia len
nimi, sa nezlúčia do jednej objednávky. Musí to tak byť — razítko sa aplikuje
raz na objednávku.

A **prenášajú sa do ďalšieho riadku**, rovnako ako typ lístka. Druhý riadok je
skoro vždy ďalší kus toho istého bloku, a znova odkliknúť „za pilierom" pri
každom riadku je presne ten krok, ktorý sa vynechá.

### Čo sa muselo zmeniť vzadu

`OrderInput.restrictions` v TypeScripte aj v Ruste, `restrictions_json`
pribudlo do INSERTu lístkov. Prešlo to cez `clean_restrictions` — tú istú
funkciu, ktorú používa editor lístka, takže orezanie, zmenšenie písmen
a odstránenie duplikátov funguje rovnako, a kód z novšej verzie sa nezahodí.

Prázdny zoznam dá `"[]"`, čo je presne DEFAULT toho stĺpca.

SQL som prepočítal ručne: **15 stĺpcov, 14 otáznikov + literál `'available'`,
14 parametrov.** Sedí.

Pole som musel doplniť do **dvanástich** miest, kde sa `OrderInput` skladá
ručne (väčšinou testy). Každé jedno som potom overil, že je naozaj vnútri
`OrderInput`, a nie v nejakom inom structe, čo má tiež `tier` a `seats`.

### Jedna vec, ktorú by kompilátor nechytil

`rightAlign={[0, 6]}` → `[0, 7]`. Vložením stĺpca na pozíciu 4 sa Price/ea
posunulo o jeden. Nič to netypuje proti zoznamu hlavičiek, takže by to bolo
ticho zarovnalo nesprávny stĺpec doprava.

---

## 2. Z Inventory urobiť sale — UŽ TO MÁŠ, od 2.65.0

Na Order Detail je tlačidlo **„Add sale"**, ktoré otvorí formulár predaja
s tou objednávkou už vybranou. Skryje sa, keď na objednávke nie je nič
predajné.

Nevidel si to, lebo **2.65.0 ani 2.66.0 ešte nikdy nebežali** — nie sú
zostavené, čakajú na CI.

---

## 3. Tmavšie okná — NESPRAVIL SOM TO

Hľadal som, čo stmaviť, a nenašiel som nič.

**Každé okno v apke je `bg-surface` = `#141415` na podklade `#0b0b0c`.**
To je **tá istá hodnota, do znaku**, akú majú panely v laboch, ktoré sa ti
páčili. `surface-raised` (`#1f1f23`) je svetlejšie, ale používa sa na **jednom
jedinom mieste** — v jednej bublinke.

Prešiel som všetky `.tsx` súbory a hľadal svetlé podklady, ktoré nemajú tmavú
variantu. **Nula skutočných nálezov.**

Spočítal som aj, čo by plošné stmavenie spravilo:

| | teraz | tmavšie |
|---|---|---|
| text na karte | 17,0 | 17,7 |
| odstup podklad → karta | 0,0037 | 0,0027 |

Kontrast by sa mierne **zlepšil**, ale odstup medzi podkladom a kartou by
klesol o tretinu. A `index.css` priamo hovorí, že tmavý režim *nesie svoju
hĺbku v tom tónovom kroku*. Karty by sa začali strácať v pozadí.

**Moja najlepšia hypotéza:** téma je nastavená na **system**
(`lib/theme.ts`). Ak máš Mac alebo laptop v svetlom režime, pozeráš sa na
svetlú tému — a tá je svetlá správne.

V apke je prepínač svetlá/tmavá vľavo dole. Skús ho, a ak je to ono, máme
hotovo. Ak nie, **pošli mi screenshot toho okna, ktoré ti je presvetlé** —
budem presne vedieť, čo stmaviť, namiesto toho, aby som hádal a rozbil tie
karty.

---

## Čo som netestoval

Na tomto Macu nie je `npm` ani `cargo`. **Nespustil som nič.** Overené je
počítanie v SQL, že všetkých dvanásť doplnení sedí v správnom structe, a že
každý vložený blok je vyvážený.

**Prvý kompilátor, ktorý tento kód uvidí, je CI.** Teraz ho čakajú tri
nevydané verzie naraz — 2.65.0, 2.66.0 aj 2.67.0.

---

## Súbory

- `src/pages/OrderRowsModal.tsx` — stĺpec, pole na riadku, tvar, prenos, odoslanie
- `src/lib/types.ts` — `OrderInput.restrictions`
- `src-tauri/src/models.rs` — to isté v Ruste
- `src-tauri/src/commands/orders.rs` — INSERT lístkov
- 12 miest v testoch, kde sa `OrderInput` skladá ručne

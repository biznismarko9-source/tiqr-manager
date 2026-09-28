# TIQR Manager 2.60.0 — obmedzenia na lístkoch, mesto pod eventom

---

## 1. Obmedzenia na lístkoch

> *„chcem aby sme vedeli pridat restriction pri listkov napr ako restricted
> view, 16+ atd, take najhlavnejsie"*

Pri úprave lístka je nové pole **Obmedzenia**. Naklikáš, čo platí:

| | |
|---|---|
| **Obmedzený výhľad** | za stĺpom, bokom, čokoľvek, čo treba povedať dopredu |
| **18+** / **16+** | veková hranica |
| **Na meno / treba doklad** | lístok je na meno alebo pýtajú doklad |
| **Bez opätovného vstupu** | keď vyjdeš, späť ťa nepustia |
| **Maloletý len s dospelým** | |
| **Na státie** | |
| **Bezbariérové miesto** | |

**Dá sa zapnúť viac naraz** — sedadlo môže byť pokojne aj za stĺpom, aj 18+.

### Kde ich uvidíš

V **detaile objednávky** ako farebné odznaky **priamo pri sedadle**. Tam sa
pozeráš, keď riešiš, ktoré miesto to je — a presne tam patria, lebo obmedzený
výhľad je vlastnosť **sedadla**, nie celej objednávky. Jedna objednávka môže
byť pokojne dve čisté miesta a dve za stĺpom.

Dôležité obmedzenia (výhľad, vek, doklad) sú **oranžové**, tie menej dôležité
modré alebo sivé — aby ti to, čo stojí peniaze, skočilo do očí.

### Hromadne

Vieš ich nastaviť aj **naraz pre viac označených lístkov**. Celá objednávka
býva rovnaká a preklikávať ich po jednom je istý spôsob, ako ich nenastaviť
vôbec.

### Jedna vec, ktorú som musel spraviť opatrne

Ukladanie lístka je napísané tak, že **čo formulár nepošle, to vymaže**. Keby
som obmedzenia pridal medzi ostatné polia, **každá iná cesta, ktorá upravuje
lístok, by ti ich ticho zmazala.**

Preto sa ukladajú **samostatne** a len vtedy, keď ich naozaj posielaš. Prázdny
zoznam ich zmaže — ale len keď to naozaj chceš.

---

## 2. Mesto pod názvom eventu

> *„pri inventory a sales aj vidno mesto v stlpci event pod nazvom"*

V **Inventory** aj v **Sales** je teraz pod názvom eventu **mesto**, menším
sivým písmom.

**Nepotreboval na to žiadnu novú migráciu** — mesto v databáze bolo od úplného
začiatku, len sa nikdy neprenášalo až do týchto zoznamov. Doplnil som to na
štyroch miestach (objednávky, lístky, predaje, zoskupené predaje).

Pri zoskupenom predaji z viacerých eventov („Mixed events") **mesto nie je** —
rovnako ako tam nie je názov. Nevymýšľam jedno mesto tam, kde ich je viac.

---

## 3. Širšie stĺpce

> *„trochu urobme tie stlpce vacsie aby sa to tam zmestilo"*

Stĺpec Event dostal viac miesta, aby sa tam tie dva riadky pohodlne zmestili:

| Kde | Predtým | Teraz |
|---|---|---|
| Inventory (úzke okno) | 44 % | **48 %** |
| Inventory (široké okno) | 33 % | **39 %** |
| Inventory (bez stĺpca Order) | 26 % | **32 %** |
| Sales | 16 % | **22 %** |

Miesto som zobral tým najširším susedom (Sedadlá, Náklady, Stav), nie jednému.
**Overil som, že všetkých päť sád stĺpcov stále dáva presne 100 %** — inak by
sa tabuľka ticho prestala zarovnávať.

---

## Čo som overil a čo nie

**Overené — spustené, nie prečítané:**

- **všetkých 37 migrácií** na čistej databáze
- **mesto aj obmedzenia naozaj dotečú** cez spojenie tabuliek až do appky
  (skúšané na naozajstnom riadku)
- **70 SQL dotazov** v lístkoch a predajoch proti skutočnej schéme
- **normalizácia obmedzení**: `"RV"`, `" rv "` a dvojklik sú jedno `rv`;
  prázdne sa zahodia; **neznáme kódy sa zachovajú** (kvôli syncu medzi verziami)
- **všetkých 5 sád šírok stĺpcov dáva 100 %**
- **tvary dát sedia medzi Rustom a appkou** pre všetky štyri typy (objednávka,
  lístok, predaj, zoskupený predaj) — tu by CI spadlo, keby nie
- zátvorky a importy vo všetkých zmenených súboroch

**Poznámka:** kontrola zátvoriek hlási jednu nezrovnalosť v `OrderDetail.tsx`.
Hlásila ju **aj pred mojou úpravou** (moja zmena pridala 7 otváracích a 7
zatváracích), takže je to chyba môjho nástroja na dlho fungujúcom súbore, nie
chyba v kóde.

**Neoverené:** appku nezostavím (nemám Node ani Rust) — to spraví tvoj build.

# TIQR Manager 2.47.0 — fotka vyplní viac objednávok naraz, rohy sú ostrejšie

Vybral si si **mriežku** — riadková tabuľka zostáva. K tomu tri veci, ktoré si
žiadal.

---

## 1. Fotka urobí toľko riadkov, koľko skupín na nej je

Toto bolo najväčšie.

Appka zo screenshotu **vždy prečítala všetky bloky sedadiel** — to vedela už
predtým. Lenže formulár ťa nechal vybrať **jeden** a napísal ti, že druhú
objednávku si máš spraviť ručne. Zvyšok sa zahodil.

Teraz je **každá skupina jeden riadok**.

### Čo je jedna objednávka a čo dve

Tu je to dôležité: **nerozhoduje o tom fotka.** Riadky sa položia tak, ako ich
obrázok prečítal, a potom platí **presne to isté pravidlo, ako keď riadky píšeš
sám** — musí sedieť sektor, rad, cena, mena, typ, platforma aj pull, **a
sedadlá musia ísť tesne za sebou**. Inak sú to samostatné objednávky.

| Na fotke | Riadky | Objednávky | Prečo |
|---|---|---|---|
| A/12 14–15 · A/12 16–18 · VIP/1 22 | 3 | **2** | A/12 sa spojí na 14–18, VIP ide zvlášť |
| A/12 14–15 **@201** · A/12 16–18 **@185** | 2 | **2** | iná cena sa nezlučuje |
| A/12 14–15 · A/12 **19** | 2 | **2** | sedadlá nie sú vedľa seba |
| **A**/12 · **B**/3 | 2 | **2** | iný sektor vždy zvlášť |

Robím to takto naschvál: keby mala fotka vlastné pravidlo zlučovania, ten istý
nákup by skončil ako iné objednávky podľa toho, či prišiel fotkou alebo si ho
napísal. To by bolo horšie než užitočné.

### Dve veci, ktoré ti nič nezmažú

- **Prázdny formulár** sa nahradí prečítanými riadkami. **Rozpísaný** sa
  doplní — to, čo si už napísal, ostáva.
- **Fotka bez detailu lístkov** (len číslo objednávky, platforma…) riadok
  **nezmaže**, len doplní, čo z nej appka vyčítala.

## 2. Fotka je teraz na všetkých štyroch

Events, Inventory a Pulls ju mali. **Sales bol jediný bez nej.**

Riadky v predaji sú **skutočné lístky** z tvojich objednávok, takže tie zo
screenshotu vyčarovať nejde a ani to nepredstieram. Čo fotka vyplní, je **samotný
predaj**: dátum, platforma, kupec, stav platby — a **cena za kus aj poplatok sa
predvyplnia do riadkov, ktoré ešte žiadnu nemajú**. Cenu, ktorú si napísal ty,
neprepíše.

## 3. Ostrejšie rohy

Z **13 / 20 / 24 px** na **6 / 8 / 10 px**.

Je to **jedna zmena v jednom súbore** (`tailwind.config.js`) — tým istým
mechanizmom, akým sa mení farebná paleta. Chytí to celú appku naraz vrátane
Finance, bez hrabania sa po stránkach.

**Guličky, avatary a pilulky som nechal okrúhle.** Je ich 46 a sú to stavové
bodky — bodka s rohom nie je ostrejší design, to je chyba.

A nie je to nula: povedal si „nie úplne".

---

## Čo som overil

**Rozdeľovanie skupín som prepísal do Pythonu a spustil** na šiestich
prípadoch — všetky štyri riadky tabuľky vyššie sedia, plus:

- prázdny formulár → 1 riadok (nahradený), rozpísaný → 2 riadky (pôvodný ostal),
- fotka bez lístkov → riadok ostal, sektor sa nestratil, platforma a mena sa
  doplnili.

Ďalej:

- **Zoznam súborov** oproti 2.46.1: **0 pribudlo, 0 zmizlo, 6 zmenených.**
- **Zátvorky aj JSX značky** proti 2.46.1 — identické; jediný rozdiel je nový
  `<AiImportPanel>` v Sales, čo je presne to, čo sa pridávalo.
- **Žiadny nepoužitý import**, každý import dohľadateľný v exporte.
- **Všetkých osem miest**, kde sa AI import používa, ďalej sedí — staré
  formuláre berú `group`, riadkové berú `groups`.
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad (Node ani Rust tu nie sú, prvý reálny build je
CI) a ako ostro ti tie rohy sadnú oku — 6 px je môj odhad toho „ostrejšie, nie
úplne". Ak to chceš ešte ostrejšie alebo naopak mäkšie, je to jedno číslo
v jednom súbore.

**Finance:** ostrejšie rohy tam naskočili samé. Formulár na transakciu som
**nemenil na riadky** — transakcia je jedna vec, nie N blokov lístkov, takže by
to bola mriežka nasilu. Ak chceš zapisovať viac výdavkov naraz, poviem si
a spravím to ako piaty riadkový formulár.

---

**Verzia:** 2.47.0 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.

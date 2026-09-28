# TIQR Manager 2.59.0 — sync na pozadí

> *„ten sync urob tak ze on sa robi v pozadi, ze vlastne pocas toho tebe
> funguje apka a nemusis cakat kym sa to loadne"*
>
> *„po kazdej zmene sa to musi niekde zapisovat ... a automaticky su len ked
> zapinas a vypinas apku v pozadi"*

---

## Najprv to dôležité: našiel som chybu, ktorú som ti spravil v 2.58.0

Sync odkladá sťahovanie a zlučovanie, kým „niečo píšeš" — aby ti appka
nepreskočila pod rukami.

Lenže bolo to napísané tak, že **stačilo, aby mal kurzor v akomkoľvek poli na
písanie — aj úplne prázdnom.**

Nový editor poznámok z 2.58.0 je **celý poskladaný z takých polí**. Čiže od
2.58.0 platilo: **kým si mal otvorenú poznámku, automatický sync sa odkladal
donekonečna.** Len si ticho vypisoval banner.

Zvonka to vyzerá presne ako „sync nefunguje".

Teraz sa pýta na to, o čo naozaj ide: **je tam rozpísaný text, o ktorý by si
prišiel?** Prázdne pole už nič neodkladá.

---

## Sync je teraz tak, ako si povedal

| | Predtým | Teraz |
|---|---|---|
| **Automaticky** | každých 5 minút | **pri zapnutí a pri zatváraní** |
| **Počas sťahovania** | appka zamknutá prekrytím | **appka funguje**, dole len malý pásik |
| **Ručne** | kedykoľvek | kedykoľvek (nezmenené) |

**Každá zmena si sama poznačí, že je čo poslať** — to už fungovalo a overil som,
že to platí aj pre nové tabuľky poznámok. Preto je ručný sync vždy správny.

---

## Zatváranie appky

Pri zatváraní sa ešte pošle, čo je neposlané.

**Maximálne 8 sekúnd.** Potom sa appka zavrie tak či tak. Nič sa nestratí — čo
neprešlo, ostane označené a pošle sa pri najbližšom spustení.

### Toto som skoro prehliadol a stálo by ťa to nervy

Keď sa do zatvárania okna zapojí vlastný kód, **prestane okno zatvárať systém**
a musí ho zatvoriť ten kód. A robí to spôsobom, ktorý appka **nemala povolený**.

Keby som si to nevšimol, **appka by sa nemusela dať zavrieť vôbec.** Pridal som
to povolenie a zatváranie som napísal tak, že ho appka riadi na **každej** ceste.

**Preveril som všetkých 6 priebehov** — nič na poslanie, prešlo, odmietnuté,
zaseklo sa, nevie sa rozhodnúť, offline. **Všetkých 6 skončí zavretým oknom.**

---

## Čo som overil a zámerne nemenil

Prešiel som rozhodovanie syncu riadok po riadku:

- **ako sa rozhoduje** (poslať / stiahnuť / zlúčiť / nič) — **správne**
- **ako zistí, že druhý počítač má novšie dáta** (porovnáva verziu súboru na
  Drive) — **správne**
- **označovanie zmien** — funguje pre všetko vrátane nových poznámok

Nič z toho som nechytal. Nemá zmysel prepisovať niečo, čo je v poriadku.

---

## Čo z tvojho screenshotu vyplýva

Poslal si mi históriu z Macu. **Mac je zdravý:**

- o 7:07 zistil zmeny a **poslal ich**
- všetko ostatné `idle` — „už je všetko zosynchronizované"
- **ani jeden `error`, žiadne `offline`, žiadne `off`**

Čiže na Macu prihlásenie aj sieť fungujú a sync robí presne to, čo má.

**Takže ak sa ti dáta neprenášajú, dôkaz je na Windowse, nie na Macu.**

Pošli mi prosím ten istý screenshot z Windowsu — **Settings → Data**, tá
istá tabuľka. Konkrétne ma zaujíma:

- je tam vôbec **nejaký `push`**? (ak nie, Windows nikdy nič neposlal)
- sú tam **`error`** riadky a čo píšu?
- alebo sú tam samé `idle` — to by znamenalo, že Windows si myslí, že nemá čo
  poslať, a to je úplne iná chyba

**Bez toho by som len hádal**, a to pri syncu robiť nechcem.

---

## Čo som overil a čo nie

**Overené — spustené, nie prečítané:**

- **všetkých 6 priebehov zatvárania** skončí zavretým oknom
- **povolenie `destroy`** naozaj v konfigurácii chýbalo a je doplnené
- označovanie zmien pokrýva aj tabuľky poznámok (hook je „všetko okrem piatich
  účtovných tabuliek")
- rozhodovanie syncu a porovnávanie verzií prečítané celé — bez chyby
- zátvorky, importy, kontrola na zvyšky po premenovaní

**Neoverené:** appku nezostavím (nemám Node ani Rust). A **zatváranie okna som
si naživo neodklikal** — vyskúšaj ho prosím hneď ako prvé, a keby sa appka
nezatvárala, napíš okamžite a vrátim to späť.

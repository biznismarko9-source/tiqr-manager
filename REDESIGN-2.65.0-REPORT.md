# TIQR 2.65.0 — predaj vie, ktorú objednávku predávaš

## Čo si chcel

> *„ked davam sale tak len toto cca ukazuje, treba tam doplnit nazov, datum
> a sektor row seats aspon nech to je priehladne a taktiez z inventaru ked
> prekliknes order tak mas tam tlacitku add sale nech rovno ta tam hodi"*

Dve veci. Obe hotové, ani jedna nepotrebovala zmenu backendu.

---

## 1. Rozbaľovačka objednávok

**Predtým** ukazovala len toto:

```
OASIS-004 · OASIS LIVE '27 · 2 voľných
OASIS-002 · OASIS LIVE '27 · 2 voľných
```

Dva riadky, ktoré sa líšia jedine kódom. Keby si nevedel naspamäť, čo je
OASIS-004, nemáš sa čoho chytiť — a **dve objednávky na ten istý event sú
normálny prípad**, nie okrajový.

**Teraz:**

```
OASIS-004 · OASIS LIVE '27 · 04.07.2027 · (104 · 8 · 1-2) · 2 voľných
OASIS-002 · OASIS LIVE '27 · 04.07.2027 · (212 · 3 · 7-8) · 2 voľných
```

Obe polia — dátum aj sedadlá — **už boli po ruke**. `eventDate` je na
`OrderRecord` od 2.2.10, `seats` od 2.0.38. Takže to nie je nový príkaz ani
nové pole, je to čisto popisok.

### Prečo sú sedadlá v zátvorke

Toto nie je ozdoba a skoro som to prehliadol.

`formatSeatsSummary` spája sektor, rad a sedadlá tým **istým `" · "`**, ktorým
popisok oddeľuje svoje vlastné polia. Bez zátvoriek by to vyšlo takto:

```
OASIS-004 · OASIS LIVE '27 · 04.07.2027 · 104 · 8 · 1-2 · 2 voľných
```

Nič ti nepovie, že `104 · 8 · 1-2` je **jeden údaj** a nie tri. Pri objednávke
cez dva bloky je to horšie — bodkočiarka, ktorá ich delí, sa v tom rade bodiek
stratí úplne. Zátvorka tú hranicu vracia.

Keďže sa tu nedá spustiť `tsc`, prepísal som oba formátovače do Pythonu
a pustil ich na piatich prípadoch: tvoje dve OASIS objednávky, objednávka cez
dva bloky, státie bez sedadiel, a objednávka bez dátumu aj bez sedadiel.
Posledné dva dopadli čisto — chýbajúce pole sa **preskočí**, nevykreslí sa ako
prázdne miesto medzi dvoma bodkami.

### Jedna poctivá výhrada

`OrderRecord.seats` je podľa vlastného komentára z 2.0.38 **sedadlo každého
lístka objednávky, vrátane zrušených**. Takže sedadlá v tom popisku môžu byť
širšie než to, čo je naozaj predajné. Je to tá istá výhrada, s ktorou už žije
stĺpec Seats v zozname objednávok. **Počet voľných vedľa je to autoritatívne
číslo**, sedadlá sú na rozlíšenie, nie na počítanie.

---

## 2. Tlačidlo „Add sale" na objednávke

Na Order Detail pribudlo tlačidlo, ktoré otvorí formulár predaja **s tou
objednávkou už vybranou**.

Ide cez ten istý `location.state`, ktorý Sales už používalo pre `openCreate` —
žiadny nový mechanizmus. Vybranú objednávku navyše vložím do zoznamu aj vtedy,
keď by vypadla mimo prvých 25 výsledkov, inak by sa `<select>` ukázal prázdny.

**Nezobrazí sa, keď na objednávke nie je nič predajné.** Tlačidlo, ktoré otvorí
formulár na lístky, ktoré už nemáš, je slepá ulička.

**Tlačidlo len predvyberie — lístky nepridá.** Prísť do formulára s riadkami,
ktoré si nepýtal, je zápis, o ktorý nikto nežiadal. „Add its tickets" je hneď
vedľa.

### Toto ide vedome proti staršiemu rozhodnutiu

1.9.1 a 2.45.0 vyhodili z Orders/Tickets/Sales **každý** odkaz typu „táto
zmienka ma prehodí do inej sekcie" — na tvoju vlastnú žiadosť. Je to napísané
v kóde hneď nad miestom, kam som to tlačidlo dal.

Nemyslím si, že si to protirečí, ale chcem, aby si to vedel: tamto boli
**mimovoľné odkazy**, ktoré navigovali bez toho, aby si o to požiadal. Toto je
**tlačidlo, ktoré stlačíš zámerne**. Ak to aj tak nechceš, je to jedna
podmienka na zmazanie.

---

## Čo som netestoval

Na tomto Macu nie je `npm` ani `node`, takže **nič z toho som nespustil**.
Overené je:

- logika popisku — prepisom do Pythonu a spustením na piatich prípadoch
- že `eventDate`, `seats`, `availableCount` a `listedCount` na `OrderRecord`
  naozaj existujú — prečítané z `types.ts`
- že `formatSeatsSummary` je exportované a čo presne vracia — prečítané
- že `IconPlus`, `Button` a `navigate` sú v Order Detail už naimportované
- zátvorky a zovretie v troch zmenených súboroch

**Prvý skutočný kompilátor, ktorý tento kód uvidí, je CI.**

---

## Súbory

- `src/pages/SaleRowsModal.tsx` — nový `orderOptionLabel`, prop
  `initialOrderId`, predvýber, doplnenie vybranej objednávky do zoznamu
- `src/pages/Sales.tsx` — `orderId` popri `openCreate` v `location.state`
- `src/pages/OrderDetail.tsx` — tlačidlo „Add sale"

Deväť výskytov verzie v siedmich súboroch posunutých na **2.65.0**.

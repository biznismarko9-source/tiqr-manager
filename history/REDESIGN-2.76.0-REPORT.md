# TIQR Manager 2.76.0 - Events

**Dátum:** 9.10.2026
**Zadanie:** redesign sekcie Events · plus *"ten stock cely odstran"*

---

## 1. Zmenené súbory

| súbor | čo |
|---|---|
| `src/pages/Events.tsx` | Stock stĺpec a `StockBar` odstránené; filtre v jednej lište |
| `src/pages/EventDetail.tsx` | 10 kariet dostalo ikony a akcenty; 4 prázdne stavy dostali ikonu; doplnené importy ikon |

Žiadna iná sekcia sa nedotkla. Žiadna zdieľaná komponenta sa nemenila.

## 2. Stock stĺpec

Odstránený celý, aj s komponentou `StockBar`.

Najprv som overil, či sa nemá čo rozsypať: v tej tabuľke **nič nepočíta
stĺpce podľa indexu** - žiadny `colSpan`, žiadne `rightAlign`. (Presne na
tomto som sa v 2.67.0 skoro popálil, keď vloženie stĺpca posunulo indexy
zarovnania.)

Po odstránení: **hlavička 8 buniek, riadok 8 buniek** - overené počítaním,
nie odhadom.

Nič sa pre ten stĺpec nepočítalo zvlášť. Čítal `stats.purchasedTickets` a
`stats.availableTickets`; oboje backend naďalej posiela a detail eventu
naďalej zobrazuje. **Žiadny dotaz, typ ani výpočet sa nemenil.**

## 3. Zoznam eventov

Filtre - Search, Category, From, To, Sort - sú teraz v jednej vnorenej lište
(`surface-sunken`, tá istá, akú používa zvyšok apky) namiesto piatich voľne
stojacich ovládačov nad tabuľkou.

Tie isté vstupy, tie isté handlery, tie isté šírky. Lišta len spôsobí, že sa
čítajú ako jedna skupina. Prepínač Upcoming/Completed ostáva pripnutý vpravo.

**Zachované:** vyhľadávanie, kategória, dátumový rozsah, triedenie, obe
záložky, New Event ako fialová primárna akcia, Detect categories, Delete aj
s jeho režimom výberu, odkazy na detail, kategóriové odznaky, miesto konania.

## 4. Detail eventu - Overview

Desať `StatCard` dostalo ikonu a akcent cez **voliteľné props pridané v
2.74.0**, takže sa menil len vzhľad.

Pravidlo, ktoré som zvolil a ktoré stojí za to dodržať:

- **Tri karty s počtami** (Tickets, Sold, Available) nosia brand violet.
- **Peňažné karty** berú mapovanie z Dashboardu: cost orange, revenue blue,
  profit teal, ROI emerald.

Jeden pohľad tak oddelí "koľko lístkov" od "koľko peňazí". Sedem rovnako
sfarbených štvorcov by to nespravilo.

Trojica Potential Profit nosí tie isté tri akcenty ako jej dvojička na
Dashboarde, takže odhad vyzerá rovnako na oboch obrazovkách. Rozlíšenie
realizovaného a potenciálneho zisku ostalo nedotknuté - vrátane tónovanej
zóny a textu, že ide o odhad.

## 5. Detail eventu - Sales a Finance

Prázdne stavy dostali ikonu: lístky, predaje, objednávky, Finance.

`EmptyState` ikonu aj akciu **podporoval už predtým** (medailón + plocha) -
Events ich len neposielalo. Takže som zdieľanú komponentu nemusel meniť, čo
bolo presne to, čo zadanie žiadalo uprednostniť.

Veta pri Finance - *"Open one of this event's orders and use Record in Finance
there"* - ostala nedotknutá. Už predtým menovala presnú akciu aj kde ju nájsť.

**Zachované:** tabuľka predajov, stav platby, Open in Sales, Open in Finance,
sekcia Potential Profit aj všetky jej hodnoty a vysvetlenia.

## 6. Čo som overil

**Zmerané:**

- Hlavička a riadok tabuľky majú po odstránení rovnaký počet buniek (8 = 8).
- V `Events.tsx` nie je žiadny `colSpan` ani index-based zarovnanie.
- Zátvorky v oboch súboroch: rozdiel oproti netknutým zálohám 0.
- Každá použitá ikona má import (prekontrolovaných 8 v `EventDetail.tsx`).
- `//` komentár, ktorý som pridal k prázdnemu stavu Finance, sedí v expression
  pozícii za `? (` - tam je platný. (V JSX atribútoch by nebol; na to som
  narazil v 2.71.0.)

**Jedna vec, ktorú musím priznať:**

Pri kontrole `StockBar` v prehliadači to vyzeralo, že výplň pruhu nie je
vidieť ani pri 100 %. **Chyba bola v mojej testovacej stránke, nie v apke** -
zabudol som na `display:block` na vnútornom pruhu, ktorý appka má
(`block h-full`). Bez neho má inline span nulovú výšku.

Hovorím to preto, že som bol krok od toho ohlásiť ti chybu, ktorá neexistuje.
Stĺpec medzitým aj tak padol, takže je to už jedno - ale to pravidlo platí:
keď test povie, že je niečo pokazené, najprv over test.

**NEOVERENÉ:**

- **Neskompilované.** Node ani cargo na tomto Macu nie sú - prekontroloval
  som to znova pri 2.75.0.
- **Events som nevidel bežať.** Ani zoznam, ani detail, ani záložku Sales.
- Filtrovanie, triedenie, prepínanie záložiek, Edit, Delete, výber riadkov a
  navigácia sú **kódom nedotknuté, ale ani jedno otestované za behu.**

## 7. Zostávajúce vizuálne otázky

1. Lištu filtrov som nevidel vykreslenú - pri veľmi úzkom okne sa päť
   ovládačov zalomí do dvoch riadkov vnútri nej. Malo by to vyzerať v
   poriadku, ale je to odhad, nie pozorovanie.
2. Tabuľka má o stĺpec menej, takže zvyšné dostanú viac miesta. Či to
   nevyzerá riedko, povie až pohľad.

## 8. Ďalej

Zastavujem sa po Events, ako si napísal. Na ďalšiu sekciu čakám na tvoje slovo.

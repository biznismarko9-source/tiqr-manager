# TIQR Manager 2.2.2 — Event Workspace + 3 malé opravy

Reagujem na tvoju poslednú správu (3 screenshoty + zadanie):

> *"ked si uz vyberiem danu kategoriu, a je tam toho viac tak kludne zvacsi to okno, aby sa nemuselo tolko
> scrollovat, plati u vsetkych kategoriach"*
>
> *"tu poslednu tabulku treba dat nizsie, aby nebola jedna v druhej"*
>
> *"v price checkeri mozem sledovat len tie eventy, ktore sa este konaju, tie ktore skoncili automaticky idu
> prec"*
>
> *"Chcem teraz spraviť Event Workspace. [...] Overview | Inventory | Sales | Market | Finance | Tasks [...]
> Zatiaľ sprav iba základ: Overview: tickets, sold, available, total cost, revenue, profit, margin/ROI [...]
> Inventory, Sales, Market a Finance nech používajú existujúce dáta a funkcionalitu, nevytváraj duplicity."*

Postupne, po poriadku.

## 1. Lookups — väčšie okno pri dlhších zoznamoch

Presne ako si napísal — všetky 3 zoznamy (Platforms, Event categories, Finance categories) mali zoznam
položiek zamknutý na pevných 224 pixelov výšky, bez ohľadu na to, koľko miesta okolo bolo voľné. Pri 15
platformách to znamenalo neustále scrollovanie aj v malom vnútornom okienku. Teraz sa zoznam prispôsobuje
výške okna (do 60 % výšky obrazovky) — výrazne menej scrollovania, a rovnaká zmena je vo všetkých troch
kategóriách naraz.

## 2. Event Detail — medzera pod poslednou tabuľkou

Toto som si overil priamo v kóde: tabuľka Orders mala pod sebou správnu medzeru, ale tabuľka Tickets (posledná
na stránke) ju nemala — box "Potential Profit" sa tak nalepil hneď pod ňu bez akéhokoľvek odstupu, čo pôsobilo,
akoby boli zlepené do seba. Doplnil som rovnakú medzeru, akú má Orders. Ak si mal na mysli niečo iné (napr.
inú tabuľku alebo iné miesto), pošli mi prosím čerstvý screenshot s kde presne — rád to doladím.

## 3. Price Checker — len eventy, ktoré sa ešte konajú

Výber eventu v Price Checkeri teraz zobrazuje len tie, čo majú stav "Upcoming" (presne to isté pole, aké už
používajú tvoje vlastné záložky Upcoming/Completed v Events) — akonáhle event niekde inde označíš ako
"Completed" (alebo je "Cancelled"), z Price Checkera potichu zmizne sám, netreba nič ručne mazať.

## 4. Event Workspace

Toto bola najväčšia zmena. Presne podľa zadania — Event Detail je teraz rozdelený na 6 záložiek:
**Overview | Inventory | Sales | Market | Finance | Tasks.**

- **Overview** — presne tvoj zoznam a nič naviac: Tickets, Sold, Available, Total cost, Revenue, Profit,
  Margin, ROI. Poznámky k eventu (ak nejaké má) ostali tiež tu, lebo patria k základným info o evente.
- **Inventory** — presne tie isté tabuľky Orders a Tickets, čo mal Event Detail už doteraz, len presunuté do
  vlastnej záložky. Nič na nich som nemenil.
- **Sales** — nová záložka, ale nie nová logika: použil som presne ten istý príkaz, čo používa filter "Event"
  na stránke Sales, len rovno zúžený na tento event. Tlačidlo "Open in Sales" ťa dostane na plnú stránku, ak
  chceš niečo viac než prehľad.
- **Market** — spojil som existujúci box "Potential Profit" (tvoj odhad z nepredaných lístkov, bez zmeny
  výpočtu) so skutočnými trhovými číslami (Market lowest/average, Recommended price, Expected profit/ROI) —
  presne tie isté čísla, čo ukazuje Price Checker, len automaticky načítané pre tento event. "Open in Price
  Checker" ťa dostane tam, kde sa dajú pridávať marketplacy a spúšťať skeny (to sa sem nekopírovalo, bolo by
  to zbytočné duplicitné).
- **Finance** — zoznam všetkých Finance záznamov spojených s objednávkami tohto eventu (využíva presne to
  prepojenie z minulej verzie 2.2.1). Žiadny nový príkaz na pozadí — len sa spýta na záznamy každej objednávky
  tohto eventu a spojí ich do jedného zoznamu.
- **Tasks** — túto záložku si spomenul len menom, bez ďalšej špecifikácie, tak som ju zámerne nechal ako
  jednoduché miesto "pripravuje sa" namiesto toho, aby som hádal, čo presne má obsahovať. Napíš mi, ako si
  predstavuješ jednu "úlohu" pri evente (čo sa dá pridať, čo sa dá odškrtnúť, termín?) a rád to postavím.

Nikde som nič neduplikoval — Sales/Market/Finance vždy volajú presne ten istý príkaz na pozadí, čo už appka
mala, len ho zúžia na jeden konkrétny event.

## Čo som overil

Táto verzia je celá na frontende — žiadna zmena v Ruste, žiadna nová migrácia.

```
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.2.2 build" v hlavičke)
```

Existujúca sada 934 Rust testov nie je touto verziou ovplyvnená (nemenil som žiadny `.rs` súbor), takže som ju
znova nespúšťal — bola zelená naposledy pri 2.2.1 a nič na backende sa odvtedy nezmenilo.

## Zmenené súbory

- `src/pages/Settings.tsx` — zväčšené zoznamy vo všetkých 3 Lookups modáloch
- `src/pages/EventDetail.tsx` — celý prerobený na Event Workspace (6 záložiek)
- `src/pages/PriceChecker.tsx` — filter eventov len na "Upcoming"

**Verzia (9 miest v 7 súboroch):** `2.2.2`.

## STOP

2.2.2 hotové, otestované a zabalené. Skontroluj:

1. **Settings → Lookups** — otvor ktorýkoľvek zoznam s viacerými položkami, over že sa zmestí viac riadkov
   naraz.
2. **Ktorýkoľvek Event → Event Detail** — teraz uvidíš 6 záložiek hore. Preklikaj všetky, over že čísla v
   Overview sedia, že Inventory má tvoje Orders/Tickets, že Sales/Market/Finance ukazujú správne dáta pre
   ten konkrétny event.
3. Skontroluj medzeru pod poslednou tabuľkou v **Inventory** záložke (Tickets tabuľka vs. čokoľvek pod ňou) —
   ak toto nebolo to, čo si myslel bodom 2 vyššie, daj vedieť s presnejším popisom/screenshotom.
4. **Price Checker** — otvor výber eventu, over že tam vidíš len nadchádzajúce eventy.

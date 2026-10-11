# TIQR Manager 2.78.0 — Sales

**Dátum:** 10.10.2026
**Zadanie:** zarovnať Sales s Dashboard / Events / Inventory, bez nového dizajnového jazyka

---

## 1. Zmenené súbory

| súbor | čo |
|---|---|
| `src/pages/SaleDetail.tsx` | 5 ručných kariet → `StatCard`; 6 popiskov panelu zjednotených; doplnené importy |
| `src/pages/Sales.tsx` | filtre do jednej vnorenej lišty |

To je všetko. Žiadna zdieľaná komponenta sa nemenila, žiadna iná sekcia sa
nedotkla.

## 2. Čo obhliadka našla

Sales mal **presne ten istý problém ako Inventory pred 2.77.0**: päť
metrických kariet kreslených ručne cez `<Card>` s vlastným popiskom
a hodnotou, kým Dashboard, Events a (od 2.77.0) Inventory používajú
`StatCard`.

A tie isté zastarané popisky s `uppercase`, ktoré ten štýl stratil v 3.0.

Takže nebolo treba nič vymýšľať — len dorovnať.

## 3. Päť kariet

Revenue, Fees, Cost, Profit, ROI → `StatCard`.

Profit si cez `tone` drží **presne to isté** emerald/red/neutral pravidlo,
aké mal predtým. `formatMoneyOrMixed` a `formatPercentOrMixed` ostali, takže
„Mixed" pri predaji cez viac mien sa stále zobrazí tak ako doteraz.

Akcenty podľa tvojho zadania:

| | |
|---|---|
| Revenue | blue |
| Fees, Cost | orange |
| Profit | emerald |
| ROI | teal |

Vysvetľujúci text pod kartami — *"Revenue, fees, cost, profit and ROI above
exclude any refunded ticket"* — ostal nedotknutý.

## 4. Informačný panel

Šesť popiskov, ktoré zadanie menuje (Platform, Sale date, Currency,
Refunded, Paid, Outstanding), zjednotených na `.section-title`.

Overil som, že všetkých šesť je naozaj v tom paneli — v Inventory som pri
rovnakej zmene musel dva výskyty mimo panelu vynechať, tu žiadne také neboli.

## 5. Filtre

Do jednej vnorenej lišty (`surface-sunken`). Po tomto vydaní majú **všetky
tri zoznamy tú istú lištu** — overené: jeden výskyt v `Events.tsx`,
`Orders.tsx` aj `Sales.tsx`.

Tie isté vstupy, handlery, šírky aj zalamovanie. Prepínač Pending/Completed
ostáva vpravo.

## 6. Čo už bolo v poriadku a nechal som tak

Súhrnný pruh výsledkov — Results, Tickets, Revenue, Profit, Paid,
Outstanding, Refunded — **už používal `SummaryStat`**, čo je presne tá
kompaktná súhrnná komponenta, akú zadanie žiadalo. Nedotknutý.

Tabuľka predajov tiež: všetky stĺpce, ich poradie aj význam nezmenené.

## 7. Čo som overil

**Spustené:**

| kontrola | výsledok |
|---|---|
| Props `StatCard`/`EmptyState` naprieč **siedmimi** stránkami | **58 použití, 0 chýb** |
| Hodnoty `accent` a `tone` | 0 neplatných |
| Komponenty v `Sales.tsx` / `SaleDetail.tsx` majú import | 0 nevyriešených |
| Zátvorky v oboch súboroch | rozdiel oproti zálohám 0 |
| Zvyšné ručne kreslené karty v Sales | **0** |
| Rovnaká lišta filtrov vo všetkých troch zoznamoch | áno, 1:1:1 |

Jeden falošný poplach: kontrolór hlásil `T` ako nevyriešenú komponentu —
je to generický parameter z `function uniform<T>(...)`. Overil som to, než
som čokoľvek menil.

**NESPUSTENÉ:**

- **Build, type-check, lint, testy.** Prostredie som kontroloval pri
  validácii 2.76.0 aj teraz: node, npm, cargo ani Homebrew tu nie sú.
- **Sales som nevidel bežať.** Ani zoznam, ani detail predaja.
- Pending/Completed, filtre, triedenie, navigácia, vytvorenie, úprava,
  ovládače stavu lístka/doručenia/výplaty — **kódom nedotknuté, ale ani
  jedno otestované za behu.**
- Úzke a široké okno som netestoval.

## 8. Zostávajúce otázky

1. Päť `StatCard`-ov teraz beží v mriežke `summary-bar` (min. 144px na
   kartu) namiesto pevného `grid-cols-5`. Pri širokom okne to vyjde
   rovnako; pri užšom sa zalomí skôr, než sa predtým zmenšovalo. To je
   zámer — rovnaké správanie ako všade inde — ale uvidíš to až ty.
2. Karty sú o niečo nižšie (`p-3` vs `p-4`), rovnako ako v Inventory.

## 9. Ďalej

Zastavujem sa po Sales. Na ďalšiu sekciu čakám na tvoje slovo.

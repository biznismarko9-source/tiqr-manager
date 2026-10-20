# TIQR Manager 2.75.0 - Dashboard dokončený

**Dátum:** 9.10.2026
**Zadanie:** dokončiť redesign Dashboardu (Overview + Financials + Activity), overiť, zastaviť sa pred Events

---

## 1. Prostredie - odpoveď na bod 8, hneď

Overil som to ako prvú vec:

```
node NIE JE   npm NIE JE   npx NIE JE   pnpm NIE JE
yarn NIE JE   bun NIE JE   cargo NIE JE  rustc NIE JE
node_modules: neexistuje
```

**Takže: žiadny build, žiadny lint, žiadny type-check, žiadne testy, appka
nespustená.** Nebudem predstierať opak.

Čo som namiesto toho spravil: **vykreslil som nové komponenty v prehliadači
s tvojimi reálnymi číslami a merateľne ich skontroloval.** Nájdlo to dve
chyby, ktoré by čítanie kódu nenašlo (bod 4).

## 2. Čo bolo hotové z 2.74.0 a čo zostávalo

| | stav po 2.74.0 |
|---|---|
| KPI karty (ikona, akcent, sparkline) | hotové |
| Balance Cashflow | hotový, monotónna interpolácia overená |
| Sales by Platform | komponenta postavená, **nezapojená** |
| Financials | nedotknuté |
| Activity | nedotknuté |

Toto vydanie dokončuje posledné tri riadky.

## 3. Sales by Platform - zapojené bez straty

`SalesByPlatformCard` mala zoznam platforiem s tržbou, pruhom a „N sold",
plus rozbaľovanie cez `ShowMoreToggle`.

**Prstenec som pridal NAD ten zoznam, nie namiesto neho.** Zoznam je jeho
legenda - už aj tak menuje každú platformu, jej tržbu aj počet lístkov, a už
sa aj rozbaľuje. Druhá legenda by znamenala dva rozpady na jednej obrazovke.

Preto `PlatformDonut` z 2.74.0 zmizol a nahradil ho **`PlatformRing`** -
samotný prstenec, bez vlastnej hlavičky a legendy.

Navyše: každý riadok zoznamu dostal **bodku a pruh vo farbe svojho výseku**
a percento. Farby berú obe strany z jednej funkcie `platformFill(index)`,
takže sa nemôžu rozísť. Backend už triedi platformy podľa tržby zostupne
(je na to test `sales_by_platform_groups_and_orders_by_revenue_descending`),
takže index 0 je najväčšia platforma na oboch stranách.

**Zachované:** zoznam, rozbaľovanie, počty lístkov, prázdny stav, názov
karty aj ikona.

## 4. Dve chyby, ktoré našlo až pozeranie

### 4.1 Na KPI kartách zmizli sumy

Vykreslil som riadok piatich kariet s tvojimi číslami a **hodnoty tam neboli.**
Profit, Revenue, Purchase cost aj Tickets sold ukazovali len popisok,
sparkline a trend. Číslo preč. Vidno bolo len ROI - jediná karta bez
sparklinu.

Príčina: 2.74.0 dalo hodnotu a sparkline na jeden riadok, ako na referenčnom
screenshote. Sparkline mal pevných 64px a hodnota `min-w-0 truncate`, takže
v úzkej karte prehrala a orezala sa **na nič**.

Zmerané, nie odhadnuté (`scrollWidth > clientWidth`):

```
orezany text: ANO - €3,539.61 | €7,473.78 | €3,934.17 | 41
```

**Oprava:** sparkline má vlastný riadok pod hodnotou, cez celú šírku karty.
Je to odchýlka od referencie, ktorá dáva kartám oveľa viac miesta než má
táto apka - ale hodnota, ktorá zmizne pri užšom okne, za zhodu s obrázkom
nestojí.

Po oprave, pri **najužšej karte, akú `.summary-bar` dovolí** (152px, jej
minimum je 9rem = 144px):

```
orezany text: ziadny
sparkline presahuje kartu: nie
```

### 4.2 Môj prvý test bol prísnejší než appka

Prvý beh testu používal `minmax(0,1fr)`, kým `.summary-bar` v `index.css` má
`minmax(9rem,1fr)`. Čiže môj test staval karty užšie, než apka kedy postaví.
Opravil som test na skutočné pravidlo a premeral - preto to číslo 152px
vyššie. Hovorím to preto, že inak by som ti ohlásil orezanie, ktoré by v
apke nenastalo.

## 5. Financials

**Bar chart „Revenue vs Purchase Cost" v tvojej appke NIE JE.** Prehľadal
som Dashboard na `BarChart`, `Revenue vs`, `vs Purchase` - jediné výskyty
`IconBarChart` sú import, ikona Revenue karty a ikona karty platforiem.

Ten widget je len na referenčnom screenshote, ktorý si poslal. **Nie je čo
odstraňovať**, takže som nič neodstránil a nevymyslel som si odstránenie.

Financials má tri bloky a všetky ostali: Current inventory (all time),
Cashflow (all time), Inventory & Potential Profit.

Doplnené: **ikony a akcenty ôsmim peňažným kartám** - Total cost (orange),
Revenue (blue), Profit (teal), Paid (emerald), Outstanding (orange),
Inventory cost (orange), Listing value (blue), Potential profit (teal).

Štyri karty s počtami (Available/Listed/Sold/Purchased) ostali **bez ikon**
zámerne - päť farebných štvorcov nad radom počtov je šum, a zadanie žiada
„restrained colour coding", nie farbu všade.

Rozdiel medzi realizovaným a potenciálnym ziskom ostal nedotknutý vrátane
tónovanej zóny a textu „This is an estimate, not realized profit."

## 6. Activity

`AttentionCenterBlock`, `AttentionSection` so štyrmi kartami aj Recent
events/orders/sales tam **už boli, aj s odkazmi na správne sekcie**.

Doplnené: `AlertCard` dostal **voliteľnú ikonu**. Jej podklad je jantárový,
keď počet nie je nula, a neutrálny, keď je - to je to isté pravidlo, ktoré
už má samotné číslo. Karta s nulou je dobrá správa a nemá vyzerať ako
výstraha.

Zapojené na Pulls near deadline, Pending sales, Missing listing price.
Upcoming events je `RecentCard` so zoznamom, tú som nechal.

## 7. Súbory, ktoré som zmenil

| súbor | čo |
|---|---|
| `src/components/ui.tsx` | `StatCard` - sparkline na vlastný riadok (oprava); `METRIC_ACCENTS`, `Sparkline` |
| `src/components/DashboardCharts.tsx` | `PlatformDonut` -> `PlatformRing`; `platformFill`, `PLATFORM_RING_STROKE`, `PLATFORM_SLICE_FILL`; odstránený nepoužitý `ReactNode` import |
| `src/pages/Dashboard.tsx` | prstenec v karte platforiem, farebné pruhy a percentá v zozname; ikony/akcenty na 8 Financials kartách; `AlertCard` + ikony; import `IconTag` |

Nové prezentačné komponenty: **`Sparkline`**, **`PlatformRing`**,
**`CashflowChart`** (z 2.74.0).
Znovupoužité: `Card`, `StatCard`, `RecentCard`, `ShowMoreToggle`,
`EmptyRow`, `SEGMENTED_TRACK`, `segmentedItemClass`, `MetricChart`,
`bucketLabel`, `AlertCard`, `AttentionSection`.

## 8. Čo som overil

**Spustené a zmerané v prehliadači, na tvojich reálnych dátach:**

- KPI riadok: žiadne orezanie pri 152px karte; sparkline nepresahuje kartu.
- Prstenec: segmenty v súčte presne obvod kružnice.
- Percentá v zozname: spolu 100 %.
- Cashflow (2.74.0): vrchol krivky na správnom mesiaci, odchýlka 0,5px.

**Overené čítaním:**

- Zátvorky vo všetkých troch súboroch: rozdiel oproti zálohám 0.
- Každá použitá ikona má import (prekontrolované všetkých 10).
- `ReactNode` je v `Dashboard.tsx` importovaný; z `DashboardCharts.tsx`
  odstránený, keď prestal byť potrebný.
- Bar chart v Financials neexistuje (tri nezávislé vyhľadávania).

**NEOVERENÉ - poviem to znova, lebo je to podstatné:**

- **Appka nebola skompilovaná ani spustená.**
- **Žiadnu z troch obrazoviek som nevidel bežať.** Videl som komponenty,
  nie zložený Dashboard.
- Prepínanie záložiek, perzistencia vybranej záložky, dátový filter, tlačidlá
  rýchlych akcií, notifikácie a navigácia z attention kariet - **všetko
  nedotknuté kódom, ale ani jedno otestované za behu.**

## 9. Zostávajúce rozdiely oproti referencii

1. **Sparkline je pod hodnotou, nie vedľa nej.** Dôvod v bode 4.1.
2. **Prstenec nemá vlastný výber obdobia.** Karta dedí obdobie Dashboardu;
   druhý selektor by bol druhý zdroj pravdy.
3. **Donut nemá "Other" v praxi** - máš štyri platformy, zoskupovanie sa
   zapne až od šiestej.

## 10. Čo ďalej

Events som sa **nedotkol**, ako si napísal.

Ak chceš, aby som Dashboard naozaj videl bežať a porovnal so screenshotmi:
**cloud session s node a cargo**. Tam appku skompilujem, spustím a spravím
skutočné screenshoty všetkých troch obrazoviek.

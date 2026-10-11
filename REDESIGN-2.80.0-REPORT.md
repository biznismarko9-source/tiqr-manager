# TIQR Manager 2.80.0 — Finance

**Dátum:** 10.10.2026
**Zadanie:** kompletný redesign Finance, s prioritou č. 1 — znovupoužiť Dashboard cashflow graf

---

## 1. Zmenené súbory

| súbor | čo |
|---|---|
| `src/pages/finance/Overview.tsx` | `CashflowChart` zapojený ako predvolený režim; 5 kariet dostalo ikony a akcenty; filtračná lišta; importy |
| `src/pages/finance/Transactions.tsx` | filtračná lišta |
| `src/pages/finance/Reports.tsx` | filtračná lišta |

**`Dashboard.tsx` ani `DashboardCharts.tsx` sa nezmenili** — overené časovou
značkou. Žiadna zdieľaná komponenta sa nemenila.

## 2. Priorita č. 1 — graf

### Čo audit našiel

Finance graf **už znovupoužíval Dashboard komponentu**. V 2.45.0 si o to
sám požiadal (*"vo finance ako je income vs expense widget tak urobit taky
isty graf ako ktory je na dashboarde"*) a vtedy to bol `MetricChart`.

Lenže Dashboard medzitým dostal **druhý** graf — `CashflowChart` v 2.74.0,
ten s dvoma plynulými krivkami a gradientmi. To je ten, ktorý myslíš, a to
je presne tá medzera.

### Ako som to vyriešil

Symetricky. Karta grafu vo Finance teraz nesie **oboje**, presne ako tá na
Dashboarde:

- pilulka **Cashflow**, predvolená → `CashflowChart`
- tri pilulky **Income / Expenses / Net** → `MetricChart`, nedotknuté

### Je to skutočná komponenta, nie napodobenina

`import { CashflowChart } from "../../components/DashboardCharts"`.

Žiadne kopírovanie CSS, žiadny druhý graf, ktorý by sa postupne rozišiel.
Ten istý súbor, tá istá monotónna interpolácia, tie isté gradienty, mriežka,
osi, zvýraznený bod aj riadok s hodnotou.

**Nebolo treba Dashboard refaktorovať.** Keď som `CashflowChart` v 2.74.0
písal, dal som mu dátovo agnostické API — `points` plus `formatValue`, ktorý
patrí volajúcemu — práve preto, aby ho mohol použiť ktokoľvek. Tu sa to
vyplatilo: Dashboard je bitovo nedotknutý.

### Adaptér

Jeden `SeriesBucket` → jeden `CashflowPoint`. Tri riadky.

**Nič sa neprepočítava.** `incomeCents` a `expenseCents` sú tie isté
per-bucket hodnoty, aké kreslí metrický graf pod tým, a `bucketLabel` je
Dashboardov vlastný exportovaný formátovač — takže oba grafy napíšu mesiac
rovnako.

### Nadpis

V režime Cashflow ukazuje **net za obdobie** — tú istú hodnotu, akú má karta
„Net Cash Flow" nad ním, **podanú zhora**, nie sčítanú z vykreslených
bucketov.

To je dôležité: `buildSeries` strope „All time" na 24 mesiacov, takže súčet
vykreslených bodov by ticho nesedel s kartami na tej istej obrazovke. Pôvodný
kód na to mal komentár a ja som to pravidlo zachoval.

## 3. Karty a filtre

Päť `StatCard`-ov dostalo ikony a akcenty — Current Balance emerald, Owed to
you orange, Income blue, Expenses orange, Net Cash Flow teal. Akcent hovorí,
čo tá hodnota **je**, nie je to dekorácia.

Tri filtračné lišty do vnorenej lišty. Po tomto vydaní má **osem líšt
naprieč siedmimi súbormi** rovnaký tvar — overené počítaním.

## 4. Čo už bolo v poriadku

Finance bolo na tom lepšie než Inventory a Sales boli:

- `StatCard` už používalo (6 výskytov) aj `summary-bar`
- **žiadne ručne kreslené metrické karty** v žiadnom zo štyroch podpohľadov
- zastarané popisky s `uppercase` tam nie sú — okrem nadpisu grafu, ktorý
  zámerne kopíruje Dashboard a preto ostal

## 5. Čo som NEUROBIL — povedané rovno

Tvoje zadanie píše „complete visual redesign" všetkých štyroch podpohľadov,
vrátane prestavby tabuľky transakcií, zoznamu účtov, rozložení reportov
a všetkých dialógov.

**To som neurobil.** Spravil som prioritu č. 1 (graf, poriadne a
znovupoužitím) plus konzistenčný prechod, ktorý Finance dorovnal so zvyškom
apky.

Dôvod poviem rovno: bez kompilátora je prestavba štyroch podpohľadov naraz —
2 782 riadkov, štyri dialógy, tabuľka s desiatimi stĺpcami — tá najistejšia
cesta, ako rozbiť fungujúcu sekciu o peniaze. Tvoje vlastné pravidlo č. 2 je
zachovať 100 % funkčnosti.

Ak to chceš celé, poďme na to po podpohľadoch — Transactions zvlášť, Accounts
zvlášť, Reports zvlášť. Každý bude overiteľnejší a pôjde vydať samostatne.

## 6. Čo som overil

**Spustené:**

| kontrola | výsledok |
|---|---|
| Props `StatCard`/`EmptyState` naprieč **12 stránkami** | **71 použití, 0 chýb** |
| Hodnoty `accent` a `tone` | 0 neplatných |
| Komponenty v troch zmenených súboroch majú import | 0 nevyriešených |
| `bucketLabel` je exportovaný z `MetricChart` | áno |
| `Granularity` = `"day" \| "month"` sedí s tým, čo `bucketLabel` čaká | áno |
| `useMemo` importovaný v Overview | áno |
| `Dashboard.tsx` a `DashboardCharts.tsx` nedotknuté | áno |
| Zátvorky v troch súboroch | rozdiel oproti zálohám 0 |
| Rovnaká lišta filtrov naprieč apkou | 8 líšt, 7 súborov, jeden tvar |

Jeden falošný poplach: `PendingSummary` hlásené ako nevyriešená komponenta —
je to `interface`, a môj kontrolór hľadal len `type`. Overil som to.

**NESPUSTENÉ:**

- **Build, type-check, lint, testy.** Node, npm, cargo ani Homebrew tu nie sú.
- **Finance som nevidel bežať.** Ani jeden zo štyroch podpohľadov.
- **Nevidel som ten graf vykreslený s finančnými dátami.** Matematiku
  `CashflowChart` som overil v 2.74.0 na reálnych číslach (vrchol krivky na
  správnom mesiaci, odchýlka 0,5px), ale to boli predajné dáta, nie účtovné.
- Dátové filtre, Personal/Business, prepínanie metrík, vytvorenie/úprava/
  mazanie transakcií a účtov — **kódom nedotknuté, ale neotestované za behu.**

## 7. Akceptačné kritérium zo zadania

*"If screenshots of the Dashboard chart and Finance chart are placed side by
side, they should clearly look like the same chart component."*

Budú — je to **doslova tá istá komponenta**, nie podobná. Rozdiel môže byť
len v dátach, popiskoch a hodnotách, presne ako si napísal.

Potvrdiť to screenshotmi ale neviem, kým appku nespustím.

## 8. Ďalej

Zastavujem sa. Notes ani Settings som sa nedotkol.

# TIQR Manager 2.79.0 — Pulls

**Dátum:** 10.10.2026
**Zadanie:** zarovnať Pulls s Inventory a Sales

---

## 1. Zmenené súbory

| súbor | čo |
|---|---|
| `src/pages/Pulls.tsx` | prepínač Given/Received na zdieľané tokeny; obe filtračné lišty zoskupené; doplnené importy |

To je všetko — **jeden súbor**. Žiadna zdieľaná komponenta sa nemenila, takže
bod 8 („ak zmena zdieľanej komponenty ovplyvní inú sekciu, over ju") nemal čo
overovať.

## 2. Čo obhliadka našla

Toto je prvá sekcia, kde bolo menej práce, než zadanie čakalo. Tri veci, ktoré
si pýtal, už boli hotové alebo sa netýkajú:

**`FormGroup` sa už používa.** Oba modály majú polia zoskupené — päť skupín:
„Pull", „Seats & details", „Fee & tracking" v Given, „Pull" a „Fee & link"
v Received. Presne to vizuálne zoskupenie, aké bod 6 popisuje. Nedotkol som sa
ho.

**Pulls nemá detail stránku.** Hľadal som `/pulls/:id` — neexistuje.
`PullFormModal` sa otvára priamo zo zoznamu, lebo pull nemá žiadne podradené
entity (negeneruje lístky). Bod 7 teda nemá čo zarovnávať.

**Zastarané popisky tam nie sú.** V Inventory aj Sales som musel opravovať
`text-xs font-medium uppercase` — v Pulls je **0 výskytov**. Bolo to čisté už
predtým.

## 3. Čo som naozaj zmenil

### Prepínač Given / Received

Toto bola jediná skutočná nekonzistencia. Bol kreslený ručne:

```
<div className="inline-flex rounded-lg border border-slate-200 ... p-0.5">
  <button className={category === c ? "bg-brand-600 text-white" : "..."}>
```

kým zvyšok apky segmentuje cez `SEGMENTED_TRACK` + `segmentedItemClass` —
Dashboard taby, prepínač grafu, aj `TabSwitcher`, ktorý používajú Events,
Inventory a Sales.

Prevedený na zdieľané tokeny. Stojí za to povedať, že **aktívny stav bol už
predtým byte-identický** (`bg-brand-600 text-white`) — menila sa len dráha
(`bg-surface-sunken` namiesto rámika) a neaktívne/focus štýlovanie. Tie isté
dve tlačidlá, ten istý handler, ten istý stav. Pribudlo `aria-pressed`.

### Filtre

Obe vetvy dostali tú istú vnorenú lištu, akú majú ostatné zoznamy.

Po tomto vydaní má **päť filtračných líšt naprieč štyrmi súbormi** rovnaký
tvar — overené počítaním: Events 1, Orders 1, Sales 1, Pulls 2.

## 4. Čo je zachované

- **Rozdiely medzi Given a Received** — iné polia, iné filtre, iné stĺpce.
  Zadanie výslovne hovorilo ich nezjednocovať, a ja som sa ich nedotkol.
- Stavy (paid/unpaid, done/not done, completion), výpočty poplatkov,
  pravidlá dokončenia.
- Vyhľadávanie, filtre, triedenie, výber riadkov, hromadné mazanie.
- Oba modály — polia, poradie, validácia, podmienené polia, ukladanie.
- `StatusDots`, `TableSkeleton`, `EmptyState` aj varovanie pred termínom.

## 5. Čo som overil

**Spustené:**

| kontrola | výsledok |
|---|---|
| Props `StatCard`/`EmptyState` naprieč **ôsmimi** stránkami | **60 použití, 0 chýb** |
| Hodnoty `accent` a `tone` | 0 neplatných |
| Komponenty v `Pulls.tsx` majú import | 0 nevyriešených |
| Zátvorky v `Pulls.tsx` | rozdiel oproti zálohe 0 |
| Zvyšné ručne kreslené segmenty | **0** |
| Rovnaká lišta filtrov naprieč zoznamami | 5 líšt, 4 súbory, jeden tvar |

**NESPUSTENÉ:**

- **Build, type-check, lint, testy.** Prostredie som kontroloval naposledy
  pri validácii 2.76.0: node, npm, cargo ani Homebrew tu nie sú.
- **Pulls som nevidel bežať.** Ani Given, ani Received, ani jeden modál.
- Vyhľadávanie, filtre, triedenie, prepínanie Given/Received, vytvorenie,
  úprava, mazanie, ovládače stavov — **kódom nedotknuté, ale ani jedno
  otestované za behu.**
- Screenshoty som nerobil.

## 6. Zostávajúce otázky

1. Prepínač je teraz `text-xs` namiesto `text-sm` (to nesie
   `segmentedItemClass`), čiže o kúsok menší. Je to zámer — rovnaký ako
   všade inde — ale je to viditeľná zmena v hlavičke stránky.
2. Lištu filtrov v oboch vetvách som nevidel vykreslenú. Given má viac
   ovládačov než Received, takže sa zalomí skôr; malo by to byť v poriadku,
   ale je to odhad.

## 7. Ďalej

Zastavujem sa po Pulls. Finance, Notes ani Settings som sa nedotkol.

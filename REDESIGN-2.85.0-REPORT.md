# TIQR Manager 2.85.0 — globálny prečesávací prechod

**Dátum:** 11.10.2026
**Zadanie:** master prompt na redesign celej aplikácie

---

## A. Zhrnutie — a jedna vec rovno na začiatku

Tvoj master prompt žiada redesign celej apky. **Väčšina z neho je hotová** —
robili sme to posledných jedenásť vydaní:

| vydanie | sekcia |
|---|---|
| 2.74.0, 2.75.0 | Dashboard — KPI karty, Balance Cashflow, prstenec platforiem |
| 2.76.0 | Events |
| 2.77.0 | Inventory |
| 2.78.0 | Sales |
| 2.79.0 | Pulls |
| 2.80.0–2.82.0 | Finance — všetky štyri podpohľady + znovupoužitie grafu |
| 2.83.0 | Notes |
| 2.84.0 | Settings |

A bod 1.1 toho promptu hovorí presne to, čo som urobil:
*"Preserve previous work that is already correct... Extend and refine them
instead of accidentally reverting them."*

Takže som **nezačínal odznova**. Namiesto toho som spravil tie dve fázy,
ktoré hotové naozaj neboli: **B (globálny shell)** a **G (prečesávací
prechod naprieč celou apkou)**.

## B. Po sekciách

| oblasť | tento release | kedy sa robila |
|---|---|---|
| **Globálny shell** | **preskúmaný, nedotknutý** — dôvod nižšie | — |
| **Dizajnový systém** | 3 posledné odchýlky zjednotené | priebežne |
| Dashboard | 1 prázdny stav (+1 zdokumentovaná výnimka) | 2.74–2.75 |
| Events | — | 2.76 |
| Inventory | 1 prázdny stav | 2.77 |
| Sales | 1 prázdny stav | 2.78 |
| Pulls | — | 2.79 |
| **Finance** | **3 ovládače + 2 prázdne stavy** | 2.80–2.82 |
| Notes | — | 2.83 |
| Settings | 1 ovládač + 1 prázdny stav | 2.84 |
| Zdieľané komponenty | `Recap.tsx` prázdny stav | priebežne |

## C. Čo prechod našiel

### Tri posledné ručne kreslené segmentované ovládače

Sken celej apky — a jeden nález bol prekvapivý:

**`Finance.tsx` — hlavný štvorzáložkový pruh sekcie Finance.** Jeho
**vlastné podstránky** už zdieľaný ovládač používali (Transactions od
2.82.0, Overview pri grafe), len jeho navigácia nie. Presne tá vec, ktorú
prejde len systematický sken a nie práca po sekciách.

Plus `finance/Reports.tsx` (výber obdobia) a `finance/Accounts.tsx`
(Personal/Business v opakovanom výdavku).

Vo všetkých troch bol aktívny stav **už predtým byte-identický**
(`bg-brand-600 text-white`) — menia sa dráha, neaktívne farby a focus ring,
plus pribudlo `aria-pressed`. Tie isté tlačidlá, handlery a stav.

### Prázdne stavy: 20 z 27 malo ikonu

Doplnených **šesť**. Jeden z nich si zaslúži zmienku: `Recap.tsx` používa
`EmptyState` na zobrazenie **chyby**, nie neprítomnosti. Dostal výstražnú
ikonu, aby to bolo vidieť skôr, než sa veta prečíta.

Siedmy ikonu **zámerne nedostal**: `EmptyRow` na Dashboarde je generický
riadok, ktorý používa veľa kariet s rôznymi vetami („No sales yet", „No
upcoming events"…), takže akákoľvek jedna ikona by bola pre väčšinu z nich
nesprávna. Zapísal som to priamo do kódu ako dokumentovanú výnimku, nie ako
prehliadnutie.

Stav po prechode: **26 z 27 s ikonou, 1 zdokumentovaná výnimka.**

## D. Globálny shell — preskúmaný, nedotknutý

Fáza B žiada prerobiť bočný panel. Pozrel som si ho a **nerobil som nič**,
a chcem povedať prečo.

Má zdokumentované iterácie z 2.6.0, 2.29.0 a 2.43.0, s úvahami o tom, ako
sa aktívna položka hľadá v 192-pixelovom pruhu: tint plus akcentový prúžok,
pretože samotný tint sa strácal vedľa hoveru. Focus ring, uvážené rozmery,
jedna definícia pre celý panel.

Je to jedna z najstarostlivejšie navrhnutých častí apky. Prerábať ju kvôli
tomu, že to stojí v zozname, by bola churn — a bod 17 zakazuje
*"unnecessary full-page rewrites"*.

## E. Zmenené súbory

| súbor | čo |
|---|---|
| `src/pages/Finance.tsx` | štvorzáložkový pruh na zdieľané tokeny |
| `src/pages/finance/Reports.tsx` | výber obdobia |
| `src/pages/finance/Accounts.tsx` | prepínač Personal/Business |
| `src/pages/finance/Overview.tsx` | 2 prázdne stavy |
| `src/pages/OrderDetail.tsx` | 1 prázdny stav |
| `src/pages/SaleDetail.tsx` | 1 prázdny stav |
| `src/pages/Settings.tsx` | 1 prázdny stav |
| `src/components/Recap.tsx` | 1 prázdny stav (chybový) |
| `src/pages/Dashboard.tsx` | zdokumentovaná výnimka `EmptyRow` |

## F. Overenie

| kontrola | výsledok |
|---|---|
| `StatCard`/`EmptyState` props naprieč stránkami **aj komponentami** | **76 použití, 0 chýb** |
| `EmptyState` s ikonou | **26 / 27**, jedna zdokumentovaná výnimka |
| Nevyriešené identifikátory v zmenených súboroch | žiadne |
| Zátvorky v 9 zmenených súboroch | rozdiel oproti zálohám 0 |
| Zvyšné ručne kreslené pilulkové pruhy | **0** |

## G. Vizuálne overenie

**Žiadne.** node, npm, cargo ani Homebrew tu nie sú — prehľadal som to
opakovane (PATH, 12 umiestnení, `find`, shell profily).

Bod 1.3 tvojho promptu hovorí: *"Static code inspection is not runtime
testing."* Súhlasím a hovorím rovno: **nič v tomto vydaní, ani v jedenástich
pred ním, nebolo spustené.**

## H. Čo ostáva nedokončené

Toto je dôležitejšie než zoznam toho, čo je hotové.

1. **Nič z jedenástich vydaní nebolo skompilované ani spustené.** To je
   najväčšie neoverené riziko celého tohto redesignu.
2. **Notes editor** (2.83.0) — 1 295 riadkov s vlastným save mechanizmom,
   zámerne nedotknutý. Bod 11 žiada hlbší UX redesign editora; ten som
   nedodal a povedal som to vtedy aj teraz.
3. **Finance podpohľady** (2.82.0) — tabuľky a dialógy som zjednotil
   a doladil, ale neprestaval. Bod 10 miestami chce viac.
4. **Akceptačné kritérium pre graf** — Dashboard a Finance používajú
   doslova tú istú komponentu, čo viem z kódu. Dva screenshoty vedľa seba
   som nespravil, lebo appku nespustím.

Keď budeš mať prostredie s node a cargo, prvá vec, ktorú by som spravil, je
prejsť všetkých jedenásť vydaní buildom a pozrieť si každú sekciu naživo.
Dovtedy je všetko vyššie **overené čítaním a počítaním, nie pohľadom**.

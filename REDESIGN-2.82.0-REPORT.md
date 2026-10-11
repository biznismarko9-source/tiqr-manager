# TIQR Manager 2.82.0 — Finance dokončené

**Dátum:** 10.10.2026
**Zadanie:** dokončiť celú sekciu Finance v jednom kroku

---

## 0. Prostredie

**node, npm, cargo ani Homebrew tu nie sú.** Takže build, type-check, lint,
testy ani spustenie **neprebehli**. Bod 12 to pripúšťa a žiada povedať rovno —
hovorím.

## 1. Zmenené súbory

| súbor | čo |
|---|---|
| `src/pages/finance/Transactions.tsx` | farba sumy výdavku; dva prepínače formulára; ikona prázdneho stavu; import |
| `src/pages/finance/Reports.tsx` | farebné pruhy rozpadu; ikona prázdneho stavu; import |
| `src/pages/finance/Accounts.tsx` | ikona prázdneho stavu; import |
| `src/components/ui.tsx` | `FormGroup` povýšený a exportovaný |
| `src/pages/Orders.tsx` | lokálna kópia `FormGroup` nahradená importom |
| `src/pages/Pulls.tsx` | to isté |

`Overview.tsx` a `FinanceCategoryBadge.tsx` boli hotové v 2.80.0/2.81.0.

**`Dashboard.tsx` ani `DashboardCharts.tsx` sa nezmenili** — overené časovou
značkou.

## 2. Audit ma dvakrát zastavil

Toto považujem za dôležitejšie než samotné zmeny.

**Tabuľkové triedy.** Finance používa `.th`/`.td`, kým zoznamy ako Sales
a Orders používajú `.th-c`/`.td-c`. Vyzeralo to ako nekonzistencia a skoro som
to „zjednotil".

Bola by to chyba. `.th`/`.td` je **legitímna rozvoľnenejšia konvencia**
(px-3 py-2.5 oproti px-2 py-2), ktorú používa aj Dashboard, Settings
a EventDetail. Index.css ich drží ako dve samostatné triedy zámerne.
Nechal som ju.

**Rozlíšenie príjmu a výdavku.** Zadanie žiada, aby nestálo len na farbe.
Už nestojí: každý riadok má `Badge` so slovom *Income* / *Expense* **aj**
znamienko `+`/`−` pred sumou. Nemenil som to, lebo nebolo čo.

## 3. Čo som naozaj zmenil

### Transactions

**Suma výdavku je rose, nie neutrálna sivá.** Príjem bol už emerald, a karta
Cashflow Forecast na Overview používa pre výdavky `text-rose-600` od 2.1.0 —
takže v rámci jednej sekcie mala tá istá vec dve farby. Zvolil som ten istý
tlmený rose, nie krikľavú červenú: účtovná kniha, kde každý druhý riadok
kričí, sa číta horšie, nie lepšie.

Znamienko a odznak typu som nechal — rozlíšenie nikdy nestálo len na farbe
a ani teraz nestojí.

**Prepínače Type a Scope** vo formulári prešli na zdieľané
`segmentedItemClass`, rovnako ako Given/Received v Pulls (2.79.0). Aktívny
stav bol už predtým byte-identický; mení sa dráha, neaktívne farby a pribudol
focus ring.

`SEGMENTED_TRACK` som tu **ne**použil, a je to zámer: nesie `w-fit`, kým tieto
dva majú vyplniť bunku mriežky. Dráha si teda ponechala vlastnú šírku
a prevzala len povrch.

### Reports

Pruhy v rozpade nosia farbu kategórie — rovnaká oprava, akú dostal Overview
v 2.81.0, cez tú istú `financeCategoryFill`. Rozpad podľa účtu farbu nemá,
takže si drží fialovú, ktorú mal.

### Accounts

Prázdny stav opakovaných výdavkov dostal ikonu. Ten druhý ju už mal.

## 4. `FormGroup` povýšený

Žil ako **bitovo zhodná** kópia v `Orders.tsx` aj `Pulls.tsx` — porovnal som
ich riadok po riadku, než som sa čohokoľvek dotkol. Ich vlastný komentár
hovoril, že ostáva lokálny, lebo *"still only a handful of forms want this
kind of sectioning"*.

Teraz je v `ui.tsx`, exportovaný. Markup je **nezmenený**, takže Orders aj
Pulls renderujú presne to, čo predtým — presunula sa definícia, nič sa
nereštylovalo.

Overené: jeden export, nula lokálnych kópií, oba súbory importujú z `ui`,
a **všetkých 9 použití** (5 v Pulls, 4 v Orders) sa stále vyrieši.

## 5. Čo som NEVNÚTIL — a priznávam prečo

`FormGroup` do transakčného formulára, čo bol pôvodný dôvod toho povýšenia.

Vnucuje `grid grid-cols-2 gap-4` na svoje deti. Ten formulár má vlastné
mriežky — `grid-cols-[1fr_110px]` pre sumu a menu. Obaliť ho do `FormGroup`
by to rozbilo, a to kvôli zoskupeniu, ktoré formulár aj tak už má cez `Field`
a `space-y-3`.

Povýšenie tým nestráca zmysel — duplikát je preč a je to overené. Ale dôvod,
ktorý ma k nemu viedol, nevyšiel, a nebudem predstierať, že áno.

## 6. Dialógy (bod 8)

Prešiel som ich. **Už sú v dizajnovom systéme**: `Modal`, `ModalFooter`,
`Field`, `Select`, `Input`, `Textarea`, `ConfirmDialog` — všetko zdieľané
komponenty. Transactions má 1 modál a 3 potvrdenia, Accounts 3 modály
a 3 potvrdenia.

Zmenil som v nich jedinú vec: tie dva prepínače. Zvyšok nepotreboval nič,
čo by nebolo prepisovanie fungujúceho kvôli vkusu — čo bod 11 zakazuje.

## 7. Overené

| kontrola | výsledok |
|---|---|
| Props `StatCard`/`EmptyState` naprieč **28 stránkami** | **75 použití, 0 chýb** |
| Hodnoty `accent` a `tone` | 0 neplatných |
| Všetky štyri Finance pohľady: každý identifikátor má import | 0 nevyriešených |
| `FormGroup` po povýšení | 1 export, 0 kópií, 9 použití platí |
| Zátvorky v šiestich zmenených súboroch | rozdiel oproti zálohám 0 |
| `Dashboard.tsx` / `DashboardCharts.tsx` | nedotknuté |

## 8. NEOVERENÉ

- **Build, type-check, lint, testy** — nedá sa.
- **Finance som nevidel bežať.** Ani jeden zo štyroch pohľadov, ani jeden
  dialóg.
- **Ten cashflow graf som stále nevidel vykreslený s účtovnými dátami.**
  Akceptačné kritérium „oba screenshoty vedľa seba" teda **splnené nie je**.
  Je to doslova tá istá komponenta — to viem z kódu — ale potvrdiť to
  obrázkami neviem.
- Filtre, scope, triedenie, vytváranie/úprava/mazanie transakcií a účtov,
  reporty — kódom nedotknuté, neotestované za behu.

## 9. Riziká

Najvyššie riziko nesie **povýšenie `FormGroup`**, lebo sa dotklo dvoch
súborov mimo Finance. Zmiernil som ho tým, že som overil bitovú zhodu oboch
kópií pred presunom a po ňom skontroloval, že sa všetkých 9 použití vyrieši.
Markup je nezmenený, takže vizuálne by Orders a Pulls mali vyzerať presne
ako predtým — ale aj to je overenie čítaním, nie pohľadom.

Zvyšok je výmena tried a tri ikony.

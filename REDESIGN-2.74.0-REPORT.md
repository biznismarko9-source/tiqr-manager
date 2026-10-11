# TIQR Manager 2.74.0 - redesign, fáza 3-4

**Dátum:** 9.10.2026
**Zadanie:** master prompt na kompletný vizuálny redesign + referenčné screenshoty

---

## 0. Jedna vec na rovinu, hneď

Tvoj bod 13 žiada spustiť appku, pozrieť si Overview/Financials/Activity a
porovnať so screenshotmi. **Na tomto Macu to neviem** - nie je tu node ani
cargo. A ty sám tam píšeš *"do not claim that visual verification was
performed if it was not"*, tak to hovorím vopred, nie v poznámke pod čiarou.

Čo som namiesto toho spravil: hlavný graf som **naozaj vykreslil v prehliadači
s tvojimi reálnymi číslami** a zmeral na ňom, či neklame. Našlo to chybu,
ktorú by čítanie kódu nenašlo (bod 4 nižšie). Integrovanú appku overí CI a ty.

Ak chceš, aby som to vedel overiť naozaj: **cloud session s node a cargo** by
to vyriešila - tam by som appku skompiloval aj spustil.

## 1. Čo sa ukázalo pri analýze (fáza 1)

Dôležitejšie než čokoľvek iné: **väčšina toho, čo referencia žiada, už
existuje.**

- Dashboard už má Overview/Financials/Activity, perzistované v `app_settings`.
- Už má segmentovaný prepínač, `StatCard`, `SalesByPlatformCard` aj
  rozloženie "graf + platformy vedľa".
- `DashboardData` už nesie `cashflow`, `revenueTimeSeries`, `salesByPlatform`
  aj `previousPeriod` (= trendy).
- Päť KPI metrík z referencie (Profit, Revenue, Purchase cost, ROI, Tickets
  sold) tam už je, aj s percentami oproti minulému obdobiu.
- Grafová knižnica žiadna - je tam ručné SVG (`MetricChart`), ktoré už vie
  gradient, mriežku aj zvýraznený bod.

Takže je to naozaj vizuálna práca, presne ako píšeš. Nič som nemusel
vymýšľať a nič som nemusel prepisovať.

## 2. StatCard: ikona, akcent, sparkline

Tri **voliteľné** props. Každý existujúci volajúci - Event Detail, Sales,
Pulls, Finance - renderuje presne to, čo predtým. Pridáva ich len Overview.

Akcenty podľa tvojej referencie: teal (Profit), blue (Revenue), orange
(Purchase cost), emerald (ROI), violet (Tickets sold).

Ten violet ukazuje na **brand rampu**, nie na Tailwindov vlastný violet. Tú
rampu si vybral ty sám zo živého náhľadu a `PROTECTED_AREAS` hovorí nesiahať
na ňu bez opýtania - tak ju redesign používa namiesto toho, aby vedľa nej
postavil druhú fialovú.

**Sparkliny sú reálne.** Každá je stĺpec z `revenueTimeSeries` - tie isté
buckety, ktoré kreslí graf pod tým. ROI žiadnu nemá: na tom rade per-bucket
ROI neexistuje a čiara vymyslená do diery je presne to, čo táto apka robiť
nesmie.

## 3. Balance Cashflow

Income = `revenueCents` bucketu. Expenses = `cogsCents + sellingFeesCents`.
Oboje z dát, ktoré už máš. Veľké číslo hore berie `data.period.profitCents` -
nie prepočet z bucketov - takže graf a Profit karta nad ním sa nemôžu rozísť.

Má: plynulé krivky, priesvitné gradienty pod nimi, striedmu mriežku, tlmené
popisky, zvýraznený bod s prerušovanou vodiacou čiarou a riadok s hodnotou.

**Pridaný ako štvrtá možnosť** na existujúci prepínač metrík, nie namiesto
neho. Revenue/Profit/Cost je reálna funkcia od 1.7.5 a redesign ju nemaže
kvôli screenshotu. Cashflow je východzí; ostatné sú jeden klik ďaleko.

## 4. Chyba, ktorú našlo až pozeranie

Prvá verzia používala Catmull-Rom interpoláciu. Pustil som ju v prehliadači
na tvojich vlastných štyroch mesiacoch:

| mesiac | income | expenses |
|---|---|---|
| Apr | 700,00 € | 270,00 € |
| Aug | 2 494,32 € | 1 384,22 € |
| Sep | 3 771,80 € | 2 113,75 € |
| Oct | 508,66 € | 166,20 € |

**Vrchol krivky pristál medzi augustom a septembrom**, nie na septembri.
Prudký pád do októbra ťahá predchádzajúci kontrolný bod. Kto by z toho grafu
čítal najvyšší mesiac, dostal by zlú odpoveď - a vlastný komentár v tom kóde
pritom hovorí *"a line that rounds past the actual number is a chart that
lies politely"*.

Nahradené **Fritsch-Carlson monotónnou interpoláciou**: kde dáta rastú,
krivka rastie; kde klesajú, klesá; a nikdy neopustí interval medzi dvoma
susednými hodnotami. Vrchol je ploché tangens, čo je presne tá jedna vec,
ktorá bránila prestreleniu.

Overené **meraním na skutočne vykreslenej ceste** (`getPointAtLength`, 400
vzoriek):

```
najvyssi mesiac: Sep na x=513.3
vrchol krivky na x=512.8   odchylka 0.5px -> OK
```

Pri tom istom pohľade sa ukázalo aj to, že osové popisky 9px vo viewBoxe
760px sa renderujú ako ~5px - nečitateľné. Zväčšené na 11px, vodiacich čiar
5 -> 3.

## 5. Čo som NEZAPOJIL a prečo

**PlatformDonut** je postavený a overený (segmenty v súčte presne obvod,
percentá 100 %), ale **zatiaľ nezapojený.** `SalesByPlatformCard` má
rozbaľovanie zoznamu, ktoré by sa výmenou stratilo. Tvoje pravidlo č. 1 je
"preserve the existing application", tak to robím v ďalšom kroku poriadne,
nie teraz ticho.

**Fázy 5-9** - Financials, Activity, Events, Inventory, Sales, Pulls, Finance,
Notes, Settings - nie sú v tomto vydaní.

Dôvod poviem rovno: bez kompilátora je rozdiel cez osem obrazoviek naraz tá
najistejšia cesta, ako rozbiť fungujúci produkt. `StatCard` a tokeny sa
pritom prejavia **všade**, kde sa používajú, takže zdieľaný jazyk už posunutý
je. Zvyšok ide hneď, ako uvidím, že toto prešlo buildom.

## 6. Čo je otestované a čo nie

**Naozaj spustené:**

- Matematika oboch grafov v prehliadači, na tvojich reálnych dátach.
  Všetky kontroly prešli: žiadne NaN, všetky body v ploche, segmenty donutu
  presne obvod, percentá 100 %, vrchol krivky na správnom mesiaci.

**Overené čítaním:**

- Zátvorky vo všetkých štyroch dotknutých súboroch: rozdiel oproti zálohám 0.
- `React.ReactNode` som najprv použil v súbore, ktorý `React` neimportuje -
  s novým JSX transformom by to bola chyba. Opravené na `ReactNode`.
- Štyri ikony (`IconDatabase`, `IconWallet`, `IconGauge`, `IconTicket`) som
  najprv použil bez importu. Doplnené a prekontrolované.
- `bucketLabel` exportovaný z `MetricChart`, aby oba grafy písali mesiac
  rovnako.
- Verzia 2.74.0 vo všetkých 9 výskytoch v 7 súboroch.

**Neoverené:**

- **Neskompilované.**
- **Dashboard som nevidel bežať.** Komponenty áno, zloženú stránku nie.

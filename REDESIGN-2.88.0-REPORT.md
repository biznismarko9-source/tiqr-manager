# TIQR Manager 2.88.0 — report

**Čo si zadal:** „odstran to kolecko ten donut a tie male grafy odstran"

Oboje je preč.

---

## 1. Donut v „Sales by platform"

Krúžok (`PlatformRing`) nad zoznamom je odstránený. Karta teraz začína rovno
zoznamom platforiem.

Nič sa tým nestratilo, a to je jediný dôvod, prečo to bolo jednoduché:

- **Zoznam pod ním bol vždy ten skutočný rozpad.** Menuje platformu, sumu,
  percento aj počet predaných kusov, a dá sa rozbaliť. Krúžok ho len
  zopakoval.
- **Stredové číslo krúžku** (`7 913,78 € / Revenue`) je presne to isté, čo
  ukazuje karta **Revenue** hore na Overview. Nebolo to nikde inde.

Čo zostalo: `platformTotal`. To nie je len stredové číslo krúžku — je to
menovateľ pre percentá v samotnom zozname (43 %, 31 %, 15 %, 11 %). Keby som
ho odstránil spolu s krúžkom, zmiznú percentá.

Opravil som aj dva komentáre v kóde, ktoré by inak klamali: jeden hovoril, že
farebný štvorček má „rovnakú farbu ako oblúk vyššie" (žiadny oblúk už nie je),
druhý opisoval `platformTotal` ako „the ring's own total".

---

## 2. Sparkliny v KPI kartách

Odstránené zo všetkých štyroch, ktoré ich mali — **Profit, Revenue, Purchase
cost, Tickets sold**. ROI ju nikdy nemala (na tej sérii nie je ROI per
bucket a vymyslená čiara je presne to, čo táto appka nesmie robiť).

Karty teraz nesú číslo, trend a podriadok. **Reálny graf je priamo pod tým
riadkom a nezmenil sa** — tie sparkliny kreslili `revenueTimeSeries`, čo je
presne to, čo ten graf pod nimi kreslí aj tak.

---

## 3. Čo som ponechal — a prečo

`PlatformRing` v `DashboardCharts.tsx`, `Sparkline` a prop `spark` na
`StatCard` v `ui.tsx` **zostali v kóde**, len ich už nič nevolá.

Dôvod: sú exportované, takže build ich nerieši (overil som si aj, že
`noUnusedLocals` je v `tsconfig.json` vypnuté, čiže ani nepoužitý import by
build nezhodil — ten import som aj tak odstránil). Vymazať ich znamená
zasiahnuť do `ui.tsx`, ktorý importuje každá stránka v appke, bez akéhokoľvek
prínosu pre správanie.

**Ak ich mám vymazať nadobro, povedz a spravím to.** Nechávam to na teba,
nerobím to sám — je to väčší zásah, než si pýtal.

---

## 4. Overenie

Toto som nečítal, pozrel som sa na to. Vyrenderoval som statickú repliku oboch
widgetov **s tvojimi vlastnými číslami** (3 703,05 € / 7 913,78 € / 4 210,73 €
/ 87,9 % / 43, a Seatiks–Viagogo–discord–Whatsapp) pri šírke 1400 px:

- Päť KPI kariet sedí v **jednom riadku**, každá hodnota je celá čitateľná —
  to je vec, ktorú treba kontrolovať, lebo v 2.74.0 presne tu sparkline
  stlačil hodnoty na nulovú šírku a karty boli prázdne.
- „Sales by platform" začína zoznamom rovno pod hlavičkou, bez diery po
  krúžku.

Plus statické kontroly na `Dashboard.tsx`: vyváženie zátvoriek v poriadku,
obe triedy komentárovej chyby 0 výskytov.

**Čo to nie je:** nie je to kompilácia ani `tsc`. Na tomto Macu nie je node
ani cargo — **CI je prvý kompilátor**. To platí rovnako ako minule.

---

## 5. Verzia

2.87.0 → **2.88.0**, 9 výskytov v 7 súboroch plus `CURRENT_STATE.md`.
Overené, že nikde nezostal `2.87.0`.

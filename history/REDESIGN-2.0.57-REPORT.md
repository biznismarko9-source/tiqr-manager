# TIQR Manager 2.0.57 — voľba meny pri New Sale + Convert to EUR

## Čo si mi napísal

*"taktiez pri new sale ma byt moznost si vybrat v akej menej je predaj, potom sa da dodatocne converovat na eura"*

## Čo je nové

Vo formulári **New Sale** (Sales → New Sale), na kroku, kde zadávaš ceny jednotlivým lístkom, je teraz
políčko **Sale currency** — presne vedľa Quick-fill price/fees, v tej istej lište. Predtým tam bolo natvrdo
"EUR" a nedalo sa to zmeniť.

- Vyberieš si z bežných mien (rovnaký zoznam ako pri New Order) alebo cez "Other..." napíšeš vlastný kód
  meny.
- Appka je pri otvorení kroku "details" rovno predvyplní rozumne: ak všetky vybrané lístky majú rovnakú
  nákupnú menu, predvyplní tú istú (napr. všetko v GBP → predaj rovno v GBP); ak sú lístky namiešané
  (niektoré EUR, niektoré USD), predvyplní EUR. Ak si menu zmeníš ručne, appka si to zapamätá a už do toho
  sama nezasahuje.
- Je to **jedna mena pre celý predaj** (celý batch lístkov v tomto okne), nie osobitne pre každý lístok —
  presne tak, ako sme si to potvrdili, a rovnako, ako už appka funguje pri Quick-fill price/fees.
- Keď si zvolíš inú menu, ako v akej sú lístky nakúpené, objaví sa tlačidlo **Convert to EUR** — funguje
  úplne rovnako, ako už poznáš z New Order (2.0.50): stiahne aktuálny kurz naživo, prepočíta ceny aj fees na
  EUR, prepne menu na EUR a napíše ti presný kurz aj k akému dátumu platí.

Predajová suma (Total revenue) sa teraz vždy zobrazuje v mene, ktorú si zvolil pre predaj — nie v mene
lístka.

## Prečo pribudla aj oprava v troch ďalších miestach (dôležité)

Appka odjakživa dodržiava jedno pravidlo: **rôzne meny sa nikdy nemiešajú do jedného čísla** — ak niečo nejde
spočítať v jednej mene, appka radšej ukáže "Mixed" ako predstierať výsledok. Doteraz to ale nebolo treba
riešiť pri jednotlivom predaji, lebo mena predaja bola vždy odvodená z meny lístka — nemohli sa teda nikdy
rozísť.

Keď som povolil, aby si mena predaja bola iná ako mena, v akej bol lístok nakúpený (presne to, čo si chcel),
vznikla situácia, ktorá predtým nemohla nastať: jeden riadok predaja, kde sa **cost** (v mene lístka) a
**tržba** (v tvojej zvolenej mene predaja) rátajú v dvoch rôznych menách. Marža a ROI sú pomer tržby ku
costu — a pomer dvoch rôznych mien bez prepočtu je zavádzajúce číslo, nie skutočný zisk.

Preto appka teraz na každom riadku predaja sama pozná, či si menu zvolil rovnakú ako pri nákupe lístka, a ak
nie:

- **Marža a ROI** sa pre daný riadok neráta a nezobrazí sa (namiesto toho jasne "Mixed"), presne tak, ako
  appka doteraz robievala pri iných nezlučiteľných menách.
- V **detaile predaja** (Sale Detail) sa cost aj profit takéhoto riadku zobrazia ako "Mixed" s vysvetlením
  po prejdení myšou, prečo.
- V **zozname predajov** (Sales), keď je toto jediný riadok v skupine, appka správne vyhodnotí menu skupiny
  ako "Mixed" — nezobrazí ju omylom ako jednu konkrétnu menu.
- V **CSV exporte** sa v takom prípade stĺpec profit nechá prázdny namiesto nesprávneho čísla.

Toto nie je samostatná pridaná funkcia navyše — je to nutný dôsledok toho, že si sa rozhodol povoliť
nezhodu mien pri predaji. Bez tejto opravy by appka na niektorých miestach potichu ukazovala zavádzajúce
čísla marže/zisku. Riešenie je len doplnkové (nový príznak k existujúcim dátam), nie zásah do toho, ako sa
peniaze počítajú a ukladajú — cost aj tržba samotné ostávajú vždy presné skutočné čísla, len sa nesčítavajú
do jedného pomeru, keď by to bolo zavádzajúce.

## Oprava: prvý zip mal chybu v publikačných skriptoch

Prvá verzia zipu prešla `release.ps1` vlastnou kontrolou a spadla so "STOPPED" — `package.json`,
`tauri.conf.json` a `Cargo.toml` správne hovorili 2.0.57, ale samotný `release.ps1` (a `1-CLICK-UPDATE.bat`)
mal vo vnútri natvrdo napísané staré `v2.0.56`, lebo moja kontrola pri zvyšovaní verzie prehľadávala len
`.rs/.ts/.tsx/.json/.toml/.md` súbory a tieto dva skripty (`.ps1`/`.bat`) vynechala. Skript teda urobil presne
to, na čo bol postavený — zastavil sa namiesto tichého publikovania nekonzistentného stavu — len príčinou
nebol zlý/neúplný stiahnutý zip, ale moje opomenutie pri predchádzajúcom zvyšovaní verzie. Opravené (obe
miesta teraz hovoria `v2.0.57`), navyše som pri tom opravil aj `$CommitMsg` v `release.ps1`, ktorý ešte od
2.0.50 opisoval úplne inú (dávno odoslanú) zmenu — teraz opisuje presne toto, čo je v tomto reporte.

## Kde to funguje a kde (zatiaľ) nie

Rovnako, ako pri Convert to EUR na New Order, aj tu je to tlačidlo len pri **vytváraní** predaja, nie pri
úprave už existujúceho — appka si menu aj sumy k predaju uloží a ďalej ich nemení potichu.

Appka má v kóde aj starší, appkou už nepoužívaný spôsob vytvorenia predaja (jednotlivo, nie cez dávku) —
tade v UI sa nikdy nevolá (appka vždy ide cez dávkové vytvorenie, aj pri jednom lístku), preto som ho
zámerne nechal bez zmeny. Nemá to na nič vplyv, keďže appka ho reálne nepoužíva.

## Ako som to overoval

```
cargo test --lib  -> 625 testov, všetky prešli (617 pôvodných + 8 nových na túto zmenu)
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

Nové testy pokrývajú: že zvolená mena predaja naozaj prebije menu lístka na každom riadku, že bez zvolenej
meny appka funguje presne ako doteraz (kvôli existujúcim miestam v appke, čo dávku vytvárajú bez explicitnej
meny), že prázdna mena sa odmietne, že marža/ROI zmiznú presne vtedy, keď sa mena predaja rozíde s menou
lístka (a naopak, že zostanú, keď sa zhodujú), a že sa skupina v zozname predajov správne označí ako Mixed.

Keďže appku v tomto prostredí neviem naozaj spustiť ako desktopovú appku, postavil som si dočasnú
náhľadovú stránku (mimo appky, zmazanú hneď po použití) a v nej si samotný formulár New Sale naozaj
vykreslil a poklikal v prehliadači — vyskúšal som predvyplnenie meny, Quick-fill, prepnutie na inú menu,
zobrazenie "Mixed" s rozpisom nákladov podľa meny, kliknutie na Convert to EUR aj Light/Dark režim. Všetko
sedelo presne tak, ako malo, vrátane novej hnedo-béžovej témy z 2.0.56.

## Čo teraz urobiť

1. Nainštaluj 2.0.57.
2. Choď do Sales → New Sale, vyber si objednávku a aspoň jeden lístok, klikni Continue.
3. Skontroluj, že sa mena predaja predvyplnila podľa meny lístka, a skús ju zmeniť na inú (napr. "Other..."
   → USD).
4. Klikni Convert to EUR a over, že sa ceny aj fees prepočítajú a appka napíše kurz.
5. Ak necháš menu inú, ako je mena lístka, skontroluj v spodnej časti "Estimated profit" — malo by tam byť
   "Mixed" s rozpisom nákladov podľa meny namiesto jedného čísla.
6. Po vytvorení takéhoto predaja skontroluj aj Sale Detail a CSV export — cost/profit by tam mal byť
   "Mixed"/prázdny, nie nesprávne číslo.

## Zmenené súbory

**Backend (Rust):**
- `src-tauri/src/models.rs` — nové pole `currency` na `SaleBatchInput`, nové pole `currency_mismatch` na
  `Sale`
- `src-tauri/src/commands/sales.rs` — voľba meny pri vytváraní dávky, výpočet `currency_mismatch`,
  skrytie marže/ROI pri nezhode, oprava zoskupovania na "Mixed" v zozname predajov
- `src-tauri/src/commands/csv_export.rs` — prázdny profit v CSV pri nezhode meny
- `src-tauri/src/commands/orders_sheet_sync.rs` — prepojenie na existujúci import zo Sheetu (bez zmeny
  správania, len nová voliteľná hodnota)
- `src-tauri/src/commands/dashboard.rs` — drobná úprava testu

**Frontend:**
- `src/lib/types.ts` — nové polia v type definíciách
- `src/pages/Sales.tsx` — políčko Sale currency, Convert to EUR, prepočítané zobrazenie súčtov
- `src/pages/SaleDetail.tsx` — zobrazenie "Mixed" pri nezhode meny v detaile predaja

**Publikačné skripty (oprava, viď vyššie):**
- `release.ps1` — `$Version` z `v2.0.56` na `v2.0.57`, plus aktualizovaný `$CommitMsg`
- `1-CLICK-UPDATE.bat` — titulok a hláška z `v2.0.56` na `v2.0.57`

**Verzia (8 miest):** ako vždy, všetkých na `2.0.57` — vrátane `release.ps1` a `1-CLICK-UPDATE.bat`, ktoré
som si teraz pridal do vlastného kontrolného zoznamu natrvalo.

## STOP

2.0.57 hotové — voľba meny pri New Sale funguje presne podľa zadania, plus oprava marže/ROI/exportu, aby
appka pri rôznych menách nikdy neukázala zavádzajúce číslo namiesto "Mixed". Skús kroky vyššie, hlavne bod 5
a 6 (to je tá časť, ktorú si nepýtal priamo, ale bola nutná).

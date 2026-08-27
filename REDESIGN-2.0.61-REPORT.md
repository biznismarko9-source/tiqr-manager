# TIQR Manager 2.0.61 — hotfix: "Fix sync" mazalo Status/Delivery status a Total Cost

## Čo si mi napísal

*"najskor treba opravit este tabulku, vsimol som si ze ked som dal fix shete, tak uplne odstrani Status a
Delivery status, tatkiez som si vsimol ze je problem s Total Cost, ukazuje 0 a nic nevypocitava, prosim
oprav vsetko. a taktiez aj skontroluj tu tabulku celu aby fungovala ako doteraz"*

Toto je moja chyba z 2.0.60 a beriem to na seba — nové tlačidlo malo len opravovať to, čo Push sales
nedopísal, nie meniť čokoľvek iné. Nižšie presne to, čo sa stalo, čo je opravené, a čo si prosím skontroluj
priamo v hárku, keďže tam mohlo dôjsť ku skutočnej strate hodnôt.

## Čo sa presne pokazilo

### 1. Fix sync mazal Status/Delivery status

Sale zaznamenaný cez appku samotnú (Sales alebo tlačidlo "New sale" na Dashboarde) **nikdy nezapisuje**
pole "resale status"/"delivery status" na lístok — appka tieto dve polia napĺňa len jedným jediným
spôsobom: keď sa "Sales sync" (čítanie z hárku) prečíta stĺpce Status/Delivery status priamo z hárku. Sale
zaznamenaný priamo v appke teda tieto dve polia v appke necháva prázdne (appka o nich jednoducho nič nevie
— nie je to "appka si myslí, že majú byť prázdne", appka na ne nemá názor vôbec).

Fix sync som napísal tak, že prázdnu appkinu hodnotu premenil na **prázdny reťazec** a ten porovnal s tým,
čo je v hárku — a keďže prázdny reťazec sa nerovná "Listed"/"Not yet" (čo tam už reálne bolo, buď ručne
napísané, alebo z predošlého pushu), appka to vyhodnotila ako "nesedí" a **prepísala to prázdnym**. Presne
toto si videl.

**Oprava:** Fix sync teraz pole, na ktoré appka nemá vlastný názor, **vôbec nezapisuje** — ani prázdne, ani
inak. Prepíše len bunku, pre ktorú appka skutočne má konkrétnu hodnotu a tá sa líši od toho, čo je v hárku.
Pridal som 2 automatizované testy presne na tento scenár (sale zaznamenaný v appke, nie synchronizovaný z
hárku), čo by boli tento konkrétny bug odhalili už predtým, keby existovali — chyba bola v tom, že moje
pôvodné 3 testy z 2.0.60 (neúmyselne) všetky používali objednávky prečítané z hárku, kde appka tieto polia
vždy má, takže scenár, čo si narazil, sa v nich vôbec neobjavil.

### 2. Total Cost ukazovalo 0

Fix sync okrem opravy predajov spúšťal aj existujúci krok "currency catch-up" (funguje od 2.0.54, používa
ho aj Push sales/Push orders) — ten pri KAŽDOM behu porovná, čo appka eviduje ako celkovú nákupnú cenu
objednávky, s tým, čo je v stĺpci "Total Purchase Price" v hárku, a keď sa to líši, prepíše bunku hárku
appkinou hodnotou — bez kontroly, či je appkina hodnota vôbec rozumná. Toto je najpravdepodobnejšia
príčina — buniek, ktoré Fix sync tentoraz prvýkrát skutočne dosiahol (keďže predošlé Push sales pre tú istú
objednávku vôbec nedobehlo až sem), sa tento krok mohol dotknúť a zapísať 0, ak appka pre daný záznam
eviduje nákupnú cenu 0.

**Oprava, na dvakrát:**
1. Fix sync už tento krok (currency catch-up) vôbec nespúšťa — ani obnovu dropdownov/vzorcov v hárku.
   Robí už len presne to, o čo si pôvodne žiadal: doplní chýbajúce údaje o predaji/pulle. Push sales a Push
   orders zostávajú úplne bez zmeny, aj naďalej oba tento krok spúšťajú.
2. Naviac som ako poistku upravil aj samotný krok "currency catch-up" (teda aj pre Push sales/Push
   orders, nielen Fix sync): **nikdy už nezapíše presne 0** do "Total Purchase Price" — keďže objednávka za
   0 nikdy reálne nedáva zmysel, appka teraz takúto "opravu" radšej preskočí, než by ticho prepísala
   možno správne číslo nulou.

## Čo prosím skontroluj priamo v hárku

Keďže Fix sync pri jednom kliknutí prechádza **všetky** prepojené objednávky naraz (nie len tú jednu, čo
testuješ), mohlo sa to isté stať na viacerých riadkoch, nielen na jednom. Odporúčam:

1. V Google Sheets choď na **Súbor → História verzií → Zobraziť históriu verzií** (alebo klikni na hodinky
   s šípkou vedľa "Súbor"/"File").
2. Nájdi verziu tesne pred tým, než si klikol Fix sync, a porovnaj ju s aktuálnym stavom — uvidíš presne,
   ktoré bunky (Status/Delivery status/Total Purchase Price) sa zmenili.
3. Radšej než vrátiť celú verziu naspäť (to by zmazalo aj čokoľvek iné, čo si odvtedy urobil), len ručne
   dopíš späť konkrétne hodnoty, čo sa stratili, do aktuálneho hárku.

Mrzí ma to — bolo to naozaj moje zbytočné rozšírenie tlačidla nad rámec toho, o čo si žiadal, a malo to
dôjsť najskôr k dôkladnejšiemu overeniu, než sa to dostalo k tvojmu reálnemu hárku.

## Ako som to overoval

```
cargo test --lib  -> 630 testov, všetky prešli (628 + 2 nové na presne tento scenár + 1 na currency poistku)
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

## Zmenené súbory

**Backend:**
- `src-tauri/src/commands/orders_sheet_sync.rs`:
  - `apply_sales_push_internal`: force-vetva teraz nikdy nezapíše pole, na ktoré appka nemá hodnotu (Site
    Listed/Status/Delivery status/paid-by) — predtým `unwrap_or_default()` premieňal "appka nevie" na
    "appka chce prázdne", čo mazalo správne hodnoty v hárku
  - `force_push_sales_impl`: už nespúšťa currency catch-up ani obnovu štruktúry hárku — len opravu
    predaja/pullu, presne podľa pôvodnej žiadosti
  - `currency_push_cells`: nikdy nezapíše "Total Purchase Price" ako presne 0
  - 3 nové testy

**Verzia (8 miest):** `2.0.61`.

## STOP

2.0.61 opravuje regresiu, čo spôsobilo samotné 2.0.60 — Fix sync už nemaže polia, na ktoré appka nemá
názor, a nedotýka sa currency/Total Purchase Price/vzorcov v hárku vôbec. Prosím skontroluj priamo v hárku
(história verzií, vyššie), či sa niekde inde stratila hodnota, a daj vedieť, či teraz Fix sync funguje
čisto — potom sa vrátime k 3. bodu (kategórie).

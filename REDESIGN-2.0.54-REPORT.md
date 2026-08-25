# TIQR Manager 2.0.54 — Recent Sales po celých predajoch, Push Orders/Sales dopĺňa menu

## Čo si mi napísal

Tri veci naraz: (1) prevod na EUR do Sheetu z 2.0.53 ti nefungoval, chcel si aj poistku cez tlačidlá Push Orders/Push Sales, (2) Recent Sales na Dashboarde ukazuje predaj po jednom lístku namiesto celého predaja naraz, (3) eventy zo synchronizácie so Sheetom nemajú priradenú kategóriu, takže sa podľa nej nedá triediť.

Toto kolo rieši **(1) a (2)**. Bod (3) - automatická kategorizácia - je väčšia, samostatná vec (potrebuje skutočné zisťovanie, o aký event ide, nie len appku samu), na tú sa pustím hneď v ďalšom kole.

## 1. Recent Sales teraz zobrazuje celé predaje, nie jednotlivé lístky

**Príčina:** Dashboard bral dáta priamo z tabuľky `sales`, kde je (zámerne) jeden riadok na jeden lístok. Keď si predal 4 lístky na jeden raz ako jeden predaj (presne to, čo bolo na tvojom screenshote - 4× "TWO NIGHTS AT NAVY PIER" po 110,00 €), appka to poctivo ukázala ako 4 riadky, lebo v databáze to naozaj 4 riadky sú - len hlavný zoznam Sales toto už dávno rieši zoskupovaním podľa `batch_id`, kým Recent Sales na Dashboarde nie.

**Oprava:** Recent Sales teraz používa presne ten istý spôsob zoskupovania, aký už roky používa hlavný zoznam Sales (`GROUP_BASE_SELECT`/`GROUP_KEY_EXPR` v `sales.rs`) - jeden riadok na jeden predaj (nech je za ním 1 lístok alebo 10), so správnym súčtom tržieb za celý predaj. Prevzal som aj rovnaké zobrazenie ako na Sales: "Mixed events" keď predaj obsahuje lístky z rôznych eventov, a novú poznámku "X/Y refunded" pri predaji, kde bola vrátená len časť lístkov (predtým sa toto nedalo v jednom riadku vôbec vyjadriť).

## 2. Currency push do Sheetu - poistka cez Push Orders/Push Sales

Tlačidlá **Push Orders** a **Push Sales** (Settings → Integrations) teraz navyše, pri každom spustení, prejdú všetky objednávky prepojené so Sheetom a opravia stĺpce Currency/Price Per Ticket/Total Purchase Price, ak sa nezhodujú s tým, čo má appka aktuálne uložené - bez ohľadu na to, prečo sa to nestihlo predtým (napr. keby automatický prevod z 2.0.53 z nejakého dôvodu zlyhal). Zapíše sa vždy len to, čo je naozaj neaktuálne - riadok, kde sa všetko už zhoduje, sa vôbec nedotkne.

Automatický prevod hneď pri kliknutí na Convert to EUR (2.0.53) ostáva - toto je vedomá poistka navyše, presne ako si chcel, nie náhrada zaň.

## Dôležité - jedna vec, čo som pri tomto našiel a hneď opravil

Pri písaní testu na bod 2 som si všimol skutočnú chybu v **dvoch testoch, čo som ti poslal už v 2.0.53** - pri reálnom spustení `cargo test` by boli spadli (skúšali vložiť do tabuľky `sheet_sync_links` záznam, ktorý tam appka sama už vložila o riadok skôr, čo naráža na primárny kľúč tej tabuľky). Chybný bol len samotný test, nie kód appky, ktorý testoval - ale keďže v tomto prostredí nemám k dispozícii `cargo`/`rustc`, sám som to dovtedy nemohol overiť spustením. Opravené priamo (testy teraz použijú skutočne vygenerovaný marker namiesto predpokladaného).

## Ako som to overoval

Rovnaké obmedzenie ako pri 2.0.52/2.0.53 - žiadny `rustc`/`cargo` k dispozícii. Napísal som **3 nové testy** (2 na zoskupovanie Recent Sales - že 4 lístky v jednom predaji dajú 1 riadok so správnym súčtom, a že limit počíta predaje, nie jednotlivé lístky pod nimi; 1 na Push Orders/Sales poistku - že sa zapíše len naozaj neaktuálny riadok, správny sa nedotkne). Vlastný skript na kontrolu spárovania zátvoriek prešiel čisto na všetkých upravených `.rs` súboroch, TypeScript kontrola čistá na `types.ts`/`Dashboard.tsx`.

Ešte jedna drobnosť z bezpečnosti pri verziovaní: `package-lock.json` mal tentoraz balíček `node-releases` NAOZAJ na tej istej verzii (2.0.53), z ktorej som appku posúval ďalej - takže obyčajný plošný "nahraď všade" príkaz by ho omylom tiež zmenil. Zmenil som len appkine dva konkrétne riadky (podľa čísla riadku, nie podľa textu), `node-releases` zostal nedotknutý na svojej vlastnej verzii.

## Čo teraz urobiť

1. Nainštaluj 2.0.54.
2. Na Dashboarde skontroluj Recent Sales - viacnásobný predaj by teraz mal byť jeden riadok so správnym súčtom.
3. Skús Push Orders a Push Sales (Settings → Integrations) na objednávke, kde si predtým prevádzal menu - over, či sa teraz Sheet naozaj zhoduje s appkou.
4. Ak stále niečo nesedí so Sheetom aj po Push Orders/Sales, daj mi vedieť presne čo vidíš (screenshot Sheetu aj appky) - pomôže mi to nájsť, čo presne pri automatickom prevode zlyhalo.

## Zmenené súbory

**Backend (Rust, 3 súbory):**
- `src-tauri/src/commands/sales.rs` - nová `fetch_recent_groups` + 2 nové testy
- `src-tauri/src/commands/dashboard.rs` - Recent Sales prepnuté na `fetch_recent_groups`
- `src-tauri/src/commands/orders_sheet_sync.rs` - nová `reconcile_order_currencies`, zapojená do `push_orders_impl` aj `push_sales_impl` + 1 nový test; oprava 2 chybných testov z 2.0.53
- `src-tauri/src/models.rs` - `DashboardData.recent_sales` teraz `Vec<SaleGroup>` namiesto `Vec<Sale>`

**Frontend (2 súbory):**
- `src/lib/types.ts` - rovnaká zmena typu
- `src/pages/Dashboard.tsx` - prepísané zobrazenie Recent Sales

**Verzia (8 miest):** ako vždy, všetkých na `2.0.54`.

## STOP

2.0.54 hotové. Skontroluj podľa krokov vyššie - najviac ma zaujíma, či Push Orders/Push Sales teraz naozaj dorovná menu v Sheete. Keď potvrdíš, pustím sa do bodu 3 (automatická kategorizácia eventov).

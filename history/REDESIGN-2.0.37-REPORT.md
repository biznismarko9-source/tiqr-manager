# TIQR Manager 2.0.37 — tabuľky sa už nikdy nemusia posúvať do strany

## Čo je nové

Toto je pokračovanie veci, na ktorej sme sa dohodli minule: appka mala dve zvyšné úlohy z dohodnutého
plánu, obe sú teraz hotové.

1. **Žiadna tabuľka sa už nemusí posúvať do strany** - Sales, Sale Detail, Orders, Order Detail,
   Events, Tickets/Inventory, Pulls (obe podzáložky). Presne ako si chcel - "vsetok text zarovnat do
   jednej linie a vycentrovat", žiadne zalamovanie, žiadne scrollovanie do strany, nech je okno
   akokoľvek úzke.
2. **Nové Zoradenie (Najnovšie/Najstaršie)** na Tickets/Inventory a na oboch podzáložkách Pulls (Given
   aj Received) - presne také, aké už majú Sales/Orders/Events.

(Zjednotenie písma naprieč tabuľkami - aby všade vyzeral text rovnako - som doručil ešte pred touto
verziou, to už je hotové a nezmenilo sa.)

## Ako presne tabuľky teraz fungujú

Každá tabuľka teraz vie dve veci naraz:

1. **Na normálnom/väčšom okne** vyzerá presne tak ako doteraz - všetky stĺpce, rovnaké písmo ako si
   zvyknutý.
2. **Keď okno zúžiš** (appku nemáš na celú obrazovku, alebo máš menší monitor), tabuľka sa automaticky
   prispôsobí v dvoch krokoch: najprv sa mierne zmenší písmo a odsadenie v bunkách, a ak by to ešte
   stále nestačilo, schovajú sa len tie stĺpce, na ktorých sme sa vopred dohodli, že sú najmenej
   dôležité. Nič sa pritom nestráca - schovaná hodnota je vždy jeden klik od teba, na detaile
   Sale/Order/Eventu.

Hranica, kedy appka prepne medzi "normálnym" a "zúženým" zobrazením, je rovnaká pre všetky tabuľky
naraz - takže pri zmenšovaní okna prepnú všetky tabuľky v appke v ten istý moment, nič nezostane
napoly prepnuté. Funguje to spoľahlivo až po úplne najmenšie okno, aké appka vôbec dovolí.

Presný zoznam stĺpcov, ktoré sa schovajú len na zúženom okne (na normálnom okne vidno úplne všetko,
ako doteraz) - názvy presne také, aké vidíš v appke:

- **Sales:** Fees, Margin/ROI
- **Sale Detail:** Fees
- **Orders:** Notes, Platform
- **Order Detail:** Listing price
- **Events:** Margin, ROI
- **Tickets/Inventory:** Purchase date
- **Pulls - Given:** Warning, Platform
- **Pulls - Received:** Order

## Prečo to takto riešim (a nie inak)

Predtým (2.0.36) mali tabuľky nastavenú pevnú minimálnu šírku - pod ňou sa síce nič nezrazilo ani
nezalomilo, ale namiesto toho sa objavil vodorovný posuvník. To presne si mi povedal, že nechceš -
žiadne posúvanie, žiadne zalamovanie, všetko na jeden riadok. Táto verzia teda nie je len "zväčšenie
minima" ako predtým, ale skutočný prepínací systém medzi dvoma vopred vypočítanými rozloženiami
stĺpcov (normálne / zúžené), overenými tak, aby sa naozaj vždy zmestili - nie odhadom.

Pri počítaní presných šírok som znova meral skutočne vykreslené dáta (sumy v eurách/dolároch, dátumy,
percentá), nie len text nadpisu stĺpca - rovnaké poučenie ako pri 2.0.36 (slovenský formát peňazí je
citeľne širší než anglický, dôležité najmä u teba, keďže Windows máš najskôr nastavený po slovensky).

## Testy a build

```
cargo test --lib -> 491 passed, 0 failed, 3 ignored (bez zmeny - táto oprava sa Rust kódu netýka)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.37 build" v hlavičke)
```

Kontrola bola dôkladná, nie len na oko:

- Skript otvoril appku naozaj v prehliadači (so vzorovými dátami) a pri šiestich rôznych šírkach okna
  (od úplne najužšieho, čo appka dovolí, po veľmi široké) skontroloval na všetkých 8 tabuľkách, že sa
  nikdy nič neposúva do strany - 48 z 48 kontrol prešlo.
- Druhý skript overil, že pri naťahovaní okna do úzka a späť (bez reštartu appky) sa stĺpce naozaj
  správne schovajú a zase objavia presne podľa zoznamu vyššie, a že nové tlačidlo Zoradenia je naozaj
  na Tickets/Inventory aj oboch podzáložkách Pulls.
- Zip zabalený z presného zoznamu súborov (pribudol 1 nový súbor + tento report, nič sa neuberalo),
  vybalený do prázdneho priečinka, tam znova `npm ci` + `tsc -b` + `npm run build` (všetko čisté) a
  porovnaný bajt po bajte s mojím pracovným priečinkom - sedí presne.

## Zmenené súbory

**Frontend (8 súborov):**
- `src/pages/Sales.tsx`, `src/pages/SaleDetail.tsx`, `src/pages/Orders.tsx`, `src/pages/OrderDetail.tsx`,
  `src/pages/Events.tsx`, `src/pages/Tickets.tsx` (platí aj pre Inventory), `src/pages/Pulls.tsx` (obe
  podzáložky) - prepínanie medzi normálnym/zúženým zobrazením, plus (na Tickets a Pulls) nové Zoradenie
- `src/lib/useNarrowTables.ts` (nový súbor) - spoločná "je okno úzke?" logika, zdieľaná všetkými
  tabuľkami, aby prepli všetky naraz
- `src/index.css` - nový, mierne menší štýl písma pre zúžené tabuľky

**Verzia (8 miest):** `package.json`, `package-lock.json` (2×), `src-tauri/tauri.conf.json`,
`src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version`), `1-CLICK-UPDATE.bat` -
všetkých na `2.0.37`.

## STOP

2.0.37 hotové a overené (491/491 testov, čisté `tsc`/`build`, 48/48 kontrol na rôznych šírkach okna
bez posúvania do strany). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. Skús appku zmenšiť do menšieho okna (nie na celú obrazovku) - na žiadnej tabuľke by sa už nemal
   objaviť vodorovný posuvník. Text sa len mierne zmenší, a pri väčšom zúžení sa schová pár stĺpcov
   (presný zoznam vyššie).
2. Tickets aj Inventory - malo by tam pribudnúť Zoradenie "Newest first / Oldest first" vedľa
   filtrov.
3. Pulls - obe podzáložky (Given aj Received) - to isté, nové Zoradenie.
4. Ak niektorý zo schovaných stĺpcov (napr. Fees na Sales) budeš na užšom okne potrebovať vidieť
   často, daj vedieť - dá sa vymeniť za iný, menej dôležitý.

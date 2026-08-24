# TIQR Manager 2.0.40 — Automatické výpočty v Google Sheets (Pulls aj Orders & Sales)

## Čo je nové

Podľa tvojej požiadavky som doplnil automatické výpočtové stĺpce do oboch prepojených tabuliek.

**Pulls** — hneď za stĺpec **TIQR ID** (nech je aktuálne na akomkoľvek mieste v tvojej tabuľke, appka si ho
sama nájde podľa názvu, nehľadám natvrdo konkrétne písmeno) pribudol nový stĺpec **"Total price (€)"**:
v prvom riadku názov, v druhom riadku jeden vzorec `=SUM(stlpec:stlpec)` - **na celý stĺpec**, nie len na
1000 riadkov ako v tvojom návrhu. To znamená, že aj keď pridáš pull č. 5000, súčet ho automaticky zoberie
do úvahy, netreba nič prepisovať. Hlavička má aj tučné písmo a jemné podfarbenie, nech je vidieť na prvý
pohľad.

**Orders & Sales** — presne podľa zadania som nechal 2 voľné stĺpce za "how much pull" a od 3. stĺpca
začína Summary blok:

- **Summary**: Total Cost `=SUM(H:H)`, Total Revenue `=SUM(P:P)`, Total Profit `=SUM(Q:Q)`
- **Summary-Paid**: Total Paid `=SUMIF(T:T,"Paid",P:P)`
- **Summary-Unpaid**: Total Unpaid — vysvetlenie nižšie, lebo tu som zmenil vzorec

Aj tu majú nadpisy Summary / Summary-Paid / Summary-Unpaid tučné písmo a jemné podfarbenie.

## Jedna vec, ktorú som ti sľúbil skontrolovať sám - a naozaj som ju zmenil

Tvoj návrh pre Unpaid bol `=SUMIF(T:T;"Unpaid";P:P)` - teda spočítať Revenue všade tam, kde stĺpec Payout
status má presne text "Unpaid". Problém je, že appka do toho stĺpca nikdy nezapisuje text "Unpaid" - reálne
tam môže byť len "Paid" alebo "Pending" (prípadne prázdno). Keby som tvoj vzorec použil presne tak, ako si
ho napísal, vždy by vypočítal 0 - a to úplne potichu, appka aj Sheets by tvárili, že je všetko v poriadku,
pritom by to bolo celý čas nesprávne.

Namiesto toho som použil `=SUM(P:P)-SUMIF(T:T,"Paid",P:P)` - teda "celkové Revenue mínus to, čo je označené
Paid" = presne to, čo si chcel vidieť ako "Unpaid" súčet, len bez závislosti na texte, ktorý v tabuľke
neexistuje.

Mimochodom - tvoje ostatné 4 vzorce (Total Cost/Revenue/Profit aj Paid) som skontroloval oproti skutočným
stĺpcom v appke a sedia presne, vrátane písmen H/P/Q/T aj K pri Pulls - použil som ich presne tak, ako si
ich napísal.

## Čo som zámerne nedoplnil - a ako si to vieš spraviť sám za pár sekúnd

Do nových číselných buniek som nedal formát meny (napr. "1 234,56 €") - appka síce vie, akú menu má tvoja
tabuľka nastavenú (EUR/USD/GBP), ale nevie, aké má tvoj Google účet regionálne nastavenia (napr. či sa
desatinné miesta píšu čiarkou alebo bodkou). Keby som formát uhádol zle, čísla by vyzerali skôr rozbité než
krajšie. Je to jednoduché doplniť ručne: označíš bunky → Format → Number → Currency, a Sheets si samo
vyberie správny formát presne podľa tvojho nastavenia účtu.

## Kedy sa to prejaví

Netreba nič špeciálne robiť - stačí najbližší bežný Sync/Push na ktorejkoľvek z tabuliek (Pulls aj Orders &
Sales) a oba nové bloky sa doplnia automaticky. Ak to chceš vidieť hneď teraz bez čakania, choď do Settings
→ Integrations a klikni na "Update sheet" pri príslušnej tabuľke.

Ak by si niekedy náhodou omylom vymazal alebo prepísal niektorú z týchto buniek, ďalší Sync/Push ju sám
znova doplní - netreba nič manuálne opravovať.

## Testy a build

```
cargo test --lib -> 513 passed, 0 failed, 3 ignored (12 nových testov oproti 2.0.39, nič iné sa nepokazilo)
cargo check --lib -> 0 chýb
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

Táto verzia sa dotkla len backendu (Rust) - frontend sa vôbec nemenil, testy/build som pre istotu aj tak
overil, aby bolo isté, že nič nesúvisiace sa nepokazilo.

## Zmenené súbory

**Backend (3 súbory):** `src-tauri/src/commands/pulls_sheet_sync.rs` (Total price (€) - vzorec aj štýl),
`src-tauri/src/commands/orders_sheet_sync.rs` (Summary blok - vzorce aj štýl), `src-tauri/src/
google_sheets.rs` (nová malá pomocná funkcia na tučné písmo + podfarbenie hlavičky).

**Frontend:** žiadna zmena.

**Verzia (8 miest):** ako vždy, všetkých na `2.0.40`.

## STOP

1. Skús normálny Sync alebo Push na Pulls aj na Orders & Sales (alebo rovno "Update sheet") a pozri sa do
   tabuľky - sedí to presne s tým, čo si mal na screenshotoch?
2. Skontroluj hlavne Total Unpaid - vysvetlil som vyššie prečo a ako som ten vzorec zmenil oproti tvojmu
   návrhu, chcem si byť istý, že aj tak dáva zmysel pre to, ako to reálne používaš.
3. Ak chceš aj farebný formát meny na tie nové čísla, ukázal som vyššie ako si to vieš spraviť sám za pár
   sekúnd - ale ak by si chcel, aby to appka robila automaticky, daj vedieť a doplním to (len potrebujem
   vedieť, aké regionálne nastavenie má tvoj Google účet).

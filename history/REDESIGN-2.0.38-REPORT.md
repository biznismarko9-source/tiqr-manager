# TIQR Manager 2.0.38 — Pulls opravené, dátumy a ID celé všade, nový stĺpec Seats

## Čo je nové

1. **Pulls (Given) je opravené** - stĺpce **For**, **Seats** a **More info** sú teraz naozaj celé
   viditeľné aj na fullscreene, presne ako si chcel.
2. **Dátum má teraz vždy celý rok, všade kde má tabuľka svoj stĺpec Date** - namiesto "11.09.26" (alebo
   "23. 8. 2026") je to teraz vždy `11.09.2026` (deň.mesiac.rok, 4-ciferný rok). Platí na Sales, Orders,
   Tickets/Inventory ("Purchase date"), Events aj na oboch podzáložkách Pulls.
3. **ID/kódy sú teraz naozaj celé vidno všade** - Pull, Order, Sale aj Ticket kód (napr. `PULL-000123`,
   `ORD-000045`). Okrem Pulls/Orders/Sales/Tickets/Inventory som to isté našiel a opravil aj na Sale
   Detail a Order Detail (tabuľka jednotlivých lístkov pod konkrétnym predajom/objednávkou) - kódy Ticket
   aj Order tam boli tiež orezané, len si to ešte nenahlásil.
4. **Nový stĺpec Seats** na **Orders**, **Tickets/Inventory** aj **Sales** - presne ako si chcel. Na
   Orders a Tickets ukazuje všetky miesta v danej objednávke, na Sales len tie miesta, ktoré sa v danom
   konkrétnom predaji naozaj predali (refundované položky do toho počítam rovnako ako všade inde v appke).
   Formát je kompaktný, napr. `204/AA 128-131` (susedné čísla sedadiel sa spoja do rozsahu).
5. Pulls' vlastný Seats stĺpec (ten, čo tam už bol) teraz zobrazuje miesto v tom istom kompaktnom formáte
   (`204/AA 128` namiesto dlhšieho "Sec 204 · Row AA · Seat 128") - kratšie, prehľadnejšie, a pomohlo to
   aj zmestiť všetko ostatné na úzkom okne.

## Prečo to predtým nefungovalo

Minulá verzia (2.0.37) už tabuľky prerábala presne na to, aby sa nič neorezávalo - ale pri počítaní,
koľko miesta ktorý stĺpec potrebuje, som pre kód (Pull/Order/Sale/Ticket) aj pre kratšie textové stĺpce
(meno, More info) omylom použil príliš krátke vzorové dáta - v podstate len dĺžku nadpisu stĺpca, nie
skutočný obsah, aký appka naozaj vypisuje. Teraz som každý stĺpec zmeral znova, tentoraz proti tomu, čo
appka skutočne generuje (skutočný formát kódov som si overil priamo v kóde, nie odhadom) - a naviac som
appku so vzorovými dátami aj naozaj otvoril v prehliadači a odfotil/skontroloval každú bunku, nie len
teoretický výpočet. Pri tejto kontrole som ešte našiel jeden menší detail (Seats/More info na Pulls majú
zámerne mierne menšie/tlmenšie písmo než zvyšok riadku), ktorý sa do pôvodného výpočtu nedostal - aj to je
teraz opravené.

## Testy a build

```
cargo test --lib -> 501 passed, 0 failed, 3 ignored (pribudlo 10 nových testov pre Seats)
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.38 build" v hlavičke)
```

Kontrola bola dôkladná:

- Znova som premeral všetkých 8 tabuliek proti skutočnému obsahu (nie len nadpisu) a prepočítal presné
  šírky stĺpcov - všetkých 8 tabuliek sedí presne na 100 % aj v normálnom, aj v zúženom zobrazení.
- Appku som so vzorovými dátami naozaj otvoril v prehliadači (nie len teoreticky) na 7 stránkach, pri
  širokom aj pri najužšom dovolenom okne - 14 z 14 kontrol potvrdilo, že sa nikde nič neposúva do strany.
  Pri tejto kontrole som ešte skontroloval bunku po bunke, či sa text naozaj celý zmestí (nie len či sa
  nezobrazí posuvník) - tam som našiel a opravil ten detail s tlmeným písmom spomenutý vyššie.
- Zip zabalený z presného zoznamu súborov (oproti 2.0.37 iba pribudol tento nový report, nič iné sa
  nepridávalo ani neuberalo), vybalený do prázdneho priečinka, tam znova `npm ci` + `tsc -b` +
  `npm run build` (všetko čisté) a porovnaný bajt po bajte s mojím pracovným priečinkom - sedí presne.

## Zmenené súbory

**Backend (3 súbory):** `src-tauri/src/models.rs` (nový typ pre miesta + parsovanie),
`src-tauri/src/commands/orders.rs`, `src-tauri/src/commands/sales.rs` (obe teraz vracajú zoznam miest).

**Frontend (10 súborov):** `src/lib/types.ts`, `src/lib/format.ts` (nové formátovanie dátumu a miest),
`src/lib/useNarrowTables.ts` (posunutá hranica úzkeho/normálneho zobrazenia), `src/pages/Sales.tsx`,
`src/pages/Orders.tsx`, `src/pages/Tickets.tsx` (platí aj pre Inventory), `src/pages/Events.tsx`,
`src/pages/Pulls.tsx` (obe podzáložky), `src/pages/SaleDetail.tsx`, `src/pages/OrderDetail.tsx`.

**Verzia (8 miest):** `package.json`, `package-lock.json` (2×), `src-tauri/tauri.conf.json`,
`src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version`), `1-CLICK-UPDATE.bat` -
všetkých na `2.0.38`.

## STOP

2.0.38 hotové a overené (501/501 testov, čisté `tsc`/`build`, 14/14 kontrol na skutočnej appke bez
posúvania do strany a bez orezaného textu). Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. Pulls (Given) - na fullscreene aj na menšom okne by teraz malo byť vidno celé For, Seats aj More
   info, nič orezané.
2. Dátum na Sales/Orders/Tickets/Events/Pulls - mal by byť vždy v tvare `11.09.2026` (celý rok).
3. Pull/Order/Sale/Ticket kód - kdekoľvek ho appka zobrazuje (zoznamy aj Sale Detail/Order Detail),
   mal by byť vždy celý vidno.
4. Orders, Tickets/Inventory aj Sales - mal by tam pribudnúť nový stĺpec **Seats**.
5. Ak by niektorý stĺpec (napr. meno kupujúceho na Pulls) na veľmi úzkom okne stále niekedy orezal dlhšie
   meno - to je zámerné (šetrí to miesto tam, kde ho je najmenej), celý text uvidíš pri prejdení myšou.
   Ak by ti to prekážalo, daj vedieť, dá sa to doladiť.

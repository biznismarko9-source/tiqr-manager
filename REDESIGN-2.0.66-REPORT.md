# TIQR Manager 2.0.66 — Settings presun a nový "Completed" ukazovateľ

## Čo si napísal

*"okej, hned ako prvu vec co by som chcel zmenit, bude z tohto odstranit settings a presunut ich dole
dole, kde je log out alebo account settings, tak tie account settings premenujeme na settings, tam ked to
otvoris tak sa ti vsetko ukaze co je v settings, ked sale ostane takto tak ani order ani sale sa nepovazuje
za completed, az ked je vsetko sold delivered a paid tak az vtedy sa to v dashborade presunie, a taktiez
pridajme vec,, kde si vies skontrolovat tieto 3 veci ci su splneen a ak nie tak ktora chyba, taktiez sa
musi pridat nieco podobne ako status a tam si vies zistit presne toto, a to plati vsade nielen orders a
sales ale vsade aj pulls atd,"*

Predtým, než som čokoľvek menil, spýtal som sa ťa na 4 konkrétne veci (dve z nich priamo súviseli s
tvojím vlastným starším rozhodnutím z 2.0.60/2.0.59, ktoré by inak táto zmena tíško prepísala):

- Čo presne znamená "Sold" — skutočný stav lístka (mení sa sám, keď ho reálne predáš), alebo ručný
  štítok "Sold/Unlisted"? Odpovedal si: **skutočný stav lístka**.
- Čo presne znamená "Paid" pri Orders — že tebe zaplatil kupujúci, alebo že ty si zaplatil dodávateľovi?
  Odpovedal si: **že ti zaplatil kupujúci** (rovnaký význam ako pri Sales).
- Kam má táto zmena ísť — má prepísať existujúce Active/Paid (Orders) a Pending/Completed (Sales) taby,
  alebo má byť úplne nový, samostatný ukazovateľ vedľa nich? Odpovedal si: **nový, samostatný
  ukazovateľ** — Active/Paid a Pending/Completed ostávajú presne také, ako boli.
- Ako to zladiť s Pulls, ktoré nemajú sold/delivered/paid vôbec? Odpovedal si: **len zjednotiť vizuál** —
  Pulls (Given) dostane rovnaký štýl, postavený na svojej jedinej existujúcej podmienke (transfer
  hotový); Pulls (Received) zatiaľ nechať bez zmeny.

Nižšie je presne to, čo som na základe toho spravil.

## 1. Settings — presun dole, do menu s Log out

"Settings" už nie je samostatná položka v bočnom paneli. Presunul som ho presne tam, kam si chcel — do
menu, čo sa otvára kliknutím na tvoje meno/avatar dole ("Account settings" / "Log out"):

- "Account settings" je teraz premenované na **"Settings"**.
- Po kliknutí sa už neotvára len podstránka Account — otvára sa **celý Settings hub**, presne tá istá
  obrazovka so všetkými 6 kartami (Lookups, Data, Integrations, Appearance, Software, Account), čo bola
  predtým dostupná len z bočného panelu.
- "Log out" zostal presne tam, kde bol, bez zmeny.

## 2. Nový "Completed" ukazovateľ — Orders, Sales, Pulls (Given)

Na každej z týchto stránok pribudol nový stĺpec **"Completed"** — malý farebný štítok (Badge), rovnaký
štýl, aký appka už používa pre Payment/Status všade inde:

- **Zelený "Completed"** — všetko je splnené.
- **Oranžový, s konkrétnym menom** — ak chýba presne jedna vec, štítok priamo napíše ktorá (napr. "Not
  delivered"), takže nemusíš nič klikať ani hľadať, aby si zistil čo presne chýba.
- Ak chýba viac vecí naraz, štítok ukáže počet ("2 pending") a **po prejdení myšou** (hover) sa zobrazí
  presný rozpis úplne všetkých troch podmienok naraz (Sold/Delivered/Paid — hotovo/čaká pri každej
  zvlášť) — presne to "kde si vies skontrolovat tieto 3 veci", čo si žiadal.

Na **Orders** a **Sales** sú tie 3 podmienky:

1. **Sold** — pri Orders: či sa už všetky lístky z objednávky reálne predali (rovnaká definícia, akú už
   roky používajú Active/Paid taby). Pri Sales je táto podmienka takmer vždy automaticky splnená (Sale
   existuje len pre lístok, čo sa už predal) — **okrem** prípadu, že bol predaj neskôr refundovaný, vtedy
   sa lístok vráti do stavu "available" a "Sold" sa correctly ukáže ako nesplnené.
2. **Delivered** — lístok má nastavené "Delivery status = Delivered".
3. **Paid** — kupujúci/platforma ti reálne zaplatili za tento konkrétny predaj (rovnaké pole, čo pri
   Sales vidíš v stĺpci Status).

Na **Sales** je štítok naviac aj priamo v detaile predaja (Sale Detail), hneď vedľa existujúceho Status
štítku — takže presne tam, kde by si klikol "pozrieť sa bližšie", ho hneď vidíš aj s celým rozpisom.
Rovnako aj na **Order Detail**.

Na **Pulls (Given)** je nová "Completed" kolónka postavená na tej istej, jedinej podmienke, čo appka už
má — checkbox "Done" (transfer hotový) — len teraz v rovnakom vizuálnom štýle ako všade inde. Samotný
checkbox zostal presne tam, kde bol a funguje rovnako ako doteraz. **Pulls (Received)** som podľa tvojej
odpovede zatiaľ nechal bez zmeny — nemá žiadnu podobnú vlastnosť, na ktorej by sa dalo stavať.

Dôležité: **Active/Paid (Orders) a Pending/Completed (Sales) taby som sa vôbec nedotkol** — fungujú
presne tak, ako doteraz (2.0.60/2.0.59). Toto je úplne nový, samostatný ukazovateľ vedľa nich, presne ako
si v odpovedi zvolil.

## Čo som overil

```
cargo test --lib   -> 667 testov (662 + 5 nových), 0 zlyhaní, 3 ignorované
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

5 nových testov pokrýva presne tie hranice, na ktorých by sa táto logika najľahšie pokazila: že
Delivered/Paid sa počíta len z reálne predaných lístkov (nie z celej objednávky), a že refundovaný predaj
sa correctly prestane počítať ako "sold"/"paid" namiesto toho, aby zostal navždy nesprávne označený ako
"chýba platba".

## Zmenené súbory

**Backend:**
- `src-tauri/src/models.rs` — nové polia: `Order.deliveredCount/paidCount`,
  `SaleGroup.soldCount/deliveredCount/paidCount`, `Sale.ticketStatus/ticketDeliveryStatus`.
- `src-tauri/src/commands/orders.rs` — nový JOIN + 2 nové stĺpce v hlavnom Orders dopyte.
- `src-tauri/src/commands/sales.rs` — nové stĺpce v Sales aj Sale Detail dopytoch.

**Frontend:**
- `src/components/Layout.tsx` — Settings preč z bočného panelu, presunuté do account-menu.
- `src/lib/completion.ts` (nový súbor) — spoločná logika pre "Completed" štítok, aby Orders/Sales/Pulls
  všetky používali presne to isté správanie.
- `src/components/ui.tsx` — Badge vie zobraziť hover s rozpisom.
- `src/pages/Orders.tsx`, `src/pages/OrderDetail.tsx`, `src/pages/Sales.tsx`, `src/pages/SaleDetail.tsx`,
  `src/pages/Pulls.tsx` — nový "Completed" stĺpec/štítok.
- `src/lib/types.ts` — nové polia zodpovedajúce backendu.

**Verzia (8 miest):** `2.0.66`.

## STOP — 2 veci, na ktoré potrebujem tvoju odpoveď/kontrolu

1. **Dashboard.** Napísal si "az vtedy sa to v dashborade presunie" — appka dnes nemá na Dashboarde nič,
   čo by sa dalo nazvať "completed" (overil som to poriadne, naozaj tam nič také nie je). Mohol si tým
   myslieť buď (a) presne tento nový štítok, čo som práve pridal na Orders/Sales/Pulls — v tom prípade je
   toto hotové, alebo (b) že chceš aj na samotnej Dashboard obrazovke novú kartičku/sekciu, čo by
   ukazovala napr. koľko objednávok/predajov ešte nie je "Completed". To druhé som zatiaľ nerobil (je to
   ďalšia poriadna kus backendovej práce) — napíš mi, či to chceš, a pustím sa do toho v ďalšej dávke.
2. **Šírka nových stĺpcov.** Predchádzajúce stĺpce v týchto tabuľkách majú šírku poctivo odmeranú podľa
   reálneho obsahu (má to za sebou históriu opravovania, keď to bolo zle). Šírku nového stĺpca "Completed"
   som len odhadol — ak ti bude na niektorej obrazovke pripadať príliš úzky/široký alebo že niečo iné
   kvôli tomu vyzerá stlačené, napíš mi presne kde a doladím to.

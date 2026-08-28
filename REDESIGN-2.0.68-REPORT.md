# TIQR Manager 2.0.68 — Status/Delivery/Payout stĺpce a opravy Dashboardu

## Čo si napísal

*"nevidim to nikde, oprav to tak ze tu ako je status tak bude status to je/ listed, unlisted,sold ,
delivery status Delivered/not delivered, payout status pending a paid, taktiez u orders to tak urob, a
oprav aj grafy, lebo nieco sa s nimi urobilo, a taktiez Purchase cost pocita to zle, oprav to"*

(spolu s 2 screenshotmi — Sale Detail a Dashboard)

To sú 4 samostatné veci. Pri dvoch z nich som sa ťa najprv spýtal, lebo zlý odhad by znamenal poriadny kus
prerábania naspäť:

- **Purchase cost** — má sa prepočítať tak, aby vždy sedel s Profit/Margin/ROI (náklad lístkov reálne
  predaných v danom období), aj keby to znamenalo, že sa bude líšiť od toho, čo appka doteraz ukazovala?
  Odpovedal si: **áno, prepočítať (odporúčané)**.
- **Nový "Status" štítok** — má ukazovať tvoje vlastné ručné značenie Listed/Unlisted/Sold (to isté pole,
  čo si dnes nastavuješ v úprave lístka), alebo skutočný, appkou riadený stav lístka? Odpovedal si: **moje
  vlastné ručné Resale status (odporúčané)**.

Nižšie je presne to, čo som na základe toho spravil.

## 1. Dashboard — "Purchase cost počíta to zle"

Mal si pravdu — a dalo sa to aj poriadne dokázať z tvojho vlastného screenshotu. Keď som zobral tvoje
Margin/ROI a spätne z nich dopočítal, aký náklad (COGS) by musel byť za Profit, vyšlo mi cca **703,50 €**
— ale kartička "Purchase cost" ukazovala **622,16 €**. Dva rôzne čísla, čo by podľa definície mali byť
prepojené.

Príčina: "Purchase cost" sa počítal z lístkov **kúpených** v danom období (podľa dátumu nákupu), zatiaľ čo
Profit/Margin/ROI sa počítajú z lístkov **predaných** v danom období (podľa dátumu predaja). Znie to ako
detail, ale v praxi to znamená dve rôzne skupiny lístkov:

- Lístok kúpený dávno pred týmto obdobím, ale predaný až teraz → jeho náklad sa v "Purchase cost" vôbec
  neobjavil, hoci presne o tomto lístku hovorí Profit/COGS/ROI.
- Lístok kúpený v tomto období, ale ešte nepredaný → jeho náklad sa do "Purchase cost" napočítal, hoci
  Profit/COGS/ROI sa ho vôbec netýka.

Každé z tých dvoch čísel bolo samo o sebe správne pre to, čo malo merať — len vedľa seba nikdy nemohli
sedieť. Teraz **Purchase cost počíta presne to isté** ako Profit/Margin/ROI (náklad lístkov reálne
predaných v období) — takže vzorec **Revenue − Purchase cost − poplatky = Profit** bude na Dashboarde vždy
sedieť, nech zvolíš akékoľvek obdobie.

## 2. Dashboard — grafy vyzerali "pokazené"

Graf vývoja tržieb (Revenue) niekedy vyzeral, akoby klesal, aj keď v skutočnosti len chýbali predaje na
pár dní/týždňov medzi dvoma reálnymi predajmi. Príčina: graf kreslí body rovnomerne rozmiestnené podľa
**poradia**, nie podľa reálneho dátumu — a appka doteraz do grafu posielala **len dni/týždne/mesiace, kde
sa naozaj niečo predalo**. Keď si mal napríklad predaj v pondelok a potom až v piatok, graf dostal len 2
body vedľa seba a nakreslil medzi nimi jednu plynulú čiaru — čo vyzeralo ako pokles cez celý týždeň, hoci
út-štv jednoducho nemali žiadny predaj (mali by tam byť na nule, nie preskočené).

Oprava je celá na strane backendu: graf teraz dostáva **každý deň/týždeň/mesiac v zvolenom období**, aj
tie, kde sa nič nepredalo (tie majú hodnotu 0). Samotný graf (ako vyzerá, ako sa kreslí) som sa vôbec
nedotkol — stačilo mu dodať správne dáta.

## 3. Sale Detail — "Status" rozdelený na 3

Tabuľka lístkov v detaile predaja mala doteraz jediný stĺpec "Status", ktorý v skutočnosti ukazoval len
**stav platby** (pending/paid/refunded) — tvoje vlastné ručné značenie (Listed/Unlisted/Sold) aj Delivery
status tam neboli vidieť vôbec, presne ako si napísal ("nevidim to nikde"). Teraz sú to **3 samostatné
stĺpce**:

- **Status** — tvoje ručné Listed / Unlisted / Sold (rovnaké pole, čo upravuješ v úprave lístka).
- **Delivery status** — Delivered / Not delivered.
- **Payout status** — pôvodný stĺpec, len premenovaný (aby nezamieňal so "Status" vyššie) — presne ten
  istý štítok pending/paid/refunded, čo tam bol doteraz, bez zmeny správania.

## 4. Order Detail — to isté, navyše (nie namiesto)

Detail objednávky mal už predtým stĺpec **"Status"** — ale to je skutočný, appkou riadený stav lístka
(Available/Listed/Sold/Cancelled), naviazaný na tlačidlá "Mark as Available/Listed/Cancelled" nad
tabuľkou. Ten som **vôbec nemenil** — je to dôležité, živé pole. Namiesto prepisovania pribudli **3 nové
stĺpce vedľa neho**:

- **Resale status** — tvoje ručné Listed/Unlisted/Sold (rovnaké pole ako v bode 3 vyššie).
- **Delivery status** — Delivered/Not delivered.
- **Payout status** — stav platby aktuálneho (nerefundovaného) predaja tohto lístka. Toto je nové pole na
  backende (`sale_payment_status`) — využíva presne to isté prepojenie na tabuľku predajov, čo appka už
  roky používa pre cenu predaja v tejto tabuľke, takže žiadne riziko duplicitných/zle napočítaných riadkov.
  Prázdne (pomlčka) pri lístku, čo sa ešte nepredal.

## Čo som overil

```
cargo test --lib   -> 685 testov (684 + 1 nový), 0 zlyhaní, 3 ignorované
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

Pri Dashboarde som opravil aj 6 existujúcich testov, čo overovali starý (chybný) tvar grafu — teraz
overujú, že chýbajúce dni/týždne/mesiace naozaj dostanú nulový bod, nie že sa jednoducho vynechajú. Nový
test pre Purchase cost overuje presne tvoj scenár (lístok kúpený mimo obdobia, ale predaný v ňom, a
naopak) a nový test pre Payout status na Order Detail overuje, že sa ukáže vždy aktuálny predaj, nikdy
predaj, čo bol medzičasom refundovaný.

## Zmenené súbory

**Backend:**
- `src-tauri/src/commands/dashboard.rs` — prepočet Purchase cost; nové funkcie na dopĺňanie prázdnych
  dní/týždňov/mesiacov do grafu; opravených 6 existujúcich testov, 1 nový test.
- `src-tauri/src/commands/sales.rs` — nové pole `ticketResaleStatus` na `Sale` (Sale Detail).
- `src-tauri/src/commands/tickets.rs` — nové pole `salePaymentStatus` na `Ticket` (Order Detail), 1 nový
  test.
- `src-tauri/src/models.rs` — zodpovedajúce nové polia v `Sale` a `Ticket`.

**Frontend:**
- `src/components/ui.tsx` — nové farby štítkov pre Unlisted/Delivered/Not delivered.
- `src/pages/SaleDetail.tsx` — tabuľka lístkov: 1 stĺpec → 3 (Status/Delivery status/Payout status).
- `src/pages/OrderDetail.tsx` — tabuľka lístkov: pôvodný Status nedotknutý, pridané 3 nové stĺpce.
- `src/lib/types.ts` — nové polia zodpovedajúce backendu.

**Verzia (8 miest):** `2.0.68`.

## STOP — 1 vec, na ktorú sa pozri

Nové stĺpce (Status/Delivery status/Payout status na oboch obrazovkách) som musel **zúžiť stĺpec Seat**,
aby sa zvyšné stĺpce zmestili — presne tou istou metódou (percentá šírky), akou sú urobené všetky stĺpce v
appke. Šírky nových stĺpcov sú len odhadnuté podľa dĺžky textu ("Not delivered" je najdlhší), nie
poctivo odmerané ako ostatné stĺpce v týchto tabuľkách. Ak ti niektorý stĺpec bude pripadať príliš
úzky/široký, alebo že sa ti Seat zdá príliš stlačený, napíš mi presne na ktorej obrazovke (Sale Detail
alebo Order Detail) a pri akej šírke okna — doladím to.

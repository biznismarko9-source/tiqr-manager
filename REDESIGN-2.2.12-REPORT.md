# TIQR Manager 2.2.12 — Fulfillment Center

Toto je druhý z dvoch releasov z tvojej poslednej veľkej správy — ČASŤ C (Fulfillment Center).
ČASŤ A a B (Attention Center + Dashboard cleanup) už poslal ako **2.2.11** samostatne, s vlastným
reportom.

Znova: nikde som sa ťa nepýtal na spresnenie. Jedno miesto v zadaní nebolo úplne jednoznačné —
presná definícia "Ready to Complete" — a urobil som pri ňom rozhodnutie sám, vysvetlené nižšie,
aby si ho vedel opraviť, ak si to myslel inak.

## Čo je nové: Fulfillment Center

Nová stránka (nová položka v ľavom menu, hneď pod "Sales") — jedno miesto, kde okamžite vidíš, čo
z predaného ešte treba dokončiť.

**Štyri dlaždice hore** — plnia dvojitú úlohu: sú to zároveň KPI čísla aj klikateľné kategórie
(v tvojom zadaní to boli formálne dve oddelené veci, ale ide o tie isté 4 čísla, len s iným
menom — preto sú na stránke len raz, nie zbytočne dvakrát):

- **Pending Sales** (= "ALL PENDING") — každý predaj, čo ešte nie je celý hotový.
- **Awaiting Payment** (= "PAYMENT") — z toho tie, čo ešte nemajú zaplatené úplne všetko.
- **Awaiting Delivery** (= "DELIVERY") — z toho tie, čo ešte nemajú doručené úplne všetko.
- **Ready to Complete** (= "READY TO COMPLETE") — zaplatené aj doručené naraz, len to ešte
  potrebuje "dokončiť".

Klik na dlaždicu prefiltruje tabuľku dole presne na tú kategóriu. "Awaiting Payment" a "Awaiting
Delivery" sa **môžu prekrývať** — predaj, čo nemá zaplatené ANI doručené, sa objaví v OBOCH naraz
(presne tvoj testovací prípad "oboje pending").

**Tabuľka** má presne polia z tvojho zadania: Event, Ticket/Seats, Sale price, Payment status,
Delivery status, Overall status, a Action. Klik na riadok ALEBO na tlačidlo "Open" v Action
stĺpci — oboje ťa zavedie na existujúcu stránku Sale Detail, žiadna nová navigácia.

## Pravidlo "Completed vs Pending" — presne to, čo appka už robí v Sales

Táto stránka **nepoužíva žiadne nové pravidlo**. Priamo importuje tú istú funkciu, čo appka už
dnes používa v Sales pre taby Pending/Completed (`isSaleGroupDone`) — takže táto stránka sa s
Sales nemôže nikdy rozísť v tom, čo je "hotové". Predaj je Completed len vtedy, keď je predané AJ
doručené AJ zaplatené naraz (alebo plne refundovaný) — presne tvoje pravidlo, nezmenené.

## Rozhodnutie, ktoré som urobil sám — čo presne je "Ready to Complete"

Tvoje zadanie definovalo len KEDY je predaj "Completed" (predané+doručené+zaplatené), nie čo
presne znamená "Ready to Complete" ako samostatná kategória. Vystopoval som to takto:

Appka má pri každej skupine predaja pole "soldCount" — bežne sa rovná počtu tiketov, klesne LEN
vtedy, keď je časť skupiny refundovaná. Appka appka označí predaj ako Completed len vtedy, keď
"soldCount" aj "delivered" aj "paid" sedia na 100 % (alebo je to plný refund). **Z toho vyplýva:
jediný spôsob, ako môže byť predaj naraz plne zaplatený AJ plne doručený, a napriek tomu ešte
Pending, je čiastočný refund** (časť tiketov v skupine vrátená, zvyšok nie). Preto:

**Ready to Complete = plne zaplatené AJ plne doručené.** V praxi to prakticky vždy znamená "táto
dávka len čaká na dokončenie refund/resell papierovačky", nikdy nie skutočne nezaplatený alebo
nedoručený predaj — čiže presne to, čo by malo dávať zmysel ako "už len to doklikni".

**Ak si to myslel inak** — je to jedna funkcia (`isReadyToComplete`), zmena by bola rýchla a nič
iné by nezasiahla.

## Ďalšie menšie rozhodnutia (na tvoje zváženie)

- **KPI a kategórie som zlúčil do jedného riadku 4 dlaždíc** (nie zvlášť KPI karty hore a zvlášť
  filtračné tlačidlá dole) — sú to tie isté čísla, dve by pôsobili zbytočne duplicitne. Použil
  som rovnaký vizuálny štýl, aký som práve zaviedol v 2.2.11 pre Attention Center boxy — pôsobí to
  ako jeden konzistentný systém, nie dva rôzne.
- **"Delivery status" v tabuľke je nová značka** (Sales v tabuľke dnes nemá samostatný stĺpec pre
  doručenie, len spoločnú "Completed" značku) — použil som ale úplne existujúce farby (rovnaké,
  aké appka už používa pri jednotlivých tiketoch v Sale/Order Detail), žiadna nová farba.
- **Fulfillment Center som dal do menu hneď pod Sales**, nie ako úplne samostatnú sekciu (ako
  Price Checker/Finance) — je to užší pohľad NA Sales dáta, nie nová samostatná časť appky.

## Overené DÔLEŽITÉ body — každý som si overil priamo v kóde, nie len sľúbil

- **žiadna zmena refund/resell** — táto stránka len ČÍTA `refundedCount`/`paymentStatus`, presne
  tie isté polia, čo Sales tabuľka už dnes číta. Samotný refund kód som ani neotváral.
- **žiadna zmena batch_id** — nikde v novom súbore sa nepoužíva.
- **žiadna zmena money/cents** — každá suma ide cez presne tú istú funkciu, čo používa Sales
  tabuľka (`formatMoneyOrMixed`), žiadny nový výpočet s centami.
- **Listings/Price Checker/market pricing nezmenené** — nikde sa nespomínajú.
- **Tier/Level nepoužité na pricing** — nikde sa nepoužívajú vôbec.
- **Sales Completed/Pending pravidlo zostáva konzistentné** — nie je len "konzistentné", je to
  doslova tá istá funkcia, importovaná, nie prepísaná.

## Ako som to otestoval

Táto appka nemá žiadny frontend testovací framework (overil som — nikde v projekte, nikdy, nie je
vitest/jest ani jeden `*.test.*` súbor) — frontendová logika sa v tomto projekte vždy overovala
cez `tsc -b` + čítanie kódu + úvahu, presne tak ako pri `isEventDone`/`isOrderDone`/
`isSaleGroupDone` v predchádzajúcich verziách. Tentokrát som pridal navyše aj **jednorazový
skript** (esbuild + Node, mimo repozitára, po overení zmazaný — nie je súčasťou appky ani ZIPu),
ktorý si importoval SKUTOČNÉ funkcie zo skutočných súborov (nie kópie) a overil presne tvoje
testovacie scenáre:

```
payment pending .......... zaplatené menej ako celé, doručené celé -> Pending, PAYMENT kategória
delivery pending .......... doručené menej ako celé, zaplatené celé -> Pending, DELIVERY kategória
oboje pending ............. ani zaplatené, ani doručené celé -> objaví sa v OBOCH kategóriách
ready to complete ......... zaplatené aj doručené celé (čiastočný refund) -> Ready to Complete
completed sa nezobrazí .... plne hotový predaj -> vylúčený z Pending množiny
refund pravidlo ........... plne refundovaný predaj -> vylúčený, rovnako ako v Sales

21 z 21 kontrol prešlo.
```

Klik-na-riadok/Action navigáciu do Sale Detail a Attention Center klik-a-filtrovanie (2.2.11) som
overil čítaním skutočného routovacieho kódu (`App.tsx`) — to je presne to isté "over v kóde"
overenie, aké som robil pri každej predchádzajúcej frontendovej zmene v tomto projekte doteraz
(appka tu v tomto prostredí nemá displej, takže naživo cez skutočné okno appky to nejde spustiť).

```
cargo test --lib   -> 1006 passed, 0 failed, 3 ignored (nezmenené - žiadny .rs súbor sa
                       v 2.2.12 nemenil)
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

## Zmenené súbory

**Frontend (žiadny backendový `.rs` súbor sa nemenil):**
- `src/pages/FulfillmentCenter.tsx` — nová stránka (celá logika + tabuľka)
- `src/pages/Sales.tsx` — `saleGroupCompletionChecks`/`isSaleGroupDone` teraz `export` (predtým
  súkromné), bez zmeny správania
- `src/components/Layout.tsx` — nová položka menu "Fulfillment Center"
- `src/App.tsx` — nová route `/fulfillment`

**Dokumentácia:**
- `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/PROTECTED_AREAS.md`, `CHANGELOG.md`

---

Toľko k Fulfillment Center. Jedna vec na tvoje rozhodnutie: moja definícia "Ready to Complete"
(zaplatené + doručené, vysvetlené vyššie) — ak si to myslel inak, daj vedieť, je to jedna funkcia
na zmenu. Všetko ostatné, vrátane všetkých DÔLEŽITÉ bodov, je presne tak, ako si žiadal.

Tým je hotová aj celá dvojica releasov z tvojej poslednej správy (2.2.11 + 2.2.12). Zhrnutie oboch
nájdeš v mojej poslednej správe v chate.

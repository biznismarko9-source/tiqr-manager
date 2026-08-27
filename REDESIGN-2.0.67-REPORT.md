# TIQR Manager 2.0.67 — Hromadné (bulk) označenie Paid/Delivered

## Čo si napísal

*"no to paid / not paid, delivered not delvered sa musi daj urobit aj naraz, nie vsetko po jednom"*

Presne toto som spravil — priamo v zoznamoch **Orders** a **Sales** (odpovedal si mi na otázku, kam presne
to má ísť: priamo do zoznamu, nie len do detailu jednej objednávky/predaja).

## Čo je nové

V oboch zoznamoch (Orders aj Sales) už poznáš režim výberu (tie isté checkboxy, čo dnes používaš na
hromadné mazanie). Keď teraz v tomto režime vyberieš viac riadkov naraz, nad existujúcim červeným "Delete"
panelom pribudol nový **modrý panel so 4 tlačidlami**:

- **Mark Delivered** / **Mark Not delivered**
- **Mark Paid** / **Mark Pending**

Klikneš raz a zmena sa spraví na **všetkých vybraných naraz** — presne to, čo si žiadal namiesto "jedno po
druhom".

Dôležitá vec: kliknutie na jedno tlačidlo **nezruší výber**. Takže môžeš vybrať napr. 10 objednávok,
kliknúť "Mark Delivered", a hneď potom — stále s tým istým výberom — kliknúť aj "Mark Paid". Výber sa zruší
až keď klikneš na "Clear selection" (alebo "Cancel" pri mazaní).

## Ako presne to počíta, čo sa zmení

**Na Orders** — vyberieš celé objednávky, ale zmena sa aplikuje len tam, kde dáva zmysel:

- **Delivered/Not delivered** sa nastaví len na lístkoch, čo sú reálne **predané** (sold). Ak si vybral
  objednávku, kde ešte nič nie je predané, nič sa nestane — nie je to chyba, len sa nič neoznačí.
- **Paid/Pending** sa nastaví len na aktuálnom predaji každého predaného lístka — a **nikdy** na predaji, čo
  bol medzičasom refundovaný (ten ostáva nedotknutý, presne ako sa má).

**Na Sales** — vyberieš predaje (riadky v zozname, čo môžu byť aj celé dávky/batch viacerých lístkov):

- **Delivered/Not delivered** sa nastaví na **všetkých** lístkoch v predaji/dávke — aj na tom, čo bol
  neskôr refundovaný. Toto je zámerne inak ako pri platbe: či bol lístok reálne doručený je vlastnosť
  lístka samotného, nezávisí od toho, či sa platba neskôr vrátila.
- **Paid/Pending** naopak **preskočí** akýkoľvek refundovaný riadok v dávke — presne tak, ako to už dnes
  robí existujúce tlačidlo "Mark as Paid" v detaile jedného predaja. Vďaka tomu jeden refundovaný lístok v
  dávke 5 lístkov nezablokuje označenie zvyšných 4 ako zaplatených.

V oboch prípadoch: keby si napríklad vybral len objednávky/predaje, kde sa už niet čo meniť (nič predané,
alebo všetko refundované), appka to jednoducho preskočí — nikdy to nespadne na chybu.

## Čo som overil

```
cargo test --lib   -> 683 testov (667 + 16 nových), 0 zlyhaní, 3 ignorované
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

16 nových testov pokrýva presne tie hranice, kde by sa táto logika najľahšie pokazila: že sa počíta len z
reálne predaných lístkov (nie z celej objednávky), že refundovaný predaj sa nikdy "znovu neožije" cez
hromadnú akciu, a že "delivered" a "paid" sa k refundovanému riadku správajú **zámerne opačne** (delivered
ho zahŕňa, paid ho vynecháva) — presne podľa vyššie popísanej logiky.

## Zmenené súbory

**Backend:**
- `src-tauri/src/commands/tickets.rs` — nová `bulk_update_ticket_delivery_status_impl` (interná, spoločná
  pre Orders aj Sales — nikdy sa nezapisuje na dvoch miestach).
- `src-tauri/src/commands/orders.rs` — nové príkazy `bulk_set_orders_delivery_status` /
  `bulk_set_orders_payment_status`.
- `src-tauri/src/commands/sales.rs` — nové príkazy `bulk_set_sale_groups_delivery_status` /
  `bulk_set_sale_groups_payment_status`.
- `src-tauri/src/models.rs` — 4 nové vstupné typy pre tieto príkazy.
- `src-tauri/src/lib.rs` — registrácia 4 nových príkazov.

**Frontend:**
- `src/components/BulkCompletionBar.tsx` (nový súbor) — spoločný panel so 4 tlačidlami, zdieľaný medzi
  Orders aj Sales.
- `src/pages/Orders.tsx`, `src/pages/Sales.tsx` — nový panel zapojený hneď vedľa existujúceho "Delete"
  panelu vo výberovom režime.
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy a volania pre 4 nové príkazy.

**Verzia (8 miest):** `2.0.67`.

## STOP — nič, čo by som potreboval spätne overiť

Tentokrát nemám otvorenú otázku ani odhad, čo by som chcel, aby si skontroloval — táto zmena len pridáva
nové tlačidlá do už existujúceho výberového režimu, nič vizuálne neposúva ani nemení žiadny existujúci
stĺpec/šírku. Ak by ti napriek tomu niečo na tých dvoch obrazovkách sedelo inak, ako by si čakal, napíš mi
presne čo.

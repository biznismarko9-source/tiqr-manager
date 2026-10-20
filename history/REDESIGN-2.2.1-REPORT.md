# TIQR Manager 2.2.1 — Finance Accounts + Lookups redizajn, Price Checker prelinkovanie, Finance ↔ Orders

Štyri samostatné veci, ktoré si chcel v jednej správe:

> *"vo finance accounts treba nejak zjednodusit, urobit ich mensie, a nejak krajsie, necham to na teba ale
> zaberaju zbytocnje vela miesta podla mna"*
>
> *"lookups v nastaveniach treba tiez cele prerobit, chcem aby to bolo tak ze budu tam 3 riadky: Platforms /
> Event categories / Finance categories. po kliknuti sa ti otvori to kde si vies pridat alebo zmazat, ale
> chcem to mat v inom style tak to skus pomenit"*
>
> *"taktiez ked kliknes na danu order alebo sales alebo event, budes mat moznost automaticky hodit do price
> checkeru na dany event"*
>
> *"finance centrum prepojit aj s listkami, lebo kedze si tam zapisujem vsetko, aj tie biznis veci, tak by
> bolo dobre ze ked nakupim listky, tak si viem presne dat do finances a spojit s order, aby to sedelo a
> davalo zmysel"*

Postupne, po poriadku.

## 1. Finance → Accounts — zjednodušené

Predtým to bola mriežka veľkých kariet (2-3 v rade), s veľkým nadpisom, veľkým zostatkom a "opening
balance" ako samostatný riadok pod tým — reálne to zaberalo veľa miesta aj pri pár účtoch. Nechal si mi
voľnú ruku, tak som to spravil ako kompaktný zoznam riadkov (rovnaký princíp, aký už appka používa pri
platformách a kategóriách): ikonka, názov účtu, typ a mena pod ním malým písmom, a zostatok vpravo —
najvýraznejšie číslo v riadku, presne tak ako predtým, len bez toho okolo. "Opening balance" som nezmazal,
len presunul do tooltipu (podrž myš nad zostatkom). Upraviť/zmazať sú teraz malé ikonky vpravo namiesto
veľkých tlačidiel. Farebné rozlíšenie podľa typu účtu (banka/Revolut/PayPal/hotovosť/karta/iné) zostalo,
len menšie.

## 2. Settings → Lookups — nový štýl, presne 3 riadky

Presne podľa zadania: Platforms / Event categories / Finance categories sú teraz tri klikateľné riadky (s
ikonkou, názvom, počtom položiek a šípkou — rovnaký štýl, aký má hlavná stránka Settings pre svoje vlastné
sekcie), namiesto jednej dlhej vždy-otvorenej karty. Kliknutím sa otvorí presne ten istý pridávací/mazací
zoznam ako doteraz (nič na jeho funkčnosti som nemenil) — len v okne (modal), nie natrvalo rozbalené na
stránke.

## 3. Price Checker — prelinkovanie z Order/Sale (Event to už malo)

Pri kontrole som zistil, že **Event detail toto už mal** — od verzie 2.0.81 (tlačidlo "Compare to market
prices"). Chýbalo to len na dvoch miestach, tak som doplnil len tie:

- **Order detail** — nové tlačidlo "Check prices" v hlavičke, prenesie ťa do Price Checkera rovno na event
  tej objednávky.
- **Sale detail** — to isté, okrem prípadu keď jeden predaj (sale group) pokrýva lístky z viacerých rôznych
  eventov naraz — vtedy tlačidlo logicky zmizne, lebo "ten event" by nebol jednoznačný.

Technicky nič nové — použil som presne ten istý mechanizmus, ktorý Event detail používa už rok (a Orders
zoznam pri "New order for this event"), takže žiadne riziko niečo pokaziť inde.

## 4. Finance ↔ Orders — prepojenie, s tvojím rozhodnutím

Toto bola najväčšia zmena. Kým som sa do toho pustil, zistil som niečo dôležité: Finance bolo od svojho
úplného začiatku (2.0.83) **zámerne úplne nezávislé** od Orders/Sales — priamo v kóde je k tomu poznámka,
prečo (aby sa nič nikdy nezapočítalo dvakrát, raz v Dashboarde/Orders a raz vo Financiách). Tvoja požiadavka
ale presne toto čiastočne chce zmeniť — spojiť konkrétny nákup lístkov s konkrétnym Finance záznamom.

Spýtal som sa ťa preto priamo, akým presne spôsobom to urobiť, s tromi možnosťami. Vybral si
**"Prepojenie + predvyplnenie"** — presne toto som postavil:

- Na Order detail je nové tlačidlo **"Record in Finance"**. Otvorí okno, kde je suma a mena
  **needitovateľná** — je napevno rovnaká, ako má samotná objednávka. Vyplniť/zmeniť vieš len dátum
  (predvyplnený dátumom nákupu), Osobné/Biznis (predvyplnené na Biznis), kategóriu, účet a poznámku.
  Uloží sa ako bežný Finance záznam, len s neviditeľnou "niťou" naspäť na tú objednávku.
- Keď je objednávka takto už zaznamenaná, tlačidlo sa zmení na "Add another" (dá sa to urobiť aj
  viackrát — napr. záloha teraz, doplatok neskôr) a pri objednávke uvidíš koľko záznamov už má.
- **Prečo je suma needitovateľná**: to je práve to, čo zaručuje "aby to sedelo" — nie je možné, aby sa
  časom Finance záznam a objednávka rozišli v čísle, lebo sa to do Finance nikdy nezapisovalo ručne
  nanovo.
- **Prečo to stále NEROZBIJE to pôvodné pravidlo** (žiadne dvojité počítanie): je to len "mäkký" odkaz —
  nikde v appke (Dashboard, Finance prehľady) sa čísla z Orders a z Finance nesčítavajú dokopy do jedného
  súčtu. Zostávajú to dva oddelené pohľady, presne ako doteraz — len teraz medzi nimi vieš preklikať a
  appka si pamätá, ktorý záznam patrí ku ktorej objednávke.

Bežné pridávanie Finance záznamov (tlačidlo "New entry" vo Financiách, opakujúce sa výdavky) je úplne
nedotknuté — prepojenie sa objaví len vtedy, keď ho začneš z Order detailu.

## Čo som si dal pozor, aby sa nepokazilo

Keď upravuješ prepojený záznam neskôr vo Financiách (napr. len opravíš poznámku, alebo prekonvertuješ menu),
appka teraz **musí** to prepojenie na objednávku zachovať — predtým by pri obyčajnom uložení formulára
mohlo dôjsť k tichému "odpojeniu" (keďže ten formulár nemá tlačidlo na nastavenie/zrušenie prepojenia). Toto
som si všimol sám, ešte pred akýmkoľvek testovaním, a opravil na oboch miestach, kde by sa to mohlo stať.
Zapísané aj do interných poznámok projektu, nech na to nezabudne žiadna budúca úprava.

## Čo som overil

```
cargo test --lib   -> 934 testov, 0 zlyhaní, 3 ignorované (928 pôvodných + 6 nových)
cargo clippy       -> čisto (žiadne nové upozornenia, len staré, nesúvisiace s touto zmenou)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.2.1 build" v hlavičke)
```

6 nových testov pokrýva: prepojenie sa uloží a načíta správne aj s kódom objednávky, záznam bez objednávky
funguje ako doteraz, neexistujúca objednávka sa odmietne (pri vytvorení aj úprave), zmazanie objednávky
neodstráni jej Finance záznamy (len sa odpoja), a nový príkaz na vypísanie záznamov jednej objednávky vracia
naozaj len tie jej.

## Zmenené súbory

**Backend (Rust):**
- `src-tauri/migrations/021_finance_entry_order_link.sql` — nová migrácia, stĺpec `order_id` na
  `finance_entries`
- `src-tauri/src/commands/finance_entries.rs` — validácia, JOIN na kód objednávky, nový príkaz
  `list_finance_entries_for_order`, 6 nových testov
- `src-tauri/src/models.rs`, `src-tauri/src/db.rs`, `src-tauri/src/lib.rs` — registrácia
- `src-tauri/src/commands/finance_recurring.rs`, `finance_accounts.rs`, `finance_forecast.rs` — doplnené o
  nové pole (bez zmeny správania)
- `src-tauri/src/commands/database.rs` — očakávaný počet migrácií 20 → 21

**Frontend:**
- `src/pages/finance/Accounts.tsx` — nový kompaktný zoznam účtov
- `src/pages/Settings.tsx` — 3-riadkový Lookups
- `src/pages/OrderDetail.tsx` — tlačidlo "Check prices", nová karta "Finance" + okno "Record in Finance"
- `src/pages/SaleDetail.tsx` — tlačidlo "Check prices"
- `src/pages/finance/Transactions.tsx`, `Overview.tsx` — zachovanie prepojenia pri úprave/konverzii
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy a volanie

**Verzia (9 miest v 7 súboroch):** `2.2.1`.

## STOP

2.2.1 hotové, otestované a zabalené. Skontroluj:

1. **Finance → Accounts** — pozri si nový kompaktný zoznam, over že zostatky a farby podľa typu sedia, a že
   tooltip so začiatočným zostatkom funguje.
2. **Settings → Lookups** — klikni na všetky 3 riadky, over že sa v každom vie pridať/zmazať položka ako
   doteraz.
3. **Order detail** aj **Sale detail** — klikni "Check prices", over že ťa to hodí do Price Checkera na
   správny event. Pri predaji naprieč viacerými eventmi over, že tlačidlo správne chýba.
4. **Order detail → "Record in Finance"** — vytvor si testovací záznam, over že suma/mena sedí s
   objednávkou a nedá sa zmeniť. Skús aj "Add another". Potom si ten záznam pozri vo Financiách (Overview →
   Transactions) — mal by tam byť vidieť odkaz na číslo objednávky.
5. Skús ten istý záznam vo Financiách upraviť (napr. poznámku) a prekonvertovať do inej meny — over, že
   prepojenie na objednávku ostane aj po oboch úpravách.

# TIQR Manager 2.0.62 — oprava: Total Cost v Summary bloku ukazovalo 0,00 €

## Čo si mi napísal

Najprv: *"...taktiez som si vsimol ze je problem s Total Cost, ukazuje 0 a nic nevypocitava..."* a potom
obrázok s "Total Cost: 0,00 €". Neskôr, bez textu, 3 screenshoty — celý hárok s formula bar-om ukazujúcim
`AC2 = =SUM(H:H)`, a crop stĺpca H ("Total Purchase..."), kde jasne vidno reálne nenulové hodnoty (337,12 /
50,00 / 50,00 / 300,12 / 431,84 / 402,00 / 124,49 / 386,72 / 235,44 atď).

Presne tie screenshoty mi ukázali skutočnú príčinu — a je iná, než som si myslel v 2.0.61.

## Dôležité: toto NIE JE spôsobené Fix sync (ani ničím nedávnym)

V 2.0.61 som opravil `currency catch-up` krok v domnení, že práve ten spôsobil nulu v Total Cost pri
Fix sync. Táto oprava bola správna a užitočná poistka sama o sebe (nikdy viac sa nezapíše 0,00 do Total
Purchase Price bunky), ale — ako presne ukazujú tvoje screenshoty — **nebola to skutočná príčina toho, čo
si videl**. Vzorec `=SUM(H:H)` v bunke Total Cost bol nastavený zle **od úplného začiatku**, odkedy appka
v 2.0.40 tento Summary blok vôbec vytvorila. Fix sync (ani nič iné nedávne) toto nespôsobil — len si si toho
predtým nevšimol, lebo si sa na Total Cost dovtedy tak podrobne nepozeral.

## Čo sa presne deje (v ľudskej reči)

Appka do hárku zapisuje bežné hodnoty (vrátane "Total Purchase Price", teda tvojho stĺpca H) spôsobom, ktorý
Google Sheets núti brať ich **doslovne ako text**, nie ako číslo — robí to zámerne, aby si Sheets nikdy
nesplietol napríklad dátum a nepreformátoval si ho podľa vlastnej hlavy. To je v poriadku pre zobrazenie aj
pre riadkové vzorce (Revenue/Profit v každom riadku fungujú správne, lebo tie appka píše iným, špeciálnym
spôsobom presne na tento účel).

Problém je, že Google Sheets funkcia `SUM()` **potichu preskočí každú bunku, ktorá je text**, aj keď v nej
vidíš číslo — proste ju ráta ako keby tam nebolo nič. Presne to sa deje v `=SUM(H:H)`: keďže každá bunka
v stĺpci H je (zámerne, appkou) uložená ako text, `SUM()` z nich napočíta presne 0 — úplne bez ohľadu na to,
čo v nich reálne je. To vysvetľuje presne to, čo ukazuje tvoj screenshot: vzorec aj reálne dáta sú tam
v poriadku, len `SUM()` na text jednoducho nefunguje.

(Total Paid a Total Unpaid tento problém nemajú, lebo tie už od 2.0.42 používajú iný trik — pozri nižšie.)

## Oprava

Total Cost teraz namiesto `=SUM(H:H)` používa `=SUMPRODUCT((H2:H100000)*1)` — vynásobenie každej bunky
číslom 1 prinúti Google Sheets prečítať ju ako číslo aj keď je uložená ako text (kým `SUM()` text len ticho
preskočí, `SUMPRODUCT` s `*1` ho prepočíta). Presne tento istý trik appka už roky používa pre Total Paid
a Total Unpaid, takže ide o overený, konzistentný spôsob, nie o nový nápad.

**Táto oprava sa prejaví až po tom, čo appka nabudúce prepíše Summary blok v tvojom hárku** — teda po
kliknutí na ktorékoľvek z: **Order sync, Sales sync, Push orders, Push sales** (ktorékoľvek z nich pri behu
obnoví aj vzorce v Summary bloku). Netreba nič mazať ani prepisovať ručne — stačí použiť appku ako doteraz
a formula sa opraví sama.

## Čo som overil

```
cargo test --lib   -> 630 testov, všetky prešli, 0 zlyhaní, 3 ignorované (žiadne nové testy,
                       len 3 existujúce upravené tak, aby očakávali nový text vzorca)
npx tsc -b         -> 0 chýb
npm run build      -> OK (frontend touto opravou vôbec nebol dotknutý)
```

## Zmenené súbory

**Backend:**
- `src-tauri/src/commands/orders_sheet_sync.rs`:
  - `plan_orders_summary_updates`: vzorec bunky Total Cost zmenený z `=SUM(...)` na
    `=SUMPRODUCT((...)*1)` — rovnaký princíp ako Total Paid/Total Unpaid
  - 3 existujúce testy upravené tak, aby očakávali nový text vzorca (žiadne nové testy neboli potrebné,
    keďže táto funkcia je už pokrytá presnými testami na text vzorca)

**Verzia (8 miest):** `2.0.62`.

## STOP

2.0.62 opravuje skutočnú príčinu nulového Total Cost — je to samostatná, staršia chyba vo vzorci, nesúvisiaca
s Fix sync ani s ničím z 2.0.60/2.0.61. Po nainštalovaní tejto verzie stačí spustiť ktorékoľvek zo
synchronizačných/push tlačidiel a vzorec sa v hárku opraví sám.

Ak by si medzičasom v histórii verzií hárku (Súbor → História verzií, spomínal som to v 2.0.61 reporte)
našiel niečo iné, čo vyzerá zle, daj vedieť — inak podľa mňa je téma "oprav tabuľku" týmto uzavretá a môžeme
sa presunúť na 3. bod: automatické rozpoznávanie kategórie (Football/Concert/...) z názvu eventu.

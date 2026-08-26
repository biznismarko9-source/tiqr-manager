# TIQR Manager 2.0.43 — Profit len pri Sold, a orders zo syncu rovno ako paid

## Čo si mi napísal

Poslal si mi 2 screenshoty (stĺpec Profit s veľkými zápornými číslami, formula `=P9-H9` viditeľná v bunke,
a Summary blok s Total Profit -1 712,78 €) a k tomu správu:

*"ten profit sa musi pocitat az vtedy, ked je status sold, inak nie, taktiez, ked das sync sheet a vypise
to orders, tak v dashboirade sa to ukazuje ako unpaid, ked to uz je v googlee sheets tak sa to automaticky
pri sync musi zmenit na paid, zatial toto oprav"*

Obe opravy sú hotové - poďme si to prejsť po jednom.

## 1. Prečo Profit ukazoval veľké mínusové čísla

Vzorec pre Profit (`=Revenue-Total Purchase Price`) sa doteraz počítal úplne bez ohľadu na stĺpec Status.
Kým lístok ešte nie je predaný, Payout Per Ticket je prázdny (v Sheets sa počíta ako 0), takže Revenue je
naozaj 0 - lenže Profit potom vyšiel ako `0 - Cost`, teda mínus celá nákupná cena, na každom jednom
nepredanom lístku. A keďže Total Profit v Summary bloku je jednoducho súčet celého stĺpca Profit, presne
toto sa premietlo aj do tvojho -1 712,78 €.

Najjednoduchšia oprava by bola dať tam podmienku ("počítaj Profit len keď je Status Sold") - lenže presne
takáto podmienková funkcia (`IF(...)`) by mohla znova spôsobiť úplne ten istý problém s čiarkou/
bodkočiarkou, čo som opravoval minule pri Total Paid/Total Unpaid (2.0.42). Preto som to napísal inak - bez
IF, len ako násobenie: *(je Status "Sold"?) × (Revenue - Cost)*. Keď je Status "Sold", výsledok je presne
ako doteraz (Revenue mínus Cost). Keď Status "Sold" nie je, výsledok je jednoducho 0 - žiadne mínusové
číslo, žiadna zavádzajúca hodnota. A keďže je to len násobenie/odčítanie/porovnanie, nie žiadna funkcia s
viacerými argumentmi, funguje to bezpečne na akomkoľvek jazykovom nastavení tabuľky, presne ako minulá
oprava Total Paid/Total Unpaid.

Total Profit v Summary bloku som nemusel meniť vôbec - ten len sčítava, čo je v stĺpci Profit, takže sa to
opravilo samo, akonáhle je samotný stĺpec Profit správny.

## 2. Prečo sa objednávky zo syncu ukazovali v dashboarde ako unpaid

Keď Order sync vytvorí novú objednávku, appka doteraz vôbec nenastavovala, či je zaplatená - a keď sa to
nenastaví, appka si to sama doplní na "unpaid". Presne toto číslo potom počíta aj dashboard vo svojom
"unpaid" ukazovateli.

Teraz appka pri každej objednávke vytvorenej cez Order sync rovno nastaví "paid" - bez výnimky, presne ako
si napísal. Netýka sa to "Payout status" stĺpca v tabuľke (to je iná vec - či ti už platforma vyplatila
peniaze za predaný lístok) - toto je čisto o tom, či je objednávka (nákup lístkov) sama o sebe označená ako
zaplatená.

## Čo teraz urobiť

Nič špeciálne ručne netreba. Skús Order sync a pozri sa na dashboard - novo zosynchronizované objednávky by
už nemali pribúdať do "unpaid". A na Orders & Sales tabuľke skús "Update sheet" - Profit stĺpec by mal
ukazovať 0 € pri nepredaných lístkoch namiesto veľkého mínusu, a Total Profit v Summary bloku by mal byť
oveľa realistickejšie číslo. Ak niečo nesedí, pošli mi prosím screenshot ako doteraz.

## Testy a build

```
cargo test --lib -> 560 passed, 0 failed, 3 ignored (3 nové testy oproti 2.0.42)
cargo check --lib -> 0 chýb
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

Táto verzia sa dotýka len backendu (Rust) - žiadny súbor v `src/` (React/UI) sa nemenil.

## Zmenené súbory

**Backend (1 súbor):** `src-tauri/src/commands/orders_sheet_sync.rs`.

**Verzia (8 miest):** ako vždy, všetkých na `2.0.43`.

## STOP

1. Skús Order sync - novo vytvorené objednávky by sa v dashboarde už nemali počítať ako unpaid.
2. Na Orders & Sales skús "Update sheet" - Profit stĺpec by mal ukazovať 0 € pri nepredaných lístkoch
   namiesto veľkého mínusu, a Total Profit v Summary bloku by mal byť realistické číslo.
3. Čokoľvek nezvyčajné - pošli mi prosím screenshot, nech to viem rýchlo dohľadať a opraviť.

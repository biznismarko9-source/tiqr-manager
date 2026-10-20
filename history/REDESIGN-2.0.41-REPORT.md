# TIQR Manager 2.0.41 — Oprava: Orders & Sales "Update sheet" chyba

## Čo sa stalo

Toto je oprava bugu, ktorý som spôsobil ja vo verzii 2.0.40 - sorry za to. Keď si klikol "Update sheet" na
Orders & Sales, appka nahlásila chybu:

```
the sheet's dropdowns/Revenue/Profit formulas could not be refreshed this time: Google Sheets rejected
the request (400 Bad Request): Range (Orders!AB1) exceeds grid limits. Max rows: 1001, max columns: 26
```

## Prečo sa to stalo

Nový Summary blok (Total Cost/Revenue/Profit/Paid/Unpaid) sa v tvojej tabuľke umiestnil do stĺpca AB - to
je 28. stĺpec zľava. Bežná Google tabuľka má pri vytvorení štandardne len 26 stĺpcov (A až Z) - a presne
toľko mala aj tvoja, keďže doteraz nebol dôvod, aby bola širšia. Keď appka poslala príkaz na
naformátovanie hlavičky Summary bloku (tučné písmo + podfarbenie) do stĺpca AB, Google to odmietol, lebo
ten stĺpec v tabuľke ešte fyzicky neexistoval.

Horšie je, že tento formátovací príkaz cestoval v tom istom "balíku" príkazov ako existujúce dropdowny a
farebné označovanie (Status/Payout status/atď.) - a Google Sheets spracuje celý balík buď celý naraz,
alebo vôbec nič z neho. Takže keď zlyhala len tá jedna nová vec, potiahla so sebou aj dropdowny a farby,
ktoré predtým fungovali úplne v poriadku. A keďže sa celý postup zastavil hneď pri tejto chybe, Summary
blok sa v skutočnosti ešte vôbec nestihol zapísať - ani čísla, ani vzorce, nič.

## Čo som opravil

Appka si teraz pred každým takýmto formátovacím príkazom sama zistí, aký je aktuálny skutočný rozmer
tvojej tabuľky (koľko riadkov a stĺpcov naozaj má), a ak niečo, čo sa chystá zapísať, presahuje tento
rozmer, najprv tabuľku sama zväčší - v tom istom kroku, predtým než skúsi čokoľvek iné. Tak sa už nemôže
stať, že by jedna nová vec pokazila aj to, čo predtým fungovalo.

Rovnakú opravu som pridal aj do Pulls tabuľky, hoci tam sa tento problém v skutočnosti nemôže stať (TIQR
ID aj Total price ostávajú vždy dobre pod 26 stĺpcami) - je to len poistka do budúcna, pre istotu.

## Čo teraz urobiť

Skús prosím znova kliknúť "Update sheet" na Orders & Sales (alebo počkaj na najbližší normálny sync/push)
- tentokrát by sa mala tabuľka sama zväčšiť a Summary blok aj dropdowny/farby by sa mali zapísať v poriadku,
bez chyby. Ak by sa objavila čo i len podobná hláška znova, pošli mi prosím screenshot presne tak ako
minule - veľmi to pomáha.

## Testy a build

```
cargo test --lib -> 530 passed, 0 failed, 3 ignored (17 nových testov, vrátane testu, ktorý presne
                     reprodukuje tvoju reálnu chybu - stĺpec AB - a overuje, že sa už správne opraví)
cargo check --lib -> 0 chýb
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

Táto oprava sa dotkla len backendu (Rust) - frontend sa nemenil.

## Zmenené súbory

**Backend (3 súbory):** `src-tauri/src/commands/orders_sheet_sync.rs`, `src-tauri/src/commands/
pulls_sheet_sync.rs`, `src-tauri/src/google_sheets.rs` (nová pomocná funkcia, ktorá vie tabuľku podľa
potreby zväčšiť).

**Frontend:** žiadna zmena.

**Verzia (8 miest):** ako vždy, všetkých na `2.0.41`.

## STOP

1. Skús "Update sheet" na Orders & Sales - malo by to teraz prebehnúť bez chyby a Summary blok by mal byť
   vidno v tabuľke presne tak, ako bolo v pláne v 2.0.40.
2. Ak uvidíš čokoľvek podobné tejto chybe znova (alebo čokoľvek iné nezvyčajné), pošli mi prosím screenshot
   - takto rýchlo viem presne určiť, čo sa stalo, a opraviť to.

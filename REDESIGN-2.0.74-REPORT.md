# TIQR Manager 2.0.74 — Prvá dávka animácií a efektov

## Čo si vybral

Na otázku, ako veľký urobiť prvý krok pri animáciách, si vybral **"Menší, cielený krok"** — jemné prechody
na pár najviditeľnejších miestach, nie rovno veľký zásah naprieč celou appkou. Presne to je táto verzia.

## Čo som pridal

Vybral som miesta, s ktorými sa stretávaš najčastejšie pri bežnom používaní appky:

**Toasty (tie malé hlásenia vpravo dole)** — doteraz sa objavili aj zmizli úplne naraz, jedným skokom. Teraz
sa jemne "vsunú" pri objavení a jemne vyblednú pri zmiznutí namiesto toho, aby proste zmizli zo dňa na deň.
Mimochodom, pri kontrole som našiel skutočnú drobnú chybu — appka sa už predtým *pokúšala* toasty animovať,
ale odkazovala na animáciu, ktorá nikde v appke reálne neexistovala, takže sa nič nedialo. Teraz to naozaj
funguje.

**Modálne okná a potvrdzovacie dialógy** (napr. "Restore this backup?", "Remove this platform?" a
podobné) — doteraz sa objavili jedným skokom. Teraz sa pozadie jemne stmaví a samotné okno jemne "vyrastie"
namiesto toho, aby sa proste objavilo.

**Profilová ponuka vľavo dole** (meno + Settings/Log out) — rovnaký jemný nástup ako pri modálnych oknách,
prispôsobený tomu, že sa táto ponuka otvára smerom nahor.

**Tlačidlá naprieč celou appkou** — pri kliknutí sa teraz jemne "stlačia" (mierne zmenšia), presne tak, ako
to robia tlačidlá v telefóne alebo v natívnych aplikáciách. Malá vec, ale je to presne to, čo appku robí
hmatateľnejšou/prirodzenejšou na používanie namiesto toho, aby pôsobila len ako webová stránka.

Zámerne som sa **nedotkol** prechodov medzi stránkami (napr. Dashboard → Orders) — dalo by sa to spraviť, ale
poriadne by to vyžadovalo viac zásahov do toho, ako appka načítava dáta pri prepínaní stránok, a riziko, že by
sa niečo pri tom pokazilo (napr. krátke bliknutie na "Loading" tam, kde predtým nebolo), mi pri "menšom kroku"
neprišlo v pomere k tomu, čo by to pridalo. Ak sa ti táto prvá dávka bude páčiť, môžeme sa na to pozrieť
v ďalšom kole.

Nič z tohto nepridáva žiadnu novú knižnicu — je to len obyčajné CSS, presne v duchu "menšieho kroku".

## Čo som overil

```
cargo test --lib   -> 703 testov, 0 zlyhaní, 3 ignorované (nedotknuté - táto zmena je len frontend)
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

Keďže ide o animácie, obyčajný beh testov ich vizuálne neoverí — preto som navyše skontroloval priamo
vyexportovaný, zminifikovaný CSS súbor appky (presne to, čo appka reálne použije) a potvrdil, že všetky 3
nové animácie sa tam naozaj nachádzajú, so správnymi hodnotami, nič minifikátor "nezjedol" ako údajne
nepoužité — presne ten istý typ chyby, akú som pri tejto príležitosti našiel a opravil pri toastoch.

## Zmenené súbory

**Frontend:**
- `src/index.css` — 3 nové zdieľané animácie (`fadein`, `pop-in`, `toast-out`).
- `src/lib/toast.tsx` — oprava mŕtvej animácie + nová plynulá animácia zmiznutia (dvojfázové odstránenie).
- `src/components/ui.tsx` — `Modal`, `ConfirmDialog` (nástup), `Button` (stlačenie pri kliknutí).
- `src/components/Layout.tsx` — profilová ponuka (nástup).

**Verzia (8 miest):** `2.0.74`.

## Čo bude ďalej

Z tvojej pôvodnej správy zostáva už len najväčší kus — **Dashboard upozornenie vpravo hore + push
notifikácie (desktop, email, Pushover)**. Idem si to teraz poriadne premyslieť (rovnako dôkladne, ako pri
oddelených dátach účtov v 2.0.72), keďže ide o novú architektúru s citlivými údajmi (heslá/kľúče) a tromi
rôznymi spôsobmi doručenia — dostaneš to ako samostatnú, väčšiu dávku.

## STOP — nič, čo by som potreboval spätne overiť

Čisto vizuálna zmena, žiadna logika ani dáta sa nemenili. Pokojne skús kliknúť na nejaké tlačidlo, otvoriť
nejaký modál (napr. "New order") alebo počkať na nejaký toast (napr. po uložení niečoho) a pozri, či sa ti
to páči — ak by ti niektorá animácia prišla príliš pomalá/rýchla alebo zbytočná, pokojne to doladím alebo
niektorú úplne vypnem.

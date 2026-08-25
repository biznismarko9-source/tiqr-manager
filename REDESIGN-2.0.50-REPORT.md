# TIQR Manager 2.0.50 — Convert to EUR pri novej objednávke

## Čo si mi napísal

*"pri roznych menach tam musi byt auto change, napr ked budem mat nejake listky v gbp alebo usd, presne
podla realnej hodnoty meny sa to vie par klikmi zmenit, cize napr mam 20gbp, podla aktualneho kurzu je
teraz 23,38 eur tak musi to automaticky vediet a urobit"* — a potom, keď sme si vyjasnili, že na toto
netreba AI, len samotný prepočet podľa aktuálneho kurzu.

## Čo je nové

Vo formulári **New Order** (Orders → New Order) je teraz pri poli Currency tlačidlo **Convert to EUR** —
objaví sa len vtedy, keď si zvolil inú menu ako EUR (GBP, USD alebo ktorúkoľvek z ďalších 11, čo appka
pozná). Klikneš na neho a appka:

1. Stiahne si aktuálny kurz naživo (nie z appky, ale z reálnej služby na internete).
2. Prepočíta podľa neho Unit purchase price, Unit purchase fees, Other costs aj Pull fee naraz.
3. Prepne Currency na EUR.

Presne tvoj príklad — 20 GBP sa pri kurze 1 GBP = 1,1689 EUR prepočíta na 23,38 EUR. Po kliknutí ti appka
napíše aj presný kurz a k akému dátumu platí (kurzy sa menia deň čo deň), aby si vždy videl, podľa čoho to
počítalo — nič sa nedeje "potichu".

Kurz beriem z Frankfurter — bezplatná služba, čo sleduje oficiálne referenčné kurzy Európskej centrálnej
banky, netreba na to žiadny platený účet ani heslo.

## Kde to funguje a kde (zatiaľ) nie

Toto tlačidlo je zatiaľ len na formulári **New Order** — teda vo chvíli, keď objednávku ešte len vytváraš.
Zámerne som ho nedal do úpravy už existujúcej objednávky ani do ceny za listok/predaja: keď si objednávku
raz vytvoríš, appka si menu aj sumy uloží k jednotlivým lístkom a ďalej ich už nemení — takže "prepočítať"
existujúcu objednávku by znamenalo potichu prepisovať čísla, ktoré si už niekde videl/použil, a to som
nechcel robiť bez toho, aby sme si o tom pohovorili zvlášť. Ak by si chcel aj toto, daj vedieť a pozrieme sa
na to ako na ďalší krok.

Keďže kurz sa sťahuje naživo z internetu, tlačidlo potrebuje pripojenie — ak by náhodou nešlo internetu
(alebo by tá služba bola dočasne nedostupná), appka ti to jasne napíše namiesto toho, aby niečo hádala.

## Ako som to overoval

```
cargo test --lib  -> 589 testov, všetky prešli (585 pôvodných + 4 nové na tento prepočet)
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

Skutočný internetový kurz som si v tomto prostredí nevedel naživo vyskúšať (rovnaké obmedzenie ako pri
Google prihlásení/Sheets/Firebase, čo poznáš z minula) — reálnu službu som si ale overil inou cestou, aby
som mal istotu, že appka bude vedieť jej odpoveď správne prečítať, a k tomu pridal automatické testy presne
na túto odpoveď. Naostro to teda uvidíme až po inštalácii u teba.

Predtým, než som to poslal, som dal kód ešte raz prejrieť — jedno kolo na backend (Rust) časť, druhé na
frontend (to, čo vidíš na obrazovke) časť, nezávisle od seba. Chytili mi pri tom pár vecí, čo som opravil
skôr, než si to vôbec dostal: napríklad že keby si zatvoril okno New Order uprostred prepočtu a hneď otvoril
nové pre inú objednávku, mohli sa tam za istých okolností objaviť staré prepočítané čísla namiesto nových —
teraz appka takýto starý výsledok jednoducho zahodí. Podobne, keby si do niektorého poľa napísal niečo, čo
nie je platná suma, appka to predtým potichu brala ako 0 — teraz namiesto toho ukáže chybu a prepočet
nespustí, presne tak, ako to už appka robí pri samotnom vytváraní objednávky.

## Čo teraz urobiť

1. Nainštaluj 2.0.50.
2. Choď do Orders → New Order, zvoľ menu inú ako EUR (napr. GBP), zadaj nejakú sumu do Unit purchase
   price.
3. Klikni Convert to EUR a skontroluj, že suma aj mena sa zmenia a že ti appka napíše, podľa akého kurzu.

## Zmenené súbory

**Backend (Rust):** `src-tauri/src/fx.rs` (nový súbor — sťahovanie kurzu + prepočet),
`src-tauri/src/commands/currency.rs` (nový súbor — prepojenie na appku), `src-tauri/src/lib.rs`,
`src-tauri/src/commands/mod.rs`, `src-tauri/src/error.rs` (drobná úprava komentára, žiadna zmena správania).

**Frontend:** `src/pages/Orders.tsx` (nové tlačidlo a jeho logika), `src/lib/api.ts`, `src/lib/types.ts`.

**Verzia (8 miest):** ako vždy, všetkých na `2.0.50`.

## STOP

1. Nainštaluj 2.0.50 (spusti `1-CLICK-UPDATE.bat`, počkaj na zelený build).
2. Vyskúšaj Convert to EUR podľa krokov vyššie — najdôležitejšie je overiť, že to naozaj vie stiahnuť
   aktuálny kurz (na to potrebuješ bežné pripojenie na internet, nič iné).
3. Ďalej idem na tú automatickú kategorizáciu eventov pri synchronizácii so Sheetom (tá, čo sme sa bavili,
   že to bude už naozaj AI, nie len hľadanie kľúčových slov).

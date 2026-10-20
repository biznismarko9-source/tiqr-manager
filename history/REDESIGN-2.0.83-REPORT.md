# TIQR Manager 2.0.83 — Financie (nová sekcia)

> *"chcel by som mat vacsi prehlad o mojivh financiach, chcel by som mat moznost si vediet zapisat
> vydavky, kde boli peniaze minute, kedy, kolko, kolko som prijal atd"*

Najprv som ti poslal návrh (`navrh-osobne-financie.md`) a spýtal sa na 5 vecí. Odpovedal si:

1. Sekcia v TIQR Manageri (nie samostatná appka).
2. Aj osobné aj biznis peniaze naraz ("aj aj").
3. Hlavne euro, s možnosťou konverzie a analýzami v EUR.
4. Ručné zapisovanie stačí.
5. Kategórie mám navrhnúť ja.

Presne podľa toho som to postavil. Nová položka **Finance** je teraz v sidebari, úplne samostatná (rovnaké
zaobchádzanie ako dostal Price Checker).

## Ako to funguje

Hore si vyberieš **obdobie** (Dnes / Tento mesiac / Tento rok / Celé obdobie / Vlastné) a **rozsah** (Všetko
/ Osobné / Biznis) — všetko pod tým (karty, grafy, zoznam) sa prepočíta podľa tejto voľby.

Tri karty ukážu **Príjem**, **Výdavky** a **Zostatok** za vybrané obdobie (vždy v EUR). Pod nimi je graf
"Výdavky podľa kategórie" (kde presne mizli peniaze) a graf "Príjmy vs. výdavky podľa mesiacov" (trend v
čase). Úplne dole je zoznam všetkých záznamov — dá sa v ňom hľadať, filtrovať podľa typu/kategórie, a
kliknutím na riadok (alebo ceruzkou/košom vpravo) upraviť alebo zmazať.

Tlačidlom **"New entry"** pridáš nový záznam: Príjem/Výdavok, Osobné/Biznis, dátum, suma + mena, kategória
(voliteľná), miesto/od koho, poznámka.

### Osobné aj biznis peniaze naraz — ako to funguje pod kapotou

Toto je dôležité vysvetliť, lebo je to jedna z väčších rozhodnutí v tejto verzii. Financie sú **úplne
samostatný, ručne vedený zoznam** — nie je to prepojené s Orders/Sales (kde už biznis peniaze sleduješ).
Keď si zvolíš "Biznis" pri zázname, je to len štítok na tomto jednom zázname, nič viac. Zvolil som to takto
zámerne, z dvoch dôvodov:

- Keby appka sama ťahala čísla z Orders/Sales do Financií, riskuje sa, že sa niečo započíta dvakrát (raz v
  Dashboarde, raz vo Financiách) — a to by ti dalo nesprávny obrázok o peniazoch, čo je presne to, čomu sa
  chceme vyhnúť.
- Držíš sa presne toho, čo si povedal v bode 4 — ručné zapisovanie, žiadne automatické naťahovanie dát,
  ani z vlastných existujúcich tabuliek appky.

Znamená to, že ak chceš mať biznis zisk za mesiac aj vo Financiách, zapíšeš si ho tam ako jeden riadok sám
(rovnako ako všetko ostatné). Vedel by som neskôr pridať tlačidlo, ktoré ti navrhne sumu rovno z
Dashboardu (aby si nemusel prepočítavať ručne) — zámerne som to teraz nepridal, aby prvá verzia zostala
jednoduchá, ale ozvi sa, ak by sa ti to zišlo.

### Kategórie — navrhol som 11 na začiatok

Podľa tvojho bodu 5. Výdavkové: Jedlo a nákupy, Bývanie, Doprava, Zábava, Zdravie, Predplatné, Biznis
náklady, Iné výdavky. Príjmové: Výplata, Biznis príjem, Iné príjmy. Presne ako Platformy/Kategórie
podujatí doteraz — vieš ich premenovať, zmazať, pridať vlastné, v **Settings → Lookups → Finance
categories**. Nič tu nie je napevno dané.

### Konverzia do EUR

Presne rovnaký princíp ako už poznáš z Dashboardu pri objednávkach: ak má nejaký záznam inú menu ako EUR,
hore sa objaví oranžový pásik s tlačidlom "Convert to EUR" (za konkrétnu menu, alebo za všetky naraz).
Stiahne sa aktuálny kurz a suma sa natrvalo prepočíta — presne ako pri objednávkach, nedá sa to vrátiť
späť, tak sa appka najprv opýta na potvrdenie.

## Čo som zámerne nechal na neskôr

- **Export do CSV** — každý iný zoznam v appke to má, Financie zatiaľ nie. Dá sa doplniť rovnakým
  spôsobom, len som to nechal mimo tejto verzie, aby zostala prehľadná.
- **Návrh sumy z biznis zisku** — pozri vyššie, "aj aj" sekcia.

Ak by si niektorú z týchto dvoch chcel, stačí povedať.

## Čo som overil

Napísal som 19 nových testov na kategórie a záznamy (validácie, farby kategórií, veľké písmená pri mene,
poradie zoznamu) — všetky prechádzajú, spolu so všetkými 765 pôvodnými (žiadny existujúci test sa
nepokazil). Reálne som si to aj poklikal v prehliadači (vlastný dočasný testovací setup, zmazaný po
overení) — pridanie, úprava, zmazanie záznamu, aj celá konverzia do EUR — všetko fungovalo bez jedinej
chyby v konzole, v svetlom aj tmavom režime.

```
cargo test --lib   -> 784 testov, 0 zlyhaní, 3 ignorované (765 pôvodných + 19 nových)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.0.83 build" v hlavičke)
```

## Zmenené súbory

**Backend (Rust):**
- `src-tauri/migrations/015_finance.sql` — nová migrácia, 2 nové tabuľky (`finance_categories`,
  `finance_entries`)
- `src-tauri/src/commands/finance_entries.rs` — nový súbor, všetka logika (kategórie aj záznamy)
- `src-tauri/src/models.rs`, `src-tauri/src/db.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/commands/mod.rs`
  — registrácia nových typov/príkazov/migrácie
- `src-tauri/src/commands/database.rs` — jeden existujúci test upravený (očakávaný počet migrácií 14 → 15)

**Frontend:**
- `src/pages/Finance.tsx` — nová stránka
- `src/components/FinanceCategoryBadge.tsx` — nový súbor (farebné štítky kategórií)
- `src/pages/Settings.tsx` — nová sekcia "Finance categories" v Lookups
- `src/components/Layout.tsx`, `src/App.tsx` — nová položka v sidebari a routa
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy a volania

**Verzia (9 miest v 7 súboroch):** `2.0.83`.

## STOP

2.0.83 hotové, otestované a zabalené. Skontroluj:

1. V sidebari klikni na novú položku **Finance**.
2. Klikni **"New entry"**, pridaj si skúšobný výdavok aj príjem (skús aj Osobné aj Biznis).
3. Skontroluj karty hore a oba grafy — mali by sa prepočítať podľa toho, čo si pridal.
4. Skús filtre (obdobie, rozsah, hľadanie, kategória) a uprav/zmaž jeden záznam.
5. V **Settings → Lookups → Finance categories** skús pridať/zmazať vlastnú kategóriu.
6. Ak máš záznam v inej mene ako EUR, skús tlačidlo **"Convert to EUR"** hore.

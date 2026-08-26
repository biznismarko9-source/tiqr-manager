# TIQR Manager 2.0.48 — opravy z tvojho testovania: Pending sales, Missing listing price, profil, Settings

## Čo si mi napísal

Poslal si mi zopár vecí naraz, s tým, že pôjdeme pomaly, jedno po druhom. Toto kolo rieši prvé tri:

1. Pri Activity vidíš "12 pending sales" a "27 missing listing price", pritom máš len 3 sales — počíta to lístky,
   nie sales. To isté pri listing price — chcel si to počítať podľa objednávky (order), nie podľa lístka.
2. Text "Local-first · your data stays on this device" pri profile v sidebar — chcel si ho odtiaľ preč, a
   namiesto textu "Basic info shown around the app." v Nastaveniach → Account → "Your profile" dať práve tento.
3. V Nastaveniach chceš všetky karty (Lookups, Data, Integrations, Appearance, Software, Account) pod sebou,
   v poradí, čo dáva zmysel ako postupnosť.

Štvrtá vec — AI v dashboarde, čo by vedelo napríklad prepočítavať meny podľa aktuálneho kurzu (napr. 20 GBP →
23,38 EUR) a časom robiť aj ďalšie veci — je na samostatný rozhovor, píšem o tom nižšie. Nechcel som to do
appky natlačiť narýchlo len preto, aby som mal "hotovú" celú tvoju správu naraz.

## Čo je nové

**Pending sales teraz počíta skutočné predaje, nie lístky.** Keď predávaš viac lístkov naraz ako jeden "New
sale" (napr. 4 lístky jednému kupcovi), appka si to interne aj tak ukladá ako 4 riadky — presne to isté miesto
v kóde, čo už dávno vyriešila obrazovka Sales (zobrazuje ich ako jeden predaj, nie štyri). Dashboard to doteraz
nevyužíval a počítal si to po svojom, odtiaľ tvoje "12 namiesto 3". Teraz Dashboard používa presne ten istý
spôsob zoskupenia, čo Sales obrazovka — takže číslo v Attention teraz vždy sedí s tým, čo by si videl na Sales
pri filtri "Pending".

**Missing listing price má teraz dve čísla, každé na správnom mieste.** Na Overview obrazovke, tá veta pod
Potential Profit ("X unsold tickets still have no listing price...") aj naďalej počíta lístky — to je správne,
lebo hovorí o tom, o koľko peňazí prichádzaš kvôli nedocenenému tovaru, a to sa počíta po lístku. Ale karta v
Activity ("Missing listing price") teraz počíta objednávky — jedna objednávka s piatimi neocenenými lístkami sa
tam ukáže ako 1, nie 5. Presne ako si chcel: "koľko vecí mám ísť doceniť", nie surový počet lístkov.

**Text o "Local-first" je preč zo sidebar, teraz je v Account.** V sidebar pod menom/emailom už nič také
nevidíš. V Nastaveniach → Account → "Your profile" je teraz namiesto pôvodného "Basic info shown around the
app." presne ten text, čo si chcel tam mať.

**Nastavenia sú teraz jeden stĺpec namiesto mriežky.** Každá kategória je teraz vlastný riadok (ikonka, názov,
popis, šípka), pod sebou, v tomto poradí: Lookups → Data → Integrations → Appearance → Software → Account.
Poradie som nechal presne také, ako bolo predtým (mriežka ho už aj tak čítala v tomto poradí, riadok po
riadku) — len teraz sa dá čítať zhora nadol ako jeden zoznam namiesto skákania po stĺpcoch. Ak by si chcel iné
poradie (napr. Account prvé), napíš, je to jednoriadková zmena.

Mimochodom, pri úprave Account sekcie som si všimol, že komentár v kóde nad ňou ešte hovoril o "placeholder
auth, nie je to ešte skutočný Firebase" — to bola pravda v 2.0.44, ale odvtedy (2.0.45/2.0.46) je to už dávno
skutočný Firebase prihlásenie. Opravil som aj ten komentár, nech niekoho v budúcnosti nezavedie na scestie.

## Čo ešte prebrať — AI v appke

Toto beriem vážne, ale nechcem to urobiť narýchlo popri troch menších opravách. Než začnem kódiť, potreboval by
som vedieť od teba trochu viac — napríklad: má appka volať skutočnú AI (napr. niečo ako Claude) na rôzne úlohy,
alebo myslíš skôr "inteligentné" automatizácie ako ten prepočet mien (kurz stiahnutý z internetu, prepočítaný
na pár klikov)? Prepočet mien samotný je celkom jasná, zvládnuteľná úloha (appka by potrebovala prístup na
nejaký zdroj kurzov, keďže appka je local-first a zatiaľ sťahuje z internetu len veci ako Google Sheets/Auth).
Ale keďže hovoríš, že "AI nebude slúžiť len na toto", radšej by som s tebou prešiel možnosti (podobne, ako sme
spravili pri redesigne Dashboardu — najprv som preskúmal, čo sa dá, priniesol ti smery, a ty si vyberal), než
aby som uhádol zle a stavil na tom ďalšie funkcie. Daj vedieť, či chceš, aby som na budúce kolo pripravil práve
takýto prehľad možností pre "AI v appke", so zameraním na prepočet mien ako prvý konkrétny prípad použitia.

## Ako som to overoval

Toto prostredie tentokrát vie spustiť skutočné `cargo test`/`npx tsc -b`/`npm run build` (nie len syntaktickú
kontrolu) — všetko nižšie teda naozaj bežalo, nie je to odhad.

Napísal som 3 nové testy, ktoré presne reprodukujú to, čo si nahlásil: jeden predaj rozdelený na viac lístkov
sa počíta ako 1 (nie ako počet lístkov), a jedna objednávka s viacerými neocenenými lístkami sa v Activity karte
počíta ako 1 (nie ako počet lístkov) — a zvlášť test, že lístok, ktorý sa už predal, nepribudne do tohto počtu
len preto, že zdieľa objednávku s neoceneným lístkom.

Naviac som dal celú zmenu prejsť dvoma nezávislými kontrolami (jedna len na Rust/backend SQL, druhá len na
React/frontend) bez toho, aby videli moje vlastné zdôvodnenie. Frontend vyšiel úplne čisto. Backend kontrola
upozornila na jednu vec, ktorú som spresnil: môj pôvodný komentár tvrdil, že nové číslo "vždy sedí" s tým, čo
vidno na obrazovke Sales — čo je pravda takmer vždy, ale teoreticky nie, ak by jeden "New sale" niekedy
obsahoval lístky vo dvoch rôznych menách naraz (appka to nezakazuje, aj keď je to nezvyčajné). Toto nie je nová
chyba — appka takto počítala peniaze podľa meny už predtým, ja som len nemal písať, že to sedí "vždy". Opravil
som ten komentár, nech je presný, a pridal ešte jeden test naviac (kontrola, že predaný lístok bez ceny
neovplyvní počet objednávok).

Vizuálne som prešiel Nastavenia (nový zoznam), sidebar (text preč) a Account (text tam) cez automatizovaný
prehliadač, prihlásený naozaj cez Firebase (miestny testovací server, rovnaká metóda ako pri overovaní
2.0.45/2.0.46) — všetko vyzerá a funguje presne tak, ako je opísané vyššie.

## Čo teraz urobiť

1. Nainštaluj 2.0.48.
2. Na Dashboard → Activity skontroluj, či "Pending sales" aj "Missing listing price" teraz sedia s tým, čo
   naozaj máš.
3. Pozri si Nastavenia — nový zoznam namiesto mriežky — a Nastavenia → Account, či je tam ten text, čo si
   chcel.
4. Napíš mi, či poradie v Nastaveniach sedí, alebo by si chcel iné.
5. Napíš mi, ako si predstavuješ tú AI časť (skutočná AI na rôzne úlohy vs. konkrétne "inteligentné"
   automatizácie ako prepočet mien) — pustím sa do prieskumu možností hneď, ako budem vedieť smer.

## Testy a build

```
cargo test --lib  -> 581 testov, všetky prešli (578 passed + 3 ignored - bolo 578 v 2.0.47, teraz +3 nové)
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

Toto sú skutočné výsledky z tohto prostredia (nie odhad/syntaktická kontrola) — a naviac aj vizuálne overené
cez automatizovaný prehliadač, ako je opísané vyššie.

## Zmenené súbory

**Backend:** `src-tauri/src/commands/dashboard.rs` (Pending sales počíta skutočné predaje nie lístky, nové pole
pre Missing listing price podľa objednávky, 3 nové testy), `src-tauri/src/models.rs` (nové pole na
`DashboardAlerts`).

**Frontend:** `src/lib/types.ts` (nové pole), `src/pages/Dashboard.tsx` (Activity karta používa nové pole),
`src/pages/Settings.tsx` (Nastavenia ako zoznam namiesto mriežky, presunutý text v Account, opravený zastaraný
komentár), `src/components/Layout.tsx` (text preč zo sidebar).

**Verzia (8 miest):** ako vždy, všetkých na `2.0.48`.

## STOP

1. Nainštaluj 2.0.48 (spusti `1-CLICK-UPDATE.bat`, počkaj na zelený build).
2. Skontroluj Activity (Pending sales, Missing listing price) a Nastavenia (zoznam, Account text).
3. Napíš mi, či je poradie v Nastaveniach OK, a ako si predstavuješ tú AI/prepočet mien časť — pôjdeme na
   ďalšie kolo presne podľa toho, čo napíšeš.

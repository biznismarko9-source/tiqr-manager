# TIQR Manager 2.0.51 — Convert to EUR aj pre už existujúce objednávky

## Čo si mi napísal

*"najskor dokonce tu zmenu meny, tam mi to urcite nestaci, ked importujem listky s google sheets nemam
moznost tu menu vobc zmenit, musi to byt pridane v orders a to priamo vedla currency, ked to bude ine ako
EUR tak bude tam moznost to conevrtovat na eura, neni to len na GBP, ale chcem to na vsetky meny ktore tam
su v liste, a taktiez v dashborade pise toto: You have data in more than one currency. To avoid adding
different currencies together, the totals below only include EUR. Filter by event/platform to see the
others. pri tom by mala byt nejaka moznost convert a bude na vyber tie, ktore su v inej menej alebo vsetky"*

Čiže presne to, čo si napísal, aj v tomto poradí:

1. Lístky importované z Google Sheets nemali VÔBEC možnosť zmeniť menu — treba opraviť.
2. Convert-to-EUR musí byť priamo v Orders, hneď vedľa Currency, na UŽ EXISTUJÚCEJ objednávke.
3. Nielen GBP — všetky meny, čo appka pozná.
4. Aj v Dashboarde, pri hláške o zmiešaných menách, možnosť prekonvertovať jednu konkrétnu menu alebo
   všetky naraz.

## Čo je nové

**Order Detail** — pri poli Currency je teraz tlačidlo **Convert to EUR**, ktoré sa objaví vždy, keď
objednávka nie je v EUR (jedno, či si ju vytvoril ručne, cez CSV import, alebo prišla zo synchronizácie s
Google Sheets — presne tvoj prípad z bodu 1). Klikneš, appka stiahne aktuálny kurz naživo, prepočíta celú
objednávku aj s lístkami a rovno prepne menu na EUR.

**Dashboard** — pri hláške o zmiešaných menách je teraz riadok **Convert to EUR** s tlačidlom pre každú menu,
čo sa tam nachádza (napr. "GBP (3)" — tri objednávky v GBP), a keď je iných mien viac než jedna, aj tlačidlo
**All** na prekonvertovanie úplne všetkého naraz. Presne to, čo si chcel: "na vyber tie, ktore su v inej
menej alebo vsetky".

## Prečo to nebolo také jednoduché, ako to znie

Toto je citlivejšia zmena než 2.0.50 (tá sa týkala len formulára pre NOVÚ objednávku, kde sa ešte nič
neuložilo). Tu už ide o reálne uložené čísla — lístky, ktoré možno majú za sebou aj predaj. Preto appka pri
každom prevode:

- Prepočíta objednávku, KAŽDÝ jej lístok aj KAŽDÝ predaj, čo sa toho lístka niekedy týkal (aj taký, čo bol
  neskôr refundovaný) — všetko naraz, v jednom kroku. Keby appka prepočítala len lístok a zabudla na jeho
  predaj, čísla na "Revenue"/"Profit" by potichu miešali dve meny dokopy — presne to som chcel za každú cenu
  vylúčiť.
- Ak by sa niekedy (napr. ručnou úpravou zálohy) stalo, že lístky k objednávke nesúhlasia s tým, čo je
  napísané pri objednávke samotnej, appka radšej prevod odmietne s jasnou hláškou, než aby hádala a
  potenciálne prepočítala niečo zle.
- Pri hromadnom prevode (Dashboard, viac objednávok naraz) appka posudzuje každú objednávku samostatne — ak
  je s jednou problém, ostatné sa prekonvertujú aj tak, nič sa kvôli jednej pokazenej neblokuje.

## Ako som to overoval

```
cargo test --lib  -> 606 testov, všetky prešli (589 pôvodných + 17 nových na tento prevod)
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

Predtým, než som to poslal, som dal kód ešte raz nezávisle prejrieť — jedno kolo na backend (Rust) časť,
druhé na frontend (obrazovku) časť. Chytili mi pri tom tri reálne veci, čo som opravil skôr, než si to vôbec
dostal:

1. Tlačidlo "prekonvertuj len túto jednu menu" na Dashboarde by v istom prípade nespravilo vôbec nič —
   konkrétne keby bola mena pri objednávke uložená malými písmenami (čo sa reálne môže stať pri CSV
   importe). Appka si to potichu porovnávala len presne písmeno po písmene, takže "usd" a "USD" jej vyšlo
   ako dve rôzne veci. Teraz to porovnáva bez ohľadu na veľké/malé písmená, takže to vždy nájde správne
   objednávky.
2. Cena za kus (unit price) sa pri objednávke počítala trochu inak, než ako sa počítajú poplatky a ostatné
   náklady — fungovalo to správne, ale len "šťastnou náhodou", nie preto, že by to appka naozaj overovala.
   Teraz to appka aj tu poriadne overuje a počíta rovnako dôsledne ako všetko ostatné, takže sa to už nemôže
   nikdy potichu rozísť.
3. Tlačidlo na hromadný prevod v Dashboarde sa vôbec nezobrazilo v prípade, že by ÚPLNE VŠETKY tvoje
   objednávky boli v jednej a tej istej cudzej mene (nie "zmiešané", ale napríklad celý import zo Sheetu v
   GBP) — presne tvoj prípad z bodu 1. Teraz sa zobrazí správne aj vtedy.

Skutočný internetový kurz som si v tomto prostredí nevedel naživo vyskúšať (rovnaké obmedzenie ako pri
Google prihlásení/Sheets/Firebase aj pri 2.0.50 — appka jednoducho nemá odtiaľto prístup na internet). Naostro
to teda uvidíme až po inštalácii u teba — ak by kurz z nejakého dôvodu nešlo stiahnuť (napr. výpadok
pripojenia), appka ti to jasne napíše namiesto toho, aby niečo hádala.

## Čo teraz urobiť

1. Nainštaluj 2.0.51.
2. Nájdi (alebo vytvor/importuj) objednávku, čo nie je v EUR. Otvor jej Order Detail, klikni **Convert to
   EUR** vedľa Currency a skontroluj, že sa suma aj mena zmenili a appka ti napísala kurz.
3. Ak máš objednávky vo viacerých menách naraz, skontroluj na Dashboarde tú hlášku o zmiešaných menách — mal
   by tam byť riadok s tlačidlami na prevod.

## Zmenené súbory

**Backend (Rust):** `src-tauri/src/commands/orders.rs` (hlavná logika prevodu), `src-tauri/src/commands/
dashboard.rs` (nový zoznam mien pre Dashboard tlačidlo), `src-tauri/src/models.rs`, `src-tauri/src/lib.rs`.

**Frontend:** `src/pages/OrderDetail.tsx` (nové tlačidlo), `src/pages/Dashboard.tsx` (nový riadok s
tlačidlami), `src/lib/api.ts`, `src/lib/types.ts`.

**Verzia (8 miest):** ako vždy, všetkých na `2.0.51`.

## STOP

1. Nainštaluj 2.0.51 (spusti `1-CLICK-UPDATE.bat`, počkaj na zelený build).
2. Vyskúšaj Convert to EUR podľa krokov vyššie — na oboch miestach (Order Detail aj Dashboard, ak máš na čom).
3. Ďalej idem na tú automatickú kategorizáciu eventov pri synchronizácii so Sheetom (tá, čo sme sa bavili,
   že to bude už naozaj AI, nie len hľadanie kľúčových slov) — presne ako sme sa dohodli, že na to prejdem
   hneď po tomto.

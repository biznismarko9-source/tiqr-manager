# TIQR Manager 2.0.56 — appka je teraz hnedá a béžová, nie modrá a biela

## Čo si mi napísal

Po niekoľkých kolách návrhov (poslal som ti to ako samostatný dokument s prepínačom Light/Dark) si sa rozhodol pre **hnedú + béžovú** - Light mód béžové pozadie s hnedým akcentom, Dark mód naopak (hlboká hnedá ako pozadie, béžová ako akcent) - a povedal "presne to, daj to do filu".

## Čo je nové

Celá appka - Dashboard, Events, Orders, Sales, Tickets, Pulls, Settings, úplne všetko - teraz namiesto pôvodnej modrej/bielej/tmavomodrej používa tvoju schválenú hnedú a béžovú. Žiadna stránka nezostala vynechaná.

## Ako presne to funguje pod kapotou

Appka už predtým bola postavená veľmi disciplinovane - farby sa naprieč celou appkou nikde nepíšu natvrdo (napr. "modrá"), ale vždy cez dva pomenované "tokeny": `brand` (akcentová farba - tlačidlá, odkazy, zvýraznenia) a `slate` (neutrálna farba - pozadia, okraje, väčšina textu). Vďaka tomu stačilo prepísať, AKÚ FARBU tieto dva tokeny znamenajú, na jednom mieste (`tailwind.config.js`) - a keďže každá stránka v appke tieto tokeny už používala, celá appka sa prefarbila naraz, bez toho, aby som musel čo i len otvoriť čo i len jeden súbor stránky.

- `brand` (bola modrá, `#4a68f7`) je teraz bohatá hnedá (`#7d5726` v Light móde).
- `slate` (bola sivo-modrá, ako má takmer každý dashboard) je teraz kompletná škála od svetlej béžovej (`#f2e9d8` - presne farba pozadia z tvojho schváleného návrhu) až po hlbokú tmavohnedú (`#241a10` - tiež presne z návrhu).

Obe sú **celé 11-stupňové škály** (nie len tie 2 farby, čo si videl v návrhu) - Tailwind (nástroj, čo appka na štýlovanie používa) potrebuje celý rad odtieňov od najsvetlejšieho po najtmavší, nie len jeden konkrétny odtieň, aby fungovalo všetko (jemné orámovania, tlačidlá, textové zvýraznenia...) konzistentne.

**Čo som zámerne nechal tak, ako bolo:**
- Biele karty (tabuľky, boxy) v Light móde zostávajú biele, nie béžové - je to zámerný kontrast "biela karta na béžovom pozadí", rovnaký princíp, aký appka mala predtým ("biela karta na svetlosivom pozadí"). Ak by si chcel, aby boli karty tiež jemne béžové, daj vedieť - je to samostatná, menšia zmena.
- Farby, čo majú v appke svoj vlastný význam (zelená = zisk, červená = strata/refund, oranžová = upozornenie), sa nedotkli - tvoja požiadavka bola o všeobecnom vzhľade appky, nie o tom, čo tieto farby znamenajú.
- Logo appky (fialovo-ružové) zostalo bez zmeny - je to samostatný obrázok, nie súčasť tejto farebnej schémy. Všimol som si, že sa teraz farebne nestretáva s novou hnedo-béžovou schémou tak, ako sa predtým nestretávalo s modrou - je to tak, ako je, kým nepovieš inak.
- Obrazovka počas sťahovania aktualizácie (tá s logom, čo sa ukáže pri inštalovaní novej verzie) používa presne ten istý `brand` token na svoje pozadie - takže sa tiež automaticky prefarbila na hnedú, bez toho, aby som sa jej musel dotknúť.

## Ako som to overoval

Toto kolo sa netýka žiadneho Rust súboru ani inej appkinej stránky priamo - len jeden konfiguračný súbor (`tailwind.config.js`), preto:

- Súbor som overil, že je stále platný (načíta sa bez chyby).
- Keďže neviem appku v tomto prostredí naozaj vizuálne vykresliť, prepočítal som **kontrast (čitateľnosť textu)** podľa oficiálneho webového štandardu (WCAG) pre všetky dôležité kombinácie - hlavný text na pozadí, tlačidlá, sekundárny text - všetky prešli s veľkou rezervou. Jemné orámovania (napr. okraj karty) majú zámerne nízky kontrast - presne tak, ako to appka mala predtým aj s modrou (aj tam boli okraje "sotva viditeľné" naschvál) - prepočítal som aj toto priamo oproti pôvodnej modrej verzii, aby som mal istotu, že v tomto smere nič nezhoršujem, len prefarbujem.
- Prehľadal som celý zdrojový kód appky pre akúkoľvek natvrdo napísanú starú modrú farbu (mimo tých dvoch tokenov) - nič sa nenašlo, čo potvrdzuje, že appka je naozaj dôsledne postavená len na tých dvoch tokenoch.

Skutočné vizuálne overenie (ako to naozaj vyzerá na obrazovke) je opäť na tebe po nainštalovaní - to je jediná časť, ktorú v tomto prostredí naozaj nemôžem vidieť.

## Čo teraz urobiť

1. Nainštaluj 2.0.56.
2. Prejdi si appku naprieč všetkými stránkami (Dashboard, Events, Orders, Sales, Tickets, Pulls, Settings) v oboch režimoch (Light aj Dark) a skontroluj, že farby vyzerajú presne tak, ako si čakal z návrhu.
3. Ak niekde niečo nesedí (príliš svetlé/tmavé, zle čitateľné, alebo chceš aj biele karty prefarbiť na béžovo), napíš mi presne kde a ako - je to jednoduchá doladiteľná zmena.

## Zmenené súbory

**Frontend (1 súbor):**
- `tailwind.config.js` - `brand` (akcent) a `slate` (neutrálna farba) prepísané z modrej/sivomodrej na hnedú/béžovú, celé 11-stupňové škály

Žiadny iný súbor appky sa nezmenil - ani jedna stránka, ani Rust kód, ani žiadna migrácia.

**Verzia (8 miest):** ako vždy, všetkých na `2.0.56`.

## STOP

2.0.56 hotové. Skontroluj podľa krokov vyššie naprieč čo najviac stránkami v oboch režimoch - je to najväčšia vizuálna zmena appky doteraz, tak ju prosím poriadne prejdi.

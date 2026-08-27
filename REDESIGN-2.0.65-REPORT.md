# TIQR Manager 2.0.65 — Integrations menej zmätočné, jednotné Sort, viac filtrov

## Čo si napísal

*"rovno sa vrhni do Integrations, je tam vela tlacitok a takto ako to je teraz je dost zmatujuce pre inych
ludi, skus to urobit tak, aby tam bolo minimalne mnozstvo tych tlacitok, kludne mozu ostat vsetky, len to
urobit tak, aby si vedel naco sluzia, aby si sa v tom vedel orientovat ako clovek a nemusel to hladat, aby
tam nebolo zbytocne vela textu, tak aby to bolo prehladne, ciste, minimalisticke, nieco vymysli, dalsie
nielen pri orders ale vsade kde je sort on newest a oldest, tak pri newerst by sa mali zobrazit od hora
dole, cize naopak ako je teraz, urob to vsade kdeje ten sort tlacitko, taktiez viac moznosti filtrovania pri
vsetkych moznych miestach, events orders,... pulls, vsade, nech je to tak aby to davalo zmysel, a aby vsade
boli cca rovnake, pri tickets odstran tlacitko na vyhladanie section, zatial toto, potom budeme
poracovat"*

Toto je **prvá dávka** — píšeš, že budeme pokračovať ďalej, takže toto berem presne ako prvý krok, nie ako
celé zadanie.

Predtým, než som čokoľvek menil, spýtal som sa ťa na dve konkrétne veci (aby som to nerobil naslepo):

- Pri "Newest first" na Orders sú hore roky 2028, dole 2026 — čo presne má "Newest first" odteraz znamenať?
  Odpovedal si: **pri budúcich dátumoch = najbližšie hore**.
- Ktoré chýbajúce filtre mám doplniť? Odpovedal si: **Platforma** (Orders, Pulls), **Kategória** (Tickets),
  **Dátum od-do** (Orders, Events, Pulls).

Nižšie je presne to, čo som na základe toho spravil.

## 1. Settings → Integrations (Pulls aj Orders & Sales)

Nechal som **úplne všetky** tlačidlá a funkcie — nič som nezmazal, presne ako si povedal ("kludne mozu
ostat vsetky"). Zmenil som, ako vyzerajú, keď ich práve nepotrebuješ.

**Predtým:** aj po pripojení hárku zostávalo natrvalo zobrazené: vysvetľujúci text, obe polia (URL + názov
tabu), a v riadku Connect/Save, Test connection, Update sheet, Disconnect — plus dole ešte text "Zdieľaj
hárok s..." Toto všetko zostávalo na obrazovke navždy, aj keď je pripojenie roky funkčné a nikdy sa
nedotýkaš.

**Teraz:** keď je hárok pripojený, toto všetko sa zbalí do jedného riadku:

> Tab "Pulls" · [Change connection] [Test connection]

Klikneš na **"Change connection"** a presne to isté (polia, text, Save, Update sheet, Disconnect) sa znova
rozbalí — nič sa nestratilo, len to nie je natrvalo na očiach, keď to nepotrebuješ.

**Sync a Push riadky** (Order sync/Sales sync, Push orders/Push sales) teraz majú nad sebou krátky popis so
šípkou, aby bolo jasné, ktorým smerom čo ide, bez nutnosti na nič najazdiť myšou:

- ⬇ **Import from sheet** (čítanie Z hárku DO appky)
- ⬆ **Send to sheet** (písanie z appky DO hárku)

**"Fix sync"** (jediná akcia, čo môže prepísať bunku, čo už niečo obsahuje) je teraz vizuálne odlíšená —
oranžová farba a ikona výstrahy namiesto toho, aby vyzerala rovnako ako susedné tlačidlá.

## 2. Sort — "Soonest first" / "Furthest first" všade

Premenoval a zjednotil som **všetkých 6** miest, čo majú "Newest/Oldest first": Orders, Tickets, Sales,
Events, Pulls (Given), Pulls (Received). Nové názvy: **"Soonest first"** / **"Furthest first"** — a
"Soonest first" je teraz všade predvolené (predtým bolo predvolené opačne).

Pri Pulls (obe záložky) som popri tom opravil aj skutočnú chybu: appka doteraz triedila podľa toho, KEDY si
záznam napísal do appky (`created_at`), ale v tabuľke sa zobrazoval dátum EVENTU — dve úplne nesúvisiace
veci. Takže "Newest first" tam doteraz ukazovalo poradie, čo nezodpovedalo tomu, čo bolo na obrazovke.
Opravené — teraz triedi presne podľa toho istého dátumu, čo aj vidíš v stĺpci "Date", rovnako ako všade
inde.

## 3. Nové filtre

- **Platforma** — pridaná na **Orders** a na **Pulls (Given)**.
- **Kategória** — pridaná na **Tickets**.
- **Dátum od-do** — pridaný na **Orders**, **Events**, **Pulls (Given aj Received)**.

Pri Orders/Tickets to bola len frontendová práca (appka to už vedela, len sa to nikdy neposielalo). Pri
Pulls a Events som musel doplniť aj backend (nové parametre v `pulls.rs`/`pulls_received.rs`/`events.rs`).

Pulls (Received) nedostal Platform filter — tá tabuľka nemá stĺpec Platform vôbec (platforma je vlastnosť
prípadnej naviazanej objednávky, nie samotného "received pull" záznamu), takže by to nemalo čo filtrovať.

## 4. Tickets — odstránený filter Section

Presne ako si žiadal — vyhľadávanie/filter podľa Section je na Tickets preč.

## 5. Pulls — zjednotený vzhľad filtrov

Riadok s filtrami na Pulls teraz vyzerá rovnako ako na Orders/Sales/Events/Tickets (popisky nad každým
poľom, rovnaké rozostupy) — predtým to bol jediný riadok filtrov v appke, čo vyzeral inak.

## Čo som overil

```
cargo test --lib   -> 662 testov (652 + 10 nových), 0 zlyhaní, 3 ignorované
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

10 nových testov pokrýva: Platform aj dátumový filter na Pulls (Given), dátumový filter na Pulls
(Received), dátumový filter na Events (vrátane toho, že event bez dátumu sa do rozsahu nikdy nezmestí), a
nové "soonest"/"furthest" hodnoty na Sales (pridané popri starých hodnotách, nie namiesto nich — takže nič
staré sa nemohlo pokaziť).

## Zmenené súbory

**Backend:**
- `src-tauri/src/commands/pulls.rs` — Platform + dátumový filter (`list_pulls_impl`)
- `src-tauri/src/commands/pulls_received.rs` — dátumový filter (`list_pulls_received_impl`)
- `src-tauri/src/commands/events.rs` — dátumový filter (`list_events`/`list_events_impl`)
- `src-tauri/src/commands/sales.rs` — nové "soonest"/"furthest" hodnoty (pridané, staré nezmenené)

**Frontend:**
- `src/pages/Settings.tsx` — redizajn Integrations kariet (zbaľovanie, popisky, ikony, zvýraznenie Fix sync)
- `src/pages/Orders.tsx`, `src/pages/Tickets.tsx`, `src/pages/Sales.tsx`, `src/pages/Events.tsx`,
  `src/pages/Pulls.tsx` — Sort premenovanie/oprava, nové filtre, (Pulls) zarovnanie vzhľadu, (Tickets)
  odstránenie Section
- `src/lib/api.ts` — nové parametre pre `listPulls`/`listPullsReceived`/`listEvents`

**Verzia (8 miest):** `2.0.65`.

## STOP

Toto je prvá dávka z tvojej požiadavky — píšeš, že budeme pokračovať ďalej, takže čakám na ďalšie kolo. Skús
si prosím hlavne Settings → Integrations (obe karty) a Sort na pár miestach, či ti to takto dáva zmysel — a
ak by si chcel radšej pôvodné pomenovanie "Newest/Oldest" (len s opačným smerom), pokojne to zmením, toto
pomenovanie som zvolil ja sám, nie je to niečo, čo si explicitne žiadal.

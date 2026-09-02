# TIQR Manager 2.2.9 — 6 opráv a doplnkov po kontrole 2.2.8

Pozrel si si čerstvo dodané 2.2.8 (Attention Center) aj pár ďalších miest v appke cez
screenshoty a poslal si mi rýchlo za sebou 6 vecí na úpravu, bez formálneho zoznamu. Nebudem
si tu vymýšľať citáciu, ktorú si takto nenapísal — radšej to zhrniem vlastnými slovami, presne
v tom poradí, v akom si to napísal:

1. Odstrániť **Seatriks** z **Price Checkera** úplne — nech sa tam pre tento marketplace
   prestane sledovať/kontrolovať cena. Ale nech zostane presne tam, kde sa už dnes používa ako
   predajný marketplace (napr. výber pri "Add listing").
2. V Settings → Integrations premenovať sekciu "AI-assisted price reading" (Anthropic/Claude
   API kľúč) na všeobecnejší názov — ten istý uložený kľúč má časom slúžiť aj ďalším AI
   funkciám, nielen fallbacku pri čítaní ceny v Price Checkeri.
3. Pri tej istej sekcii ukázať nejaký malý indikátor "balance" — koľko Anthropic API
   kreditu/zostatku ešte ostáva.
4. Na Finance → Overview pridať tlačidlá "New entry" a "New account".
5. Zmazať existujúci per-event "Attention" zoznam v Event Workspace (Inventory Intelligence,
   2.2.6). A zároveň prerobiť práve dodaný Dashboard "Attention Center" (2.2.8) tak, aby
   zoskupoval podľa **Objednávky**, nie po jednotlivých tiketoch — na tvojom screenshote bolo
   49 riadkov pre jednu objednávku, všetky s rovnakým dôvodom "No listing price set", čo si
   správne označil, že "nedáva zmysel".
6. Prerobiť spoločné zobrazenie "Seats" (doteraz spojené lomítkom, napr. "402/56 27") na jasne
   oddelené polia Section / Row / Seat, bez "/" — všade kde sa zobrazuje: Orders, Tickets,
   Sales, Inventory, Pulls.

Toto je presne to, čo som spravil. Nikde som sa ťa nepýtal na spresnenie — pri každej
nejasnosti (a bolo ich pár) som spravil rozhodnutie sám a píšem ho tu nahlas, aby si ho vedel
opraviť, ak si to myslel inak.

## 1. Seatriks — von z Price Checkera, nie odnikiaľ

Použil som presne ten istý mechanizmus, čo appka už raz použila na odchod StubHubu
(`017_price_checker_viagogo.sql`): nová migrácia `025_deactivate_seatriks_price_checker.sql`
nastaví `marketplaces.active = 0` pre Seatriks. Vďaka tomu:

- Price Checker ho novo neponúkne žiadnemu eventu, ktorý s ním ešte nemá nič spoločné.
- Listings' "Add listing" výber (`list_marketplaces`) ho vidí úplne rovnako ako predtým —
  tento zoznam sa podľa `active` vôbec nefiltruje, takže som sa ho ani nemusel dotýkať.

**Jedna vec na upozornenie**: presne ako pri StubHube, event, ktorý už DNES má uloženú
Seatriks históriu (uložený link alebo Price Check), bude Seatriks aj naďalej vidieť vo svojom
vlastnom Price Checkeri — zmizne len ako NOVÁ ponuka pre eventy, čo s ním zatiaľ nemajú nič.
Ak chceš, aby zmizol úplne aj tam, je to malá samostatná úprava (jedna podmienka v
`get_price_checker_summary_impl`), len som nechcel spraviť silnejší zásah, než si žiadal.

## 2. a 3. Premenovanie AI kľúča + "balance"

Kartu v Settings → Integrations som premenoval z "AI-assisted price reading" na **"AI
features"** a popis prepísal tak, aby znel všeobecne (Price Checker ostal len ako konkrétny
príklad). Samotné uloženie kľúča a príkazy (`app_secrets`, `get_anthropic_api_key_configured`,
`set_anthropic_api_key`) som sa vôbec nedotkol — kľúč sa aj naďalej nikdy neposiela späť na
frontend, len príznak "je nastavený/nie je".

Pri "balance" som najprv overil v oficiálnej Anthropic dokumentácii, či sa to dá vôbec
spoľahlivo spraviť — a nedá sa, pre žiadny typ kľúča: Anthropic API nemá endpoint, ktorý by
vrátil aktuálny/zostávajúci kredit. Ich Usage & Cost API vracia len HISTORICKÉ dáta o
minulom použití, a aj to funguje iba s Admin API kľúčom alebo neobmedzeným kľúčom účtu — nie s
bežným kľúčom pracovného priestoru, aký appka dnes žiada. Pridať appke druhé, Admin-scope pole
len kvôli tomuto by bola oveľa väčšia a citlivejšia zmena, než si asi myslel pri slove
"balance", tak som to nespravil bez toho, aby si to najprv videl.

Namiesto vymysleného čísla som pridal na tú istú kartu tlačidlo/odkaz **"Check usage & balance
on the Anthropic Console ↗"**, ktorý otvorí `console.anthropic.com/settings/billing` v
prehliadači (cez `openUrl`, appka to už používa na Google prihlásenie — žiadna nová
závislosť). Je to menej, než si pýtal, ale je to pravdivé — radšej toto, než fingovaný
zostatok.

## 4. Finance → Overview: rýchle tlačidlá

"New entry" a "New account" na Overview otvárajú presne tie isté formuláre, čo dnes používajú
taby Transactions a Accounts (`EntryFormModal`, `AccountFormModal`) — nie kópie. Obe som len
zviditeľnil (`export`) z ich pôvodných súborov a napojil aj na Overview. Keď sa niekedy zmení
formulár pre nový záznam alebo účet, stále je to len jedno miesto na úpravu, nie dve.

## 5. Attention: zmazanie starého zoznamu + prerobenie nového na objednávky

Toto je najväčšia zmena z tejto dávky a má dve časti.

**a) Zmazaný per-event "Attention" zoznam.** V Event Workspace → Overview (Inventory
Intelligence, 2.2.6) som odstránil presne tie riadky s nálepkou "ATTENTION", ktoré boli aj na
tvojom screenshote — KPI, Aging, rozpad podľa tier/section/marketplace v tej istej karte
zostávajú úplne bez zmeny. Ak si myslel zmazať celú kartu Inventory Intelligence, povedz a
opravím to — vybral som si užšiu možnosť, lebo presne tú si aj ukázal. Backend
(`get_inventory_intelligence`), čo za týmto zoznamom stál, som nechal úplne netknutý, lebo ho
stále priamo (ako obyčajnú Rust funkciu, nie cez frontend) používa aj Dashboard Attention
Center — zmazať alebo zmeniť ho by ticho pokazilo aj to.

**b) Dashboard Attention Center teraz zoskupuje podľa objednávky.** Presne ako si napísal —
50 tiketov v jednej objednávke bez ceny predtým znamenalo 50 riadkov. Teraz appka 4
kategórie na úrovni tiketu (chýbajúca cena, chýbajúci aktívny listing, cena mimo trhu, predané
a nedoručené) zoskupí podľa objednávky a ukáže **jeden riadok na objednávku** — s počtom
tiketov v texte. Klik na taký riadok otvorí existujúcu stránku danej objednávky
(`/orders/:id`), ktorá už aj tak zobrazuje každý jeden dotknutý tiket so svojím vlastným
stavom/cenou/doručením — nestaval som žiadne nové rozbaľovanie na mieste, len som znovu použil
hotovú stránku. Kategória "event do 48h" ostáva bez zmeny (jeden riadok na event) — nemá jednu
konkrétnu objednávku, pod ktorú by patrila, keďže nepredané tikety jedného eventu môžu ležať vo
viacerých objednávkach naraz. Keď je pod jedným riadkom viac než jeden tiket, appka už neukazuje
sumu v EUR/USD/... pri "cena mimo trhu" — nie je tam jedna spoločná cena, ktorú by mohla ukázať,
a radšej nič než vymyslený priemer.

## 6. Seats: Section / Row / Seat namiesto lomítka

`formatSeatsSummary` (spoločný formátovač pre stĺpec "Seats" na Orders/Tickets/Sales/
Inventory/Pulls) teraz pre každú skupinu miest použije rovnaký "Sec X · Row Y · Seat Z" tvar,
čo appka už predtým používala na detailoch jedného tiketu — namiesto starého "204/56 27" je to
teraz napríklad **"Sec 204 · Row 56 · Seat 27"**. Samotné zoskupovanie (podľa
section+row) a skracovanie susediacich čísel sedadiel (napr. "128-131") som nemenil vôbec —
zmenil sa len text okolo.

Popri tom som našiel a zjednotil aj 8 ďalších miest v appke, ktoré si tento istý "/" spojenie
robili ručne, mimo spoločného formátovača — 6 v `EventDetail.tsx` (tab Tickets, tab Listings,
4× v okne "Create Sale") a 2 v `Sales.tsx` (tiež okno "Create Sale"). Všetky teraz volajú tú
istú zdieľanú funkciu, takže "/" už nikde v appke pri sedadlách nezostalo. Na dvoch miestach sa
pri úplne prázdnom mieste text zmenil z "No seat info"/samotnej pomlčky na "General
admission" — je to zámerné zjednotenie s tým, čo už dnes robia detaily objednávky a predaja pri
tom istom prípade, nie chyba.

Ešte jedna drobnosť navyše, nie tvoja požiadavka: pri kontrole tohto súboru som narazil na to,
že jeden riadok v `format.ts` (kľúč na zoskupovanie miest podľa section+row) mal namiesto
bežného textového oddeľovača doslova vpísaný netlačiteľný nulový znak — fungovalo to, ale
kazilo to bežné textové nástroje (napr. vyhľadávanie v súbore ho videlo ako binárny súbor).
Opravil som to na štandardný zápis (`\0` ako text v kóde namiesto skutočného nulového bajtu) —
správanie appky sa tým vôbec nezmenilo, len je kód odteraz normálny text.

## Zmenené súbory

**Backend:**
- `src-tauri/migrations/025_deactivate_seatriks_price_checker.sql` — nová migrácia (Seatriks
  `active = 0`)
- `src-tauri/src/db.rs` — registrácia migrácie 025
- `src-tauri/src/commands/price_checker.rs` — upravený test (Seatriks už nie je v zozname
  aktívnych marketplacov pre nový event)
- `src-tauri/src/commands/database.rs` — opravený test počtu migrácií (24 → 25)
- `src-tauri/src/models.rs` — `AttentionCenterItem` prerobený: namiesto `ticketId`/`ticketCode`
  má `orderId`/`orderCode` + `ticketIds`/`ticketCodes` (zoznam)
- `src-tauri/src/commands/attention_center.rs` — nové zoskupovanie podľa objednávky
  (`group_by_order`), rozšírené testy

**Frontend:**
- `src/lib/types.ts` — `AttentionCenterItem` podľa novej štruktúry
- `src/lib/api.ts` — upravený komentár (AI kľúč slúži viacerým funkciám)
- `src/lib/format.ts` — `formatSeatsSummary` nový výstupný tvar (Section/Row/Seat), + oprava
  nulového bajtu popísaná vyššie
- `src/pages/Settings.tsx` — karta AI kľúča premenovaná, pridaný odkaz na Anthropic Console
- `src/pages/finance/Transactions.tsx`, `src/pages/finance/Accounts.tsx` — `EntryFormModal`/
  `AccountFormModal` zviditeľnené (`export`)
- `src/pages/finance/Overview.tsx` — tlačidlá "New entry"/"New account"
- `src/pages/EventDetail.tsx` — zmazaný per-event Attention blok; 6 miest prepojených na
  spoločný `formatSeatLocation`
- `src/pages/Dashboard.tsx` — `AttentionCenterRow` teraz vedie na `/orders/:id` a ukazuje kód
  objednávky + počet tiketov
- `src/pages/Sales.tsx` — 2 miesta prepojené na spoločný `formatSeatLocation`

**Dokumentácia:**
- `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/PROTECTED_AREAS.md`, `CHANGELOG.md`

## Čo som overil

```
cargo test --lib   -> 999 passed, 0 failed, 3 ignored (995 pôvodných + 4 nové: tikety v jednej
                       objednávke sa zoskupia do jedného riadku, tikety v rôznych objednávkach
                       ostanú oddelené aj v rámci tej istej kategórie/eventu, "cena mimo trhu"
                       neukáže sumu pre skupinu s viac než jedným tiketom, "predané-nedoručené"
                       sa tiež zoskupuje podľa objednávky; plus opravený test počtu marketplacov
                       pre nový event a opravený test počtu migrácií)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.2.9 build" v hlavičke)
```

Celý pôvodný test balík prešiel bez jedinej zmeny správania — žiadna regresia v
Orders/Tickets/Sales/Listings/Finance/Price Checker/Inventory Intelligence logike. Naviac som
si ručne prešiel `grep` po celom `src/` a `src-tauri/`, aby som si overil, že nezostalo nikde
staré `.join(" / ")` pre miesta ani stará `ticketId`/`ticketCode` štruktúra Attention Center
položky.

---

Toľko k tých 6 bodom. Tri veci na tvoje rozhodnutie, keby si to chcel inak, než som odhadol:
Seatriks historicky prepojené eventy ho môžu ešte vidieť (bod 1), zmazanie Attention zoznamu je
len tie ATTENTION riadky, nie celá karta (bod 5a), a namiesto balance čísla je tam zatiaľ len
odkaz na Anthropic Console (body 2-3). Všetko ostatné je presne tak, ako si napísal.

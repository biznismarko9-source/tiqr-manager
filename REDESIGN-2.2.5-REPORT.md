# TIQR Manager 2.2.5 — Sales pohltil Finance, Add listing prerobený, Listings s filtrami a bulk akciami

Reagujem na tvoju poslednú správu:

> *"Overview Listings Sales Finance, sales a finance daj dokopy, dalej pri add listing treba zmenit to ako sa
> vyberaju, teraz to je uplne nepriehladne, urobme to ako pri sale, ze mas orders, ktore si vies prekliknut a
> vybrat dany pocet listkov, taktiez pridaj platformu seatriks"*

a potom:

> *"Chcem teraz vylepšiť Listings tak, aby sa dali reálne pohodlne riadiť. [...] Pridaj: filtre: All / Active /
> Sold / Removed, filter podľa Marketplace, search, možnosť označiť viac listingov naraz, bulk edit status,
> bulk edit price, bulk delete, select all / deselect all. V tabuľke nech zostane: Ticket | Marketplace |
> Price | Status | URL | Last updated. Bulk actions zobraz iba keď je niečo vybrané. Pri zmene ceny zachovaj
> currency a INTEGER cents. Pri bulk operáciách musí byť všetko bezpečné a ideálne transakčné — buď sa zmena
> podarí pre všetky vybrané listingy, alebo pre žiadny. Nič nemen: Tickets/Inventory status logiku, Sales,
> Orders, Finance, Price Checker, refund/resell, existujúce listing dáta."*

Postupne, po poriadku.

## 1. Tri záložky namiesto štyroch — Sales pohltil Finance

Zostali: **Overview | Listings | Sales.**

- Presunul som obsah Finance (tabuľku záznamov s dátumom/objednávkou/kategóriou/sumou/poznámkou aj odkaz
  "Open in Finance") úplne dole do Sales, pod časti, čo tam už sú z 2.2.4 (predaje, "Market vs. mine",
  "Potential Profit").
- Dáta ani logika Finance sa nemenili — len sa presunulo, kde sa to zobrazuje. Samotná stránka Finance (keď
  na ňu prejdeš cez "Open in Finance") funguje presne tak ako doteraz.

**Jedna vec, čo som si dovolil rozhodnúť sám:** minule (2.2.4) si napísal "Sales Market Finance spoj do
jedneho", ale hneď potom si vymenoval finálny zoznam so 4 záložkami, kde Sales aj Finance ostali oddelené —
vtedy som spojil len Market do Sales. Tentoraz si napísal jasne "sales a finance daj dokopy" bez žiadneho
zoznamu, čo by to poprelo, takže som to pochopil jednoznačne — Finance zmizlo, jeho obsah je v Sales (Sales
je v tvojej vete spomenuté prvé, presne ako pri každom doterajšom spájaní v tomto projekte). Ak si to myslel
opačne, daj vedieť — je to jeden ohraničený blok, dá sa rýchlo presunúť.

## 2. Add listing — výber tiketov prerobený presne ako pri Sale

Predtým bol v "Add listing" jeden plochý dropdown so všetkými tiketmi na evente — nepriehľadné, presne ako si
napísal. Teraz to funguje ako New Sale:

1. Najprv vidíš zoznam objednávok tohto eventu (dá sa v ňom hľadať),
2. otvoríš jednu objednávku a vidíš jej tikety,
3. označíš ľubovoľný počet tiketov (aj z viacerých objednávok postupne) a
4. zadáš marketplace + cenu (pri viacerých tiketoch naraz je tam aj rýchle tlačidlo "použiť túto cenu na
   všetky") a vytvoríš listing pre každý vybraný tiket naraz.

Listing ID/URL appka pýta len vtedy, keď vyberieš presne jeden tiket — pri viacerých naraz by rovnaké
ID/URL na všetkých nedávalo zmysel (každý marketplace listing má vlastné ID); pri hromadnom pridaní ich
potom dorobíš cez Edit na každom vytvorenom listingu zvlášť.

**Dôležitý rozdiel oproti bulk akciám nižšie, ktorý chcem zdôrazniť:** toto vytváranie NIE JE all-or-nothing.
Ak sa niektorý z vybraných tiketov nepodarí zalistovať (napr. duplicita), appka uloží všetko, čo sa podarilo,
a nechá vybrané len tie, čo zlyhali, aby si to mohol skúsiť znova — tvoju požiadavku na "ideálne transakčné"
som pochopil ako viažucu sa na úpravu EXISTUJÚCICH listingov (bod 4 nižšie), nie na toto pridávanie nových.
Ak si chcel all-or-nothing aj tu (čiže: buď sa zalistujú úplne všetky vybrané tikety, alebo žiadny), daj
vedieť, viem to prerobiť.

## 3. Nový marketplace: Seatriks

Pridaný do zoznamu marketplaces (rovnaké miesto, kde je StubHub/Vivid/Ticombo/Viagogo pre Price Checker aj
pre Listings) — čistá dátová zmena, žiadna zmena štruktúry. Názov/veľké písmená som napísal presne tak, ako
si to zadal ("Seatriks") — ak to má byť inak, je to jednoriadková zmena, netreba kvôli tomu žiadnu migráciu.

## 4. Listings — filtre, search, hromadné (bulk) akcie

Tabuľka ostala presne taká, ako si chcel: **Ticket | Marketplace | Price | Status | URL | Last updated** —
nič sa nepridalo ani neubralo, len pribudol stĺpec so zaškrtávacím políčkom vľavo.

Pridané presne podľa tvojho zoznamu:

- **Filtre:** All / Active / Sold / Removed (prepínače hore, rovnaký štýl ako inde v appke)
- **Filter podľa Marketplace** (dropdown)
- **Search** (hľadá naraz v tiket/marketplace/URL)
- **Výber viacerých listingov naraz** — zaškrtávacie políčko pri každom riadku, vždy viditeľné (nie je to
  skrytý "výberový režim", čo treba najprv zapnúť)
- **Select all / Deselect all** — vzťahuje sa len na to, čo práve vidíš (po filtroch/search), presne ako to
  funguje pri predajoch
- **Bulk edit status, bulk edit price, bulk delete** — lišta s týmito tlačidlami sa objaví len vtedy, keď je
  aspoň jeden listing vybraný (presne ako si žiadal), inak nie je vidno vôbec nič navyše

**Bezpečnosť bulk operácií — presne podľa tvojej požiadavky "ideálne transakčné":** všetky tri (edit
status/price/delete) sú v databáze zabalené tak, že appka najprv overí úplne všetky vybrané listingy a až
potom, v jednom kroku, zapíše zmenu na všetky naraz. Ak by čo i len jeden z vybraných listingov medzičasom
zmizol alebo bol neplatný, NEZMENÍ SA ani jeden — presne to "buď všetky, alebo žiadny", čo si žiadal. Je to
prísnejšie než napríklad hromadné mazanie pri predajoch (to naopak vedome preskočí zlyhané a nahlási, čo sa
nepodarilo) — tu som to spravil prísnejšie, lebo si to takto výslovne chcel.

**Cena a mena:** pri bulk edit price appka mení len sumu (v celých centoch, presne ako doteraz — žiadne
desatinné čísla), menu nikdy nemení. Ak by si vybral listingy vo viacerých menách naraz, appka rovno vypne
tlačidlo "Edit price" (s vysvetlením prečo) — nedá sa omylom nastaviť jednu sumu naprieč rôznymi menami.

**Jedna menšia vec, čo som spravil podľa vlastného úsudku:** zaškrtávacie políčka sú vždy viditeľné, nie
schované za samostatný prepínač "vyber viacero" (na rozdiel od zoznamu predajov, kde je to kvôli dĺžke
zoznamu). Pri Listings je zoznam pri jednom evente typicky oveľa kratší, tak mi to prišlo pohodlnejšie — ak
by ti to prekážalo, viem to zjednotiť so Sales.

## Čo som overil

```
cargo test --lib   -> 959 passed, 0 failed, 3 ignored (+11 nových testov oproti 2.2.4: 4 pre bulk status,
                       4 pre bulk price vrátane mixed-currency a all-or-nothing, 3 pre bulk delete;
                       + 1 existujúci test v Price Checkeri upravený kvôli novému 4. marketplace)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.2.5 build" v hlavičke)
cargo clippy       -> žiadne nové upozornenia v súboroch, ktoré sa menili tento krát
```

Nových 11 testov pokrýva presne to, čo si žiadal v zozname "Pridaj relevantné testy pre": bulk status (mení
len vybrané, odmietne neplatný status, all-or-nothing, dedup rovnakého ID viackrát), bulk price (mení len
vybrané a zachová menu, odmietne zmiešané meny, odmietne zápornú cenu, all-or-nothing) a bulk delete (zmaže
len vybrané, all-or-nothing, odmietne prázdny výber). Filtre/search/select-all/mixed selection sú v tomto
projekte doteraz vždy čisto frontendová logika bez vlastných backend testov (rovnako ako pri Sales) — prosím
over ich podľa checklistu nižšie.

## Zmenené súbory

**Backend:**
- `src-tauri/migrations/023_add_seatriks_marketplace.sql` — nový marketplace (čisto dátová zmena)
- `src-tauri/src/commands/ticket_listings.rs` — 3 nové bulk príkazy (status/price/delete) + 11 nových testov
- `src-tauri/src/commands/price_checker.rs` — 1 existujúci test upravený kvôli 4. marketplace
- `src-tauri/src/models.rs` — `BulkTicketListingsStatusInput`, `BulkTicketListingsPriceInput`
- `src-tauri/src/lib.rs`, `src-tauri/src/db.rs` — registrácia nových príkazov/migrácie
- `src-tauri/src/commands/database.rs` — aktualizovaný počet migrácií v teste (22 → 23)

**Frontend:**
- `src/pages/EventDetail.tsx` — 3 záložky namiesto 4, Add listing prerobený na order-browse výber, Listings s
  filtrami/search/bulk akciami, Finance presunuté do Sales
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy a API funkcie pre bulk operácie

**Nezmenené (podľa tvojho "nič nemen"):** Tickets/Inventory status logika, Orders, dáta a logika Finance,
Price Checker (okrem 1 testu vyššie), refund/resell, existujúce listing dáta.

**Verzia (9 miest v 7 súboroch):** `2.2.5`.

## STOP

2.2.5 hotové, otestované a zabalené. Skontroluj:

1. **Ktorýkoľvek Event → záložky hore** — over, že vidíš presne 3 záložky: Overview, Listings, Sales.
2. **Sales** — skontroluj, že úplne dole (pod predajmi a Market sekciou) vidíš aj Finance tabuľku so
   záznamami a odkaz "Open in Finance". Ak si Finance chcel radšej nechať oddelene, daj vedieť (bod 1
   vyššie v reporte).
3. **Add listing** — otvor ho a skús: vyhľadaj objednávku, otvor ju, označ 2-3 tikety naraz, over že sa dá
   nastaviť cena pre všetky naraz, ulož a skontroluj, že vznikol listing pre každý označený tiket.
4. **Listings — filtre a search** — prepni All/Active/Sold/Removed, vyskúšaj filter podľa marketplace aj
   search, over že tabuľka reaguje správne.
5. **Listings — bulk akcie** — označ viac listingov (checkboxy), over že sa lišta s akciami objaví až teraz;
   skús bulk edit status, bulk edit price (aj to, že sa tlačidlo vypne pri rôznych menách) a bulk delete;
   skús aj Select all/Deselect all.
6. Over, že **Tickets/Inventory stav, Orders, Finance dáta, Price Checker a refund/resell logika sa
   nezmenili** — nič z toho by touto zmenou nemalo byť ovplyvnené.
7. Skontroluj nový marketplace **Seatriks** — mal by sa objaviť v ponuke marketplace všade, kde sa dnes
   vyberá (Add listing, Price Checker).

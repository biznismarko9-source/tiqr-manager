# PRICE CHECKER — MARKET ANALYSIS — REPORT (verzia 2.2.0)

Toto je priama odpoveď na tvoje zadanie "PRICE CHECKER — MARKET ANALYSIS 2.2" (19 sekcií) plus tvoje samostatné, cez AskUserQuestion potvrdené rozhodnutie zmazať StubHub z Price Checkera úplne vrátane histórie. Celé je to postavené NAD existujúcim Visible Scannerom (2.1.9) — nová vrstva `commands/price_checker_analysis.rs` číta len to, čo scanner session už nazbierala (`session.listings`), a nikdy sa nedotýka scannerovho vlastného okna/eval/lifecycle kódu. Report je štruktúrovaný presne podľa tvojej vlastnej požiadavky — REAL DATA / DERIVED DATA / UNAVAILABLE DATA — a potom ide do detailu po jednotlivých bodoch tvojho zadania.

## REAL DATA

Toto je presne to, čo stránka/DB skutočne obsahuje, nikdy nie dopočítané ani odhadnuté:

Z scanneru (`NormalizedListing`, cez `price_checker_scan.js`): cena, mena (rozpoznaná len z pevnej tabuľky symbolov $/€/£ → USD/EUR/GBP, nikdy hádaná z voľného textu), section, row, quantity, tier/level (`tierFor` — buď inline text priamo v kontajneri listingu, alebo najbližší predchádzajúci heading-like element nad ním, presne ako to stránka sama ukazuje, žiadna normalizácia obsahu), listing_id (ak stránka má vlastný `data-*id*` atribút) a marketplace (ktorý reader výsledok vyprodukoval). Toto je identická sada polí, akú už scanner mal v 2.1.9 — Market Analysis pridáva jediné nové pole, `tier`.

Z "Your Tickets" (tvoj vlastný `tickets` tabuľka): section, row_label, currency, purchase_cost_cents + purchase_fees_cents + other_costs_cents, listing_price_cents — presne tie isté stĺpce a presne ten istý filter (`status IN ('available','listed')`), aký už používa `get_price_checker_summary_impl`, žiadna nová logika na to, čo sa počíta ako "nepredané".

## DERIVED DATA

Toto je dopočítané z real data cez jednoduchý, transparentný vzorec — nikdy AI, nikdy odhad:

Lowest/median/average/highest/count — na úrovni celej meny, každého tieru aj každej sekcie v tieri — je to ten istý `compute_scan_stats` z Visible Scanneru, len znovupoužitý (nie duplikovaný) na menšie skupiny. `data_quality` (Strong/Section/Tier/Partial comparable) hovorí, koľko štruktúrovaných dát ten-ktorý listing má, nezávisle od akéhokoľvek referenčného lístka. Comparable `level` (Exact/Close/Tier/General) hovorí, ako dobre sa ten istý listing zhoduje s JEDNÝM konkrétnym referenčným lístkom, ktorý zadáš. Price recommendation — vezme najužší neprázdny pool (exact → close → tier → general), jeho najnižšiu cenu podrazí o rovnakých 5 % (`RECOMMENDED_PRICE_UNDERCUT_PCT`), aké už používa existujúci manuálny Price Checker súhrn — teda rovnaký vzorec, nie druhý vymyslený nanovo. Confidence (High/Medium/Low) hovorí, nakoľko je odporúčanie podložené. `mixed_currencies` a `uncurrencied_listing_count` sú tiež odvodené — čisté počty/flagy, nikdy pretavené do jedného zmiešaného čísla.

## UNAVAILABLE DATA

`YourTicketGroup.tier` je VŽDY `null` — toto je overené, nie chýbajúce. Tabuľka `tickets` má `section`/`row_label`/`seat`, ale žiadny stĺpec pre tier/level. Jediné pole, ktoré by sa dalo za tier omylom zameniť, `ticket_type`, je v skutočnosti spôsob doručenia ("E-ticket"/"PDF"/"Mobile transfer"/"Physical"/"Will call" — `TICKET_TYPES`, Orders.tsx), nie sediace miesto/tier. Toto som si overil priamo v kóde PRED písaním, presne podľa tvojho bodu #18 ("najprv preskúmaj, nikdy nevymýšľaj pole") — keby som `ticket_type` použil ako tier, boli by to vymyslené, nezmyselné skupiny.

Druhá vec, ktorá tu chýba zámerne: doslovná mapa/sedadlový plán. Tvoje zadanie to explicitne nevyžadovalo ("NOT required to be a literal seating chart"), takže "## MAP / SECTION ANALYSIS" je implementovaná ako prehľadný zoznam tierov zoradených od najlacnejšieho, s vlastnými sekciami pod každým — nie graficky.

Tretia vec (nie nová, ale stále platná): reálny StubHub-nástupca (Viagogo) / Vivid Seats / Ticombo markup nie je v tomto sandboxe overiteľný — žiadny sieťový prístup na tie domény, presne ako pri 2.1.8/2.1.9. `tierFor` detekcia tieru teda funguje len na základe rozumných predpokladov o štruktúre stránky, nie na overenom reálnom markupe.

---

## TIER DETECTION (## TIER PRICING)

`group_by_tier` zoskupí listingy podľa `tier` — listing bez použiteľného tieru padne do doslovnej skupiny `"Unclassified"` (nikdy sa nezahodí, nikdy sa nehádaji, do ktorého tieru patrí). Skupiny sú zoradené od najlacnejšej.

Pri nezávislej adversariálnej kontrole tejto session (pozri sekciu TESTS nižšie) som našiel a opravil reálnu chybu: `group_by_tier` aj `group_by_section` pôvodne zoskupovali podľa PRESNÉHO (case-sensitive) reťazca, zatiaľ čo porovnávanie pre "## COMPARABLE MARKET" (`same_str`) je case-insensitive. Keďže `tierFor` vracia surový text zo stránky bez normalizácie, ten istý reálny tier sa na jednej stránke môže objaviť ako "Level 100" a inde ako "LEVEL 100" — bez opravy by sa to potichu rozdelilo na dva riadky v TIER PRICING namiesto jedného. Opravené tak, že zoskupovanie teraz interne používa lowercase kľúč, ale zobrazuje presne tú podobu, akú prvýkrát uvidelo (rovnaký princíp "prvý nájdený vyhráva", aký už `compute_scan_stats` používa pre menu). Dva nové regresné testy to overujú.

## SECTION DETECTION (## MAP / SECTION ANALYSIS)

`group_by_section` robí to isté v rámci JEDNÉHO tieru — listing bez sekcie sa jednoducho nezobrazí ako vlastný riadok (nie je ho čím čestne označiť), ale stále sa počíta do štatistík celého tieru. Rovnaká case-insensitivity oprava ako vyššie sa týka aj tejto funkcie.

## COMPARABLE LOGIC (## COMPARABLE MARKET)

`classify_comparable` implementuje presne tvoje poradie priorít ("same section, same tier, nearby sections in same tier, same quantity, nearby rows"): zhoda sekcie vyhráva okamžite (`exact_comparable`); pri zhode tieru sa dodatočne pozerá na blízku sekciu (do 3 čísel od seba), rovnaké quantity, alebo blízky row — ak čokoľvek z toho sedí, je to `close_comparable`, inak len `tier_comparable`; inak `general_market`.

Dôležitá poznámka k interpretácii: tvoje zadanie vymenúva "same quantity" a "nearby rows" ako samostatné položky v zozname priorít, no keďže výsledná klasifikácia má len 4 úrovne (nie 5+), implementácia ich berie ako DODATOČNÉ signály, ktoré platia len v rámci UŽ zhodného tieru (teda "same quantity" samo o sebe, bez zhody tieru, dnes nevytvorí žiadnu comparable úroveň, len `general_market`). Toto je rozumný, ale nie jediný možný výklad tvojho textu — ak si to predstavoval inak (napr. že rovnaké quantity má vlastnú váhu aj bez zhody tieru), daj vedieť a upravím.

`data_quality` a comparable `level` sú DVE nezávislé, čestné klasifikácie o tom istom listingu — jedna nikdy nepodmieňuje druhú. Listing môže byť `exact_comparable` (sedí sekcia) aj keď jeho vlastný `data_quality` je stále len `partial` (nič iné o ňom nie je potvrdené) — presne podľa tvojho poradia priorít, kde sa sekcia kontroluje ako prvá, bez podmienky. Počas návrhu som mal pôvodne pravidlo, ktoré by `partial`-kvalitný listing nútilo vždy do `general_market` bez ohľadu na zhodu sekcie — všimol som si to ako chybu ešte pred napísaním kódu (potláčalo by to skutočnú zhodu sekcie) a opravil som to skôr, než na tom čokoľvek záviselo.

## MARKET STATISTICS (## MARKET OVERVIEW)

Lowest/median/average/highest/count sa počíta na úrovni každej meny (`overall`), každého tieru a každej sekcie v tieri — vždy cez ten istý znovupoužitý `compute_scan_stats`, nikdy druhý raz napísaný nanovo. Výkonovo (tvoj bod "## PERFORMANCE"): scanner session sa prečíta RAZ (`compute_market_analysis` si vyžiada listingy zo session locku, hneď ho pustí, a všetko ostatné — štatistiky, tiery, sekcie, Your Tickets — sa počíta z tej istej kópie v pamäti, nikdy sa stránka nečíta znova).

## RECOMMENDATION (## PRICE RECOMMENDATION)

`recommend_price` vezme najužší neprázdny comparable pool (v poradí exact → close → tier → general), jeho najnižšiu cenu zníži o `RECOMMENDED_PRICE_UNDERCUT_PCT` (5 %, ten istý vzorec, aký už používa existujúci manuálny Price Checker súhrn). `market_average_price_cents` je zámerne CELKOVÝ priemer za danú menu (nie priemer zúženého poolu) — tvoje zadanie ich vymenúva ako dve rôzne čísla, nie jedno prepočítané dvakrát. Profit a ROI idú cez `finance::safe_ratio`, ktorý je bezpečný aj pri nulových nákladoch (vráti `null`, nikdy nespadne).

Confidence (High/Medium/Low) — tvoje zadanie explicitne pomenúva len "Low" pre riedke dáta; zvyšok (kedy je niečo High vs Medium) je rozumné, ale moje vlastné rozhodnutie, nie doslovne tvoje. Aktuálne pravidlo: 3+ exact comparables = High, menej než 3 exact = Medium, 2+ close alebo 3+ tier comparables = Medium, všetko ostatné = Low.

## HISTORY (## PRICE HISTORY)

Nová tabuľka `price_check_tiers` (migrácia 019) ukladá per-tier lowest/median/count k uloženému checku — presne to, čo si žiadal ("Rozšír ju o: tier lowest, tier median, listing count"), pripravené na budúci graf vývoja jedného tieru v čase. Samostatná child tabuľka, nie nové stĺpce na `price_checks` — check môže mať 0, 1 alebo veľa tierov, čo pevný počet stĺpcov nevie vyjadriť.

Jedna teoretická nezrovnalosť, ktorú som si všimol, ale zámerne NEOPRAVIL: `session.currency` (z `compute_scan_stats` v scanneri) berie prvú nájdenú menu bez orezania whitespace, zatiaľ čo `analysis.byCurrency[].currency` (z `partition_by_currency`) menu orezáva. Keďže mena sa vždy rozpoznáva cez pevnú tabuľku symbolov (nikdy zo surového textu), reálne táto nezrovnalosť dnes nemá ako nastať — ale ak by sa niekedy zmenil spôsob rozpoznávania meny, "Save to history" by mohol tichy nespárovať tier breakdown so session menou. Zámerne nechané tak (žiadny reálny dopad, nechcel som meniť viac, než bolo treba), ale zapisujem to sem, aby to nebolo prekvapenie neskôr.

## VISUALIZATION

Štyri nové komponenty v `PriceChecker.tsx`: `CurrencyMarketBlock` (tier/section prehľad pre jednu menu), `YourTicketsTable` (tvoje nepredané lístky + odporúčania), `MarketAnalysisPanel` (spája oboje pod live scanner kartou, refreshne sa po každom novom skene), `ComparableMarketTool` (samostatný nástroj — zadáš section/tier/row/quantity/menu, dostaneš zoradený zoznam s Exact/Close/Tier/General pilulkami). Ako je spomenuté vyššie, zámerne bez doslovnej mapy — tvoje zadanie ju nevyžadovalo.

## TESTS

40 nových Rust testov v `commands/price_checker_analysis.rs` (38 pôvodných + 2 pridané počas vlastnej adversariálnej kontroly tejto session — presne tie, čo dokazujú opravu case-sensitivity chyby vyššie). Pokrývajú: tier aj section zoskupovanie (vrátane toho, že KAŽDÝ tier má vlastné lowest/median/average, nie kópiu iného tieru — samostatný test na to), Exact/Close/Tier/General klasifikáciu vo všetkých kombináciách, nezávislosť `data_quality` od `level`, mixed-currency správanie (vrátane DB-backed testu), chýbajúci tier/section, odporúčací vzorec (najužší pool, market average z celku, fallback na general market), confidence pravidlo, a celý `compute_your_tickets` flow (zoskupenie, vylúčenie predaných lístkov, žiadne odporúčanie kým sa danú menu ešte neskenovalo).

"Multiple scans" a "dedup" z tvojho testovacieho checklistu (bod #17) sú zámerne NEOTESTOVANÉ tu znova — tie už pokrýva vlastná testovacia sada Visible Scanneru (`price_checker_scanner.rs`, `fingerprint_for`), keďže Market Analysis číta len to, čo scanner session už nazbierala a nededuplikuje nič sama.

Celá Rust sada beží čisto: **928 passed, 0 failed, 3 ignored**. `cargo check --lib --tests` aj `cargo clippy --lib --tests` čisté (nová vrstva má nula clippy warningov po tejto session; jediný existujúci warning v celom projekte je nesúvisiaci nepoužívaný `fetch_recent` v `sales.rs`, nie z tejto práce). Frontend: `tsc -b` aj `npm run build` prešli bez chýb.

Nezávislá adversariálna kontrola: pôvodne plánovaná ako dvaja samostatní subagenti (backend + frontend) — obaja narazili na týždenný rate limit skôr, než čokoľvek vrátili, takže som kontrolu urobil sám, rovnako dôkladne (migrácia 020 kvôli foreign key bezpečnosti, currency-safety vo všetkých funkciách, `classify_comparable` hraničné prípady, nezávislosť `data_quality`/`level`, SQL v `compute_your_tickets`, AppState locking poradie, `useEffect` race-safety v `MarketplaceCard`, required currency v `ComparableMarketTool`, React kľúče, spätná kompatibilita `SavePriceCheckModal`). Jediný reálny nález bol case-sensitivity chyba popísaná vyššie — opravená a otestovaná.

---

## STUBHUB — ÚPLNÉ ZMAZANIE

Tvoje samostatné rozhodnutie ("Zmazať úplne vrátane histórie", potvrdené cez otázku) je hotové — migrácia `020_remove_stubhub.sql`. Maže `price_check_tiers` → `price_checks` → `event_marketplace_links` → samotný riadok v `marketplaces`, v tomto poradí, v transakcii — explicitne, aj keď existujúce `ON DELETE CASCADE` (z 014_price_checker.sql) by to isté urobili aj samé. Toto som si pri kontrole overil naschvál ako prvé (najrizikovejšia vec v celej session — zabudnutá cudzia väzba by buď potichu nechala visieť "osirotené" riadky, alebo — keďže appka má `PRAGMA foreign_keys = ON` — celú migráciu na štarte tvrdo zhodila pre kohokoľvek, kto mal StubHub históriu). Je to v poriadku, nič nie je osirotené ani rozbité.

Dôsledok: 2 staré testy (`migration_017_safety_tests`), ktoré overovali `StubHub.active == false`, som musel upraviť na `COUNT(*) WHERE name = 'StubHub'` = 0, keďže riadok už neexistuje — pôvodný účel tých testov (overiť transakčnú bezpečnosť migrácie 017) ostáva zachovaný, len assert sedí na novú realitu. Migračný canary test (počet migrácií) prešiel z 18 na 20.

## DESIGN ROZHODNUTIA, KTORÉ SI ZASLÚŽIA TVOJU POZORNOSŤ

Zhrnutie vecí vyššie, ktoré som spravil ako svoje vlastné, zdôvodnené rozhodnutie nad rámec tvojho doslovného zadania — nie chyby, ale miesta, kde som sa rozhodol sám a chcem, aby si o tom vedel:

1. `ComparableReferenceInput.currency` je POVINNÉ pole, nie voliteľné — tvoje "## COMPARABLE MARKET" zadanie to nešpecifikovalo, ale tvoje vlastné "## CURRENCY" pravidlo (nikdy nemiešaj meny) by inak malo v tomto jednom flow dieru.
2. `data_quality` a `level` sú navrhnuté ako nezávislé (pozri COMPARABLE LOGIC vyššie).
3. `YourTicketGroup.tier` je vždy `null` (pozri UNAVAILABLE DATA vyššie).
4. Prahová hodnota "blízkosti" pre nearby section/row (`NEARBY_NUMERIC_THRESHOLD = 3`) je rozumný, ale NEOVERENÝ odhad — žiadny reálny marketplace markup na to nebol k dispozícii.
5. Confidence pravidlo (High/Medium/Low) je moje rozhodnutie nad rámec tvojho doslovného "Low for thin data" (pozri RECOMMENDATION vyššie).
6. Teoretická (dnes neškodná) nezrovnalosť session.currency vs. analysis currency (pozri HISTORY vyššie).
7. Interpretácia "same quantity"/"nearby rows" ako podmienených zhodou tieru (pozri COMPARABLE LOGIC vyššie).

## WHAT WAS LEFT UNTOUCHED

Visible Scanner (`price_checker_scanner.rs`) — session lifecycle, open/scan/cancel/close, `WebviewWindow` handling — nezmenené, jediný zásah je pridanie `tier` poľa a `pub(crate)` na 2 funkcie kvôli znovupoužitiu. Sales, Orders, Tickets/Inventory, Finance, Dashboard, Events, Pulls, Settings, Sheets sync, zálohy — nič z toho sa v tejto session nemenilo. Žiadne nové API kľúče, cloud služby, Puppeteer ani obchádzanie anti-bot ochrany — presne podľa tvojho zoznamu "must not add". Reálny StubHub-nástupca/Vivid Seats/Ticombo markup ostáva neoverený (pozri UNAVAILABLE DATA).

## STOP

Toto je celé, čo bolo v tejto session urobené pre Market Analysis 2.2 a úplné zmazanie StubHub. Ďalší krok je zdvihnutie verzie na 2.2.0 a zabalenie — poviem ti, keď to bude hotové.

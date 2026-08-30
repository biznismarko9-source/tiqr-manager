# TIQR Manager 2.0.81 — Price Checker (nová sekcia)

> *"Chcem teraz pridať úplne novú sekciu do appky s názvom Price Checker... Chcem, aby som si vybral konkrétny Event a ku každému eventu vedel uložiť link na: StubHub, Vivid Seats, Ticombo... Dôležité: začni čo najjednoduchšie, žiadne API, žiadny cloud, žiadny nový komplikovaný systém."*

## Čo je nové

V sidebari (dole pod Pulls) je nová, úplne samostatná sekcia **Price Checker**. Funguje presne v poradí, ako si to opísal:

1. **Vyberieš Event** hore (alebo prídeš rovno naň — na Event Detail je teraz tlačidlo **"Compare to market prices →"** priamo v Potential Profit boxe, ktoré ťa hodí do Price Checkera s už vybraným eventom).
2. Pre ten event vidíš 3 karty — **StubHub, Vivid Seats, Ticombo**. Do každej vložíš link na listing stránku toho eventu a klikneš Save.
3. Tlačidlo **"Check Prices"** na karte otvorí malé okienko, kam ručne zapíšeš, čo vidíš na tej stránke: najnižšia cena, priemerná cena, najvyššia cena, počet listingov, mena. Okienko sa samo predvyplní poslednými číslami, ktoré si tam zadal naposledy — takže druhá a ďalšia kontrola je len úprava pár čísel, nie písanie od nuly.
4. Každý "Check Prices" sa **pridá do histórie**, nikdy nič nezmaže ani neprepíše — presne ako si chcel, aby sa dalo vidieť, či cena šla hore alebo dole. Na karte preto hneď vidíš aj malú poznámku typu "12 € vyššie ako predtým" s farebnou šípkou, a staršie kontroly nižšie v tabuľke.
5. Hore na stránke je **"My inventory"** — počet nepredaných lístkov na ten event, tvoja priemerná nákupná cena, tvoja priemerná listing cena.
6. Nižšie je **"Market vs. mine"** — trhová najnižšia cena, trhová priemerná cena, **Recommended price**, **Expected profit**, **Expected ROI**.

### Dôležité: žiadne API, žiadny scraping

Presne podľa tvojej inštrukcie — pred tým, než som čokoľvek staval, som si overil, či niektorá z tých troch stránok ponúka bežnému predajcovi prístup k cenám bez toho, aby appka musela obchádzať ich ochranu:

- **StubHub** — žiadne verejné API pre bežného predajcu, navyše aktívne blokuje automatické sťahovanie stránok.
- **Vivid Seats** — API existuje, ale len pre schválených partnerov/veľkých predajcov, nie pre jednotlivca.
- **Ticombo** — žiadne verejné API.

Takže presne podľa tvojho zadania appka nikde nič automaticky nesťahuje ani neobchádza — všetky ceny zadávaš ty sám, ručne, po tom, čo sa pozrieš na stránku. Nižšie v sekcii "Nápady" je návrh, ako by sa dalo toto ručné zadávanie aspoň zrýchliť bez toho, aby appka čo i len raz sama navštívila tie stránky.

## ⚠️ Rozhodnutia, ktoré som urobil sám — over si ich

1. **Market lowest/average ráta len z checkov v TVOJEJ mene.** Ak má event nepredané lístky napr. v EUR, appka do "Market vs. mine" zoberie len najnovší check z každej marketplace, ktorý je tiež v EUR — check v inej mene sa do priemeru nezamieša (rovnaké pravidlo ako všade inde v appke — meny sa nikdy neblendujú naslepo). Ak by si mal nepredané lístky na jeden event vo viacerých menách naraz, appka to jasne napíše a market porovnanie proste nezobrazí (nedá sa vybrať, s akou menou porovnávať).
2. **"Moja" priemerná nákupná/listing cena počíta len z NEpredaných lístkov** toho eventu — lístky, čo si už predal, sa do toho nerátajú (Price Checker má zmysel len pre to, čo ešte máš na sklade).
3. **Trend (hore/dole) porovnáva len posledné 2 kontroly tej istej marketplace** — StubHub sa neporovnáva s Vivid Seats, len sám so sebou v čase.
4. **Marketplaces (StubHub/Vivid Seats/Ticombo) sú v appke ako spravovateľný zoznam** v databáze — presne ako Platforms alebo Event Categories. Backend už vie 4. marketplace pridať/zmazať (otestované), ale keďže si povedal "**neskôr** chcem vedieť jednoducho pridať ďalšie", zatiaľ som na to nepridával tlačidlo do UI (drží sa to tvojho "začni čo najjednoduchšie"). Ak to chceš mať hneď, daj vedieť — je to malá vec (pridám sekciu "Marketplaces" do Settings → Lookups, rovnako ako Platforms).
5. **Zmazanie eventu alebo marketplace zmaže aj jeho uložené linky a históriu cien.** Je to zámerné — link a cenová história sú "dá sa znova zadať" dáta, nie financie, takže nemá zmysel nechávať ich osirotené v databáze (na rozdiel od objednávok/predajov, tie appka chráni oveľa prísnejšie).
6. **Recommended price = 5 % pod najnižšou trhovou cenou** a **história sa ukladá navždy** — obe podľa tvojich vlastných odpovedí spred implementácie, len to tu pripomínam pre úplnosť.

## Ako presne to funguje pod kapotou

Tri nové tabuľky v databáze: zoznam marketplaces, uložené linky (jeden na event+marketplace), a história cenových kontrol (jeden nový riadok pri každom "Check Prices", nikdy sa needituje starý). Celá stránka sa pri otvorení eventu načíta jedným volaním, ktoré appke povie linky, celú históriu za každú marketplace, aj prepočítané "Market vs. mine" čísla naraz.

## Čo som overil

```
cargo test --lib   -> 765 testov, 0 zlyhaní, 3 ignorované (18 nových testov pre Price Checker)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.0.81 build" v hlavičke)
```

Medzi 18 novými testami sú aj také, čo overujú presne veci z bodov vyššie: trhové porovnanie ignoruje check v inej mene; priemer sa počíta len z nepredaných lístkov; druhý "Check Prices" na tú istú marketplace pridá nový riadok, nikdy neprepíše starý; recommended price sedí presne na 5 % pod najnižšou trhovou cenou.

Naviac, keďže appka nemá `git`, zip sa balil ručne (nie automaticky "všetko okrem .gitignore" ako inokedy) — tak som si to poistil extra krokom: rozbalil som hotový `tiqr-manager-2.0.81.zip` do prázdneho priečinka a odtiaľ znova spustil `npm ci`, `npx tsc -b`, `npm run build` a `cargo check --lib` — všetko prešlo čisto, takže v zipe je naozaj presne to, čo má byť, nič navyše ani chýbajúce.

## Zmenené súbory

**Backend (Rust):**
- `src-tauri/migrations/014_price_checker.sql` — 3 nové tabuľky (`marketplaces`, `event_marketplace_links`, `price_checks`), StubHub/Vivid Seats/Ticombo predvyplnené rovno v migrácii
- `src-tauri/src/commands/price_checker.rs` — nový súbor, všetka logika (linky, ukladanie kontrol, výpočet súhrnu za event), 18 testov
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` — zaregistrovaný nový modul a 6 nových príkazov
- `src-tauri/src/models.rs` — nové dátové typy pre Price Checker

**Frontend:**
- `src/pages/PriceChecker.tsx` — nová stránka (celá sekcia)
- `src/App.tsx`, `src/components/Layout.tsx` — nová route + nová položka v sidebari
- `src/pages/EventDetail.tsx` — tlačidlo "Compare to market prices →"
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy a volania na backend
- `src/pages/Orders.tsx` — zoznam mien (`CURRENCIES`) sprístupnený aj pre Price Checker (bez duplikovania)

**Verzia (9 miest v 7 súboroch):** `2.0.81`.

## Nápady, ako by sa to dalo časom vylepšiť / viac zautomatizovať

Pýtal si sa, či ma napadá niečo, čo sa dá spraviť lepšie alebo viac zautomatizovať — tu je pár reálnych, konkrétnych nápadov (žiadny z nich neobchádza ochranu žiadnej stránky):

1. **Tlačidlo na pridanie 4./5. marketplace priamo v appke** (Settings → Lookups) — backend je pripravený, chýba len UI. Malá vec, keď budeš chcieť.
2. **Pripomienka na kontrolu cien** — appka už má notifikačný systém (Settings → Notifications). Vedel by som doplniť: "tento event sa koná o X dní a ceny si nekontroloval Y dní" → appka ti sama pošle pripomienku, aby si nezabudol.
3. **Rýchlejšie ručné zadávanie** — namiesto prepisovania 4 čísel z obrazovky by appka vedela prijať kus textu, čo si sám skopíruješ zo stránky (napr. celý riadok s cenami), a sama by z neho vytiahla čísla do formulára. Stále 100 % ručné (appka nikam sama nechodí), len menej klikania.
4. **Ak by niektorá z tých troch stránok niekedy oficiálne otvorila API** (napr. pre väčších predajcov) — architektúra je na to pripravená, dá sa doplniť ako voliteľný "automatický" zdroj vedľa manuálneho, bez toho, aby sa čokoľvek z doterajšieho muselo meniť.

## STOP

2.0.81 hotové, otestované a zabalené. Ako si žiadal — zastavujem sa tu a čakám na tvoju spätnú väzbu, kým budem pokračovať na čomkoľvek ďalšom. Spusti `1-CLICK-UPDATE.bat` a skontroluj:

1. V sidebari klikni na **Price Checker** (nová položka dole pod Pulls).
2. Vyber ľubovoľný event → hore by si mal vidieť "My inventory", nižšie "Market vs. mine" a 3 karty (StubHub/Vivid Seats/Ticombo).
3. Vlož link do jednej karty, ulož, over že sa uložil (skús znova otvoriť ten istý event).
4. Klikni "Check Prices", vyplň čísla, ulož → hore v karte by sa mali objaviť, aj "Market vs. mine" by malo prepočítať Recommended price/Expected profit/ROI.
5. Skús "Check Prices" na tú istú marketplace ešte raz s inými číslami → over, že vidíš poznámku "vyššie/nižšie ako predtým" a starý záznam v histórii pod tým (nič sa neprepísalo).
6. Otvor detail ľubovoľného eventu → over tlačidlo "Compare to market prices →" v Potential Profit boxe.
7. Prezri si bod ⚠️ vyššie, hlavne bod 4 (UI na pridanie marketplace) — daj vedieť, či to chceš hneď, alebo to môže počkať.

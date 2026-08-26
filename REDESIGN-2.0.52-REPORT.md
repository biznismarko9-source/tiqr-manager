# TIQR Manager 2.0.52 — appka sa už nedá otvoriť viackrát naraz

## Čo si mi napísal

*"nemozem vediet otvorit apku viackrat, vzdy moze bezat len 1x"*

## Čo je nové

Keď appka už beží a skúsiš ju otvoriť znova (dvojklikom na skratku, cez Štart, čokoľvek), namiesto DRUHÉHO okna sa teraz jednoducho vráti do popredia to PRVÉ, už bežiace okno appky - aj keby bolo minimalizované alebo schované za inými oknami. Druhý proces sa sám ukončí skôr, než by čokoľvek stihol urobiť.

## Prečo je to dôležitejšie než len "otravné dve okná"

Popri tom, že dve okná tej istej appky vedľa seba (presne ako na tvojom screenshote) sú mätúce, je v tom aj skutočné riziko: appka drží celú svoju databázu v jednom SQLite súbore na disku. Keby si mal appku spustenú dvakrát naraz a v oboch oknách si niečo upravoval súčasne (napr. v jednom vytváral objednávku, v druhom mazal predaj), dva úplne oddelené procesy pristupujúce k tomu istému súboru naraz sú presne ten typ situácie, kde sa dáta môžu potichu stratiť alebo poškodiť - nič v appke doteraz pred týmto nechránilo. Táto oprava to rieši úplne na koreni: druhý proces sa nikdy ani nedostane k otvoreniu databázy.

## Ako presne to funguje

Použil som oficiálny Tauri plugin určený presne na toto (`tauri-plugin-single-instance`) - rovnaký princíp, akým bežné appky riešia to isté (Discord, Spotify a podobné): keď appka zistí, že už beží, namiesto nového okna len zavolá tú prvú inštanciu, nech sa ukáže dopredu. Registrovaný je ako úplne prvý plugin (presne to plugin sám vyžaduje) - musí bežať skôr než čokoľvek iné v appke, vrátane toho, čo appka doteraz robila hneď na začiatku (otvorenie databázy).

## Dôležité - ako som to overoval (prosím, prečítaj si to)

Toto kolo má oproti minulým jedno reálne obmedzenie navyše, ktoré ti chcem povedať narovinu: moje prostredie tentoraz nemalo k dispozícii ani len samotný `rustc` na kontrolu jedného súboru (na rozdiel od 2.0.47/2.0.50/2.0.51, kde to išlo) - `cargo test`/`npx tsc -b`/`npm run build` som teda nemohol spustiť vôbec, ani čiastočne. Overil som teda všetko, čo sa dalo overiť inak:

- `Cargo.toml` (kam pridávam nový riadok pre závislosť) som overil skutočným TOML parserom (Python `tomllib`) - súbor je syntakticky v poriadku a nová závislosť je v ňom správne zapísaná.
- `lib.rs` (kam pridávam samotné zapojenie pluginu) som overil vlastným skriptom, ktorý prejde celý súbor a skontroluje, že každá zátvorka/zložená zátvorka má svoj pár - vyšlo to čisto, žiadna nevyvážená zátvorka nikde v súbore.
- Samotný spôsob zápisu (`tauri_plugin_single_instance::init(...)`, callback s `app`/`argv`/`cwd`) som postavil presne podľa toho istého vzoru, akým appka už 4-krát registruje iné pluginy v tom istom súbore (dialog, process, updater, opener) - je to konzistentné s tým, čo appka už robí, nie nový, neoverený spôsob zápisu.
- `Cargo.lock` som (ako vždy) neupravoval ručne mimo čísla verzie appky samej - nová závislosť sa doň doplní automaticky, keď `cargo build` nabudúce pobeží so skutočným pripojením na internet (čiže pri tvojom `1-CLICK-UPDATE.bat`/GitHub Actions). To je úplne normálne správanie Cargo, nie chyba.

Zhrnutie: som si istý, že zápis je správny a zodpovedá presne tomu, ako appka už 4-krát robí to isté s inými pluginmi - ale keďže som si to tentoraz nevedel dať ani len skompilovať jeden súbor (nie ešte spustiť), toto by som chcel, aby si po nainštalovaní 2.0.52 vyskúšal obzvlášť pozorne, viac než zvyčajne.

## Prečo žiadne nové testy

Toto sa dotýka toho, ako appka štartuje ako celý proces (dva rôzne bežiace .exe naraz) - appkine testy (`cargo test --lib`) testujú funkcie priamo vo vnútri appky, nie spúšťanie dvoch skutočných procesov appky vedľa seba, takže sa to takto poctivo otestovať nedá. Skutočný test je presne to, čo popisujem nižšie - vyskúšať to naozaj u teba.

## Čo teraz urobiť

1. Nainštaluj 2.0.52.
2. Otvor appku, počkaj kým sa načíta.
3. Skús ju otvoriť znova (znova klikni na skratku/ikonu, alebo spusti .exe ešte raz).
4. Očakávané správanie: NEOTVORÍ sa druhé okno - namiesto toho sa dopredu ukáže to prvé, už bežiace.
5. Skús to aj vtedy, keď je prvé okno minimalizované - malo by sa obnoviť a ukázať dopredu.

## Zmenené súbory

**Backend (Rust, 2 súbory):**
- `src-tauri/Cargo.toml` - nová závislosť `tauri-plugin-single-instance`
- `src-tauri/src/lib.rs` - registrácia pluginu (ako prvý v poradí) + callback, čo privedie existujúce okno dopredu

Žiadny frontend súbor, žiadna migrácia, žiadna zmena existujúcej funkčnosti.

**Verzia (8 miest):** ako vždy, všetkých na `2.0.52`.

## STOP

2.0.52 - prosím over obzvlášť dôkladne (dôvod vyššie):

1. Skús otvoriť appku dvakrát za sebou - druhé okno by sa nemalo otvoriť, prvé by malo vyskočiť dopredu.
2. Skús to aj s prvým oknom minimalizovaným.
3. Over, že appka inak funguje úplne normálne (Dashboard, Orders, atď.) - táto zmena by sa nemala dotknúť ničoho iného.
4. Keď potvrdíš, že toto funguje, pokračujem podľa dohody na automatickú kategorizáciu eventov.

# TIQR Manager 2.116.0 — obrázok pre každú udalosť a aplikácia, ktorá padá bezpečne

Tvoja požiadavka (po anglicky): *„TIQR Manager — Smart Event Artwork + Full
Stability & Reliability Audit"*. Úloha mala dve časti:

- Ku každej udalosti nájsť správny obrázok interpreta alebo tímu, zobrazovať
  ho všade rovnako a nikdy nepriradiť cudzí.
- Prejsť celú aplikáciu a opraviť, čo ju môže zhodiť, zamraziť alebo
  potichu stratiť chybu.

---

## 1. Obrázky udalostí

### Kde ich vidno

Ten istý obrázok udalosti je na všetkých týchto miestach:

| Obrazovka | Kde | Veľkosť |
|---|---|---|
| Events | pri názve v zozname | 40 px |
| Stránka udalosti | v hlavičke (klik naň otvorí výber) | 72 px |
| Inventory | pri názve udalosti v zozname | 28 px |
| Objednávka | v hlavičke | 56 px |
| Sales | pri názve udalosti v zozname | 40 px |
| Predaj | v hlavičke (pri predaji z viacerých udalostí je prázdny rámik) | 56 px |
| Dashboard | v zozname Recent events | 40 px |

- **Obrázok sa vyberá len na stránke Events.** Ostatné obrazovky ukazujú
  obrázok udalosti, ku ktorej záznam patrí, a nikdy nehľadajú vlastný.
- **Výška riadkov v tabuľkách je presne taká ako predtým.** Je to zmerané
  pred aj po zmene: Events 48 px, Inventory 32 px, Sales 46 px. Obrázok
  zasahuje do okraja bunky, takže sa do zoznamov zmestí rovnako veľa
  riadkov.
- **Bez obrázka je v rámiku tichá sivá ikona.** Nikdy tam nie je obrázok
  niekoho iného.
- **Rámik má svoju veľkosť od začiatku**, takže sa nič neposunie, keď
  obrázok dorazí.
- Obrázok má alt text („Picture for …") a pri prejdení myšou ukáže, čo je
  na ňom a odkiaľ je.
- Logá a erby (napr. Real Madrid) sú na bielom štvorci celé. Fotky
  interpretov vyplnia rámik.

### Ako sa obrázok vyberá

**Nová udalosť (New Event):**
- Pribudol stĺpec **Picture**. Keď napíšeš názov, TIQR hneď hľadá.
- Malý náhľad ukáže, čo našiel. Otáznik znamená, že treba potvrdiť.
- Klik na náhľad otvorí pod tabuľkou výber.
- Voľba sa uloží až pri **Create**. Zrušený formulár nemení nič.
- Riadky s rovnakým názvom (turné) dostanú ten istý obrázok.

**Úprava udalosti (Edit)** má ten istý výber pod názvom. Voľba sa uloží
pri **Save changes**.

**Stránka udalosti:** tlačidlo **Picture**, alebo klik na obrázok v
hlavičke. Tu sa voľba uloží hneď. Dá sa:
- **Yes, use it**: prijať pravdepodobný obrázok.
- **Choose another**: vybrať z nájdených. Pri každom je zdroj, popis a miera
  istoty.
- **Upload**: nahrať vlastný. TIQR ho zmenší a meno súboru neukladá.
- **No picture**: nechať bez obrázka. Aj to je voľba, ktorá sa pamätá.
- **Back to automatic**: zrušiť ručnú voľbu.
- **Search again**: opýtať sa služieb znova.
- **All N events named „…" / Only this event**: voľba pre všetky udalosti s
  týmto názvom, alebo len pre túto jednu.

Hľadanie nikdy nezdrží uloženie udalosti. Formulár sa uloží hneď, aj keď sa
ešte hľadá.

### Odkiaľ sú obrázky

| Služba | Na čo |
|---|---|
| Deezer | hudba a komédia |
| Wikipedia | tímy, súťaže, festivaly, divadlo a preteky |

- **Obe sú zadarmo a bez kľúča.** Nič sa nemôže pokaziť tým, že vyprší účet
  alebo predplatné.
- **Posiela sa len názov udalosti.** Nič iné z TIQR neodchádza.
- **Obrázky sa sťahujú len z ich vlastných serverov** (`*.dzcdn.net`,
  `upload.wikimedia.org`). Platia rovnaké kontroly ako pri obrázkoch v
  maili.
- **Obrázok je uložený v TIQR**, nie ako odkaz. Je viditeľný aj offline a
  nezmizne, ak ho služba neskôr zmaže.
- **Vypína sa v Settings → Integrations → Event pictures** (pre každý počítač
  zvlášť). Keď je vypnuté, nič sa nehľadá, kým nestlačíš Search.

### Kedy si je TIQR istý

Každý nájdený obrázok má mieru istoty a dôvody, ktoré sú vidieť v zozname.

- **Istý** (napr. Coldplay, jediný interpret toho mena s 25 miliónmi fanúšikov)
  - ukáže sa sám a dá sa vymeniť.
- **Pravdepodobný** (napr. dvaja interpreti rovnakého mena, porovnateľne
  známi) - neukáže sa, kým ho nepotvrdíš.
- **Neistý** - nikdy sa neukáže sám, len ho môžeš vybrať zo zoznamu.

**Pravidlá, ktoré platia vždy:**
- **Meno musí sedieť presne.** Veľké písmená, diakritika a interpunkcia sa
  nepočítajú. „Coldplay Tribute Band" nikdy nie je „Coldplay".
- **Kategória rozhoduje, čo sa hľadá.**
  - Zápas sa hľadá medzi tímami, nikdy medzi hudobníkmi.
  - Pri koncerte musí Wikipedia písať o kapele alebo spevákovi.
  - Udalosť bez kategórie je najviac pravdepodobná, ibaže ide o naozaj
    známeho interpreta.
- **Dve stránky, ktoré obe sedia, znamenajú neistotu**, takže treba
  potvrdiť.
- **„Liverpool"** nájde najprv mesto, preto TIQR skúsi aj „Liverpool
  football" a nájde klub. **„Slovakia vs Germany"** v kategórii Football
  nájde národný tím, nie krajinu.

### Čo sa nikdy nestane

- **Ručná voľba sa nikdy neprepíše automaticky.** Výsledky hľadania sa
  ukladajú oddelene a do ručných volieb ich nič nekopíruje. Ani hľadanie,
  ktoré dobehne neskoro, ani hľadanie na druhom počítači ju nenahradí.
- **Podobné mená obrázok nezdieľajú.** Dva zápasy jedného tímu (Real Madrid
  vs Barcelona a Real Madrid vs Bayern) sú dve udalosti s vlastnou voľbou.
- **Päť termínov jedného interpreta zdieľa jeden obrázok.** Udalosti s
  rovnakým názvom sú jeden predmet. „Coldplay - Prague" a „Coldplay -
  Vienna" sú dva predmety, ale jedno hľadanie, takže dostanú rovnaký
  automatický obrázok aj obrázok vybraný ručne pre „Coldplay".
- **Po reštarte sa nič nehľadá znova.** Pred akýmkoľvek hľadaním sa použije
  už potvrdená voľba.
- **Pre jeden predmet beží naraz len jedno hľadanie**, aj keď sa pýta viac
  obrazoviek naraz.

### Dva počítače, záloha

- **Ručná voľba sa prenáša na druhý počítač** cez živú synchronizáciu aj
  cez zálohu na Drive. Keď obe zmeny vzniknú oddelene, vyhrá novšia, ako pri
  iných záznamoch. Firebase pravidlá netreba meniť.
- **Automatický obrázok ostáva na počítači, ktorý ho našiel.** Druhý počítač
  si ho nájde sám. Vďaka tomu automatická odpoveď nikdy neprepíše ručnú voľbu
  na druhom počítači.

### Na pozadí

Udalosti, ktoré ešte obrázok nemajú, TIQR prejde postupne:
- Nanajvýš tri hľadania naraz, najprv najbližšie udalosti, len keď je okno
  otvorené v popredí.
- Každá služba má vlastné tempo. Keď povie „too many requests", TIQR ju
  nechá na pokoji.
- Bez internetu si TIQR na tri minúty dá pauzu.
- Neúspech sa skúsi znova o 10 minút, potom o hodinu, 6 hodín a 24 hodín.
- Chyba nikdy nenahradí istý obrázok, ktorý už je uložený.

---

## 2. Stabilita: čo bolo zlé a čo sa opravilo

| Problém | Príčina | Teraz |
|---|---|---|
| **Chyba pri kreslení obrazovky zhodila celé okno** (prázdna obrazovka, stratený rozpísaný formulár). | React pri nezachytenej chybe odstráni všetko, a nikde nebola zachytávacia vrstva (*error boundary*). | Každá obrazovka má vlastnú vrstvu a celá aplikácia ešte jednu. Spadne len tá obrazovka, ukáže ID chyby (napr. *E-7K2Q*) a tlačidlá *Try again*, *Go to the Dashboard* a *Copy diagnostic report*. |
| **Pád aplikácie nezanechal žiadnu stopu.** | Release build sa pri panike rovno ukončí (`panic = "abort"`) a nikde sa nič nezapísalo. | Pred ukončením sa zapíše poznámka (kde a čo, bez citlivých údajov). Pri ďalšom štarte sa raz zobrazí žltý pás „TIQR closed unexpectedly last time" s odkazom na report. |
| **Pád pri správe na Discord**, keď dátum udalosti mal za časom nezvyčajný znak (napr. z importu „19:3€"). | Kód rezal text na 5 bajtov, čo môže prerezať znak napoly. V Ruste to je panika, teda pád celej appky. | Bezpečné čítanie. Pribudol regresný test. |
| **„Detect categories" zamrazil okno**, pri 50 udalostiach aj na desiatky sekúnd. | Príkaz bežal na vlákne, ktoré kreslí okno, a počas čakania na AI držal databázu zamknutú, pre každú udalosť zvlášť. | Beží na pozadí a databázu drží len pri čítaní a zápise. Kategória zvolená ručne počas čakania sa neprepíše. Pribudli dva testy. |
| **Migrácia, ktorá zlyhala v polovici, zablokovala databázu.** | Migrácia a záznam o nej sa neukladali spolu. Pri ďalšom štarte sa spustila znova a zlyhala na tom, čo už prebehlo. | Každá migrácia sa uloží spolu so svojím záznamom naraz, alebo vôbec. Test to overuje na migrácii, ktorá zámerne spadne. |
| **Escape zavrel naraz dialóg aj formulár pod ním** (stratený rozpísaný formulár). | Každé okno počúvalo samo a po zatvorení horného sa ďalšie pokladalo za horné. | Jedno rozhodnutie pri stlačení klávesu, zatvorí sa len horné okno. Overené v prehliadači. Aj potvrdzovací dialóg sa dá zrušiť cez Escape. Dialógy sú označené pre čítačky obrazovky. |
| **59 miest zahadzovalo chyby potichu** (`.catch(() => {})`). | Pri chybe na pozadí (zoznam platforiem, stav synchronizácie, oznámenia…) sa nič nikde nezapísalo. | Každá sa zapíše do záznamu (*warning*), na obrazovke sa nič navyše nezobrazí. |
| **Chyby mimo Reactu** (vyhodené mimo obrazovky, neodchytený *promise*) sa nikde neukázali. | Nikto ich nezachytával. | Zachytia sa a zapíšu. |

Ďalej sa preverilo a je v poriadku, bez zmeny:
- **Sieť má všade časový limit.** Klienti bez výslovného limitu majú
  predvolených 30 sekúnd.
- **Rezanie textu podľa bajtov** sa našlo len na jednom ďalšom mieste a to
  bolo opravené už v 2.109.1.
- **Ostatné `unwrap`/`expect` v bežiacom kóde** majú kontrolu tesne pred
  sebou.
- **Databáza** má zapnuté cudzie kľúče, WAL a `busy_timeout` 5 s.
- **Sieťové príkazy pri maile, Drive a Sheets** bežia mimo vlákna okna.

## 3. Diagnostika

- **Záznam chýb:** súbor `logs/tiqr.log` vedľa dát TIQR.
  - Každý riadok má čas, závažnosť (INFO/WARNING/ERROR/FATAL), krátke ID,
    miesto a čo sa stalo.
  - Najviac 1 MB, predchádzajúci súbor sa ponechá ako `tiqr.1.log`.
  - Ukladajú sa tam aj chyby obrazovky. ID chyby z obrazovky je to isté ako
    v súbore.
- **Maskovanie:** pred zápisom sa z každého riadku odstránia e-mailové adresy,
  tokeny, kľúče, heslá, kódy, cookies, dlhé tajné reťazce a obrázky. Testy to
  overujú na skutočných tvaroch (Google, Firebase, Anthropic).
- **Settings → Support → Diagnostic report** (*Copy report* / *Save to a
  file*). Obsahuje:
  - verziu, systém, verziu schémy a veľkosť databázy,
  - počty záznamov (nikdy ich obsah),
  - stav obrázkov, živej synchronizácie a zálohy na Drive,
  - poznámky o pádoch a posledné riadky záznamu, všetko zamaskované.
  
  Toto je čo poslať, keď sa niečo pokazí.

---

## 4. Testy a kontroly

Spustené v tomto prostredí, po poslednej zmene:

| Kontrola | Príkaz | Výsledok |
|---|---|---|
| Rust testy | `cd src-tauri && cargo test --locked` | **1641 prešlo**, 0 zlyhalo, 4 ignorované (rovnaké ako predtým) |
| Frontend testy | `npm test` | **64 prešlo**, 0 zlyhalo |
| Typy | `npx tsc -b` | bez chyby |
| Build | `npm run build` | v poriadku |
| Clippy | `cargo clippy --locked` | 121 varovaní, rovnako ako pred prácou (všetky staršie), žiadna chyba |
| Formát | `cargo fmt --check` | repozitár nebol nikdy formátovaný cez rustfmt (3059 rozdielov už pred touto prácou), nový kód je písaný v štýle okolia, nič sa hromadne nepreformátovalo |

**Východiskový stav pred prácou:**
- `cargo test`: 1596 prešlo, 4 ignorované, 0 zlyhalo.
- `npm test`: 53 prešlo.
- Typy a build: v poriadku.
- Clippy: 121 starších varovaní, žiadna chyba.

Žiadna chyba nebola zavlečená, ani žiadna stará nezostala skrytá.

**Nové testy:**
- **Obrázky (33 v Rust module + 1 test s dvoma počítačmi):**
  - istá zhoda,
  - nič nenájdené, nejednoznačné meno,
  - päť termínov s jedným hľadaním,
  - nesúvisiace udalosti,
  - neskoré hľadanie oproti ručnej voľbe,
  - cache bez siete,
  - rovnaký obrázok na všetkých obrazovkách,
  - offline a vrátenie po pauze, „too many requests",
  - poškodený obrázok,
  - obrázok, ktorý sa nedá stiahnuť,
  - reštart so súborovou databázou,
  - veľa udalostí (3000 udalostí zmapovaných za 120 ms v debug builde),
  - súbežné hľadanie,
  - mená v rôznych jazykoch a písmach,
  - povolené servery, kontrola nahraného obrázka,
  - prenos voľby na druhý počítač a novšia voľba vyhrá.
- **Úložisko obrázkov vo frontende (7):**
  - jedno načítanie mapy a dávky obrázkov,
  - zahodenie starej odpovede,
  - chýbajúci obrázok, pauza po chybe,
  - zmenený obrázok, obmedzená veľkosť cache, nahlásenie chyby.
- **Stabilita:**
  - atómové migrácie (3),
  - Discord dátum (1),
  - detekcia kategórií (2),
  - záznam, maskovanie, poznámka o páde, rotácia, report (5 v Ruste, 4 vo
    frontende).

Všetko je deterministické, s pevnými odpoveďami. Žiadny test nejde na
internet.

**Overené v prehliadači** (Chromium s napodobeným backendom), so snímkami:
- zoznam Events, Inventory a Sales s obrázkami a rovnakou výškou riadkov,
- stránka udalosti, výber (istý, pravdepodobný, bez výsledku, zoznam
  kandidátov),
- stĺpec Picture pri novej udalosti,
- obrazovka chyby a jej zrušenie po prechode inde,
- zachytenie chýb mimo Reactu,
- Escape pri dvoch otvorených oknách.

## 5. Databáza

**Migrácia 047** len pridáva dve tabuľky:
- `event_artworks`: ručné voľby, synchronizované.
- `artwork_lookups`: odpovede služieb, len na tomto počítači.

**Čo ostáva bezpečné:**
- Nič existujúce sa nemení ani nemaže.
- Každý príkaz je `IF NOT EXISTS`, takže prerušený štart ju spustí znova
  bez škody.
- Staršia verzia TIQR nové tabuľky ignoruje.
- Pred aktualizáciou netreba nič robiť. Body obnovy (Settings → Data)
  fungujú ako doteraz.

**Spôsob, ako sa vrátiť:** obe tabuľky sú samostatné. Bez nich aplikácia
funguje ako 2.115.1, len bez obrázkov.

## 6. Čo ostáva otvorené (zapísané v `PROJECT_STATE/KNOWN_BUGS.md`)

- **Obrázky sú overené len s napodobenými odpoveďami.** Toto prostredie sa na
  Deezer ani Wikipediu nedostane, takže tvary odpovedí sú podľa ich
  dokumentácie. Každé zlyhanie končí prázdnym rámikom a novým pokusom
  neskôr, nikdy zlým obrázkom. Skutočnou skúškou je prvé použitie u teba.
- **„No picture" platí pre jeden názov.** Premenovaná udalosť patrí pod nový
  názov.
- **Synchronizácia so Sheets, ktorá vytvára nové udalosti, sa pýta AI na
  kategóriu so zamknutou databázou.** Okno môže chvíľu čakať. Táto cesta beží
  v transakcii synchronizácie a jej prerábanie by zasahovalo do
  synchronizácie, ktorú úloha nedovoľovala meniť.
- **Databáza, ktorá sa pri štarte nedá otvoriť, aplikáciu stále zavrie.**
  Dôvod je teraz v `logs/tiqr.log`. Štart na náhradnej databáze by hrozil
  tým, že by sa do nej písalo a stratilo, preto to nie je zavedené.
- **Kým nemajú oba počítače 2.116.0**, starší hlási voľby obrázkov ako
  konflikty. Nič sa nestratí a po aktualizácii to zmizne.
- Staršie body v KNOWN_BUGS (napr. príznak neodoslanej práce pri Drive) sú
  návrhové otázky, ktoré úloha nedovoľovala meniť. Zostávajú, ako boli.

## 7. Zmenené súbory

**Nové:**
- Rust:
  - `src-tauri/src/commands/event_artwork.rs`
  - `src-tauri/src/diag.rs`
  - `src-tauri/migrations/047_event_artwork.sql`
- Frontend:
  - `src/lib/artworkStore.ts` (+ test)
  - `src/lib/eventArtwork.ts`
  - `src/lib/diagnostics.ts` (+ test)
  - `src/lib/diagnosticsSink.ts`
  - `src/components/EventArt.tsx`
  - `src/components/ArtworkPicker.tsx`
  - `src/components/ErrorBoundary.tsx`
- Dokumentácia: `history/REDESIGN-2.116.0-REPORT.md`

**Upravené:**
- Rust:
  - `db.rs`: atómové migrácie, migrácia 047, cache ako bookkeeping, testy.
  - `lib.rs`: záznam pri štarte, príkazy.
  - `ai_categorize.rs` a `commands/events.rs`: detekcia kategórií mimo
    zámku.
  - `commands/discord.rs`: dátum.
  - `commands/mail_view.rs`: spoločné sťahovanie obrázkov.
  - `commands/cloud_merge.rs`: `event_artworks` v synchronizácii.
  - `commands/live_sync.rs`: test.
  - `commands/database.rs`: počet migrácií.
  - `commands/mod.rs`.
- Frontend:
  - obrazovky Events, EventRowsModal, EventDetail, Orders, OrderDetail,
    Sales, SaleDetail, Dashboard, Settings,
  - `Layout.tsx`, `App.tsx`, `main.tsx`, `ui.tsx` (dialógy), `icons.tsx`,
  - `api.ts`, `types.ts`,
  - 59 miest s tichými chybami v ďalších súboroch.
- Dokumentácia: `CHANGELOG.md`, `PROJECT_STATE/*`.

## 8. Ako vydať

Zip `TIQR-2.116.0-NA-VYDANIE.zip`, ako vždy: rozbaliť do prázdneho priečinka
a spustiť `1-CLICK-UPDATE.bat`.
- Zip obsahuje aj 2.115.0 a 2.115.1, ktoré ešte nie sú vydané.
- Kvôli 2.115.0 ostáva v návode krok s novými pravidlami Firebase (ak ešte
  nie sú zverejnené).
- Samotné 2.116.0 pravidlá nemení.

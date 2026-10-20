# TIQR Manager 2.0.63 — automatické rozpoznávanie kategórie eventu (3. bod)

## Čo si chcel

Aby appka pri evente zo synchronizácie z Google Sheets sama poznala kategóriu (napr. "Liverpool -
Nottingham Forest" → šport, "Celine Dion" → koncert), namiesto toho, aby si ju musel doteraz vždy dopĺňať
ručne. V rozhovore sme si postupne ujasnili:

- Šport ostáva **jedna** kategória "Sports" (žiadne samostatné Football/Tenis/...).
- Pri mene bez jasného signálu appka sa **nemá vzdať** (nechať bez kategórie) ani **hádať naslepo**
  (automaticky Concert) — tvoje slová: *"musi vediet zistit o co ide"*.
- Rovnako pri tvare "Tím A - Tím B" — tvoje slová: *"musi to vediet rozoznat"*, nie len slepo spoznať
  pomlčku medzi dvoma menami (to by sa pokazilo napríklad pri koncerte dvoch účinkujúcich v rovnakom tvare
  názvu).
- Staré eventy bez kategórie sa majú doplniť aj spätne, nie len tie nové.

## Dôležitý kontext, ktorý si možno nevedel

Appka už od verzie 2.0.27 má kategórie ako skutočnú tabuľku v databáze (`event_categories`) — s farbami, so
správou v appke, s filtrovaním v Events/Orders/Sales. Chýbalo presne to, o čo si žiadal: keď synchronizácia
z hárku založí nový event, kategóriu doteraz appka nechala vždy prázdnu.

## Prečo je to kombinácia pravidiel + AI

Skúsil som najprv čisto textové pravidlá (kľúčové slová, tvary v názve) — ale tie fyzicky nedokážu to, čo
si žiadal v druhom kole otázok: nemajú ako zistiť, že "Celine Dion" je speváčka, keďže jej meno samo o sebe
nič neprezrádza. Aby appka toto **naozaj vedela**, musí sa niekoho opýtať — presne to si aj potvrdil, keď
si zo 3 možností vybral kombináciu.

**Ako to funguje (`ai_categorize.rs`):**

1. Appka najprv zadarmo skúsi rozpoznať jasné, bezpečné signály v názve: "Grand Prix"/"Formula 1"/"MotoGP"
   → Motorsport, "Festival" → Festival, "Musical"/"Divadlo"/"Theatre" → Theatre / Musical,
   "Comedy"/"Stand-up" → Comedy. Toto nestojí nič a nepotrebuje internet.
2. Čo takto nerozpozná — čo je väčšina prípadov, vrátane **každého** športového zápasu a každého holého
   mena umelca — pošle appka jedným malým dopytom modelu Claude Haiku (najlacnejší/najrýchlejší model),
   spolu so zoznamom tvojich reálnych kategórií, a opýta sa, ktorá z nich (ak vôbec nejaká) sedí. Model
   nemôže "vymyslieť" novú kategóriu — smie odpovedať len jedným z mien, čo mu appka ponúkla, alebo "neviem".
3. Čokoľvek zlyhá — žiadny kľúč, výpadok internetu, model odpovie niečím, čo appka nepozná — appka to berie
   presne rovnako ako "nedá sa určiť": event ostane bez kategórie, presne ako dnes. Nikdy nič neuhádne
   naslepo a nikdy to nezablokuje synchronizáciu.

**Cena:** jeden takýto dopyt stojí zlomok centu (Claude Haiku: $1 za milión vstupných tokenov, $5 za milión
výstupných — a tento dopyt má rádovo stovky vstupných a pár výstupných tokenov). Navyše sa pýta LEN na
eventy, ktoré si kľúčové slová samé nevyriešia — nie na každý event.

## Čo musíš urobiť TY, aby AI časť naozaj fungovala

Bez tohto appka funguje ďalej presne tak, ako je opísané vyššie — len druhý krok (AI) sa jednoducho
preskočí a všetko okrem Motorsport/Festival/Theatre/Comedy ostane bez kategórie, presne ako dnes. Nič sa
nepokazí, nič nezlyhá — len kým toto neurobíš, appka nebude vedieť, kto je Celine Dion.

1. Choď na **console.anthropic.com** a vytvor si tam API kľúč (ak ešte nemáš účet, treba si ho založiť).
2. Na GitHub v tvojom repozitári: **Settings → Secrets and variables → Actions → New repository secret**
3. Meno: `ANTHROPIC_API_KEY`, hodnota: kľúč, čo si dostal v kroku 1.
4. Ďalší build cez `release.ps1` (alebo test build z Actions) tento kľúč automaticky zabuduje — presne tým
   istým spôsobom, akým appka už dnes zabuduje kľúč pre Google Sheets.

## Kde to v appke nájdeš

- **Automaticky:** odteraz, keď Order sync/Sales sync narazí na úplne nový event (ešte v appke nie je),
  appka sama skúsi kategóriu doplniť — presne, ako keby si ju hneď po vytvorení eventu ručne vybral.
- **Spätne:** na stránke **Events** je nové tlačidlo **"Detect categories"** — prejde všetky eventy, čo
  ešte kategóriu nemajú, a skúsi ju doplniť. Bezpečné kliknúť opakovane — nikdy sa nedotkne eventu, ktorý už
  kategóriu má (nech ju získal akokoľvek).

## Čo som overil

```
cargo test --lib   -> 650 testov (630 + 20 nových), 0 zlyhaní, 3 ignorované
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

20 nových testov pokrýva: kľúčové slová (Motorsport/Festival/Theatre/Comedy) aj to, že tvoje dva vlastné
príklady ("Celine Dion", "Liverpool - Nottingham Forest") správne NEPADNÚ do žiadneho voľného pravidla;
spracovanie AI odpovede (odmietne kategóriu, čo appka vôbec nepozná, aj keby ju model "vymyslel"); že nový
event zo sync skutočne dostane kategóriu; že existujúci event sa nikdy nedotkne; a celé správanie tlačidla
"Detect categories" vrátane toho, že druhé spustenie po sebe už nič nerobí.

## Zmenené súbory

**Backend:**
- `src-tauri/src/ai_categorize.rs` (nový súbor) — pravidlá, AI dopyt, rozhodovacia logika
- `src-tauri/src/commands/orders_sheet_sync.rs` — `resolve_or_create_event` teraz pri novom evente skúša
  kategóriu doplniť
- `src-tauri/src/commands/events.rs` — nový príkaz `detect_event_categories` (spätné doplnenie)
- `src-tauri/src/models.rs` — nový `CategoryDetectionResult`
- `src-tauri/src/lib.rs` — registrácia nového modulu a príkazu
- `src-tauri/build.rs` — zabudovanie `ANTHROPIC_API_KEY` rovnakým spôsobom ako Google kľúč
- `.github/workflows/build-windows.yml` — nový (voliteľný) secret + vysvetlenie v hlavičke súboru

**Frontend:**
- `src/pages/Events.tsx` — nové tlačidlo "Detect categories" + potvrdzovací dialóg
- `src/lib/api.ts`, `src/lib/types.ts` — nová funkcia/typ pre výsledok

**Verzia (8 miest):** `2.0.63`.

## STOP

Toto je posledný z tvojich pôvodných 3 bodov. Skús si "Detect categories" na Events stránke a skontroluj,
či pri ďalšej synchronizácii nové eventy dostávajú kategóriu podľa očakávania — a keď budeš chcieť, aby sa
rozpoznávali aj mená ako Celine Dion, urob si prosím ten Anthropic kľúč podľa krokov vyššie. Daj vedieť, ako
to funguje, alebo čo by si chcel upraviť/pridať ďalej.

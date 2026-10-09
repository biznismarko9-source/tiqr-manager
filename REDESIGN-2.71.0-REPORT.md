# TIQR Manager 2.71.0

**Dátum:** 8.10.2026
**Zadania:** restrictions ako dropdown · convert mien nefunguje · nájdi chyby · zmenšiť AI usage

---

## 1. Restrictions: osem písmen -> dropdown so slovami

Mal si pravdu, bolo to nečitateľné. V 2.67.0 som usúdil, že osem názvov sa do
bunky mriežky nezmestí, tak dostal každý jedno písmeno a slovenský názov sa
schoval do `title`. Presne to si nevedel prečítať.

Nová komponenta **`MultiSelect`** v `ui.tsx` - `Select` pre hodnotu, ktorá je
zoznam:

- **Zatvorená** vyzerá ako Platform vedľa nej a číta sa `Výhľad, 18+`.
- **Otvorená** vypíše všetkých osem **plným slovenským názvom**, s fajkou na
  vybraných.
- Sú to tie isté názvy, ktoré už ukazuje per-lístkový editor v `Tickets.tsx` -
  takže obe obrazovky sa nemôžu rozísť.

Natívny `<select multiple>` by to nevyriešil: renderuje sa ako trvalo otvorený
list box, ktorý sa do bunky tabuľky nezmestí. Preto je to tlačidlo nakreslené
ako zatvorený `Select` plus panel.

**Panel ide cez portál.** `.table-shell` je `overflow-auto`, takže panel
umiestnený v bunke by scroll kontajner odstrihol, hneď ako je vyšší než riadok
- čo s ôsmimi možnosťami je vždy. Scroll a resize panel **zatvárajú**,
nepresúvajú - panel, ktorý sa vlečie za vlastným tlačidlom po tabuľke
scrollujúcej v dvoch smeroch, vyzerá ako chyba.

Poradie ostalo: štyri, čo stoja peniaze, sú stále prvé. `RESTRICTION_GLYPH`
zmizol, existoval len pre tie prepínače.

## 2. Convert mien: `US $` nie je mena

Ten 404 nebol výpadok služby. **Objednávka YELIVE-001 má menu uloženú ako
`US $`**, nie `USD` - AI import to tak prečítal zo screenshotu z Dallasu
a `fx::normalize_currency` ten tvar nepoznala, takže ho `to_uppercase()`
nechal tak a poslal do URL.

Overil som to naživo proti tej službe:

| dotaz | odpoveď |
|---|---|
| `from=GBP&to=EUR` | 200, kurz 1.1814 |
| `from=US $&to=EUR` | **404 `{"message":"not found"}`** - presne tvoja chyba |
| `from=EUR&to=EUR` | 422 `{"message":"bad currency pair"}` |

Endpoint aj sieť boli teda v poriadku. Chybná bola tá jedna hodnota.

**Dve opravy v `fx.rs`:**

1. `normalize_currency` pozná symbol s predponou krajiny - `US$`, `A$`, `C$`,
   `NZ$`, `HK$`, `S$`, `R$` - a to aj s medzerou vnútri. Neznámy kód stále
   prepadne na obyčajný veľký tvar, ako predtým.
2. Kód, ktorý nemá tri ASCII písmená, sa **odmietne pred odoslaním** a chyba
   pomenuje konkrétnu hodnotu. Kontroluje sa PRED skratkou `from == to`, inak
   by dve kópie tej istej nepoužiteľnej hodnoty ticho vrátili kurz 1.0 a
   prepočítali reálne peniaze kurzom, ktorý nikto nezistil.

### Tú objednávku ti to neopraví

Normalizácia beží na **vstupe**. Uložená hodnota `US $` tam ostáva, takže
YELIVE-001 sa nepodarí skonvertovať ani po tejto oprave. Dáta sú tvoje,
nesiahol som na ne. Najčistejšie je zmazať tú objednávku a založiť ju znova
s `USD` - je jedna a má jeden lístok.

## 3. Kontrola dát

Pätnásť integritných dotazov nad tvojou živou databázou (kópia, originál som
neotvoril na zápis).

**Čisté:** meny lístkov vs objednávok · quantity vs počet lístkov · osirelé
predaje · `sold` bez predaja · `available` s aktívnym predajom · záporné sumy
· duplicitné kódy · objednávky bez eventu · `restrictions_json` (platnosť aj
neznáme kódy) · statusy · payment_status.

**Jediný nález** je tá mena z bodu 2.

**Falošný poplach:** osem `batch_id` tvaru `SAL-000xxx`, ktoré nezodpovedajú
žiadnemu kódu predaja. To je stav z čias pred 2.30.0, keď sa kódy prečíslovali
na event-ové a `batch_id` ostalo. `GROUP_BASE_SELECT` s tým vedome počíta -
zobrazovaný kód berie z `MIN(id)` riadku, nie z `batch_id`. Zoskupovanie
funguje správne, overil som to na dátach. **Nedotkol som sa toho.**

## 4. AI usage - zmeral som to a šetriť je na čom málo

Apka si spotrebu počíta sama (`ai_usage_by_month`). Tvoje reálne čísla:

| | skenov | vstup | výstup | cena |
|---|---|---|---|---|
| September | 61 | 147 472 tok | 16 046 tok | **$1.14** |
| Október (3 dni) | 3 | 7 080 tok | 521 tok | $0.05 |

To je **1,87 centa na sken**. Rozdelenie: **65 % vstup, 35 % výstup.**

Čiže to, čo platíš, je **obrázok** - 2 417 vstupných tokenov na sken, z toho
väčšina je samotný screenshot. Výstup je len 263 tokenov.

**Čo som NEZMENŠIL a prečo:**

- **Rozlíšenie obrázka** (1568 px na dlhšej hrane) je ten jediný veľký lever
  na vstupe - tokeny rastú s plochou, takže 1100 px by ušetrilo asi 40 %.
  **Nesiahol som na to.** Pri screenshotoch s malým textom to priamo zhoršuje
  čítanie ceny a sedadla, a jedna zle prečítaná cena ťa stojí viac než
  celoročný AI rozpočet tejto funkcie.
- **Effort** je už na `medium`, pod API defaultom `high`. Pri 263 výstupných
  tokenoch na sken by `low` ušetrilo asi 12 centov za rok.

**Jediný lever, ktorý reálne niečo znamená:** model. Teraz je to
`claude-opus-5` za $5/$25 za milión tokenov. Sonnet je rádovo päťkrát lacnejší
- $1.14 by spadlo na asi 25 centov za mesiac.

**Nespravil som to sám**, lebo ten model čítá z obrázkov tvoje peniaze
a je to tvoje rozhodnutie, nie moje. Ak povieš, prepnem to - treba pri tom
zmeniť aj cenovú tabuľku v `ai_import.rs` (`INPUT_CENTS_PER_MTOK` /
`OUTPUT_CENTS_PER_MTOK`), inak by ti apka počítala cenu podľa zlého modelu.

Môj názor: pri $1.14 za mesiac to nechaj na Opusovi. Presnosť je tu drahšia
než tokeny.

## 5. Čo je otestované a čo nie

**Overené:**

- Zátvorky a fragmenty v `ui.tsx`, `OrderRowsModal.tsx` a `fx.rs`: rozdiel
  oproti netknutým zálohám 0 vo všetkých druhoch.
- `MultiSelect` používa len to, čo existuje: `IconCheck` (icons.tsx:151),
  `createPortal` (react-dom je už závislosť, nie nová), `.input`,
  `shadow-overlay`, `pop-in` - všetko overené v `index.css`.
- Import `MultiSelect` je v abecednom poradí v existujúcom bloku.
- `TICKET_RESTRICTIONS` typ sedí na `options` (pole `tone` navyše je pri
  priradení z premennej v poriadku).
- Kurzová služba otestovaná naživo, päť rôznych dotazov.
- Verzia 2.71.0 vo všetkých 9 výskytoch v 7 súboroch.

**Neoverené:**

- **Neskompilované** - node ani cargo tu nie sú, prvý kompilátor je CI.
- **Ten dropdown som nevidel bežať.** Je to nová interaktívna komponenta
  a React sa bez buildu spustiť nedá, takže som ju kontroloval čítaním.
  Pozri sa na ňu ako prvú vec po builde: či sa panel otvorí na správnom
  mieste, či ho scroll zatvorí a či sa text v zatvorenej bunke needoreže
  inak než trojbodkou.

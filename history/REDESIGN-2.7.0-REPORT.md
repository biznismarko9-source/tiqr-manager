# TIQR Manager 2.7.0 — AI Import Assistant

Report k tvojmu zadaniu: screenshot → Claude → štruktúrované dáta → review →
tvoje potvrdenie → **existujúci TIQR create flow** → databáza.

---

## ⚠️ Najprv dve veci, ktoré musíš vedieť hneď

### 1. Nespustil som build ani testy

Na Macu nie je Node ani Rust — vybral si "bez ničoho, len statická kontrola".
Tentokrát to znamená viac než minule: pribudol **nový Rust modul s 28
unit testami, ktoré nikdy nebežali**, a nové typy na oboch stranách IPC.

**Pred publikovaním tagu spusti na Windowse:**
```
npm install
npx tsc -b
npm run build
cargo check --lib
cargo test --lib
```
`Cargo.lock` a `package-lock.json` som verziu prepísal ručne — po prvom
`cargo check` a `npm install --package-lock-only` sa to zregeneruje.

### 2. Funkcia potrebuje `ANTHROPIC_API_KEY` secret

Ten istý GitHub Actions secret, ktorý od 2.0.63 poháňa automatickú detekciu
kategórií. Workflow ho už do oboch build ciest posiela — takže netreba nič
meniť, **ale ak si ten secret ešte nikdy nepridal, AI import nebude fungovať.**

Rozdiel oproti detekcii kategórií: tá bez kľúča ticho fungovala ďalej (free
keyword pravidlá). **AI import žiadny free fallback nemá** — panel poctivo
povie *"AI import isn't available in this build"*. Radšej jasná slepá ulička
než hádanie.

Pridáš to takto: Settings → Secrets and variables → Actions → New repository
secret, meno `ANTHROPIC_API_KEY`, hodnota kľúč z console.anthropic.com.

---

## 1. Kde je AI Import Assistant

V troch existujúcich formulároch, ako kompaktný panel nad poľami:

| Formulár | Kde presne |
|---|---|
| **New Event** | v `EventFormModal` — **len pri novom evente**, nie pri editácii (prepísať už zadaný event zo screenshotu je iná funkcia, ktorú si nepýtal) |
| **New Order** | v `OrderFormModal`, nad sekciou Event |
| **New Sale** | v `SaleFormModal`, **až na kroku Details** — nie na kroku výberu ticketov (prečo, nižšie) |

Žiadna nová stránka, žiadny nový route, žiadny tab, žiadny sidebar. Presne
podľa tvojho *"Nechcem veľký AI dashboard. Nechcem chat."*

Panel má 4 stavy: **Drop / Upload / Ctrl+V** → *Analyzing image...* →
**Review extracted data** → *(ty klikneš)* **Fill form**.

---

## 2. Ako funguje Ctrl+V

Listener na `document` (nie na paneli — aby si nemusel najprv klikať do neho).

Kľúčové správanie, a to najdôležitejšie je to negatívne:

- Ak clipboard **obsahuje obrázok** → zoberie ho, `preventDefault()`, ukáže
  náhľad, spustí analýzu.
- Ak clipboard **neobsahuje obrázok** → **nespraví vôbec nič**. Žiadny
  `preventDefault()`. Normálne vkladanie textu do ktoréhokoľvek políčka v celej
  aplikácii funguje presne ako predtým. Toto bolo tvoje explicitné *"nič
  nerozbíjaj"* a je to jediné, na čom by sa dal takýto listener naozaj pokaziť.
- Listener je aktívny len kým je panel na obrazovke, a je vypnutý počas
  prebiehajúcej analýzy (nedá sa teda zaradiť druhý request).

Číta aj `clipboardData.files`, aj `clipboardData.items` — jedno pokrýva
screenshot z OS clipboardu, druhé obrázok skopírovaný z prehliadača/inej appky,
a ani jedno nie je spoľahlivo nadmnožinou druhého.

Žiadna externá clipboard dependency. Čisté WebView API.

---

## 3. Upload a drag & drop

- **Upload**: skrytý `<input type="file">`, klik na drop zónu ho otvorí. Žiadny
  dialog plugin netreba.
- **Drag & drop**: obyčajné HTML drop eventy.

**Jedna vec, ktorú si treba zapamätať:** aby drag & drop vôbec fungoval, musel
som v `tauri.conf.json` nastaviť **`dragDropEnabled: false`** na hlavnom okne.
Tauri má default `true`, čo znamená, že drop zachytí OS-level handler a do
webview sa **žiadny HTML drop event nedostane** — ticho, bez chyby. Overil som,
že v celej appke nič nepoužíva Tauri vlastný drag-drop event, takže je to
bezpečné. Zapísané v `PROTECTED_AREAS.md` — ak drag & drop niekedy prestane
fungovať, toto je prvá vec na skontrolovanie.

---

## 4. Čo vie vytiahnuť pre **Event**

`name`, `eventDate`, `venue`, `city`, `country`, `category`, `status`

Kam to ide:
- názov / venue / city / country → priamo do polí
- dátum → do `<input type="date">`, **len ak je to naozaj `YYYY-MM-DD`** (o tom nižšie)
- kategória → **iba ak sa presne trafí do existujúcej kategórie**. AI nikdy
  nevytvorí novú lookup položku — to zostáva tvoje "+ New" tlačidlo.
- status → **iba ak je slovo doslova na obrázku** a je jedno z troch, ktoré
  appka má. AI status nikdy neurčuje úvahou — presne ako si žiadal.

---

## 5. Čo vie vytiahnuť pre **Order**

**Order-level**: `eventName`, `eventDate`, `venue`, `orderReference`,
`platform`, `purchaseDate`, `totalPrice`, `currency`

**Per-ticket (ticketGroups)**: `quantity`, `tier`, `section`, `row`,
`ticketType`, `unitPrice`, `fees`, `seats[]`

### Viacero ticket groups

Presne tvoj príklad funguje: Group 1 = 4 tickety, Tier 100, Section 102, Row 14,
Seats 21–24, €180 each; Group 2 = 2 tickety, Tier 200, Section 205, Row 8,
Seats 11–12, €120 each. **Vrátia sa oddelene a nikdy sa nezlievajú.**

Ale je tu vec, ktorú ti musím povedať rovno, lebo som ju nemohol vyriešiť bez
zmeny business logiky (a tú meniť nemám):

> **`OrderInput` v tejto appke nesie JEDNU section / row / tier / cenu pre celý
> order.** Takže dve rôzne skupiny = dva ordre. Nie je to obmedzenie AI, je to
> tvar, ktorý má objednávka v TIQR odjakživa.

Riešenie, ktoré som zvolil: panel ukáže **všetky skupiny**, ty si prepneš medzi
nimi číslami, vyplní sa tá vybraná — a explicitne ti napíše *"This screenshot
has 2 groups with different seating or prices. An order holds one, so fill this
group first, then create a second order for the next."* Žiadne ticho stratené
dáta, žiadny vymyslený multi-group order.

Seats idú do existujúceho `seatsRaw` poľa ako `21, 22, 23, 24` — to `parseSeats`
už vie spracovať. **Žiadne nové ticket creation pravidlá som nevytváral**,
tickety naďalej vytvára `insert_order_with_tickets` presne ako doteraz.

`orderReference` nemá vlastný stĺpec (order code generuje backend, ORD-000001),
tak ide do `notes` ako riadok `Order ref: X` — pripojí sa, nikdy neprepíše to,
čo tam už máš.

`totalPrice` **zámerne nikam nepočítam**. Nedelím ho počtom ticketov (to si
zakázal) — vidíš ho v review zozname a rozhodneš sa sám.

---

## 6. Čo vie vytiahnuť pre **Sale**

`eventName`, `orderReference`, `saleDate`, `quantity`, `salePrice`,
`sellingFees`, `currency`, `marketplace`, `buyerReference`, `paymentStatus`,
`deliveryStatus`

Kam to ide: sale date, cena, fees, currency, marketplace (match na existujúcu
platformu), buyer reference, payment status (len ak je doslova na obrázku a je
jeden z troch, ktoré appka má).

**Čo AI pri Sale zámerne NEROBÍ — a je to dôležité:**

Sale sa v TIQR vždy zapisuje **proti už existujúcemu ticketu**
(`SaleInput.ticketId`). Ktoré tickety si predal, sa vyberá z databázy v prvom
kroku formulára. Preto:

- panel je až na kroku **Details**, nie na výbere ticketov,
- v zozname polí pre `sale` **nie je section / row / seat / tier** — schválne.
  Keby tam boli, extrahovaný text zo screenshotu by nemal kam ísť a bola by to
  pozvánka pre niekoho, aby raz spravil "vytvor tickety zo screenshotu".

`eventName`, `orderReference` a `quantity` sa aj tak vytiahnu — vidíš ich v
review zozname ako kontrolu oproti tomu, čo si vybral.

---

## 7. Structured JSON schema

AI nevracia voľný text. Je držaná striktnou JSON schémou
(`output_config.format`), ktorá je pre každý kind iná:

```json
{
  "readable": true,
  "fields": [
    { "field": "section", "value": "402", "confidence": "high" },
    { "field": "row",     "value": null,  "confidence": "low"  }
  ],
  "ticketGroups": [
    {
      "quantity": "4", "tier": "100", "section": "102", "row": "14",
      "ticketType": "E-ticket", "unitPrice": "180", "fees": null,
      "seats": ["21","22","23","24"]
    }
  ]
}
```

Tri veci, ktoré túto schému robia bezpečnou:

1. **`field` je `enum`** — obsahuje len tie mená polí, ktoré ten konkrétny
   formulár naozaj má. Vymyslené meno poľa je odmietnuté už na API hranici.
2. **Každá hodnota je string alebo `null`.** Žiadne čísla, žiadne enumy, žiadne
   dátumové typy. Presne v tom tvare, v akom to formuláre držia — takže nikde
   nie je druhé miesto, kde by sa hodnota mohla ticho zaokrúhliť alebo
   preinterpretovať skôr, než ju uvidí existujúca validácia.
3. **`readable: false`** = obrázok je nečitateľný. Vtedy sa neukážu
   polovičné polia, ale tvoja vlastná hláška *"Image quality too low to
   reliably extract data."* + Retry.

A ešte jedna vrstva navyše, `sanitize_result` v Ruste:
- pole, ktoré formulár nepozná → **zahodené**
- duplicitné pole → **platí len prvé** (neskoršia protirečivá hodnota nevyhrá)
- prázdna / whitespace hodnota → **`null`** (nikdy neprepíše niečo, čo už máš napísané)
- confidence mimo troch úrovní → **`low`** (bezpečný smer zlyhania)
- úplne prázdna ticket group → zahodená; seat labely orezané a prázdne vyhodené

**Jediná výnimka z "kopíruj doslova": dátumy.** Model ich musí vrátiť ako
`YYYY-MM-DD`, lebo appka má `<input type="date">`, ktorý pri "14 Sep 2026"
ticho neukáže nič. Ale rok si vymýšľať nesmie — ak rok nie je na obrázku,
dátum je `null`. A frontend má ešte poistku (`isIsoDate`): ak by prišlo niečo
iné, do dátumového poľa sa to nezapíše, ale **ostane viditeľné a editovateľné
v review zozname**, takže sa nič nestratí.

---

## 8. Confidence a review flow

Po analýze sa **nič neuloží**. Ukáže sa zoznam:

```
Event name       [ Coldplay Munich        ]  ● high
Event date       [ 2026-09-14             ]  ● high
Tier             [ 100                    ]  ● high
Section          [ 102                    ]  ● high
Row              [ 14                     ]  ● high
Purchase price   [ 720                    ]  ● medium
City             [ not on the image       ]
```

- **každé pole sa dá prepísať** priamo tam
- **confidence** je farebná bodka + slovo: high zelená / medium jantárová /
  low červená
- **low confidence pole má navyše červený rámik** (`input-error`) — nedá sa
  prehliadnuť
- pole, ktoré na obrázku nebolo, má placeholder `not on the image` a ostane
  prázdne
- dole: **Fill form** / **Re-analyze** / **Discard**, plus text
  *"Nothing is saved until you submit the form"*

---

## 9. Ako sa to mapuje do existujúcich formulárov

Toto je celá integrácia — nič viac:

```
AI panel  →  onApply({ fields, group })  →  setEventId(...) / setQuantity(...) / setForm(...)
          →  ty si to pozrieš  →  existujúce tlačidlo Create
```

Panel **nemá prístup** k `api.createEvent` / `createOrder` / `createSaleBatch`.
Jediné, čo vie, je zavolať `onApply`. Formuláre sú inak nedotknuté —
validácia, Create tlačidlo aj všetky backend commandy sú presne tie isté ako
pred touto verziou.

**Validácia sa neobchádza.** AI hodnoty prejdú tou istou validáciou ako čokoľvek,
čo napíšeš ručne — lebo doslova skončia v tých istých `useState` hodnotách, z
ktorých formulár skladá `OrderInput` / `EventInput` / `SaleBatchInput`. Neplatný
dátum, nečíslo, chýbajúce povinné pole → zachytí to tá istá kontrola ako vždy.

---

## 10. Ako sa šetrí Claude API usage

Vynucované na **oboch** stranách, nielen vo frontende:

**Frontend (`AiImportSession`):**
- 1 obrázok = 1 request
- **fingerprint (FNV-1a) každého obrázka** — ten istý screenshot sa v rámci
  jednej session **nikdy neanalyzuje druhýkrát**. Pretiahneš ten istý obrázok
  znova? Zadarmo. Zavrieš a otvoríš review? Zadarmo.
- **úprava vytiahnutých polí NIKDY nevolá AI znova** — je to čistý lokálny state
- Retry len po kliknutí (a ten cache zámerne obchádza — to je celý zmysel "skús to znova")
- žiadny request pri otvorení, žiadny timer, žiadny background pass

**Backend (`ai_import.rs`):**
- presne **jedno** volanie na invokáciu
- maximálne **jeden** retry, a len pri prechodnom statuse (429 / timeout / 5xx) —
  zlý kľúč alebo zlý request sa neopakuje, to by len dvakrát zaplatilo za tú
  istú odpoveď
- pevný `max_tokens` strop, žiadny interný loop
- obrázok sa pred odoslaním zmenší na max 1568px dlhšiu hranu — **ale len ak
  treba**; bežný screenshot ide bajt na bajt, lebo každé re-encode ubere z
  čitateľnosti malého textu

**Model:** `claude-opus-5`. Vedome som **nepoužil** Haiku, ktoré používa
`ai_categorize.rs` — to je správna voľba pre jednoslovnú klasifikáciu, nie na
vyťahovanie dvanástich presných hodnôt z obrázka. Zle prečítaný seat range alebo
cena je horšia než žiadna extrakcia, lebo si to v review nemusíš všimnúť. Cenovú
páku som miesto toho stiahol cez `effort: "medium"` (default API je `high`).
**Obe sú jediné dve konštanty na vrchu `ai_import.rs`** — ak chceš ísť lacnejšie,
je to zmena dvoch riadkov, povedz a spravím to.

---

## 11. Zmenené súbory

**Nové (3):**
- `src-tauri/src/commands/ai_import.rs` — celý backend + 28 testov
- `src/components/AiImportPanel.tsx` — zdieľaný panel
- `src/lib/aiImport.ts` — image handling, clipboard/drop, fingerprint, guardy

**Upravené (10):**
- `src-tauri/src/commands/mod.rs` — +1 riadok (`pub mod ai_import;`)
- `src-tauri/src/lib.rs` — +1 registrovaný command
- `src-tauri/src/ai_categorize.rs` — **jedno slovo**: `fn` → `pub(crate) fn`
  pri `is_retriable_anthropic_status`, aby sa retry politika zdieľala namiesto
  druhej kópie. Nič iné.
- `src-tauri/tauri.conf.json` — +1 riadok (`dragDropEnabled: false`)
- `src/lib/types.ts` — AI import typy (append)
- `src/lib/api.ts` — +1 IPC volanie
- `src/pages/Events.tsx`, `src/pages/Orders.tsx`, `src/pages/Sales.tsx` — panel + fill funkcia
- `.github/workflows/build-windows.yml` — **len komentár** (že ten secret teraz
  gatuje aj túto funkciu)

**Verzia (7 súborov, 9 výskytov):** package.json, tauri.conf.json, Cargo.toml,
release.ps1 (`$Version` + `$CommitMsg`), 1-CLICK-UPDATE.bat (title + echo),
Cargo.lock, package-lock.json

**Dokumentácia (4):** CURRENT_STATE.md, PROTECTED_AREAS.md, CHANGELOG.md, tento report

---

## 12. DB / migration zmeny

**Žiadne.**

- žiadna nová migrácia — ďalšia nová je stále **027**
- žiadna nová tabuľka, žiadny nový stĺpec, žiadny index
- žiadna nová dependency (ani Rust, ani npm) — `reqwest`, `serde`, `serde_json`
  už v projekte boli kvôli `ai_categorize.rs`

A najdôležitejšie: **nový backend modul nemá k databáze prístup vôbec.**
`analyze_import_image` neberie `AppState`, neotvára `Connection` a nemá žiadnu
zapisovaciu cestu. Nie je to náhoda aktuálnej implementácie — je to spôsob, akým
je tvoje pravidlo vynútené, a je to zapísané v `PROTECTED_AREAS.md`.

---

## 13. Testy

### Čo som napísal (28 Rust unit testov, `cargo test --lib`)

Pokrývajú tvoj zoznam zo sekcie 17 — všetko čisté funkcie, **žiadny live API call**:

| Tvoja požiadavka | Test |
|---|---|
| invalid file type | `png_jpeg_and_webp_are_accepted_and_everything_else_is_not` |
| too large image | `an_image_over_the_size_ceiling_is_refused_without_a_request` |
| empty clipboard / prázdny payload | `an_empty_payload_is_refused` |
| structured JSON parsing | `a_well_formed_ai_response_round_trips_through_the_camel_case_wire_shape` |
| missing fields → null | `a_missing_value_stays_null_and_is_never_filled_in` |
| confidence values | `the_three_documented_confidence_levels_survive_unchanged`, `an_unrecognized_confidence_degrades_to_low_never_to_high` |
| multiple ticket groups | `two_ticket_groups_are_kept_separate_and_never_merged` |
| multiple seats | `blank_seat_labels_are_dropped_and_the_rest_are_trimmed` |
| malformed AI response | `a_malformed_ai_response_does_not_deserialize_into_a_result` |
| duplicate prevention / sanitizácia | `a_field_the_form_has_no_slot_for_is_dropped`, `a_duplicated_field_keeps_only_the_first_value` |
| request shape | `the_request_body_has_the_exact_shape_this_model_requires` |
| schema correctness | `every_schema_object_is_closed_and_fully_required`, `the_schema_only_offers_the_field_names_this_kind_actually_has` |
| no-guessing pravidlá | `the_prompt_states_the_no_guessing_rules_and_lists_every_field`, `the_event_prompt_forbids_deciding_a_status`, `the_prompt_asks_for_iso_dates_and_forbids_assuming_a_year` |
| sale nemá seat polia | `a_sale_never_asks_for_seat_or_ticket_fields` |

### Čo som NEMOHOL spustiť

`cargo test --lib`, `cargo check --lib`, `npx tsc -b`, `npm run build` — **ani
jedno**. Tých 28 testov nikdy nebežalo.

### Čo som overil staticky

| Kontrola | Výsledok |
|---|---|
| Zátvorky v `ai_import.rs` (po odstránení stringov/komentárov) | `{}` 0, `()` 0, `[]` 0 |
| JSX štruktúra Events/Orders/Sales vs. predchádzajúca verzia | **identická** (moje vloženia sú self-closing komponenty) |
| Balance drift vo všetkých zmenených TS/TSX súboroch | **žiadny** |
| Importy v nových súboroch | **všetky** sa rozlišujú na reálny export |
| Nepoužité importy | **0** |
| Použité ikony existujú | ✓ |
| `toast` / `events` / `platforms` / `categories` / `ticketTypeOptions` v scope | ✓ overené v každom z troch modalov |
| `ANTHROPIC_API_KEY` v CI | ✓ posiela sa do oboch build ciest |
| Tauri drag-drop event použitý inde | **nikde** — `dragDropEnabled: false` je bezpečné |

Tri reálne chyby som pritom našiel a opravil: `nullable_string` ako jedna
`Value` použitá 8× v `json!` (move error), `field-invalid` použitá priamo na
`<input>` namiesto `input-error` (wrapper vs. priama trieda), a
`Array.from(FileList)` nahradené indexovaním (array-LIKE, nie spoľahlivo
iterovateľné naprieč DOM lib verziami).

### Čo skontroluj po builde očami

1. Ctrl+V so screenshotom v schránke → analýza sa spustí
2. Ctrl+V s **textom** v schránke, kurzor v poli → text sa vloží normálne
3. Drag & drop obrázka na panel
4. Screenshot s dvoma ticket skupinami → prepínanie 1/2 a hláška
5. Nečitateľný/rozmazaný obrázok → *"Image quality too low..."*
6. Ten istý obrázok dvakrát → druhýkrát okamžite, bez requestu
7. Úprava poľa v review → žiadny nový request

---

## 14. Limity

- **Model si musí byť istý formátom dátumu.** Vynútil som `YYYY-MM-DD` v
  prompte + poistku vo frontende. Ak by dátum aj tak prišiel v inom tvare,
  neprepíše sa do date poľa, ale zostane v review zozname na ručné doplnenie.
- **`totalPrice` sa nikam nemapuje.** Vidíš ho, ale nedelím ho počtom ticketov.
- **Sale neidentifikuje tickety.** Zámerne — pozri sekciu 6.
- **Dve ticket groups = dva ordre.** Nie je to obmedzenie AI, ale tvar
  `OrderInput`. Nemenil som ho.
- **Kategórie/platformy sa len párujú, nevytvárajú.** Ak platforma zo
  screenshotu v appke neexistuje, pole ostane prázdne.
- **Cena requestu**: `claude-opus-5` s `effort: "medium"` — jeden screenshot
  rádovo jednotky centov. Ak chceš lacnejšie, sú to dve konštanty.
- **Bez buildu.** Toto je najväčší limit tejto verzie.

---

## 15. Čo som zámerne NEZMENIL

refund/resell · `batch_id` · money/integer cents · Orders core logic · Tickets
core logic · Sales core logic · Listings · Finance · Fulfillment · Attention ·
Calendar · Google Sheets · **Price Checker** (žiadny monitoring, žiadny auto
scan, žiadny repricing) · schéma a migrácie · existujúce create commandy ·
existujúca validácia · ticket creation pravidlá.

Section / Row / Seat nie sú pricing factor a nikde ich tak nepoužívam. Tier /
Level je samostatný údaj, presne ako doteraz. AI nerobí žiadne pricing
odporúčania.

Žiadne ďalšie AI features, žiadny redesign, žiadne automatizácie.

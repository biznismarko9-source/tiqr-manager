# TIQR Manager 2.8.0 — Calendar redesign + advanced calendar workflow

Report k tvojmu zadaniu. Najprv som spravil presne to, čo si žiadal: **zistil
som, čo Calendar reálne podporuje a aké dátumy v TIQR naozaj existujú** — až
potom som čokoľvek písal.

---

## ⚠️ Dve veci hneď na začiatku

1. **Build som nespustil.** Na Macu nie je Node ani Rust. `cargo test --lib`,
   `cargo check --lib`, `npx tsc -b`, `npm run build` — ani jedno. Pribudlo
   9 nových Rust testov, ktoré nikdy nebežali.
2. **Ak si 2.7.0 ešte nepublikoval** (release script vtedy spadol na mojej
   chybe s úvodzovkami), tento balík obsahuje **aj celý AI Import Assistant**.
   2.8.0 je kumulatívne.

Pred publikovaním tagu:
```
npm install
npx tsc -b
npm run build
cargo check --lib
cargo test --lib
```

---

## 1. Nové Calendar views

| View | Rozsah, ktorý načíta | Layout |
|---|---|---|
| **Month** | 6-týždňová mriežka okolo mesiaca | 7 stĺpcov, chips v bunkách, `+X more` |
| **Week** | 7 dní | tie isté bunky, vyššie (viac chips na deň) |
| **Day** | 1 deň | zoznam zoskupený **podľa typu** |
| **Agenda** | dnes → +45 dní | chronologický zoznam zoskupený **podľa dňa** |

Všetky štyri idú cez **ten istý** `get_calendar(dateFrom, dateTo)` — líši sa
len okno a layout. Zdieľajú jeden filter state aj jeden search box.

**Week nemá časovú os — schválne.** Vysvetlenie v sekcii 3.

Agenda je fixné okno dopredu od dnes, takže prev/next je pri nej **skrytý**
(nie disabled) — posúvať niečo, čo je definované ako "od dnes", nedáva zmysel.

---

## 2. Podporované event types

| Typ | Existuje? | Zdroj dátumu |
|---|---|---|
| **Events** | ✅ | `events.event_date` |
| **Orders** | ✅ | `orders.purchase_date` |
| **Sales** | ✅ | `sales.sale_date` (zoskupené cez `GROUP_KEY_EXPR`) |
| **Finance** | ✅ **NOVÉ** | `finance_entries.entry_date` |
| **Recurring** | ✅ **NOVÉ** | `recurring_expenses.next_date` |
| **Pulls** | ✅ | `pulls.event_date` |
| **Attention** | ✅ | `AttentionCenterItem.event_date` |
| **Payouts** | ❌ | neexistuje |
| **Payments** | ❌ | neexistuje |
| **Fulfillment** | ❌ | neexistuje |

### Prečo payouts / payments / fulfillment stále nie sú

Pýtal si sa na ne znova, tak som to **znova overil proti živej schéme a
command kódu** (nie proti starej poznámke z 2.5.0):

- **Payout** — v celej appke nie je žiadna payout entita, tabuľka ani dátum.
  "Payout" existuje iba ako text v hlavičke Google Sheets stĺpca
  (`Payout Per Ticket`, `Payout status`), ktorý 1:1 aliasuje
  `sales.sale_price_cents` / `sales.payment_status`.
- **Payments** — tabuľka `payments` z migrácie 007 v schéme je, ale **stále má
  nula živých SQL dotazov v ktoromkoľvek command module**. Prevereril som to
  znova: `grep` cez všetky `commands/*.rs` nenašiel ani jeden `FROM payments`,
  `INTO payments`, `UPDATE payments`. Je to mŕtva schéma, nie dáta.
- **Fulfillment** — `tickets.delivery_status` je voľný text
  ("Delivered"/"Not delivered"), a **žiadny dátumový stĺpec k nemu neexistuje**.
  Ani `delivery_date`, ani `delivered_at`, ani `delivery_deadline`.

Namiesto vymýšľania som pridal **test, ktorý to stráži**:
`payouts_payments_and_fulfillment_are_still_absent_from_every_response`. Ak by
niekedy niektorý z nich vznikol naozaj, test spadne a bude sa to riešiť
vedome, nie ticho.

### Čo som našiel a čo 2.5.0 prehliadlo: **Finance**

Pôvodný výskum v 2.5.0 preveroval tvoje tri kategórie proti
Orders/Sales/Tickets — a **nikdy sa nespýtal, či má dátumy Finance modul**.

Má. A sú to reálne, používateľom zadané dátumy so živým kódom za sebou:

- **`finance_entries.entry_date`** — dátum na reálnom riadku, ktorý zapisuje
  `finance_entries.rs`. Ten istý dátum, podľa ktorého listuje Finance →
  Transactions.
- **`recurring_expenses.next_date`** — **jediný skutočne dopredu hľadiaci
  termín v celej appke.**

Pridal som ich pod **vlastnými poctivými názvami** (`Finance`, `Recurring`) —
**nie** ako prebrandovaný "payout"/"payment". To by bolo presne to vymýšľanie,
ktorému sa má táto funkcia vyhýbať.

---

## 3. Odkiaľ pochádzajú jednotlivé dátumy

Všetko sú **existujúce stĺpce**, žiadny výpočet, žiadny odhad.

Dôležitý detail, ktorý ostal nezmenený: **`pulls.transfer_deadline` sa
nepoužíva.** Je to deprecated stĺpec (od 1.9.8 ho nahradilo client-side
varovanie "N dní pred eventom" počítané z `event_date`) — a je to presne ten
typ pasce, pred ktorou tvoje "nevymýšľaj dátumy" varuje. Pull entries naďalej
sedia na `pulls.event_date`.

**Žiadna časová os**, lebo **žiadny z týchto dátumov nemá čas**. Všetky sú
date-only `YYYY-MM-DD`. Tvoje vlastné zadanie hovorilo *"ak existujúce dáta
nemajú čas, nepředstieraj presné časové pozície"* — takže Week je 7 denných
stĺpcov, nie 24-hodinový rozvrh, a Day zoskupuje podľa typu, nie podľa hodiny.

**Dátumy zostávajú stringy od začiatku do konca.** Porovnávanie, bucketovanie
aj umiestnenie do bunky sa robí na ISO texte; jediné `new Date(...)` v
`Calendar.tsx` sú na geometriu mriežky a na ľudské popisky. Preto sa žiadna
položka nemôže posunúť o deň kvôli UTC/timezone konverzii.

---

## 4. Filter systém

Chips: **Events · Orders · Sales · Finance · Recurring · Pulls · Attention**

- Chip sa zobrazí **iba pre typ, ktorý je reálne v načítanom rozsahu**
  (tvoje "nepridávaj prázdny filter"). Výnimka: typ, ktorý si sám vypol,
  ostáva viditeľný — inak by sa nedal zapnúť späť.
- Filtre fungujú **vo všetkých štyroch view**.
- Chip row je zároveň **farebná legenda** — bodka na chipe je presne tá farba,
  ktorú majú jeho položky v mriežke. Žiadna samostatná legenda netreba.
- "Reset" sa objaví, len keď je niečo vypnuté.

---

## 5. Day Detail

Klik na číslo dňa (alebo na `+X more`) otvorí modal:

**Vľavo:** všetky položky dňa ako klikateľné riadky — farebná bodka podľa
typu, ikona, názov, typ, doplnkový riadok, countdown pri eventoch, suma vpravo.

**Vpravo:** **Day summary** — počet položiek podľa typu + Total.

Summary je odvodený z **toho istého poľa `entries`**, ktoré vykresľuje zoznam
— žiadny druhý fetch, žiadne druhé pravidlo.

---

## 6. Summary / Upcoming / Overdue

Strip nad kalendárom: **Today · Tomorrow · Next 7 days · Overdue**

Jeden extra `get_calendar` call s vlastným fixným rozsahom — **ten istý
command**, takže to nikdy nie je duplicitná business logika (tvoja explicitná
požiadavka).

### Overdue je reálne, a to z jedného konkrétneho dôvodu

`recurring_expenses.next_date` je **jediný skutočný termín v appke**. Pravidlo
je **presne to isté**, ktoré už používa Finance → Accounts:

```
is_active AND next_date < today
```

Pausnuté šablóny sú **vylúčené** — ich `next_date` je zmrazený a zámerne nie
je actionable, kým ich nezapneš (je to popísané priamo v komentári migrácie aj
v `finance_recurring.rs`). Ukázať pausnutú položku ako overdue by bolo
ukázanie termínu, ktorý neexistuje.

**Overdue dlaždica sa nezobrazí vôbec, keď je počet 0** — inak by kalendár
naznačoval, že appka sleduje viac termínov než reálne sleduje.

Lookback je 90 dní. Nie náhodné číslo: `next_date` sa posúva po jednej
perióde, takže čokoľvek reálne overdue je nanajvýš pár cyklov dozadu — 90 dní
pokryje týždenné aj mesačné šablóny bez toho, aby sa z toho stalo čítanie
celej histórie.

---

## 7. Navigation

Všetko existujúce routes, žiadny nový:

| Klik na | Ide na |
|---|---|
| Event | `/events/:id` (Event Workspace) |
| Order | `/orders/:id` |
| Sale | `/sales/:id` |
| Finance / Recurring | `/finance` |
| Pull | `/pulls` |
| Attention | `/orders/:id` alebo `/events/:id` podľa toho, čo item má |

Finance a Pulls nemajú per-record detail route (Finance je jedna stránka s
client-side tabmi), takže ich položky nesú `linkId: null` a otvárajú stránku
samotnú — rovnaký vzorec, aký Pulls používa od 2.5.0.

---

## 8. Search

**Calendar-local, nie globálny** (tvoja sekcia 15).

- Hľadá v `title` + `subtitle` toho, čo je **už načítané pre aktuálny rozsah**.
  Nerobí vlastný dotaz.
- **V mriežke (Month/Week)**: nezhody sa **stlmia** (opacity), nezmiznú — takže
  vidíš, *kde* zhody sedia voči zvyšku.
- **V zoznamoch (Day/Agenda)**: reálne filtruje.
- Ukáže počet zhôd a **"Jump to \<dátum\>"**, ktorý preskočí na Day view
  prvej zhody.

---

## 9. Redesign zmeny

Všetko cez existujúcu shared UI vrstvu z 2.6.0 — **žiadny nový dizajnový
systém**:

- `SEGMENTED_TRACK` / `segmentedItemClass` pre view switch aj prev/Today/next
- `Card`, `EmptyState`, `Input`, `Modal`, `PageHeader`, `LoadingBlock`
- `.section-title` na hlavičky sekcií
- KPI dlaždice v stripe majú **rovnaké metriky ako `StatCard`** (p-3.5, 22px)

Ďalej:
- vyššie riadky mesiaca (104px) aj týždňa (240px)
- dnešná bunka má ring + tint, dnešné číslo brand kruh
- víkendy a dni mimo mesiaca majú vlastné jemné odtiene
- **workload bar** — 3 segmenty v **jednej tlmenej farbe**, signál nesie
  *koľko je plných*, nie akú majú farbu. Presne tvoje *"NECHCEM farebný
  chaos... jemný vizuálny signál"*.
- **countdown** pri eventoch: Today / Tomorrow / In N days — a **nikdy** "3 dni
  dozadu", lebo minulý event nie je zmeškaný termín.
- light aj dark prechádzajú cez tie isté tokeny ako zvyšok appky

---

## 10. Performance approach

- **Jeden request na jeden rozsah.** Prepnutie mesiaca/týždňa/dňa načíta iba
  nové okno. Žiadne view nenačíta viac, než vykresľuje.
- Backend `get_calendar` už bol range-based od 2.5.0 — **nemusel som robiť
  žiadny nový backend systém**, len ho správne použiť.
- Filtre a search bežia **client-side nad už načítanými dátami** — nulové
  ďalšie dotazy.
- Strip je jeden extra call s vlastným fixným rozsahom.
- **Žiadny nový index.** Každý dátumový stĺpec, ktorý nové dotazy čítajú, buď
  index má, alebo ho nemá ani jeho vlastný existujúci list (rovnaké
  odôvodnenie ako pri `pulls.event_date` v 2.5.0).
- Mriežka aj zoznamy **scrollujú vo vnútri seba**, nie stránka — header,
  filtre a strip zostávajú na mieste a je vždy iba jeden scrollbar.

---

## 11. Zmenené súbory

**Backend (3):**
- `src-tauri/src/commands/calendar.rs` — 2 nové range funkcie
  (`finance_in_range`, `recurring_in_range`) + 9 nových testov
- `src-tauri/src/models.rs` — **iba doc komentáre** (`kind` a `link_kind`
  teraz menujú nové hodnoty)
- žiadny iný Rust súbor sa nedotkol

**Frontend (2):**
- `src/pages/Calendar.tsx` — prepísaný (563 → 962 riadkov)
- `src/lib/types.ts` — `CalendarEntryKind` + `CalendarLinkKind` rozšírené

**Verzia (7 súborov, 9 výskytov)** + **dokumentácia (4)**:
`CURRENT_STATE.md`, `PROTECTED_AREAS.md`, `CHANGELOG.md`, tento report.

---

## 12. DB / migration changes

**Žiadne.**

- žiadna nová migrácia — ďalšia nová je stále **027**
- žiadna nová tabuľka, žiadny nový stĺpec, **žiadny nový index**
- žiadna nová dependency
- `calendar.rs` zostáva **read-only agregátor bez zapisovacej cesty** — presne
  ako Attention Center a Inventory Intelligence

---

## 13. Testy

### 9 nových Rust testov (23 v `calendar.rs` celkovo)

| Tvoja požiadavka | Test |
|---|---|
| correct date placement | `a_finance_entry_lands_on_its_own_entry_date_with_its_own_amount` |
| month/week/day range | `a_finance_entry_outside_the_requested_range_is_not_returned` |
| overdue | `a_recurring_due_date_in_the_past_is_critical_and_one_in_the_future_is_not` |
| overdue (edge) | `a_paused_recurring_template_never_appears_at_all` |
| multiple event types same day | `several_kinds_on_one_day_all_survive_and_stay_sorted_by_date` |
| agenda ordering | tá istá — overuje date-ascending baseline |
| timezone/date-only | `a_date_only_value_is_returned_exactly_as_stored_with_no_timezone_shift` |
| data accuracy | `payouts_payments_and_fulfillment_are_still_absent_from_every_response` |
| — | `every_entry_key_is_unique_within_one_response` |
| — | `a_finance_entry_with_no_category_or_place_falls_back_to_its_type` |

Plus 14 existujúcich testov z 2.5.0, ktoré stále platia.

### Čo som NEMOHOL spustiť

`cargo test --lib`, `cargo check --lib`, `npx tsc -b`, `npm run build`.

### Čo som overil staticky

| Kontrola | Výsledok |
|---|---|
| `Calendar.tsx` zátvorky | `{}` 0, `()` 0, `[]` 0 |
| `Calendar.tsx` importy | všetky sa rozlišujú na reálny export |
| nepoužité importy / mŕtve symboly | **0** |
| `calendar.rs` zátvorky (bez stringov/komentárov) | `{}` 0, `()` 0, `[]` 0 |
| všetkých 23 `#[test]` naviazaných na `fn` | ✓ |
| stĺpce v nových SQL dotazoch | overené proti reálnemu DDL |
| CHECK constrainty vs. test seedy | overené (`entry_type`, `scope`, `amount_cents > 0`, `frequency`) |
| `payments` má živý SQL kód? | **nie**, znova overené |
| `$CommitMsg` bez úvodzoviek | ✓ (guard z 2.7.0 by inak zastavil release) |

**Jednu reálnu chybu som pritom našiel a opravil:** pri vkladaní nových test
helperov sa `#[test]` atribút odtrhol od svojho testu a "prilepil" na
`seed_finance_entry` — to by bola compile error (test funkcia s argumentmi) a
zároveň by jeden existujúci test ticho prestal bežať. Odvtedy kontrolujem, či
je každý `#[test]` naozaj nasledovaný `fn`.

### Čo skontroluj očami po builde

1. Prepínanie Month/Week/Day/Agenda + prev/Today/next v každom z nich
2. Filter chips — či sa nezobrazujú prázdne typy
3. Search: stlmenie v mriežke, filtrovanie v zozname, "Jump to"
4. Day Detail: ľavý zoznam + pravý summary
5. Overdue dlaždica — objaví sa iba ak máš reálne prešvihnutú recurring položku
6. Light aj dark
7. Či niekde nie sú dva scrollbary

---

## 14. Čo sa NEIMPLEMENTOVALO a prečo

- **Payouts / Payments / Fulfillment** — v schéme neexistuje dátum, na ktorom
  by mohli stáť. Detaily v sekcii 2. Namiesto vymýšľania je tam test.
- **Časová os vo Week view** — žiadny dátum v appke nemá čas. Predstierať
  hodinové pozície bolo v tvojom zadaní explicitne zakázané.
- **Quick add (task/reminder/note)** — v celej schéme **neexistuje žiadna
  task, note ani reminder tabuľka** (overené). Tvoja sekcia 13 hovorila, že v
  takom prípade nemám stavať nový task database systém, tak som to vynechal.
  Ak to niekedy budeš chcieť, potrebuje to vlastný design pass, nie stĺpec
  prilepený ku kalendáru.
- **Per-pull a per-finance-entry detail** — tie routes neexistujú. Položky
  otvárajú `/pulls` resp. `/finance`, rovnako ako doteraz.
- **Globálny search** — tvoja sekcia 15 to explicitne vylučovala.

---

## Čo sa zámerne NEZMENILO

refund/resell · `batch_id` · money/integer cents · Orders core · Tickets core ·
Sales core · Listings · Finance business logic · Fulfillment · Attention ·
Price Checker · Google Sheets · schéma a migrácie · existujúce routes.

Calendar je aj naďalej **iba aggregation/read view** — `calendar.rs` nemá a
nedostal žiadnu zapisovaciu cestu.

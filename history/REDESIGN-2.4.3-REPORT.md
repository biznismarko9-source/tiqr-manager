# TIQR Manager 2.4.3 — Ticket Control Center

Krátky report k novej funkcii "Ticket Control Center" — jedna centrálna
pracovná obrazovka (`/control-center`) na správu a kontrolu ticketov
naprieč všetkými eventmi naraz, postavená celá nad existujúcimi
tickets/listings/sales dátami. Žiadny nový paralelný ticket systém.

## 1. Zmenené súbory

**Nové:**
- `src-tauri/src/commands/ticket_control_center.rs` — nový backend modul
  (1 read-only query + `#[tauri::command]` wrapper).
- `src/pages/TicketControlCenter.tsx` — nová frontend stránka.

**Upravené (backend):**
- `src-tauri/src/models.rs` — nové structs `ControlCenterFilters` /
  `ControlCenterTicket`; `BulkTicketField` enum má nový variant `Tier`.
- `src-tauri/src/commands/tickets.rs` — `LIST_CAP` z `private` na
  `pub(crate)`; `bulk_update_tickets_impl` vie nastaviť aj `tier`; 1 nový
  test.
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` — registrácia
  nového modulu/commandu.

**Upravené (frontend):**
- `src/lib/types.ts` — `BulkTicketField` type + nové `ControlCenterFilters`
  / `ControlCenterTicket` typy (zrkadlia Rust structy).
- `src/lib/api.ts` — nový `listControlCenterTickets` wrapper.
- `src/components/BulkTicketEditBar.tsx` — nová možnosť "Tier / Level" v
  bulk edit bare (zdieľaná komponenta, takže to isté pribudlo aj do Sale
  Detail/Order Detail).
- `src/components/icons.tsx` — nová ikonka `IconLayoutGrid` (nav).
- `src/App.tsx` — nová route `/control-center`.
- `src/components/Layout.tsx` — nová položka "Control Center" v navigácii,
  hneď za Tickets.

**Dokumentácia + verzia (2.4.2 → 2.4.3):** `package.json`,
`src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `release.ps1`,
`1-CLICK-UPDATE.bat`, `Cargo.lock` / `package-lock.json` (regenerované cez
`cargo check` / `npm install --package-lock-only`, nie ručne),
`PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/PROTECTED_AREAS.md` (nová
sekcia "2.4.3"), `CHANGELOG.md`.

Nič v Listings/Sales/Finance/Orders core, refund/resell, `batch_id`, ani v
money/cents stĺpcoch sa nemenilo. Žiadna nová DB migrácia.

## 2. Čo pribudlo

- **`/control-center` stránka** — jedna tabuľka nad všetkými ticketmi zo
  všetkých eventov naraz, so zobrazením presne podľa zadania: Event +
  dátum (spolu v jednej bunke), Order, Ticket/Seats, Tier, Purchase price,
  Listing price, Listing status (+ marketplace ako subtext), Sale status,
  Payment status, Delivery status, Overall status.
- **Filtre** (sticky nad tabuľkou): Event, Date range, Tier, Section, Row,
  Ticket status, Listing status, Sale status, Payment status, Delivery
  status, Marketplace.
- **8 Quick filters**: All / Unsold / Unlisted / Listed / Sold / Pending
  payment / Pending delivery / Refunded.
- **Search** (1 pole): ticket code, order, event, section, row,
  marketplace, listing ID/URL.
- **Bulk akcie** cez existujúce mechanizmy (bod 3): zmena
  Section/Row/**Tier**/Seat/Listing price, zmena listing statusu, export
  vybraných do CSV. Žiadny bulk refund/resell.
- **Klik na riadok** otvorí existujúci Sale Detail alebo Order Detail
  (podľa toho, čo je k dispozícii).
- **Nový read-only signál `isRefunded`** — refundovaný-a-ešte-neprepredaný
  ticket sa dá teraz odfiltrovať cez Quick Filter "Refunded"; predtým sa
  nedal odlíšiť od nikdy-nepredaného ticketu (obidva majú
  `sale_payment_status = null`). Iba číta, nič nezapisuje, refund/resell
  logiky sa to netýka.
- Tabuľka má **vlastný scroll**, filtre aj hlavička ostávajú sticky, žiadny
  zbytočný scroll celej stránky — prvá takáto tabuľka v appke.

## 3. Použité existujúce logiky

Nič sa nevymýšľalo nanovo — Control Center je jeden nový SQL view-query nad
existujúcimi tabuľkami plus tenká UI vrstva:

- **Join shape** je ten istý guarded join na aktívny sale, aký už používa
  `tickets::BASE_SQL` (`sa.payment_status != 'refunded'`, migrácia
  004/BUG#1) — nedotknutý, iba zopakovaný v novom module.
- **`ticket_listings` fan-out** (1 riadok na marketplace listing) je ten
  istý vzor, aký už používa
  `ticket_listings::list_ticket_listings_for_event_impl`.
- **Bulk update** ide cez existujúci `bulk_update_tickets_impl` /
  `BulkTicketEditBar` — pribudol iba nový stĺpec (Tier), samotný
  all-or-nothing transaction mechanizmus je nezmenený.
- **Bulk listing status** volá existujúci `bulk_update_ticket_listings_status`
  priamo, bez zmeny.
- **Export selected** volá existujúci `export_tickets_csv_selected` priamo
  (rovnaký dialog-flow ako `ExportPickerModal`) — `csv_export.rs` sa vôbec
  nemenil.
- **`LIST_CAP = 5000`** — rovnaký safety cap ako `tickets.rs`, teraz
  zdieľaný, nie duplikovaný.
- **`useNarrowTables()` + `CHECKBOX_CLASS`** — rovnaký responzívny systém
  a checkbox štýl ako zvyšok appky.

## 4. Testy

```
cargo test --lib   -> 1038 passed, 0 failed, 3 ignored
                      (+12 nových oproti 2.4.2: 11 v ticket_control_center.rs
                       + 1 bulk tier test v tickets.rs)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.4.3 build")
```

11 nových backend testov pokrýva: ticket bez listingu sa zobrazí presne
raz; ticket s viacerými listingmi sa fan-outne správne bez straty
unlisted ticketov; refundovaný+nanovo predaný ticket sa zobrazí presne raz;
refundovaný-a-neprepredaný ticket je označený s prázdnym payment statusom;
`refundedOnly` filter; filter podľa event id; filter podľa comma-list
ticket statusov; filter podľa marketplace + listing status; search cez
marketplace name aj listing id; date range; tier substring match. Plus 1
nový test na bulk tier update.

Regresie: zvyšných 1026 pôvodných testov (Orders/Tickets/Sales/Listings/
Finance/Inventory/refund/resell atď.) prešlo bez zmeny správania — nič
mimo tohto nového modulu sa nedotklo.

Nebol k dispozícii žiadny prehliadač/Playwright na vizuálne overenie
layoutu (sticky filtre, vlastný scroll tabuľky) — overené iba cez
`tsc -b` / `npm run build` / code review.

## 5. Limity

- **Sticky/scroll layout (`68vh`) je neodmeraná konštanta** — funkčne
  správne poskladané (sticky-per-`<th>` vzor, ktorý appka nikde predtým
  nemala), ale bez vizuálnej kontroly v prehliadači. Ak to bude pritesné
  alebo priveľké, je to jednoriadková zmena.
- **Stĺpce zredukované z tvojho 12-položkového zoznamu kvôli hustote**:
  Event name+date sú jedna bunka (nie dva stĺpce); Marketplace nemá vlastný
  stĺpec — je to subtext v Listing status bunke (inak by dva riadky toho
  istého ticketu na dvoch marketplacoch vyzerali identicky). V úzkom okne
  (< 1649px) sa navyše skrývajú Order/Tier/Purchase price stĺpce — rovnaký
  breakpoint ako zvyšok appky, ale výber "ktoré 3 stĺpce sa skryjú" je môj
  odhad, nie meraný proti reálnemu obsahu.
- **Payment status filter dropdown nemá možnosť "Refunded"** — filtrovanie
  aktívneho-sale-joinu na `payment_status='refunded'` by vždy vrátilo 0
  riadkov. "Refunded" je dostupné iba cez Quick Filter. Zámerné, nie chyba.
- **Žiadny sort control** — zadanie sort nespomínalo, takže je pevné
  poradie (najbližší event date prvý, potom ticket id). Ak chceš meniteľné
  triedenie, poviem si o rozsah.
- **`isRefunded`** rieši iba zobrazenie/filter — refund/resell business
  logika sa nemenila ani nerozširovala.

---

Verzia zvýšená **2.4.2 → 2.4.3**. STOP — žiadne ďalšie features.

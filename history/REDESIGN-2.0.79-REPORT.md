# TIQR Manager 2.0.79 — Dashboard úpravy + staré CSV exporty

Traja rôzne požiadavky, ktoré si postupne poslal - každý vysvetlený nižšie.

## 1. Preč s tlačidlami "New Event / New Order / ..." z Overview

> *"v overview je toto, treba to odstranit"* (k screenshotu s riadkom tlačidiel New Event/New Order/New Sale/Import CSV/Export CSV)

Celý ten riadok som z Dashboard → Overview odstránil. Bol duplicitný - Events/Orders/Sales majú svoje vlastné "New ..." tlačidlo priamo na svojej stránke, a Import/Export CSV je v Settings → Data. Nič iné sa nezmenilo.

## 2. CSV exporty boli naozaj staré - doplnil som chýbajúce stĺpce

> *"taktiez csv su stare, treba ich updatnut vsetky"*

Prešiel som všetky 3 exporty (Orders, Tickets/Inventory, Sales) a porovnal ich so skutočnými dátami, ktoré appka dnes ukladá. Chýbalo toho dosť - niektoré polia pribudli pred desiatkami verzií a export ich odvtedy nikdy nezachytil:

- **Orders export** - pridaný stĺpec **category** (kategória eventu - Events export ju už mal, Orders nie).
- **Tickets aj Inventory export** (je to tá istá funkcia v appke) - pridané **resale_status** a **delivery_status** (tvoje vlastné ručné Listed/Unlisted/Sold a doručovací status - existujú od verzie 2.0.10, ale v exporte neboli nikdy).
- **Sales export** - dostal najviac nového: **order_code** (aby si vedel sale spárovať s objednávkou), **section/row/seat** (kde presne bolo miesto), **margin** a **roi** (percentá, rovnaký výpočet ako v appke), **resale_status/delivery_status** a **refunded_at/refund_reason** (kedy a prečo bol sale vrátený - predtým sa to dalo zistiť len v appke, nie v exporte).

Import CSV šablóna (na vytváranie objednávok) som nemenil - tá je stále aktuálna, nič jej nechýba.

## 3. Dashboard Activity: "Unpaid payments" nahradené za "Pulls near deadline"

> *"v dashboard activity treba tieto unpaid payment zmenit na sales"*

Keďže appka už mala samostatnú dlaždicu "Pending sales" (nezaplatené predaje od kupujúcich) vedľa "Unpaid payments" (nezaplatené objednávky u dodávateľa) - dve different veci - opýtal som sa ťa, čo presne s "Unpaid payments" spraviť. Odpovedal si:

> *"tie objednavky zrusit a namiesto toho dat nieco s pulls"*

Spravil som presne to:

- Dlaždica **"Unpaid payments"** (nezaplatené objednávky) je z Attention sekcie aj zo zvončeka preč.
- Namiesto nej je nová dlaždica **"Pulls near deadline"** - počíta pully (Given), ktoré ešte nemajú zaškrtnuté "Transferred" a ich event date sa už blíži (rovnaký 3-dňový interval, aký Pulls stránka sama používa pri svojom stĺpci "Deadline") alebo je už po evente. Klik vedie na `/pulls`, presne ako pri ostatných dlaždiciach.
- Samotný počet nezaplatených objednávok (`unpaid_orders_count`) som **nezrušil** - len ho už appka neukazuje na Dashboarde. Stále funguje presne ako doteraz pre desktop/ntfy notifikácie (2.0.76-2.0.78), keďže si sa pýtal len na Dashboard Activity, nie na notifikácie. Ak chceš, aby notifikácie prestali hlásiť nezaplatené objednávky tiež, napíš mi a upravím to.

## Čo som overil

```
cargo test --lib   -> 738 testov, 0 zlyhaní, 3 ignorované
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

Medzi novými testami sú aj také, čo overujú presne hranicu 3 dní pre "Pulls near deadline" (pull tesne v okne sa počíta, tesne mimo nie, už transferovaný sa nepočíta nikdy, pull bez event date sa nepočíta vôbec) a že každý nový CSV stĺpec (category, resale/delivery status, order_code, seat, margin, roi, refund detaily) sa naozaj exportuje so správnou hodnotou.

## Zmenené súbory

**Backend (Rust):**
- `src-tauri/src/commands/csv_export.rs` — nové stĺpce v Orders/Tickets/Inventory/Sales exportoch.
- `src-tauri/src/commands/dashboard.rs` — nový výpočet `pulls_needing_transfer_count`.
- `src-tauri/src/models.rs` — nové pole v `DashboardAlerts`.
- `src-tauri/src/commands/notifications.rs` — testovací fixture doplnený o nové pole (bez zmeny správania).

**Frontend:**
- `src/pages/Dashboard.tsx` — Quick Actions preč, "Unpaid payments" → "Pulls near deadline" (dlaždica aj zvonček).
- `src/lib/types.ts` — nové pole `pullsNeedingTransferCount`.

**Verzia (9 miest v 7 súboroch):** `2.0.79`.

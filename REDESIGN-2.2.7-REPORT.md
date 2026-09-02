# TIQR Manager 2.2.7 — Ticket metadata: Tier / Level + Section / Row

Reagujem na tvoju správu:

> *"Pome na ďalší focused task: TIQR Manager 2.2.7 — Ticket metadata: Tier / Level + Section / Row. [...]
> Doplniť ticket metadata tak, aby každý ticket mohol mať: tier / level, section, row. Požiadavky: tier/level
> = samostatné pole, NESMIE sa zamieňať s ticket_type; section a row zachovaj ako samostatné polia; všetko
> nepovinné, aby existujúce tikety ostali validné; pridaj forward-only SQLite migration; existujúce dáta
> nesmieš meniť ani prepisovať; Add Ticket + Edit Ticket formulár musí umožniť tieto údaje zadať/upraviť; CSV
> import/export aktualizuj o nové polia, ale zachovaj backward compatibility so starými CSV; Inventory
> Intelligence → Breakdown musí reálne začať zobrazovať Tier/Level; pripraviť dáta tak, aby ich neskôr vedel
> používať Market Analysis / Repricing; žiadna zmena refund/resell logiky, batch_id, money/integer cents
> logiky, Orders/Sales/Finance core logiky; Listings systém nemen; Price Checker scraping/scanner nemen.
> Nevymýšľaj automatické hodnoty tier/section/row pre existujúce tikety — dostanú NULL/empty. Nepridávaj AI
> ani automatické mapovanie štadiónov. [...] STOP po tomto tasku. Žiadne ďalšie features."*

Toto je presne to, čo som spravil, krok po kroku.

## 1. Nová databázová kolóna — `tickets.tier`

`section` a `row` (v databáze `row_label`) v appke existovali od úplne prvej migrácie (001) — tie som nechal
presne tak, ako sú, žiadna zmena. **Tier/level je jediné skutočne nové pole** — nová migrácia
`024_ticket_tier.sql`:

```sql
ALTER TABLE tickets ADD COLUMN tier TEXT;
```

Nepovinný textový stĺpec, žiadny CHECK zoznam hodnôt (rovnaký prístup ako pri `resale_status`/
`delivery_status` z 2.0.10 — je to tvoj vlastný slovník, appka ho nemá vynucovať). Žiadny backfill,
žiadne dopočítavanie — každý existujúci tiket má po migrácii `tier = NULL`, presne ako si žiadal.

**Prečo je to samostatné pole a nie `ticket_type`:** `ticket_type` je spôsob DORUČENIA (E-ticket/PDF/Mobile
transfer/Physical/Will call) — toto zamieňanie som už dvakrát predtým zapísal do `PROTECTED_AREAS.md` (verzie
2.2.0 a 2.2.6), takže tento tretí raz som to rovno postavil ako úplne oddelený stĺpec od začiatku.

## 2. Kde sa to dá zadať/upraviť

V appke neexistuje žiadny samostatný "Add Ticket" formulár — tikety vždy vznikajú len cez vytvorenie
objednávky (rovnako ako doteraz `section`/`row`). Tier/level som preto pridal na presne tie isté dve miesta,
kde sa dnes zadáva/upravuje section a row:

- **Nová objednávka** (`Orders.tsx`) — pole "Tier / Level" hneď vedľa "Row", nastaví sa raz a skopíruje sa na
  každý tiket, ktorý objednávka vygeneruje.
- **Edit ticket** (`Tickets.tsx`) — to isté pole, upraviteľné na jednom konkrétnom tikete kedykoľvek neskôr.

Obe sú malé, obyčajné textové polia — žiadny nový dizajn, žiadne nové sekcie formulára.

## 3. CSV import/export

- **Import** rozpoznáva stĺpec `tier` (alebo `level` ako synonymum — niekedy jedno, niekedy druhé meno
  používaš). Starý CSV bez tohto stĺpca importuje úplne rovnako ako doteraz (nový stĺpec jednoducho chýba →
  `tier = NULL`, žiadna špeciálna vetva kódu na to netreba).
- **Export** — tikety, sales aj šablóna na stiahnutie pre import — všetky teraz obsahujú `tier`, hneď za
  `row`. Popis vo Settings ("Required format: ...") som upravil rovnako.

## 4. Inventory Intelligence — Breakdown podľa Tier/Level

2.2.6 report (bod 5) hovoril, že breakdown podľa tier sa nedá vyrátať, lebo appka nemala tier stĺpec vôbec.
Teraz existuje, takže breakdown reálne funguje — presne rovnako ako breakdown podľa section a podľa
marketplace: zobrazuje počty a hodnoty podľa skutočne uložených hodnôt, klik naň prefiltruje Tickets tabuľku
rovnako ako ostatné breakdown položky. Prázdny/nevyplnený tier sa v tejto skupine zobrazí ako **"Unknown"**
(zámerne iné slovo ako "No section" pri section breakdown, presne ako si to takto pomenoval).

## 5. Market Analysis / Repricing — pripravené, ale zámerne nezapojené

Povedal si "pripraviť dáta tak, aby ich neskôr vedel používať Market Analysis / Repricing" — bral som to
doslovne ako "priprav, ale ešte nepoužívaj". Skutočný stĺpec `tickets.tier` teraz existuje a je pripravený na
čítanie, ale `price_checker_analysis.rs`-ovo pole `YourTicketGroup.tier` zostáva zámerne stále `None` — nič
som tam nezapájal. Keď budeš chcieť, aby Market Analysis/Repricing tento údaj reálne používali, je to
jednoduchý, samostatný krok (dáta už na to čakajú), ale je to tvoje rozhodnutie, nie niečo, čo som si mal
domyslieť teraz.

## 6. Čo som zámerne nechal bez zmeny

- **Bulk edit tickets** (`BulkTicketField`) — tier tam nepridané, bulk úprava tier/level nebola súčasťou
  zadania.
- **Žiadne zoznamy/tabuľky** (Tickets, Order Detail, Sales, Sale Detail) nedostali nový stĺpec "Tier" —
  overil som, že ani section/row sa tam dnes nezobrazujú ako stĺpce, takže je to konzistentné, nie medzera.
- **Google Sheets sync objednávok** nie je na `tier` napojený — v hárku pre to neexistuje stĺpec a pridať ho
  by bolo samostatné rozhodnutie (aký názov stĺpca, či ho vôbec chceš v hárku).
- **Refund/resell logika, `batch_id`, money/integer cents logika, Orders/Sales/Finance core logika, Listings
  systém, Price Checker scraping/scanner** — nič z toho som sa ani nedotkol.

## Zmenené súbory

**Backend:**
- `src-tauri/migrations/024_ticket_tier.sql` — nová migrácia
- `src-tauri/src/db.rs` — registrácia migrácie + 2 nové testy na upgrade existujúcej DB
- `src-tauri/src/commands/database.rs` — kontrola počtu migrácií (23 → 24)
- `src-tauri/src/models.rs` — `tier` pridané do `Ticket`, `TicketUpdateInput`, `OrderInput`;
  `InventoryIntelligence` dostalo `breakdown_by_tier`
- `src-tauri/src/commands/tickets.rs` — `tier` v SELECT/UPDATE + 2 nové testy
- `src-tauri/src/commands/orders.rs` — `tier` v INSERT pri vytváraní objednávky + 2 nové testy
- `src-tauri/src/commands/csv_import.rs` — rozpoznanie stĺpca `tier`/`level` + 3 nové testy
- `src-tauri/src/commands/csv_export.rs` — `tier` v exporte tiketov/sales/šablóny + 3 nové testy (a oprava
  5 existujúcich testov, ktorým sa posunul index stĺpca)
- `src-tauri/src/commands/inventory_intelligence.rs` — nový breakdown podľa tier + 1 nový test
- `src-tauri/src/commands/orders_sheet_sync.rs` — `tier: None` na mieste, kde appka stavia objednávku z
  Google Sheets (zámerne nezapojené, pozri bod 6)
- `src-tauri/src/commands/price_checker_analysis.rs` — iba komentár aktualizovaný (správanie nezmenené)

**Frontend:**
- `src/lib/types.ts` — `tier` v `Ticket`/`TicketUpdateInput`/`OrderInput`, `breakdownByTier` v
  `InventoryIntelligence`
- `src/pages/Orders.tsx` — nové pole "Tier / Level" vo formulári novej objednávky
- `src/pages/Tickets.tsx` — nové pole "Tier / Level" v Edit ticket formulári
- `src/pages/EventDetail.tsx` — nový "By tier" breakdown stĺpec v Inventory Intelligence bloku
- `src/pages/Settings.tsx` — popis CSV formátu aktualizovaný

**Dokumentácia:**
- `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/PROTECTED_AREAS.md`, `CHANGELOG.md`

## Čo som overil

```
cargo test --lib   -> 985 passed, 0 failed, 3 ignored (+13 nových testov)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.2.7 build" v hlavičke)
```

Špeciálne som si dal záležať na regresiách: celý existujúci test balík (972 testov pred týmto taskom) prešiel
bez jedinej zmeny okrem tých, čo sa priamo dotýkali pridaného stĺpca (posunuté indexy v pár CSV export
testoch, popísané vyššie) — Orders/Tickets/Sales/Listings/Finance/refund-resell logika zostala funkčná presne
ako predtým.

---

Teraz k tvojim siedmim bodom, presne v poradí, ako si ich chcel:

**1. Zmenené súbory** — pozri zoznam vyššie (10 backend + 5 frontend súborov, plus 3 stavové dokumenty).

**2. Migration** — `024_ticket_tier.sql`, jeden nový nepovinný stĺpec `tickets.tier TEXT`, žiadny backfill,
žiadna zmena existujúcich dát. Forward-only, ako všetky doterajšie migrácie v appke.

**3. Čo pribudlo v UI** — pole "Tier / Level" (malé textové pole) v "Nová objednávka" formulári (hneď vedľa
Section/Row) a v "Edit ticket" formulári. Žiadny veľký redesign, presne ako si žiadal.

**4. CSV kompatibilita** — starý CSV bez stĺpca `tier`/`level` sa importuje bezo zmeny (nový stĺpec ostane
prázdny). Nový CSV so stĺpcom `tier` (alebo `level`) sa naimportuje správne na každý vygenerovaný tiket.
Export (tikety, sales, šablóna) obsahuje `tier` hneď za `row`.

**5. Inventory Intelligence zmeny** — breakdown "By tier" teraz reálne existuje a funguje presne ako
section/marketplace breakdown (klikateľný, filtrovanie Tickets tabuľky). Prázdna hodnota = "Unknown".

**6. Test výsledky** — `cargo test --lib`: 985 passed / 0 failed / 3 ignored (+13 nových testov: migrácia
upgrade + čerstvá DB, create/update tiketu s tier, CSV import starý/nový formát + `level` synonymum, CSV
export obsahuje tier, Inventory Intelligence tier grouping). `tsc -b` aj `npm run build` bez chýb.

**7. Prípadné limity** —
- Market Analysis/Repricing (`YourTicketGroup.tier`) zámerne stále vracia `None` — dáta sú pripravené, ale
  nezapojené, presne ako si žiadal ("pripraviť, nepoužívať ešte").
- Bulk edit tickets nemá tier medzi upraviteľnými poľami — nebolo súčasťou zadania.
- Žiadny zoznam/tabuľka v appke (Tickets, Order Detail, Sales) nezobrazuje tier ako stĺpec — konzistentné s
  tým, že ani section/row sa tam nezobrazujú.
- Google Sheets sync objednávok nie je na tier napojený (v hárku preň nie je stĺpec).

## STOP

2.2.7 hotové, otestované a zabalené. Ako si žiadal — žiadne ďalšie features, končím tu.

# TIQR 2.28.0 — Market Map odstránená

---

## Čo je preč

Zmazané, nie skryté:

| súbor | čo |
|---|---|
| `src-tauri/src/commands/price_checker_map.rs` | **zmazaný** — modul, `compute_market_map`, 13 testov |
| `src/components/MarketMapView.tsx` | **zmazaný** |
| `src-tauri/src/models.rs` | `MarketMap` + 4 štruktúry preč |
| `src/lib/types.ts` | `MarketMap*` rozhrania preč |
| `src/lib/api.ts` | `computeMarketMap` preč |
| `src-tauri/src/commands/mod.rs`, `lib.rs` | registrácia preč |
| `src/pages/PriceChecker.tsx` | stav mapy, `scanTotal`, effect aj render preč |

`grep` na `MarketMap`, `market_map`, `marketMap` a `price_checker_map` naprieč
`src` aj `src-tauri/src` **nevracia nič**. Príkazov je späť **181**.

Z `PROTECTED_AREAS.md` som vyhodil sekciu s invariantmi mapy — nie je už čo
chrániť.

---

## Čo som nechal — a prečo

Nič z tohto nebola mapa a všetko to funguje ďalej:

- **Automatický beh scanu** (`start_price_scan_run`) — jedno kliknutie prečíta
  celú stránku na pozadí, TIQR ostáva použiteľné. Notifikácia aj toast už
  hovoria **„Price scan finished"**, nie „Market map is ready" — nemá zmysel
  inzerovať funkciu, ktorá neexistuje.
- **Oprava viagogo readera** v `price_checker_scan.js` — skutočná chyba
  scannera, ktorú našiel audit pri stavaní mapy. Viagogo stránky označovali
  **každý** listing ako `"generic"`. To ostáva opravené.
- **Oprava macOS user agenta** („An outdated browser…").
- **Scan sa sám ukladá do histórie.**
- **Orezaná karta marketplace.**
- **Oprava kolízie `€1,815.0064`.**

---

## Verzia

**2.28.0**, posunutá **dopredu**, nie recyklovaná — tvoje vlastné pravidlo:
updater odmieta zopakované číslo, takže ani revert nesmie siahnuť po starom.
9 výskytov v 7 súboroch.

---

## Čo som overil

- **Žiadna zvyšná referencia** na mapu (grep vyššie).
- **181 príkazov** sedí medzi `api.ts` a `lib.rs` v oboch smeroch.
- Zátvorky vyvážené vo všetkých dotknutých TS/TSX súboroch; Rust vyvážený
  presným lexerom.
- **Preskenoval som celý strom na tú chybu, čo položila 2.27.0** — JSX komentár
  `{/* … */}` vo výrazovej pozícii. **Nula.**
- Pri mazaní sekcie z `PROTECTED_AREAS.md` mi poistka v skripte zachytila, že
  môj rozsah by bol zmazal aj sekciu 2.25.0. Opravil som hranicu; sekcie
  2.27.0, 2.26.1, 2.25.0 a 2.24.0 sú na mieste.

**Nespustené tu:** `cargo test --lib`, `cargo check --lib`, `npx tsc -b`,
`npm run build` — Rust ani Node na tomto Macu nie sú.

**Nedotknuté:** refund/resell, `batch_id`, peniaze, Orders, Tickets, Sales,
Listings, Finance, Fulfillment, Attention, Calendar, Google Sheets, Sync, AI
Import. Žiadna migrácia (ďalšia je stále 029), žiadna nová závislosť.

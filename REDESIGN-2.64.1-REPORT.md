# TIQR Manager 2.64.1 — oprava buildu

**2.64.0 sa nezostavilo. Moja chyba, a chcem presne povedať akú.**

---

## Čo sa pokazilo

```
src/pages/SaleDetail.tsx(528,26): error TS2339:
Property 'eventDate' does not exist on type 'Sale'
```

Dátum eventu v predaji som napísal proti políčku `eventDate`. To políčko má typ
**`OrderRecord`** — objednávka. Predaj má typ **`Sale`**, a ten ho nemal.

V objednávke to preto fungovalo. V predaji nie.

## Prečo som to nezachytil

Toto je tá horšia časť. Pred odoslaním som si to „overoval" takto:

```
print("eventDate is on the Sale line type:", "eventDate" in types.ts)
```

Pýtal som sa, **či sa to slovo niekde v tom súbore vyskytuje**. Vyskytovalo sa —
na inom type. Odpoveď bola `True` a ja som išiel ďalej.

To nie je kontrola. To je otázka, na ktorú sa nedá odpovedať zle.

## Čo som spravil teraz

Políčko **naozaj existuje**, a to na všetkých štyroch miestach, kde musí:

| Kde | Čo pribudlo |
|---|---|
| `models.rs` | `Sale.event_date` |
| `sales.rs` | `e.event_date` v dotaze |
| `sales.rs` | `event_date` v mapovaní riadku |
| `types.ts` | `Sale.eventDate` |

**Overil som to spustením, nie hľadaním slova.** Vzal som ten dotaz z Rustu tak,
ako je, a pustil ho proti skutočnej schéme:

- event s dátumom → vráti `2026-07-12`
- event bez dátumu → vráti prázdno, čo appka ukáže ako `TBD`

Ešte som overil, že `Sale` sa v celom Ruste skladá **na jedinom mieste**, takže
nové políčko nemôže nikde chýbať.

---

## Druhá vec, o ktorej musíš vedieť

Build spadol v `tsc -b`. Ten beží **pred** Rustom.

**Takže žiadny Rust z 2.64.0 sa nikdy nekompiloval.** Zelené ani červené CI o ňom
nepovedalo nič — nedostal sa k nemu.

Prešiel som preto všetkých päť rustových zmien z 2.64.0 znova, riadok po riadku,
a ku každej overil predpoklad priamo v zdroji:

- obe kolónky s menou sú naozaj `String`, nie `Option`
- `fx` je v `orders.rs` naozaj importovaný a už tam bol volaný
- súbor migrácie 039 naozaj leží na disku, a je 39 z 39
- `Sale` sa naozaj skladá len na jednom mieste

A jednu vec som preventívne prepísal: pri dvoch príkazoch som mal `mut` priamo
v parametri. Teraz je to obyčajná premenná vnútri funkcie, takže to nezávisí od
toho, ako si Tauri prepisuje zoznam parametrov.

---

## Čo je v tomto balíku

**Všetko z 2.64.0, nezmenené** — chýbajúce peniaze v súčtoch, Listings preč
z Events, dátum eventu v objednávke aj v predaji. Detaily sú v
`REDESIGN-2.64.0-REPORT.md`.

**Plus tá oprava.**

---

## Čo som overil

- **ten dotaz spustený** proti skutočnej schéme, s dátumom aj bez neho
- **39 migrácií** na čistej databáze
- **187 SQL príkazov** v `sales.rs`, `orders.rs` a `dashboard.rs` proti schéme
- **241 príkazov** Rust ↔ appka 1:1
- **`Sale`: 36 políčok v štruktúre, všetky nastavené** v mapovaní
- **žiadny TypeScript kód nikde neskladá `Sale` ručne**, takže nové povinné
  políčko nemôže nič rozbiť

**Neoverené:** appku nezostavím, nemám tu Node ani Rust. To povie až tvoj build —
a tentoraz sa aspoň dostane aj k Rustu.

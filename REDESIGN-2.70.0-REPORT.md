# TIQR Manager 2.70.0 - oprava buildu

**Dátum:** 5.10.2026
**Zadanie:** build padol na Windowse aj na Macu

---

## 1. Čo CI našlo

```
error[E0063]: missing field `restrictions` in initializer of `OrderInput`
  --> src/commands/csv_import.rs:250
  --> src/commands/orders_sheet_sync.rs:1015
```

Toto je **moja chyba z 2.67.0**. Vtedy som pridal `OrderInput.restrictions`
a napísal som, že som ho doplnil do 12 literálov. Literálov je **18**. Dva
som vynechal a nemal som ako to zistiť - kompilátor na tomto Macu nie je,
takže CI bol prvý, kto to prečítal.

## 2. Čo som opravil

### Dva, ktoré hlásilo CI

Oba zdroje žiadne obmedzenia nenesú, takže `None` je pravda, nie výplň:

- **`csv_import.rs:250`** - importný formát nemá stĺpec pre restrictions.
  Importovaná objednávka príde bez obmedzení a označíš ich po lístku potom.
- **`orders_sheet_sync.rs:1015`** - hárok ho tiež nemá. Rovnaký precedens
  ako `tier: None` priamo nad tým riadkom.

### Tri, ktoré CI vidieť nemohlo

`tauri build` preskakuje `#[cfg(test)]`, takže tieto by padli až na
`cargo test`:

- **`sales.rs:1502`** - tiež `OrderInput`, tiež moja chyba z 2.67.0.
- **`tickets.rs:893`** a **`tickets.rs:924`** - `TicketUpdateInput`,
  chýbajúce už od **2.60.0**. Čiže `cargo test` nekompiloval štyri verzie
  a žiadny release build to nemohol povedať.

(`tickets.rs:936` a `:937` sú v poriadku - používajú `..blank()`, čo chýbajúce
polia dopĺňa samo.)

## 3. Ako viem, že už nič nechýba

Toto je podstatnejšie než tie opravy. V 2.67.0 som literály prechádzal ručne
a napočítal zle. Teraz som si napísal audit, ktorý to spraví za mňa:

> Nájdi v `src/` každý literál každej štruktúry, ktorá v `models.rs`
> deklaruje pole `restrictions` - a over, či ho ten literál má, alebo či
> používa `..` syntax, ktorá ho doplní.

Štruktúry: `OrderInput`, `Ticket`, `TicketUpdateInput`.

**Výsledok po oprave: 0 chýbajúcich.** Pred opravou to isté hľadanie našlo
presne tých päť, vrátane dvoch, ktoré CI nehlásilo.

Toto je kontrola, ktorú som mal spraviť v 2.67.0 namiesto počítania po pamäti.

## 4. Čo je otestované a čo nie

**Overené:**

- Audit vyššie: 0 chýbajúcich literálov zo všetkých troch štruktúr.
- Zátvorky v štyroch upravených `.rs` súboroch: `{}` vyrovnané všade.
  `orders_sheet_sync.rs` má jednu nepárovú `(` - je v komentári, bola tam
  predtým, a moje tri vložené riadky žiadnu zátvorku neobsahujú. Súbor sa
  predtým kompiloval, CI hlásilo len chýbajúce pole.
- Verzia 2.70.0 vo všetkých 9 výskytoch v 7 súboroch, nikde nezostalo 2.69.0.

**Neoverené:**

- **Stále neskompilované.** Na tomto Macu nie je node ani cargo. Opravil som
  presne to, čo kompilátor pomenoval, plus tri rovnaké prípady, ktoré našiel
  audit - ale či build prejde celý, povie až CI.

## 5. Čo z toho plynie

Môj odhad "12 literálov" bol v 2.67.0 napísaný ako hotový fakt. Nebol -
bolo to počítanie po pamäti bez kompilátora, ktorý by ho vyvrátil.

Keď v tomto projekte pribudne povinné pole do štruktúry, ten audit sa dá
spustiť znova - je to krátky skript a nájde to, čo oko prehliadne.

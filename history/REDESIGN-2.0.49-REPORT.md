# TIQR Manager 2.0.49 — preč s malými grafmi pod kartami

## Čo si mi napísal

*"taktiez chcem odstranit tie maly grafy uplne"* — myslel si tie malé čiarové grafy pod Revenue, Profit a
Tickets sold na Overview (pridané v 2.0.47).

## Čo je nové

**Malé grafy sú preč, všade.** Šípka a percento ("↑ 12.4 % vs. previous period") pod kartami ostávajú presne
také, ako boli — to si nespomínal, tak som sa toho nedotkol. Zmizla len tá malá čiarová krivka pod číslom, na
všetkých troch kartách, čo ju mali (Revenue, Profit, Tickets sold).

Odstránil som to naozaj celé, nielen skryl — samotný kód pre ten graf (komponent `Sparkline`) som z appky
vymazal, keďže ho už nikde nič nepoužíva. Nič iné sa nezmenilo — veľký graf nižšie na stránke (ten, čo sa dá
prepínať medzi Profit & Loss/Revenue/Sales) používa tie isté dáta ako predtým a vyzerá presne tak, ako doteraz.

## Ako som to overoval

```
cargo test --lib  -> 578 testov, všetky prešli (žiadny backend súbor sa tentokrát nemenil)
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

Toto sú skutočné výsledky, nie odhad. Keďže sa nezmenil žiadny Rust súbor, `cargo check` v čistom prostredí
tentokrát netreba (robím to len keď sa mení `src-tauri/src`) — bežný `cargo test` v appke stačí, a prešiel.

## Čo teraz urobiť

1. Nainštaluj 2.0.49.
2. Skontroluj Dashboard → Overview — pod Revenue/Profit/Tickets sold by už nemal byť žiadny malý graf, len
   šípka s percentom.

## Zmenené súbory

**Frontend:** `src/components/ui.tsx` (odstránený `Sparkline` komponent aj jeho prop na `StatCard`),
`src/pages/Dashboard.tsx` (odstránené 3 miesta, čo ten graf používali).

**Verzia (8 miest):** ako vždy, všetkých na `2.0.49`.

## STOP

1. Nainštaluj 2.0.49 (spusti `1-CLICK-UPDATE.bat`, počkaj na zelený build).
2. Over, že malé grafy sú preč.
3. Pracujem teraz na tom prepočte mien (GBP/USD → EUR podľa aktuálneho kurzu) — dám vedieť, keď budem mať
   niečo na ukázanie. Pri tej automatizácii na kategórie eventov zo Sheetu mám ešte jednu otázku pre teba,
   píšem v chate.

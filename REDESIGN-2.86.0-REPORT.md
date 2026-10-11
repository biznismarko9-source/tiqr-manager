# TIQR Manager 2.86.0 — build fix

**Dátum:** 11.10.2026
**Zadanie:** CI zhodilo build — Rust aj TypeScript

---

## 1. Dve nezávislé chyby

| | pôvod |
|---|---|
| TypeScript vo všetkých troch Finance súboroch | **moja chyba z 2.80.0** |
| Rust testy v `price_checker_scanner.rs` | **staršia hniloba**, ktorú našiel môj job z 2.73.0 |

## 2. TypeScript — a musím priznať dve veci naraz

### Syntax

Komentár `{/* ... */}` som umiestnil hneď po `{period === "custom" && (`.

To je **JavaScript expression** pozícia, nie JSX children — tam sa `{...}`
parsuje ako objektový literál, a komentár v ňom je syntaktická chyba.
TypeScript to ohlásil ako *"JSX element 'Card' has no corresponding closing
tag"*, čo je zavádzajúce, ale príčina je toto.

**Je to presne tá istá trieda chyby, ktorú som si sám chytil v 2.71.0** —
vtedy to bol `//` v zozname JSX atribútov a komentár som preto presunul do
children pozície. Teraz som to spravil zrkadlovo opačne.

**Moja kontrola zátvoriek to nemohla vidieť.** `{/*` a `*/}` sú vyvážené
zátvorky, takže počítadlo hlásilo 0/0/0 a bolo spokojné. To je konkrétna
diera v mojom overovaní a chcem ju pomenovať, nie obísť.

### A horšie: nastylovaný bol nesprávny element

Keď som sa na to pozrel poriadne, zistil som druhú vec. Ten div nie je
filtračná lišta — je to **podriadok s vlastnými dátumami vnútri už existujúcej
`Card`**.

Finance toolbary **už v `Card` boli**. Tá sunken lišta z 2.76–2.78 riešila
voľné divy v Events, Orders a Sales — Finance ju nepotrebovalo. Čiže 2.80.0
„zjednotilo" niečo, čo zjednotené nemalo byť, a ešte to rozbilo.

**Riadok je vrátený presne do pôvodného tvaru** vo všetkých troch súboroch
(overené: `className="flex flex-wrap items-end gap-3">` je späť 1× v každom).

## 3. Rust — job z 2.73.0 si odpracoval svoje

`ScanJsPayload` dostal pole `summary` a `ScanJsCandidate` pole `incomplete`
(2.9.0). Tri testovacie pomocníky ich nikdy nedostali, takže `cargo test`
nekompiloval — ako dlho, neviem.

**Toto nie je moja chyba a stojí za to povedať prečo je to dobrá správa:**
je to presne tá hniloba, na ktorú som ten `rust-tests` job v 2.73.0 pridal.
`tauri build` preskakuje `#[cfg(test)]`, takže release build o tom mlčí.
Na prvom behu to našiel.

Opravené:
- `summary: ScanJsSummary::default()` — fixtúra nemá reálne počty na hlásenie
- `incomplete: false` — ručne postavený kandidát je čistý údaj z definície

Preaudítované: **5/5 literálov `ScanJsPayload`** a **3/3 `ScanJsCandidate`**
majú teraz svoje polia.

## 4. Čo som skúsil a vzdal — a prečo to píšem

Chcel som z toho spraviť trvalú kontrolu, aby ma tá trieda chyby neprekvapila
tretíkrát. Skúsil som dve:

**JSX tag balance.** Trikrát falošné poplachy. Posledný pokus hlásil 16
nepárových tagov — všetky falošné, lebo self-closing tag so šípkovou funkciou
v props (`onChange={(e) => ...}`) obsahuje `>`, a môj regex sa na ňom láme.

**Rust štruktúrne literály.** Tiež tri pokusy. Posledný hlásil **239 z 569**
— `impl X {`, `-> X {` a typové anotácie sa mi miešali s literálmi.

**Nedodávam ti kontrolu, ktorej sám neverím.** Na túto triedu chýb je
kompilátor. Jediná vec, ktorá ju spoľahlivo chytá, je CI — a to práve
dvakrát odviedlo presne tú prácu, ktorú malo.

Jedna kontrola z toho **ostala**, lebo tá funguje a našla skutočnú chybu:
hľadanie `{/* */}` v expression pozícii. Naprieč všetkými 47 `.tsx` súbormi
hlási **0**.

## 5. Zmenené súbory

| súbor | čo |
|---|---|
| `src/pages/finance/Overview.tsx` | komentár opravený, div vrátený |
| `src/pages/finance/Transactions.tsx` | to isté |
| `src/pages/finance/Reports.tsx` | to isté |
| `src-tauri/src/commands/price_checker_scanner.rs` | tri testovacie literály doplnené |

## 6. Overené

| kontrola | výsledok |
|---|---|
| `{/* */}` v expression pozícii, 47 `.tsx` súborov | **0** |
| Pôvodný div späť vo všetkých troch | 1× v každom |
| `ScanJsPayload` literály s `summary` | **5/5** |
| `ScanJsCandidate` literály s `incomplete` | **3/3** |
| Zátvorky v `price_checker_scanner.rs` | rozdiel 0 |

## 7. NEOVERENÉ

**Nespustil som build.** node, npm ani cargo tu nie sú. Opravil som presne
to, čo kompilátor pomenoval, plus jednu vec navyše, ktorú som pri tom našiel
(ten nesprávny element).

Či build prejde celý, povie až CI. Pošli mi výstup.

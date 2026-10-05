# TIQR Manager 2.69.0 - jeden predaj je jeden predaj

**Dátum:** 5.10.2026
**Zadanie:** *"ked tam mas nejaku order, kliknes na nu a das add its tickets tak sa to rozdeli na 2 sales, ale v skutocnosti to je len jeden ... skus nieco vymysliet"*

---

## 1. Krátka odpoveď

**Appka to už robí presne tak, ako to chceš.** Dva lístky zapísané naraz sú
jeden predaj. Jeden lístok zapísaný sám je vlastný predaj. Nič v dátach sa
nerozdelilo.

Klamalo len **tlačidlo**, ktoré hovorilo "Record 2 sales".

## 2. Ako to viem - overené, nie odhadnuté

Toto som nepredpokladal. Pozrel som kód aj tvoje reálne dáta.

**Formulár posiela jedno volanie, nie dve.** `SaleRowsModal.tsx:266` volá
`createSalesBatch` raz, so všetkými riadkami naraz (`lines: rows.map(...)`).

**Backend dá celej dávke spoločné `batch_id`.** `sales.rs:819`:

```rust
// Only a real multi-ticket batch gets a batch_id ...
let batch_id: Option<&str> = if codes_batch.len() > 1 {
    Some(codes_batch[0].as_str())
} else {
    None
};
```

Viac než jeden lístok -> spoločné ID. Jeden lístok -> `NULL`, čiže samostatný
predaj. To je presne tvoje zadanie, zapísané v kóde.

**Všetky zoznamy to zoskupujú späť.** `sales.rs:46`:

```rust
pub(crate) const GROUP_KEY_EXPR: &str = "COALESCE(s.batch_id, 'single:' || s.id)";
```

Používa to Sales list (`sales.rs:489`), dashboard (`dashboard.rs:472`),
kalendár (`calendar.rs:190`) aj CSV export. Jeden riadok na jeden predaj.

**A tvoje skutočné dáta to potvrdzujú:**

| batch_id | riadkov | kódy |
|---|---|---|
| OASIS-001 | 2 | OASIS-001, OASIS-002 |
| UEFA-001 | 2 | UEFA-001, UEFA-002 |
| ENGLAND-001 | 8 | ENGLAND-001 … ENGLAND-008 |
| SAL-000145 | 4 | GARTH-001 … GARTH-004 |
| *(NULL)* | 1 | CELINE-003 |
| *(NULL)* | 1 | CELINE-009 |

Tie OASIS-001 a OASIS-002, o ktoré si prišiel pri tom syncu - to **nie sú dva
predaje**. To je jeden predaj dvoch lístkov. V Sales liste ich vidíš ako jeden
riadok.

**OrderDetail** ukazuje len súhrn (revenue, profit), žiadny zoznam riadkov.
Takže ani tam sa to nikdy nezobrazilo ako dva predaje.

## 3. Čo som zmenil

Dva reťazce v `src/pages/SaleRowsModal.tsx`. Nič viac.

**Tlačidlo.** Bolo `Record ${rows.length} sales`, teraz je natvrdo
**`Record sale`**. Nie podmienené - tento formulár nikdy nevytvoril N
predajov, vždy jeden. Modal nemá edit režim a volá sa z jediného miesta
(`Sales.tsx:996`), takže "vždy jeden" platí bez výnimky.

**Súhrn dole.** Bol `2 tickets · €490,00 revenue · €323,80 profit`,
teraz je **`One sale · 2 tickets · €490,00 revenue · €323,80 profit`**.
Fakt, ktorý ti chýbal, je povedaný presne tam, kde sa rozhoduješ.

## 4. Ako teda spravíš dva samostatné predaje

Zapíšeš ich **po jednom**. Otvor New sale, nechaj v mriežke jeden lístok
(krížik vpravo odstráni riadok), zapíš. Potom znova pre druhý.

Dva samostatné zápisy -> dve `batch_id = NULL` -> dva predaje v zozname.
Jeden zápis s dvoma lístkami -> jedno spoločné `batch_id` -> jeden predaj.

Obe cesty fungovali celý čas.

## 5. Čoho som sa nedotkol

`batch_id`, `create_sales_batch`, `GROUP_KEY_EXPR`, schéma databázy, peniaze.
Toto bola chyba v texte, nie v dátovom modeli, a opravil sa text. Meniť kvôli
tomu jadro predajov by bolo opravovanie niečoho, čo nebolo pokazené.

## 6. Čo je otestované a čo nie

**Overené:**

- Starý reťazec `Record ${` v súbore už nie je; `"Record sale"` je tam raz.
- Zátvorky aj fragmenty `<>…</>`: rozdiel oproti netknutej zálohe 0/0/0/0.
- Oba nové komentáre sú v children pozícii (`{/* … */}`), nie v zozname JSX
  atribútov. `//` komentár medzi atribútmi som najprv napísal, potom presunul
  - nevedel som ho bez kompilátora overiť, tak som to riziko nenechal stáť.
- Verzia 2.69.0 vo všetkých 9 výskytoch v 7 súboroch, nikde nezostalo 2.68.0.

**Neoverené:**

- **Neskompilované.** Na tomto Macu nie je node ani cargo, prvý kompilátor je
  CI. Platí to aj pre 2.65.0 až 2.68.0.
- Okno som nevidel bežať, kontroloval som ho čítaním.

## 7. Jedna otvorená vec, ktorú som sa rozhodol nevymýšľať

V mriežke zadávaš cenu **za lístok** (`Sale/ea`). Ak predáš dva spolu za jednu
dohodnutú sumu za pár, musíš si ju vydeliť na dve sám.

Dalo by sa pridať pole "cena spolu", ktoré by sa rozpočítalo na riadky - ale
nepovedal si to a per-lístok cena je potrebná na per-lístok zisk, takže som
nič nevymýšľal. Ak to chceš, povedz a spravím to samostatne.

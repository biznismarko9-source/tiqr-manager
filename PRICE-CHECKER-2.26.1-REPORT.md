# TIQR 2.26.1 — Price Checker orezaný na to, čo naozaj čítaš

---

## 1. Check Prices okno — preč, aj s tlačidlom

Otváralo formulár, kde si mal **ručne vypísať lowest / median / average /
highest** — čísla, ktoré ti scanner práve prečítal. Presne to si označil za
zbytočné.

**Takže scan je teraz jediná cesta, ako sa price check zapíše — a zapíše sa
sám.**

Tri veci, ktoré to auto-uloženie robí správne:

- **Čaká na tier breakdown**, aby v histórii ostal ten riadok „· 1 tier".
- **Poistka je číslo scanu, nie boolean.** Effect sa znova spustí, keď
  `analysisLoading` preklopí na false — s booleanom by sa ten istý scan zapísal
  do histórie **dvakrát**.
- **Bez meny sa neukladá nič.** Price check, ktorého menu by som musel hádať,
  je horší než žiadny. Karta to povie jednou vetou.

---

## 2. Karta marketplace = tlačidlo na scan + história

Presne posledný obrázok, ktorý si poslal: názov, URL, Visible Scanner, Latest
check, päť čísel, tabuľka histórie. **Nič iné.**

Čo zmizlo spod kariet: tabuľka nascanovaných listingov, Min/Max filtre, Export
CSV, tlačidlo „Save to history", Market Map, Market Analysis panel a „Compare a
specific ticket".

**766 riadkov nedosiahnuteľného UI som zmazal, nie skryl.** `PriceChecker.tsx`
šiel z **2258 na 1499 riadkov**.

---

## 3. Mapa je preč spod kariet a je JEDNA

Tvoje slová: *„mapa by mala byt niekde inde nie tam dole a mapa by mala byt pre
vsetky platformy rovnaka a tie listingy sa spoja."*

`compute_market_map` už **nie je viazaná na jednu scanner session, ale na
EVENT**. Každá otvorená session pre ten event prispeje svojimi listingami do
**jednej mapy**, a mapa sa kreslí **nad kartami**, nie v niektorej z nich.

Takže: nascanuješ Viagogo, potom Ticombo — a section 102 ukáže **oboje vedľa
seba**.

**Zámerne sa NEDEDUPLIKUJE naprieč platformami.** To isté sedadlo vypísané na
dvoch stránkach sú **dve skutočné ponuky za dve skutočné ceny**, a scanner nemá
žiadnu cross-site identitu listingu, ktorou by odlíšil naozajstný duplikát od
dvoch rôznych predajcov, ktorým sa náhodou zhoduje cena.

---

## 4. Tá kolízia na screenshote bola skutočná chyba

`€1,815.0064` — Highest narazený do Listings.

Riadok so štatistikami bol **`grid-cols-5`: päť PEVNÝCH stĺpcov** bez ohľadu na
šírku karty. Tie karty sedia **tri vedľa seba** (`lg:grid-cols-3`), takže
pätina z tretiny stránky nestačí na štvorciferné euro — pretieklo to do
vedľajšej bunky.

Teraz **počet stĺpcov sleduje šírku** (`auto-fit` + `minmax(6.5rem, 1fr)`),
takže sa čísla zalomia do druhého riadku a **každé je celé vidieť**. Je to tá
istá trieda chyby a tá istá oprava ako pri `.summary-bar` v 2.19.0.

**Overené v prehliadači pri skutočnej šírke** (1400 px, tri karty vedľa seba) —
pred opravou sa €1,815.00 a 64 tlačia do seba, po oprave sú v dvoch riadkoch a
čitateľné.

---

## 5. Čo som NEODSTRÁNIL a prečo

- **„Market vs. mine"** hore na evente (ten screenshot, čo si poslal počas
  práce) **ostal** — to je porovnanie, ktoré reálne čítaš, a nie je pod kartami.
- **Market Analysis výpočet** beží ďalej, ale **neviditeľne** — potrebujem z
  neho iba tier breakdown do histórie.
- **Scanner lifecycle** (open / scan / stop / close) je nedotknutý. Žiadny
  auto-scan, žiadny polling, žiadny background monitor.

---

## 6. Testy a overenia

**Spustené tu:**
- **Karta vykreslená v prehliadači pri 1400 px** (tri vedľa seba) — pred aj po
  oprave, kolízia potvrdená a odstránená.
- **JSX a zátvorky** vo všetkých zmenených súboroch — stack-based kontrola,
  0 problémov. (Prvý pokus o automatické mazanie komponentov mi súbor rozbil;
  vrátil som ho z 2.26.0 zipu a zmazal ich podľa overených hraníc riadkov.)
- **181 príkazov** sedí medzi `api.ts` a `lib.rs` v oboch smeroch.
- Rust: lex čistý, 13 testov v `price_checker_map.rs` nedotknutých.

**Nespustené tu:** `cargo test --lib`, `cargo check --lib`, `npx tsc -b`,
`npm run build` — Rust ani Node na tomto Macu nie sú.

**Čo si po stiahnutí over ako prvé:** že sa scan naozaj sám zapíše do histórie
(spusti scan a pozri, či pribudol riadok) a že mapa nad kartami ukáže listingy
z viac než jednej platformy naraz.

---

## 7. Zmenené súbory

| súbor | čo |
|---|---|
| `src/pages/PriceChecker.tsx` | −766 riadkov, auto-save, mapa na event level, oprava mriežky |
| `src-tauri/src/commands/price_checker_map.rs` | príkaz kľúčovaný na event, spája sessions |
| `src/lib/api.ts` | `computeMarketMap(eventId)` |
| `CHANGELOG.md`, `PROJECT_STATE/*` | dokumentácia |

**Nedotknuté:** scanner, market analysis backend, `price_checker.rs`, refund/
resell, `batch_id`, peniaze, Orders, Tickets, Sales, Listings, Finance,
Calendar, Sheets, Sync. Žiadna migrácia (ďalšia je stále 029), žiadna nová
závislosť.

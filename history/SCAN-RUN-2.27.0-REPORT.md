# TIQR 2.27.0 — jedno kliknutie prečíta celú stránku, glitching preč

---

## 1. Glitching mal dve príčiny. Obe sú preč.

**Príčina 1 — mapa sa pri každom obnovení odmontovala.**

Mapa sa prepočítava po každom prechode scanu. Starý kód pritom **vymenil celý
panel za loading box**: mapa zmizla, na jej miesto prišlo nízke okienko, všetko
pod ňou vyskočilo hore, potom sa mapa vrátila a všetko spadlo dole. Niekoľkokrát
za minútu. To je presne to, čo si videl.

Teraz je obnovenie **neviditeľné** — v hlavičke sa objaví jedine slovo
„updating". Veľký loading stav je už len pre **prvé** postavenie mapy, keď
naozaj ešte nie je čo ukázať.

**Príčina 2 — ovládanie zoomu.**

`zoom` je neštandardná CSS vlastnosť, ktorá **prelayoutuje celý podstrom**, a
bola nanovo aplikovaná pri každom rendri. **Je preč celá** — chcel si, aby to
bolo nehybné. Bloky majú pevnú, čitateľnú veľkosť, plocha sa scrolluje, panel má
**pevnú minimálnu výšku** (takže pridanie section už nemení jeho veľkosť) a z
blokov som odstránil aj hover a farebné prechody.

---

## 2. Jedno kliknutie prečíta celú stránku — a beží na pozadí

Tlačidlo je teraz **„Read the whole page"**. Scan → scroll → scan, kým stránka
prestane dávať čokoľvek nové.

**Príkaz sa vráti okamžite** a beh pokračuje na vlákne v backende. Takže:
**môžeš odísť z Price Checkera, prepnúť sa na inú stránku v appke, dať TIQR za
iné okno — beží to ďalej.** Presne to si chcel.

Každý prechod posiela ten istý scan-result event ako doteraz, takže čokoľvek,
čo počúva, sa priebežne aktualizuje.

---

## 3. Toto NIE JE Live Market Monitor, ktorý si v 2.4.2 zmazal

Poviem to rovno, lebo je to hranica, na ktorej ti záleží:

- **Žiadny plán, žiadny časovač, žiadny polling.**
- **Nič sa nespustí samo.** Beh začne iba tlačidlom, na okne, ktoré si sám
  otvoril.
- **Beh je konečný z podstaty** a skončí sám. Keď skončí, je koniec — nič ho
  nenaštartuje znova.

Končí na: **3 prechodoch po sebe bez novej položky**, **60 prechodoch**, **300
sekundách**, chybe, zavretom okne, alebo na Stop.

Tie tri stropy sú **bezpečnostná vlastnosť, nie ladiace čísla**. Bez nich by
stránka, ktorá donekonečna dokladá obsah (nekonečný feed, kolotoč reklám, čo
stále vyrába niečo, čo vyzerá ako cena), premenila beh na proces, ktorý scanuje,
kým nezavrieš appku.

Prázdnych prechodov sú **tri, nie jeden**, zámerne: lenivo načítavaná stránka
bežne potrebuje chvíľu na ďalší blok, a zastaviť pri prvom prázdnom prechode by
také stránky odrezalo v polovici.

**Stop funguje presne ako doteraz** — kontroluje sa medzi prechodmi aj vnútri
každého prechodu.

---

## 4. Oznámenie, keď to dobehne

- **Systémová notifikácia** („Market map is ready — N listings read") — dorazí
  aj keď je TIQR za iným oknom. To je celý zmysel toho, že si medzitým robíš
  svoje.
- **Toast v appke**, nech si kdekoľvek — listener je v `Layout.tsx`, nie na
  stránke Price Checkera, z ktorej si už odišiel.
- **Beh, ktorý si zastavil sám, nehlási nič.**

Karta navyše povie **prečo** beh skončil, backendovými slovami — nie len
„hotovo". Beh, ktorý narazil na strop, to napíše.

**Čo som neurobil:** samostatný trvalý widget na Dashboarde. To by potrebovalo
nové úložisko pre notifikácie, a ty si povedal „oznamenie alebo notifikacia" —
systémová notifikácia + toast to pokrývajú bez novej tabuľky. Ak chceš navyše
trvalý záznam na Dashboarde, povedz a dorobím ho.

---

## 5. „An outdated browser…" — bola to orezaná hlavička, nie starý prehliadač

Je to **iba macOS**. WKWebView posiela user agent, ktorý končí za
`AppleWebKit/605.1.15 (KHTML, like Gecko)` a **nemá vôbec príponu
`Version/… Safari/…`**. Kontrola prehliadača na stránke teda nenájde žiadnu
verziu a spadne do vetvy „outdated".

Scanner teraz posiela tú príponu, ktorú engine vynecháva — hovorí Safari/WebKit,
čo je presne to, čo stránku vykresľuje. **Windows som nechal tak**: WebView2 už
hlási aktuálnu verziu.

**Nič to neobchádza.** Challenge stránky sa naďalej detegujú a poctivo hlásia,
nikdy neobchádzajú.

---

## 6. Čo som overil — a jedno priznanie

**Spustené tu:**
- **Logika ukončenia behu vykonaná** proti scenárom stránky: normálna stránka
  (12 produktívnych prechodov → skončí po 15), nekonečný feed (narazí na strop),
  mŕtva stránka (3 prechody), chyba v polovici, zavreté okno, Stop, a stránka s
  **dvojprechodovou lenivou medzerou** (neodreže sa, dobehne do konca).
  **Každý scenár končí, a z tej správnej príčiny.**
- **`WebviewWindowBuilder::user_agent` a `WebviewWindow::eval` som overil proti
  skutočnej dokumentácii tauri 2.11.5**, nie odhadom. Ani jedno tento projekt
  doteraz nepoužíval, a presne takáto nepreverená API domnienka mi už raz
  položila build.
- **182 príkazov** sedí medzi `api.ts` a `lib.rs` v oboch smeroch.
- Rust: zátvorky vyvážené **presným lexerom** (hrubá kontrola hlásila falošný
  poplach −1; lexer, ktorý rozumie blokovým komentárom a znakovým literálom,
  hlási 0, rovnako ako pred zmenou). JSX vyvážené vo všetkých troch dotknutých
  súboroch.

**Nespustené tu:** `cargo test --lib`, `cargo check --lib`, `npx tsc -b`,
`npm run build` — Rust ani Node na tomto Macu nie sú.

**Čo over ako prvé po stiahnutí:** spusti beh, prepni sa na inú stránku v appke
a over, že (a) beží ďalej, (b) príde notifikácia, (c) mapa sa medzitým
neposkakuje.

---

## 7. Zmenené súbory

| súbor | čo |
|---|---|
| `src-tauri/src/commands/price_checker_scanner.rs` | `perform_one_scan` vyňatý nezmenený, `start_price_scan_run`, stropy, user agent |
| `src-tauri/src/commands/notifications.rs` | `send_desktop_notification` → `pub(crate)` |
| `src-tauri/src/models.rs` | `ScanRunFinishedPayload` |
| `src/components/MarketMapView.tsx` | zoom preč, žiadne odmontovanie pri obnove, pevná min. výška |
| `src/components/Layout.tsx` | toast po dobehnutí, pre celú appku |
| `src/pages/PriceChecker.tsx` | tlačidlo spúšťa beh, listener konca, stavový riadok |
| `src/lib/api.ts`, `src/lib/types.ts` | `startPriceScanRun`, typ payloadu |

**Nedotknuté:** extrakčná logika (`price_checker_scan.js` okrem ničoho), market
analysis, `price_checker.rs`, refund/resell, `batch_id`, peniaze, Orders,
Tickets, Sales, Listings, Finance, Calendar, Sheets, Sync. Žiadna migrácia
(ďalšia je stále 029), žiadna nová závislosť.

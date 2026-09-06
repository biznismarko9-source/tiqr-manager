# TIQR Manager 2.6.0 — Kompletný vizuálny redesign

Toto je report k tvojmu zadaniu *"Kompletný vizuálny redesign celej TIQR
Manager aplikácie … VÝLUČNE UI/UX a visual redesign. Žiadne nové business
features. Žiadne nové workflow. Žiadne nové databázové systémy."*

Držal som sa toho doslovne. Aby to nebolo len tvrdenie:

- **`src-tauri/` je bajt na bajt identický s 2.5.2** — overené `diff -rq`.
  Žiadny Rust súbor, žiadny command, žiadna migrácia (ďalšia nová je stále
  **027**).
- **Celý diff neobsahuje ani jedno `api.` volanie, ani jeden `useState` /
  `useEffect` / handler, ani jednu zmenu routovania.** Prešiel som celý diff
  filtrom presne na tieto veci — jediné "nevzhľadové" riadky v ňom sú: jeden
  type-only import, `size="sm"` na jednom tlačidle, a nové čisto
  prezentačné komponenty.
- Žiadna nová dependency. `package.json` má rovnaké balíčky ako 2.5.2.

---

## ⚠️ Jedna vec, ktorú musíš vedieť hneď na začiatku

**Tento release som NEOVERIL buildom.** Na Macu, na ktorom som pracoval, nie
je nainštalovaný ani Node.js, ani Rust. Takže `npx tsc -b`, `npm run build`
ani `cargo check --lib` sa nedali spustiť — pýtal som sa ťa a vybral si
"bez Node, len statická kontrola".

Čo to znamená prakticky:

1. **Pred publikovaním tagu spusti na Windowse:**
   ```
   npm install
   npx tsc -b
   npm run build
   cargo check --lib
   ```
2. `Cargo.lock` a `package-lock.json` som verziu prepísal **ručne** (lebo
   `cargo check` / `npm install --package-lock-only` som spustiť nemohol).
   Po prvom `cargo check` a `npm install --package-lock-only` sa to samo
   zregeneruje správne.

Čo som namiesto buildu overil staticky, je nižšie v sekcii **Testovanie**.

---

## Ako som k tomu pristúpil

Tvoje zadanie hovorilo: *"pracuj cez shared UI/design vrstvu, nezačni
prerábať každú stránku samostatne, najprv nájdi spoločné komponenty a
styling patterns."*

Presne to bol prvý krok — a ukázalo sa, že appka na to už bola pripravená
lepšie, než by sa čakalo. Zistil som, že stránky používajú zdieľané veci
naozaj husto:

| Zdieľaný komponent | Počet použití v appke |
|---|---|
| `Button` | 160 |
| `Input` | 132 |
| `Field` | 127 |
| `Card` | 65 |
| `Select` | 64 |
| `StatCard` | 52 |
| `Spinner` | 50 |
| `Badge` | 42 |
| `EmptyState` | 30 |
| `ConfirmDialog` | 28 |
| `Modal` | 27 |

Plus `.input` / `.label` / `.th` / `.td` / `.card` triedy v `index.css` a
plné farebné škály `brand` / `slate` v `tailwind.config.js`.

Preto **nebolo treba prerábať 23 000 riadkov stránok**. Redesign je zmena
vrstvy — a stránky som otváral len preto, aby prevzali to, čo im vrstva
odteraz dáva (t.j. aby som zmazal ich vlastnú ručne prepísanú kópiu niečoho
zdieľaného), nikdy nie preto, aby dostali vlastný vzhľad.

---

## 1. Design system

Vzhľad celej appky je odteraz definovaný v **štyroch súboroch a nikde inde**:

### `tailwind.config.js`

- **`slate` škála preladená.** Toto je hlavná páka celého redesignu: prepísať
  túto jednu škálu znamená, že všetkých ~23 000 riadkov už napísaných
  `bg-slate-N` / `text-slate-N` / `border-slate-N` sa zmení naraz, bez
  jediného zásahu do stránky. (Je to presne ten istý mechanizmus, ktorým sa
  robilo 2.0.56 aj jeho revert v 2.0.58.)
  - Odtieň som stiahol z pomerne modrého Tailwind slate do tichšej
    modro-sivej — aby brand modrá bola jediná sýta farba na obrazovke.
  - **Dark mode už nie je invertovaný light.** `950` (pozadie appky) a `900`
    (povrch kariet a sidebaru) sú nadvihnuté a odmodrené oproti pôvodným
    `#020617` / `#0f172a`. Predtým bola tmavá karta takmer čierna na takmer
    čiernom pozadí; teraz je tam reálny krok pozadie → povrch, a `800`/`700`
    dávajú okraje, ktoré na tom naozaj vidieť.
  - `50`/`100` (svetlé pozadie, hover) sú o kúsok teplejšie, aby veľké
    svetlé plochy nepôsobili studeno.
- **`brand` škála som ZÁMERNE nechal bajt na bajt.** `#4a68f7` je akcent,
  ktorý si potvrdil v 2.0.56 a ktorý prežil revert v 2.0.58. Zmena palety už
  raz vyskúšaná a odmietnutá bola, tak som sa jej nedotkol — redesign mení
  len to, **koľko** modrej je vidieť, nie akej.
- **Jedna tieňová škála pre celú appku**: `shadow-card` → `shadow-raised` →
  `shadow-overlay`. Dvojvrstvové (tesný kontaktný tieň + širší ambientný),
  nie Tailwindov jednoduchý rozmaz — to je to, čo v malom pôsobí ako hĺbka a
  nie ako ťažoba. V dark mode tieň nefunguje (čierny tieň na tmavom povrchu
  nevidno), tak tam hĺbku robí 1px svetlý highlight navrchu karty.
- **Rádiusy zmenšené**, presne kvôli tvojmu "žiadne obrovské rounded cards":
  ovládacie prvky `lg` = 8px, kontajnery `xl` = 10px.
- **Motion budget**: `transitionDuration.DEFAULT` = 150ms, plus 120/150/180.
  Nič v appke nemá dlhší prechod, lebo default je tá hodnota.

### `src/index.css`

- Základná typografia: jemne stiahnutý tracking na UI veľkostiach, a
  **`tabular-nums` na všetkých dátach** (`.td`, `.th`, `.tnum`) — čísla už
  neposkakujú, keď sa menia číslice.
- Premenné pre povrchy a linky (`--surface`, `--surface-muted`, `--line`,
  `--line-soft`), definované pre light a dark **nezávisle**, nie inverziou.
- Prekopané: `.input`, `.label`, `.th`, `.td`, `.card`.
- Nové: `.section-title` (jeden štýl malého nadpisu nad blokom),
  `.field-invalid` (error state), `.skeleton`, `.card-interactive`,
  `.table-shell`, `.table-flush`, `.table-shell-compact`, `.row-selected`.
- `prefers-reduced-motion` vypína všetky prechody a animácie v celej appke.

### `src/components/ui.tsx` a `src/components/Layout.tsx`

Popísané nižšie vo vlastných sekciách.

---

## 2. Sidebar a layout

**Informačná architektúra je nedotknutá** — rovnaké položky, rovnaké poradie
(Dashboard, Tickets, Price Checker, Pulls, Finance, Ticket Center, Calendar),
rovnaké routy, rovnaká skupina "Tickets", rovnaké správanie theme togglu.

Čo sa zmenilo vizuálne:

- **Active state**: tichý brand-tintovaný povrch **plus krátky akcentový
  prúžok** pri ľavej hrane položky. V 192px širokom pruhu bol samotný tint
  ťažko rozoznateľný od hoveru — prúžok je to, čo dá aktuálnu stránku nájsť
  na prvý pohľad.
- **Hover** je jemnejší a všetky nav položky (vrátane hlavičky skupiny a
  theme togglu) používajú **doslova tie isté triedy** — nemôžu sa už
  rozísť.
- **Section separation**: tenké linky namiesto ťažších okrajov. Guide rail
  pod skupinou "Tickets" je 1px linka, nie border — skupina pôsobí odsadene,
  nie zaboxovane.
- **Logo lockup** dostal jemný ring a vlastnú linku pod sebou.
- **Profile area**: avatar je plná brand kružnica namiesto svetlého tintu,
  meno je výraznejšie, dropdown má nový overlay tieň a vlastné padding-y.
- **Šírka sidebaru zostáva `w-48` (192px)** — zúžil si ju sám v 2.0.30, aby
  tabuľky (Pulls) mali viac miesta. Priestor navyše si redesign vzal z
  vnútorného paddingu a menšieho typu, nie zo šírky tabuliek.
- **Layout gutter**: 24px → 28px horizontálne / 20px vertikálne. Žiaden
  `max-width` cap sa nevrátil (tvoje rozhodnutie z 2.0.31 platí).

**Page header** (`PageHeader`) je odteraz jeden tvar pre celú appku: title →
subtitle/meta → actions vpravo, a pod tým vlasová linka, ktorá oddeľuje
hlavičku od obsahu. 12 stránok ho už používalo, takže to prebrali samé.

---

## 3. Tabuľky — najdôležitejšia časť

Napísal si *"Toto je veľmi dôležité. TIQR je data-heavy appka."* Súhlasím,
tak sem šlo najviac práce.

**Problém, ktorý som našiel**: presne ten istý wrapper —

```
<div className="overflow-x-auto rounded-xl border border-slate-200
     dark:border-slate-800 bg-white dark:bg-slate-900 shadow-sm">
```

— bol skopírovaný okolo **14 tabuliek**, a `<thead>` s vlastným pozadím
okolo **21**. Takto sa veci rozchádzajú.

**Riešenie**: jeden `.table-shell` (a `.table-flush` pre tabuľky, ktoré už
vlastný scroll box majú). **Všetkých 22 tabuliek v appke je teraz naň
napojených** — Orders, Tickets, Sales, Listings, Inventory, Events, Event
Detail (5×), Order Detail, Sale Detail, Pulls (2×), Ticket Center, Finance
(3×), Price Checker (4×), Settings.

Čo z toho vyplýva:

- **Sticky header** — hlavička zostáva prilepená hore pri scrollovaní, vo
  všetkých tabuľkách naraz. A je **nepriehľadná**: presne ten typ chyby, kde
  sa v dark mode dá cez priesvitnú hlavičku vidieť text riadkov, si hlásil v
  2.4.4 a vtedy sa opravil na jednej stránke. Teraz je to opravené
  systémovo.
- **Interný scroll namiesto scrollovania celej appky** — dlhá tabuľka
  scrolluje sama v sebe pod pripnutou hlavičkou, page header a filtre
  zostávajú na mieste. Čo sa zmestí, nescrolluje vôbec.
- **Row hover** — jeden treatment, jemnejší než predtým (predtým bol hover
  presne tá istá farba ako pozadie hlavičky, čo bolo ploché).
- **Selected state** — `.row-selected`: ľavý akcentový prúžok + tint, nie
  plná modrá výplň. Pri 20 označených riadkoch to zostáva čitateľné. (Order
  Detail a Sale Detail na to prešli.)
- **Deliče riadkov** sú vlasové linky, nie plný border.
- **Čísla zarovnané** cez `tabular-nums` na `.td`.

**Čo som pri tabuľkách zámerne NEZMENIL** (a je to dôležité):

- Šírky stĺpcov, všetky `colgroup` percentá, `table-fixed`.
- Breakpoint `useNarrowTables()`.
- **`.th-c-narrow` / `.td-c-narrow` presné metriky** — `px-1` a `text-[11px]`
  zostávajú bajt na bajt. Tie hodnoty boli v 2.0.37 odmerané na reálnych
  dátach v troch locale, aby pri minimálnej šírke okna nikdy nevznikol
  horizontálny scrollbar. Zmenil som im len farbu a tracking.
- Obsah dát — ani jeden stĺpec nepribudol, nezmizol ani sa neprepočítal.

Jedna vec na priznanie: `.table-shell` má strop výšky
`calc(100vh - 13.5rem)`, čo je odhad "page header + jeden riadok filtrov nad
tabuľkou". Na stránke s vyššou hlavičkou bude okrem tabuľky trochu scrollovať
aj stránka. Má to únikový ventil (`--table-inset`), a Event Detail, kde sú
dve tabuľky nad sebou na jednom tabe, používa `.table-shell-compact` s pevným
stropom. Ak niekde uvidíš dva scrollbary naraz, povedz mi ktorá stránka a
doladím jej inset — je to zmena jedného čísla.

---

## 4. Karty / KPI

`StatCard` (52 použití) je nižšia a tichšia — presne tvoje *"Karty nemajú byť
obrovské"*:

- Padding `p-4` → `p-3.5`, hodnota `text-2xl` → `22px` s `leading-none`.
- Label je menší a tichší (`.section-title`), **číslo je jediná hlasná vec na
  karte**.
- Trend a secondary info sú v jednom tesnom meta riadku pod tým.
- Jemný border + `shadow-card`, hover lift **len tam, kde je karta naozaj
  klikateľná** (`card-interactive` / `interactive` prop) — statická karta,
  ktorá sa dvíha pod kurzorom, pôsobí ako chyba.

Na rovnaké metriky som previedol aj **Dashboard Attention boxy** a **Ticket
Center filter dlaždice**, aby KPI dlaždica a Attention dlaždica boli
viditeľne ten istý objekt. Ich selected state je teraz brand ring namiesto
hrubšieho borderu — vďaka tomu rad piatich dlaždíc pri preklikávaní
nepodskakuje o pixel.

---

## 5. Formuláre

- `.input` je spoločný pre `Input`, `Select`, `Textarea` — nový hover state
  (border stmavne), a **focus je mäkké brand halo** (`shadow-focus`) namiesto
  tvrdého 2px ringu. V hustom formulári focus už nekričí.
- **Error state**: `Field` teraz zafarbí aj samotný ovládací prvok, nielen
  text pod ním. Spravené cez wrapper triedu `.field-invalid`, takže
  **všetkých 127 existujúcich `<Field>` to dostalo bez toho, aby sa čo i len
  jedno volanie muselo zmeniť**. Error ring je shadow, nie border → nič
  neposkočí.
- `label` má viac priestoru pod sebou, hint a error text sú konzistentné.
- Checkbox (`CHECKBOX_CLASS`) prešiel na rovnaký focus-visible ring.
- **Focus všeobecne**: celá appka prešla na `focus-visible` — klik myšou už
  nenechá za sebou ring, klávesnica áno.

`Button` dostal voliteľný `size` (`sm` / `md`), default `md`, takže **každé
zo 160 existujúcich volaní vyzerá presne tak veľké ako predtým**. Press
feedback je jemnejší (0.98 namiesto 0.97).

---

## 6. Statusy

Tvoje *"BUSINESS LOGIC SA NESMIE MENIŤ"* — nezmenila sa. `STATUS_TONES` má
**presne tie isté kľúče** a presne to isté mapovanie hodnôt ako v 2.5.2
(available, listed, sold, cancelled, upcoming, completed, unpaid, partial,
paid, pending, refunded, demo, active, soldout, unlisted, delivered,
not delivered, mixed). Zmenil sa len recept, ako každý z nich vyzerá:

- Jeden vzorec pre všetky: tintovaný povrch + zodpovedajúci text + vlasový
  inset ring, v oboch režimoch.
- **Vedúca bodka** pred textom — dedí `currentColor`, takže je automaticky
  správna pre každý tón bez druhej tabuľky, ktorú by bolo treba udržiavať.
- `InlineStatusSelect` (editovateľný status na Sale Detail / Order Detail) má
  **ten istý tvar, ring aj bodku** ako read-only `Badge` — editovateľný a
  needitovateľný status teraz pôsobia ako ten istý objekt. Správanie
  (uloženie pri výbere, saving state) je nedotknuté.

---

## 7. Taby

V appke boli **dva vzhľady toho istého** — zdieľaný `TabSwitcher` a tri ručne
prepísané rady na Dashboarde. Teraz je jeden: `SEGMENTED_TRACK` +
`segmentedItemClass` v `ui.tsx`.

Prešli naň:

- `TabSwitcher` (Events / Orders / Tickets / Sales — Active vs Completed)
- Dashboard: hlavný tab row, period picker, metric picker
- Welcome: Log in / Sign up
- Calendar: prev / Today / next

Vzhľad: **zapustená dráha s vyvýšeným svetlým "thumbom"** na aktívnom tabe,
namiesto plnej brand modrej výplne. Bežný filter zoznamu nemá byť
najhlasnejší prvok na stránke.

---

## 8. Light / Dark

Nie inverzia. Light a dark majú v `index.css` **nezávisle napísané** premenné
a v `tailwind.config.js` nezávisle preladené stupne škály:

| | Light | Dark |
|---|---|---|
| Pozadie appky | `slate-50` | `slate-950` (nadvihnuté, odmodrené) |
| Povrch (karty, sidebar, tabuľky) | biela | `slate-900` |
| Tlmený povrch (hlavičky tabuliek, footer modalu) | `slate-50` | vlastná hodnota, nie inverzia |
| Linky | `slate-200` | `slate-800` |
| Hĺbka | dvojvrstvový tieň | 1px svetlý highlight navrchu |

Prešiel som to cez všetky plochy, ktoré si menoval: sidebar, karty, tabuľky,
modaly, formuláre, Calendar, Price Checker, Finance.

---

## 9. Animácie a efekty

Presne v tvojom rozsahu 120–180ms, nič dlhšie a nič permanentné:

- Default prechod 150ms s jednou spoločnou krivkou.
- Hover elevation len na klikateľných kartách.
- Button press feedback.
- Modal / dialog / dropdown entrance (existujúce `fadein` / `pop-in`, len
  jemnejšia štartovacia mierka — pôsobí to ako nadvihnutie, nie ako zoom).
- Selected/tab/row prechody sú farebné, nie pohybové.
- **`prefers-reduced-motion` vypína všetko.** Jeden detail: nechal som
  duration na `0.01ms` namiesto `none` schválne — `lib/toast.tsx` sekvencuje
  zmiznutie toastu cez `animationend`, takže animácia, ktorá by nikdy
  nebežala, by ho zasekla.

---

## 10. Loading a empty states

- Nové `Skeleton` a `TableSkeleton` v `ui.tsx`.
- **Šesť zoznamových stránok** (Orders, Sales, Tickets, Events, Pulls,
  Ticket Center) drží pri načítaní svoj layout skeletonom namiesto toho, aby
  spadli na vycentrovaný spinner a potom poskočili.
- Ostatné `LoadingBlock` volania som **nechal tak schválne** — skeleton má
  zmysel len tam, kde je známy tvar toho, čo príde. Napísal si "ale
  nepreháňaj to".
- `EmptyState` (30 použití): ikona v mäkkom ohraničenom medailóne namiesto
  veľkého sivého glyfu, reálny povrch, lepšia hierarchia textu. Action
  tlačidlo tam, kde už existovalo — **žiadne nové som nepridal**.

---

## 11. Jednotlivé stránky

- **Dashboard** — tri ručné tab rady prešli na zdieľaný segmented control;
  Attention boxy majú metriky StatCard a ring selection; veľké číslo pri
  grafe je rovnako veľké ako každé iné KPI; dropdown má overlay tieň.
  Business obsah nedotknutý.
- **Calendar** — logika nedotknutá (`get_calendar` params rovnaké, navigácia
  rovnaká). Vizuálne: vyššie riadky mesiaca (96 → 104px) a týždňa (220 →
  240px), dnešná bunka má ring nie len tint, dnešné číslo je brand kruh,
  víkendy a dni mimo mesiaca majú vlastné jemné odtiene, chipy majú viac
  vzduchu a hover ring namiesto `brightness` filtra (ten v dark mode vyžieral
  tintované chipy), prev/Today/next prešli na zdieľaný segmented control.
- **Ticket Center** — order-based logika nedotknutá. Filter dlaždice na
  metriky StatCard, riadky a badge cez zdieľanú tabuľkovú vrstvu, skeleton
  pri načítaní.
- **Price Checker** — **scanner logic, manual scan, visible WebView scanner,
  Market Analysis, Tier/Level logika aj história sú úplne nedotknuté.**
  Ani jeden riadok logiky. Vizuálne prevzal: 4 vnútorné tabuľky na
  `.table-flush` so sticky hlavičkami, `.section-title` na 6 miestach,
  spoločné karty/badge/inputy. Žiadny monitoring, žiadny Auto Monitor,
  žiadne Live Event Intelligence, žiadne automatic repricing.
- **Finance** — **money logic, cents, transactions, accounts, transfers,
  výpočty úplne nedotknuté.** Tri tabuľky (Overview / Transactions /
  Accounts / Reports) prevzali tabuľkovú vrstvu, zvyšok prišiel automaticky
  cez Card/Badge/Input/Button.
- **Events / Orders / Tickets / Inventory / Sales / Listings / Event Detail /
  Order Detail / Sale Detail / Pulls** — prevzali zdieľanú vrstvu, žiadna
  vlastná zmena.
- **Settings** — import preview tabuľka na `.table-flush`, `.section-title`
  na 4 miestach, zvyšok cez zdieľané komponenty.
- **Welcome / Reset Password / Pending Approval / Database Error** — logo
  lockup a Log in / Sign up switch na zdieľaný segmented control; tieňová
  škála zjednotená.

---

## 12. Performance

- **Žiadna nová dependency**, žiadna animation library.
- Startup: nezmenený — nepridal som žiadny web font (appka je offline-first;
  "Inter" je v stacku, ale nikdy sa nesťahoval a naďalej sa nesťahuje, takže
  padá na systémové písmo).
- Tabuľky: **menej DOM tried a menej inline utility tried než predtým** —
  hover, deliče a hlavička idú z CSS, nie z per-riadok tried.
- Re-rendery: žiadne nové. Nepribudol ani jeden `useState`, `useEffect` ani
  context.

---

## 13. Zmenené súbory

**Design vrstva (4):**
`tailwind.config.js`, `src/index.css`, `src/components/ui.tsx`,
`src/components/Layout.tsx`

**Stránky, ktoré prevzali vrstvu (21):**
`Calendar.tsx`, `Dashboard.tsx`, `DatabaseError.tsx`, `EventDetail.tsx`,
`Events.tsx`, `OrderDetail.tsx`, `Orders.tsx`, `PendingApproval.tsx`,
`PriceChecker.tsx`, `Pulls.tsx`, `ResetPassword.tsx`, `SaleDetail.tsx`,
`Sales.tsx`, `Settings.tsx`, `TicketCenter.tsx`, `Tickets.tsx`,
`Welcome.tsx`, `finance/Accounts.tsx`, `finance/Overview.tsx`,
`finance/Reports.tsx`, `finance/Transactions.tsx`

**Komponent (1):** `components/UpdateOverlay.tsx` (len tieň)

**Verzia (7 súborov, 9 výskytov):** `package.json`,
`src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `release.ps1`
(`$Version` + `$CommitMsg`), `1-CLICK-UPDATE.bat` (title + echo),
`src-tauri/Cargo.lock`, `package-lock.json`

**Dokumentácia (4):** `PROJECT_STATE/CURRENT_STATE.md`,
`PROJECT_STATE/PROTECTED_AREAS.md`, `CHANGELOG.md`, tento report

**Backend: 0 súborov.**

---

## 14. Testovanie

### Čo sa NEDALO overiť

`npx tsc -b`, `npm run build`, `cargo check --lib`, `cargo test --lib` —
**ani jedno**. Na stroji nie je Node.js ani Rust. Toto je jediná reálna
medzera tohto releasu a musíš ju zavrieť ty na Windowse pred publikovaním.

### Čo som overil staticky (a čo to reálne dokazuje)

| Kontrola | Výsledok |
|---|---|
| `src-tauri/src` vs. 2.5.2 (`diff -rq`) | **IDENTICAL** — backend sa nedotkol |
| Vyváženie `{}` `()` `[]` vo všetkých 25 zmenených súboroch | **žiadny drift** oproti originálu (porovnané po súboroch, nie absolútne) |
| Každý import z `components/ui` | **všetky sa rozlišujú** na reálny export |
| Chýbajúce importy (použitý symbol bez importu) | **0** |
| Nepoužité importy | 1 (`Button` v `Dashboard.tsx`) — **existoval už pred mojou zmenou**, a `noUnusedLocals: false`, takže build to nezhodí; nechal som ho (nie je to môj task) |
| `<table>` vs. `<thead>` vs. wrapper | 22 / 22 / 22 — **každá tabuľka je v shell alebo flush wrapperi, žiadny thead si nenechal vlastný styling** |
| Každý `shadow-*` v TSX | 22 výskytov, **všetky sa mapujú** na `card`/`raised`/`overlay` v configu |
| Každý `theme("...")` v CSS | 11 referencií, **všetky existujú** v configu |
| Vyváženie zátvoriek v `index.css` | **0** |
| Tailwind v3 kompatibilita rizikových tried | prešiel som ich ručne; `ring-current/30` som našiel a nahradil (v3 nevie alfa na `currentColor`), a shimmer keyframe som presunul z configu do `index.css` (v3 nevygeneruje `@keyframes` pre `animate-[...]` arbitrary hodnotu) |
| Celý diff filtrovaný na `api.` / state / handlery / routing | **0 zásahov do logiky** |

### Čo treba pozrieť očami po builde

Prejdi Light aj Dark na: Dashboard, Events, Orders, Tickets, Inventory,
Sales, Listings, Finance, Ticket Center, Calendar, Price Checker, Settings.
Konkrétne sa pozri na:

1. **Sticky hlavičky tabuliek** — či je hlavička naozaj nepriehľadná pri
   scrollovaní (hlavne v dark mode).
2. **Či niekde nie sú dva scrollbary naraz** (tabuľka + stránka). Ak áno,
   povedz mi stránku — doladím jej `--table-inset`.
3. **Úzke okno** — či `.th-c-narrow` tabuľky (Sales, Orders, Pulls) stále
   nemajú horizontálny scrollbar. Metriky som nemenil, ale over to.
4. **Modal footer** — či siaha po okraje panelu (je full-bleed cez záporné
   marginy naviazané na padding modalu).
5. **Formulárový error state** — ulož niečo neplatné a pozri, či sa pole
   sfarbí na červeno.

---

## 15. Čo sa NEZMENILO — biznis stránka

Explicitne, aby to bolo čierne na bielom:

- refund / resell — nedotknuté
- `batch_id` — nedotknuté
- money / integer cents — nedotknuté
- Orders core logic — nedotknuté
- Tickets core logic — nedotknuté
- Sales core logic — nedotknuté
- Listings core logic — nedotknuté
- Finance logic — nedotknuté
- Fulfillment logic — nedotknuté
- Attention logic — nedotknuté
- Calendar logic — nedotknuté
- Google Sheets logic — nedotknuté
- Price Checker scanner logic — nedotknuté
- Databáza, schéma, migrácie — nedotknuté (ďalšia nová je **027**)
- Routy a navigácia — nedotknuté
- Status vocabulary (ktoré statusy existujú a čo znamenajú) — nedotknuté

A žiadna z vecí zo tvojho zákazového zoznamu: žiadny command palette, žiadny
global search, žiadne notifikácie, žiadne AI, žiadny monitoring, žiadna
marketplace automation, žiadne nové dashboard widgety, žiadne nové moduly,
žiadny nový workflow.

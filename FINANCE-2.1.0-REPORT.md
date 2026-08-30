# TIQR Manager 2.1.0 — FINANCE 2.1

Toto je pokračovanie Financií z 2.0.83. Poslal si mi kompletné zadanie "TIQR MANAGER — FINANCE 2.1" (21
bodov) — peňaženky (Accounts), presuny medzi nimi (Transfers), opakované výdavky (Recurring Expenses),
jednoduchá prognóza cashflow (Cashflow Forecast) a rozdelenie Financií na 4 karty (Overview / Transactions
/ Accounts / Reports namiesto jednej dlhej stránky). Postavil som presne podľa zadania, autonómne, bez
ďalších otázok — presne ako si žiadal.

## Audit existujúcich Financií (odkiaľ som staval)

Pred písaním čo i len jedného riadku kódu som si znovu prešiel celé 2.0.83: `finance_entries`/
`finance_categories` tabuľky, `commands/finance_entries.rs`, `Finance.tsx` (854 riadkov, jedna stránka).
Dôležité zistenie, ktoré určilo celý dizajn: Financie sú **úplne samostatný, ručne vedený zoznam** — nič
tam nie je prepojené s Orders/Sales (biznis peniaze z predaja lístkov). Toto pravidlo som nemenil a nová
funkcionalita ho rešpektuje rovnako — Account je len "miesto", na ktoré môže (nemusí) ukazovať existujúci
záznam alebo nová šablóna opakovaného výdavku, nič viac. Tvoje peniaze z predaja lístkov (Orders/Sales/
Dashboard) a tvoje peniaze vo Financiách sú stále dva úplne oddelené svety, presne ako doteraz.

## 1. Accounts (peňaženky)

Nová karta **Accounts** — pridáš si tam banku, Revolut, PayPal, hotovosť, kreditku alebo "iné", každú s
vlastnou menou a počiatočným zostatkom. **Aktuálny zostatok sa nikde neukladá, počíta sa vždy nanovo** z
počiatočného zostatku plus všetky príjmy/výdavky/presuny, ktoré si na ten účet zapísal — takže nikdy
nemôže "vypadnúť zo synchronizácie" sám od seba. Účet vieš upraviť alebo vypnúť (Active/Inactive — vypnutý
sa naďalej zobrazí, len sa neráta do celkových súčtov), zmazať sa dá kedykoľvek okrem prípadu, že naň ešte
ukazuje nejaký presun (to appka nedovolí, kým ten presun nezmažeš — aby žiadny presun nezostal "zavesený"
len na jednej strane).

Jedna vec, ktorú appka po vytvorení účtu už nedovolí zmeniť: **menu účtu**. Dôvod: zostatok účtu je súčet
súm v jednej mene — keby si menu zmenil dodatočne, číslo by zrazu neznamenalo to, čo hovorí (nič by sa
neprepočítalo). Ak si sa pri zakladaní pomýlil, jednoduchšie a bezpečnejšie je účet zmazať a založiť nanovo.

## 2. Transfery (presuny medzi tvojimi účtami)

Tlačidlo **"New transfer"** (na karte Accounts) — vyberieš Z akého účtu, Na aký účet, sumu a dátum. Mena sa
**nedá vybrať ručne** — vždy sa automaticky použije mena účtu, z ktorého posielaš (appka ti aj v ponuke "Na
účet" rovno ukáže len účty v tej istej mene, aby si sa ani nemohol pomýliť). Presun medzi dvoma rôznymi
menami appka v tejto verzii zámerne nedovolí — neexistuje "vymyslený" kurz, ktorý by to prepočítal, takže
radšej to úplne zablokuje, než aby ti dal nesprávne číslo. Ak by si takýto presun potreboval, over si kurz
sám a zapíš to ako dva samostatné záznamy (výdavok na jednom účte, príjem na druhom).

Presun **nikdy nie je príjem ani výdavok** — nezapočíta sa nikde do Profit & Loss, len presunie peniaze
medzi tvojimi vlastnými účtami (jeden účet klesne, druhý stúpne o presne tú istú sumu). Zoznam všetkých
presunov nájdeš v **Transactions** (spolu so všetkými ostatnými záznamami) — na karte Accounts je len
tlačidlo na vytvorenie nového, aby si históriu presunov nemusel hľadať na dvoch miestach.

## 3. Opakované výdavky (Recurring Expenses)

Tiež na karte Accounts, časť **"Upcoming recurring expenses"**. Založíš si šablónu (nájomné, predplatné,
poistenie...) s čiastkou, frekvenciou (týždenne/mesačne/štvrťročne/ročne) a dátumom prvého výskytu. **Nič sa
nezapisuje samo od seba** — appka nič nekontroluje na pozadí a nič nespúšťa pri otvorení appky. Pri každej
položke máš 3 tlačidlá:

- **Create** — zapíše dnešný výskyt ako skutočný výdavok (uvidíš ho v Transactions) a posunie šablónu na
  ďalší termín.
- **Skip** — len posunie šablónu na ďalší termín, bez zápisu výdavku (napr. mesiac, keď si to nezaplatil).
- **Pause / Resume** — pozastaví/obnoví šablónu. Kým je pozastavená, termín sa vôbec nehýbe — ak prejde
  čas, po obnovení sa jednoducho zobrazí ako "Overdue" (omeškané) a ty si to vybavíš jedným klikom, appka
  sa nič nesnaží dobiehať sama.

Toto som si aj vyslovene otestoval na scenár "čo ak appku otvorím 5-krát za sebou" — nič sa nezdvojí, kým
sám neklikneš Create.

## 4. Prognóza cashflow (Cashflow Forecast)

Na karte **Overview**, karta "Cashflow Forecast". Jednoduchý, nie AI odhad na najbližších 30 dní: **Aktuálny
zostatok** (súčet aktívnych EUR účtov) + **Očakávaný príjem** (naplánované budúce príjmy + EUR platby, na
ktoré ešte čakáš z predaja lístkov) − **Opakované výdavky** (čo je splatné v tomto okne) − **Ostatné
plánované výdavky** = **Prognóza zostatku**. Ak nemáš žiadny aktívny EUR účet, karta poctivo napíše, že
prognóza nie je k dispozícii, namiesto vymysleného čísla. Ak máš niečo v inej mene, na to ťa karta upozorní
zvlášť (nič sa neháda, žiadny kurz sa nevymýšľa).

Jedna technická poznámka, ktorá ťa asi nebude zaujímať v detaile, ale pre istotu: "Aktuálny zostatok" tu je
mierne iné číslo než "Current Balance" na Overview vyššie — Forecast počíta len s tým, čo sa reálne stalo
**do dnešného dňa** (aby sa budúci naplánovaný príjem nezapočítal dvakrát), zatiaľ čo bežný "Current
Balance" ukazuje úplne všetko vrátane akéhokoľvek budúceho záznamu, ktorý si si už vopred zapísal. V praxi
to väčšinou vyjde na to isté číslo, len keď si niečo zapíšeš s dátumom v budúcnosti, uvidíš mierny rozdiel
— to je správne, nie chyba.

## 5. Overview (prehľad)

Rovnaké obdobie/rozsah filtre ako doteraz, plus 2 nové veci: karta **"Current Balance"** (skutočný súčet
tvojich aktívnych EUR účtov, nezávislé od zvoleného obdobia — "koľko mám teraz") a karta **"Pending /
Outstanding"** (koľko čakáš od kupujúcich za nezaplatené predaje lístkov — to isté číslo, čo už poznáš z
Dashboardu, len tu prehľadne pri peniazoch). Plus nová karta **Cashflow Forecast** (bod 4 vyššie). Tabuľka
záznamov sa z Overview presunula do Transactions (nižšie) — Overview je teraz čisto prehľad na pozretie,
úpravy/mazanie záznamov robíš na karte Transactions.

## 6. Transactions

Toto je teraz hlavný zoznam — **všetky záznamy AJ všetky presuny na jednom mieste**, zoradené podľa dátumu.
Presun sa dá spoznať podľa modrého štítku "Transfer" a šípky (Z účtu → Na účet). Filtre: obdobie, rozsah
(Osobné/Biznis — presuny naň nereagujú, nemajú "rozsah"), typ (Príjem/Výdavok/Presun), účet, kategória,
hľadanie. Tlačidlo **"New entry"** aj formulár na úpravu/zmazanie záznamu sú presne tie, čo si poznal z
2.0.83, len s jedným novým poľom navyše — **Account** (voliteľné, ponuka sa sama obmedzí len na účty v tej
istej mene ako si zvolil pri sume).

## 7. Reports

Nová karta so 4 prehľadmi, počítanými priamo z tvojich dát (žiadne nové čísla z appky, len iný pohľad na tie
isté záznamy):

- **Profit & Loss** — Príjem / Výdavky / Zisk-strata za zvolené obdobie.
- **Cash Flow** — Počiatočný zostatok / Príjmy / Výdavky / Presuny / Konečný zostatok.
- **Expenses by Category** a **Expenses by Account** — kde presne miznú peniaze, podľa kategórie a podľa
  účtu (rebríček + percentá).
- **Business vs Personal** — porovnanie osobných a biznis príjmov/výdavkov vedľa seba.

**Dôležitá poznámka, aby ťa čísla neprekvapili:** Cash Flow report môže ukázať MENŠIE čísla než Profit &
Loss za to isté obdobie — to nie je chyba. Profit & Loss počíta úplne všetky tvoje záznamy. Cash Flow počíta
len tie, čo majú priradený konkrétny účet (lebo len tie reálne menia zostatok nejakej peňaženky) — a keďže
si účty zaviedol až teraz, väčšina tvojich starších záznamov žiadny účet priradený nemá. Presne túto medzeru
ti ukáže report "Expenses by Account" — riadok "No account" ti povie, koľko výdavkov ešte nemá účet.
Postupne, ako budeš nové záznamy zapisovať s účtom, sa Cash Flow bude Profit & Loss číslu približovať.

Riadok "Transfers" v Cash Flow vždy ukáže presne €0.00 — to je správne, nie chyba: presun len presúva
peniaze medzi tvojimi vlastnými účtami, tvoj celkový súčet sa tým nikdy nezmení. Nechal som ten riadok vo
výpočte viditeľný (presne ako si žiadal v zadaní), len s krátkym vysvetlením priamo v appke.

**Čo som NEurobil (tvoj bod 13, ktorý si sám označil ako nepovinný "ak je to možné bez veľkého zásahu"):**
rozdelenie biznis výdavkov na "z predaja lístkov" vs. "ostatné biznis". Skúmal som to poctivo, ale appka
dnes nikde neeviduje, ktorá kategória/záznam súvisí s predajom lístkov — bolo by treba buď zmeniť databázovú
štruktúru (nový stĺpec), alebo hádať podľa názvu kategórie (nespoľahlivé, mohol by si mať kategóriu
pomenovanú akokoľvek). Keďže si to sám podmienil "bez veľkého zásahu", radšej som to vynechal, než aby som
urobil jedno z tých dvoch narýchlo. Ak to chceš, poviem ti obe možnosti a ty si vyberieš.

## Databáza / migrácie

Jedna nová migrácia, `016_finance_v2.sql` — len pridáva, nič nemaže ani neprepisuje (rovnaké pravidlo ako
vždy doteraz). 3 nové tabuľky: `accounts`, `transfers`, `recurring_expenses`, plus `finance_entries` dostal
nový voliteľný stĺpec `account_id`. Existujúce záznamy nikto nemenil — každý starý záznam jednoducho zostal
bez priradeného účtu (`account_id = NULL`), presne ako report vyššie vysvetľuje.

## Zmenené súbory

**Backend (Rust) — nové súbory:**
- `src-tauri/migrations/016_finance_v2.sql`
- `src-tauri/src/commands/finance_accounts.rs` (Accounts + Transfers)
- `src-tauri/src/commands/finance_recurring.rs` (Recurring Expenses)
- `src-tauri/src/commands/finance_forecast.rs` (Cashflow Forecast)

**Backend — upravené súbory:**
- `src-tauri/src/models.rs`, `src-tauri/src/db.rs`, `src-tauri/src/lib.rs`,
  `src-tauri/src/commands/mod.rs` — registrácia nových typov/príkazov/migrácie
- `src-tauri/src/commands/finance_entries.rs` — pridané prepojenie na účet + kontrola, že mena záznamu sedí
  s menou účtu
- `src-tauri/src/commands/database.rs` — jeden existujúci test upravený (očakávaný počet migrácií 15 → 16)

**Frontend — nové súbory:**
- `src/pages/finance/shared.ts`, `Overview.tsx`, `Transactions.tsx`, `Accounts.tsx`, `Reports.tsx`

**Frontend — upravené súbory:**
- `src/pages/Finance.tsx` — prerobené z jednej dlhej stránky na tenký kontajner so 4 kartami vyššie
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy a volania

## Testy

Napísal som 52 nových testov (16 pre účty/presuny, 19 pre opakované výdavky vrátane presného počítania
mesiacov cez rôzne dĺžky mesiacov a priestupný rok, 11 pre prognózu vrátane presného prepočtu podľa tvojho
vlastného príkladu zo zadania, 6 na prepojenie záznamu s účtom).

```
cargo test --lib   -> 836 testov, 0 zlyhaní, 3 ignorované (784 pôvodných + 52 nových)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.1.0 build" v hlavičke)
```

## Výsledky buildu

Frontend aj backend som overoval priebežne po každej novej karte (nie až na konci) — `tsc -b` a `npm run
build` boli čisté po Overview, po Transactions, po Accounts aj po Reports, a ešte raz po zvýšení verzie.
Rust testy som spustil znova po zvýšení verzie, aby som mal istotu, že zmena verzie nič nepokazila (836/836,
rovnako ako predtým).

## Regresné testovanie

Všetkých 784 pôvodných testov (vrátane všetkých predošlých BUG-fixov a Financií z 2.0.83) prešlo bez zmeny —
nič z existujúcej logiky som neupravoval, len pridával. Rovnako `tsc -b`/`npm run build` neukázali žiadnu
novú chybu v žiadnom existujúcom súbore.

## Čo som NEmenil

Presne podľa tvojho zoznamu: predaj lístkov, Sales, Objednávky, Sklad (Inventory), refund/resell logika,
zoskupovanie predajov (SaleGroup, `batch_id`), `finance.rs`, `money.rs` (peniaze stále ako celé centy),
Backup/Restore, existujúci CSV import, Google Sheets synchronizácia, Price Checker. Žiadny z týchto súborov
som ani neotvoril na úpravu — len na čítanie, aby som overil, že sa ich nová funkcionalita naozaj netýka.

## Budúce vylepšenia (nápady, nie sľuby)

- **Ticket-business vs. Other-business** (tvoj bod 13) — pozri vyššie, potrebujem tvoje rozhodnutie medzi
  novým stĺpcom v databáze alebo odhadom podľa názvu kategórie.
- **Export do CSV** pre Accounts/Transfers/Recurring Expenses — každý iný zoznam v appke ho má, tento zatiaľ
  nie (rovnaké odloženie ako Financie samotné mali v 2.0.83).
- Žiadna zmena bankového pripojenia/AI/DPH/mzdy — presne ako si v zadaní vylúčil.

## Verzia

9 miest v 7 súboroch: `2.0.83` → `2.1.0`.

## STOP

FINANCE 2.1 hotové, otestované a zabalené. Skontroluj:

1. Na karte **Accounts** si založ 2 účty v EUR (napr. "Banka" a "Hotovosť") s nejakým počiatočným
   zostatkom.
2. Skús **"New transfer"** — presuň časť peňazí medzi nimi, over si, že sa oba zostatky správne zmenili.
3. Založ si **opakovaný výdavok** (napr. nájomné) a skús **Create** aj **Skip** — over si, že sa termín
   posunul a že Create pridal záznam aj do Transactions.
4. Skús ho aj **Pause**/**Resume**.
5. Na **Overview** skontroluj novú kartu "Current Balance" a "Cashflow Forecast".
6. Na **Transactions** over, že vidíš aj svoj presun z bodu 2 v zozname (modrý štítok "Transfer").
7. Prezri si **Reports** — všetky 4 prehľady, hlavne si všimni poznámku o Cash Flow vs. Profit & Loss
   vysvetlenú vyššie, nech ťa to neprekvapí.

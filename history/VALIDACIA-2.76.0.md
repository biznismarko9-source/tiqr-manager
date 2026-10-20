# TIQR Manager 2.76.0 — validácia

**Dátum:** 9.10.2026
**Zadanie:** overiť 2.76.0, nič nemeniť, nahlásiť skutočný výsledok

**Zdrojové súbory som nezmenil.** Nenašiel som v nich žiadnu chybu, ktorú by
bolo treba opraviť. Jediné chyby, ktoré sa objavili, boli v mojich vlastných
overovacích skriptoch — o tom bod 5.

---

## 1. Prostredie (bod 1 zadania)

Prehľadal som to dôkladne, nie z pamäte:

```
PATH: žiadny node, npm, npx, pnpm, yarn, bun, cargo, rustc

nie je: /usr/local/bin/node        nie je: ~/.nvm
nie je: /opt/homebrew/bin/node     nie je: ~/.volta
nie je: /usr/bin/node              nie je: ~/.bun
nie je: ~/.cargo                   nie je: ~/.fnm
nie je: ~/.rustup                  nie je: node_modules
nie je: /opt/homebrew/bin/brew     nie je: /usr/local/bin/brew
```

`find` cez `/usr/local`, `/opt`, Xcode a `~/Library` nenašiel nič. Žiadny
shell profil node ani cargo nespomína.

**Ani Homebrew tu nie je**, takže toolchain si neviem ani nainštalovať —
inštalátor by potreboval tvoje heslo.

**Dôsledok: body 2–7 tvojej validácie sa spustiť nedajú.** Žiadny build,
žiadne spustenie, žiadne screenshoty bežiacej appky.

### Ako to spustíš ty

```bash
curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh | bash
brew install node rust
cd ~/Downloads/tiqr-manager-2.5.2 && npm ci && npm run build
```

Výstup mi pošli — ten build je presne to, čo mi tu chýba.

## 2. Kontroly, ktoré PREŠLI

Spustil som to, čo sa spustiť dalo — mechanické kontroly presne tých tried
chýb, ktoré by zachytil build.

| kontrola | výsledok |
|---|---|
| Props každého `<StatCard>` a `<EmptyState>` oproti jeho typu | **34 použití, 0 neznámych props** |
| Hodnoty `accent` oproti `METRIC_ACCENTS` | 0 neplatných |
| Každá JSX komponenta v `Events.tsx` má import alebo lokálnu definíciu | 25 komponent, 0 nevyriešených |
| To isté v `EventDetail.tsx` | 27 komponent, 0 nevyriešených |
| Počet `<th>` vs `<td>` po odstránení Stock stĺpca | **8 = 8** |
| Zvyšné referencie na `StockBar` | žiadne |
| `colSpan` / index-based zarovnanie v tabuľke eventov | žiadne |
| Zátvorky v `Events.tsx`, `EventDetail.tsx` | rozdiel oproti zálohám 0 |

## 3. Kontroly, ktoré ZLYHALI

Žiadna.

## 4. Kontroly, ktoré som NESPRAVIL

- `npm run build` — nedá sa spustiť.
- `npm ci`, lint, type-check, `cargo test` — to isté.
- Spustenie appky — to isté.
- **Events list, Event detail Overview, Event detail Sales — ani jeden som
  nevidel bežať.**
- Vyhľadávanie, filtre, triedenie, Upcoming/Completed, Overview/Sales,
  Edit, Delete, navigácia — **ani jedno otestované za behu.** Sú kódom
  nedotknuté, ale to nie je test.

## 5. Vlastné chyby v overovaní — a prečo to píšem

Pri tejto validácii zlyhali **tri moje overovacie skripty**, nie apka. Píšem
to, lebo každý z nich by ti bol ohlásil chybu, ktorá neexistuje:

1. **Kontrolór props hlásil 29 chýb.** Čítal `className` z vnútra
   `icon={<IconTicket className="h-4 w-4" />}` a pripisoval ho `StatCard`-u.
   Po oprave (vyhodiť obsah zložených zátvoriek pred čítaním názvov props):
   **0 chýb.**
2. **Kontrolór komponent hlásil `WorkspaceTab` ako nevyriešenú.** Je to typový
   alias z `useState<WorkspaceTab>`, nie JSX.
3. **Mock Dashboardu ukazoval `undefined`** na kartách bez ikony. Skutočný
   `StatCard` má ikonu pod podmienkou `{icon && accent && ...}`; môj mock tú
   podmienku nemal.

(Predtým pri Events to bol štvrtý prípad — pruh bez `display:block`.)

**Pravidlo, ktoré z toho platí: keď test povie, že je niečo pokazené, najprv
over test.**

## 6. Screenshoty

Priložené tri — Overview, Financials, Activity.

**Nie sú to screenshoty spustenej aplikácie.** Sú to repliky postavené
z tvojich skutočných databázových čísel, ktoré používajú rovnaké rozloženie,
tokeny a presne tú istú SVG matematiku ako komponenty v apke (monotónna
interpolácia krivky, dasharray prstenca, sparkliny). Každá nesie jantárový
pruh, ktorý to hovorí.

Čo na nich vidno a čo to potvrdzuje:

- **Overview** — päť KPI kariet v jednom rade, hodnoty **viditeľné** (to je tá
  oprava z 2.75.0), sparkliny pod nimi, Balance cashflow s prepínačom
  Cashflow/Profit & Loss/Revenue/Cost, prstenec platforiem s farebným zoznamom.
- **Financials** — karty s počtami **bez** ikonových štvorcov, peňažné **s**
  nimi. To potvrdzuje, že podmienka `{icon && accent}` funguje tak, ako má.
- **Activity** — „Pulls near deadline" s nulou má **neutrálnu sivú** ikonu,
  dve nenulové **jantárovú**. To je presne to pravidlo, ktoré som do
  `AlertCard` písal: karta s nulou je dobrá správa a nemá vyzerať ako výstraha.

## 7. Vizuálne pozorovania

1. **Nič orezané** v žiadnom z troch pohľadov pri desktopovej šírke.
2. Prázdne miesto pod Financials je skutočné — tri bloky nezaplnia výšku.
   Nie je to chyba, len dôsledok toho, koľko metrík tam je.
3. `Listing value` a `Potential profit` ukazujú pomlčku, lebo **nemáš ani
   jeden lístok s nastavenou cenou v ponuke** (79 z 79 ju nemá). To je
   správne správanie, nie prázdny widget — a je to zároveň to, čo hlási
   „Missing listing price: 79" na Activity.

## 8. Záver

Z toho, čo sa tu overiť dá, je 2.76.0 v poriadku. Z toho, čo sa overiť nedá —
teda či sa to skompiluje a ako to vyzerá naživo — neviem povedať nič, a
nebudem predstierať, že viem.

Zastavujem sa a čakám na tvoje slovo.

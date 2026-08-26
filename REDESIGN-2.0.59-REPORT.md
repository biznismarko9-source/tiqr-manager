# TIQR Manager 2.0.59 — tab-prepínače na Events, Orders, Tickets a Sales

## Čo si mi napísal

*"taktiez urobit mooznost pri events, orders, tickets sales a inventory, urobit urobit nieco ako je na
dashboarde 3 veci ktore si vies prekliknut overview, financials, activity, tak pri tychto urobit nieco,
kde ked to bude completed alebo paid alebo ked uz bude po a ked to bude dokoncene, presunu sa do druheho
policka, aby tam nezavadzali, a aby tam boli a tak bude aj lepsi prehlad"*

S doplňujúcimi otázkami sme si potvrdili: nahradiť pôvodné dropdown filtre tab-prepínačom (tam, kde už
nejaký bol), na Orders počítať "Paid" len skutočne zaplatené (nie čiastočne), a Inventory nechať úplne
bez zmeny.

## Čo je nové

Presne ako na Dashboarde, teraz aj **Events, Orders, Tickets a Sales** majú nad tabuľkou modrý
tab-prepínač (rovnaký vizuál, rovnaké správanie — appka si zapamätá, na ktorej záložke si bol, aj po
reštarte). Veci, čo sú "hotové", sa presunú do druhej záložky, aby nezavadzali v hlavnom pohľade:

- **Events:** *Upcoming* / *Completed* — zrušené (Cancelled) eventy sa pridali k Completed, nie do
  samostatnej tretej záložky, aby to bolo presne dve políčka, ako si chcel.
- **Orders:** *Active* / *Paid* — do Paid patria len objednávky so statusom **Paid**; Unpaid aj Partial
  ostávajú v Active (presne podľa toho, čo sme si potvrdili).
- **Tickets:** *Active* / *Completed* — používa rovnaké pravidlo, aké appka už predtým mala na výpočet
  stĺpca Status (Sold Out a Cancelled obe padnú do Completed, zvyšok je Active).
- **Sales:** *Pending* / *Completed* — Paid aj Refunded predaje idú do Completed; predaj so statusom
  **Mixed** (dávka, kde sa časť lístkov zaplatila a časť ešte nie) zostáva v Pending. Zámerne — Mixed nie
  je "hotovo", je to rozrobené, a tab-prepínač nemá tíško rozhodovať za teba, že niečo je dokončené, keď
  to jasné nie je.

Prepínanie je okamžité — všetky dáta si appka na danej stránke stiahne raz (rovnako ako doteraz) a
záložky len prepínajú, čo sa z toho ukáže; žiadna zmena v databáze ani v appke na pozadí.

## Kde to nahradilo dropdown a kde je to úplne nové

Pri hlbšom pohľade do kódu (nie len pri pýtaní sa) vyšlo najavo, že nie všetky štyri stránky mali predtým
filter, ktorý by tab nahrádzal:

- **Tickets** aj **Sales** mali skutočný dropdown filter (Status / Payment), ktorý som teraz kompletne
  nahradil tab-prepínačom — presne, ako sme sa dohodli.
- **Events** aj **Orders** predtým **žiadny status/payment filter nemali vôbec** (len Search/Kategória/
  Zoradenie) — tab-prepínač je tam teda nová vec navyše, nie náhrada niečoho, čo tam bolo.

Na výsledku to nič nemení — presne to, čo si chcel (hotové veci nabok, lepší prehľad), platí na všetkých
štyroch rovnako — len chcem byť presný v tom, čo presne kde zmizlo a čo je nové.

## Inventory zostáva bez zmeny

Ako sme si potvrdili — Inventory je v appke tá istá obrazovka ako Tickets (zdieľajú úplne rovnaký kód),
len má natvrdo nastavený filter len na dostupné/vystavené kusy. Keďže tam teda z princípu nikdy nemôže
pribudnúť nič "hotové" (predané/zrušené kusy tam už appka nikdy neukáže), tab-prepínač by tam nemal čo
robiť — Inventory teda nemá žiadny nový tab a funguje presne ako doteraz.

## Súhrnné čísla nad tabuľkou (Sales) — čo sa počíta z aktuálnej záložky a čo nie

Na Sales je nad tabuľkou riadok so súhrnom (Results / Tickets / Revenue / Profit / Paid / Outstanding /
Refunded). Rozdelil som ho na dve rôzne pravidlá, zámerne:

- **Results, Tickets, Revenue, Profit, Refunded** — teraz sa počítajú len z toho, čo vidíš v aktuálnej
  záložke (Pending alebo Completed), presne tak, ako to robí aj tabuľka pod nimi.
- **Paid / Outstanding** — zámerne **zostáva počítané zo všetkých predajov naraz**, bez ohľadu na to, na
  ktorej záložke si. Je to súhrn "koľko peňazí je celkovo vybraných a koľko ešte dlžíš/dlžia ti" — keby sa
  prepočítaval podľa záložky, "Outstanding" by pri prepnutí na Completed skoro spadlo na nulu, čo by
  vyzeralo, že už nič nedlžíš, hoci v skutočnosti len nepozeráš na tú časť zoznamu, kde to vidno.

## Chyba, čo som si sám našiel a opravil pri kontrole (dôležité)

Pri vizuálnej kontrole pred odoslaním som si všimol nezrovnalosť: riadok "Results: N sales" pôvodne
počítal **všetky** predaje spĺňajúce filtre (obe záložky spolu), zatiaľ čo hneď vedľa neho Tickets/Revenue/
Profit už boli prepočítané len na aktuálnu záložku (viď vyššie) — a pod tým celým je tabuľka, čo tiež
ukazuje len aktuálnu záložku. Výsledok: v Pending záložke by to ukázalo napríklad "Results: 4 sales" aj
keď v tabuľke pod tým vidno len 2 riadky — vyzeralo by to ako chyba v appke. Opravil som to tak, aby
"Results" počítalo to isté, čo Tickets/Revenue/Profit aj tabuľka — teda len aktuálnu záložku. Zachytil
som to sám pri vlastnej kontrole pred odoslaním, nie si to nahlásil ty.

## Drobnosť: hláška pri viac ako 5000 predajoch

Sales mala hlášku "Showing the most recent 5,000 sales... Narrow the date range, event **or payment
filter**...", keď filtre vrátia príliš veľa predajov naraz. Keďže Payment dropdown je preč (nahradený
tabmi, ktoré ale nefiltrujú, čo sa sťahuje z appky na pozadí — len rozdeľujú, čo už appka má stiahnuté),
odstránil som z tejto hlášky zmienku o payment filtri — prepínanie záložiek totiž tejto hláške nijako
nepomôže zmiznúť. Zvyšné dve rady (dátum, event) stále platia a fungujú rovnako ako predtým.

## Ako som to overoval

```
cargo test --lib  -> 625 testov, všetky prešli (žiaden Rust súbor sa touto zmenou nemenil)
npx tsc -b        -> 0 chýb
npm run build     -> OK
```

Keďže appku v tomto prostredí neviem naozaj spustiť ako desktopovú appku, postavil som si dočasnú
náhľadovú stránku (mimo appky, zmazanú hneď po použití) a v nej som si všetky štyri stránky reálne
vykreslil s dátami pokrývajúcimi rôzne stavy (upcoming/completed/cancelled event; unpaid/partial/paid
objednávky; sold-out/cancelled lístky; pending/mixed/paid/refunded predaje) — preklikal som obe záložky na
každej stránke aj Dark mód, a práve pri tejto kontrole som našiel a opravil chybu popísanú vyššie.

## Čo teraz urobiť

1. Nainštaluj 2.0.59.
2. Na Events/Orders/Tickets/Sales skús prepnúť obe záložky a skontroluj, že rozdelenie sedí (najmä
   Cancelled event → Completed, a "Mixed" predaj → Pending).
3. Na Sales skontroluj súhrn nad tabuľkou — Paid/Outstanding by malo zostať rovnaké na oboch záložkách,
   zvyšok (Results/Tickets/Revenue/Profit) by sa mal meniť podľa toho, na ktorej záložke si.
4. Ak ti niektoré rozdelenie nesedí (napr. by si chcel Partial v Orders radšej v Paid), napíš mi — je to
   úprava jednej podmienky, nie veľký zásah.

## Zmenené súbory

**Frontend (nové zdieľané súbory):**
- `src/lib/useListTab.ts` — nový hook: načíta/uloží, na ktorej záložke si bol, rovnako ako to Dashboard
  už robil pre svoje vlastné tri záložky
- `src/components/ui.tsx` — nová zdieľaná komponenta `TabSwitcher` (rovnaký vizuál ako Dashboard)

**Frontend (upravené stránky):**
- `src/pages/Events.tsx` — nový tab-prepínač Upcoming/Completed (nová vec, žiadny dropdown predtým)
- `src/pages/Orders.tsx` — nový tab-prepínač Active/Paid (nová vec, žiadny dropdown predtým)
- `src/pages/Tickets.tsx` — tab-prepínač Active/Completed nahrádza pôvodný Status dropdown (Inventory
  bez zmeny — zdieľa tento súbor, ale má svoj filter natvrdo nastavený)
- `src/pages/Sales.tsx` — tab-prepínač Pending/Completed nahrádza pôvodný Payment dropdown; opravený
  súhrnný riadok (viď "Chyba, čo som si sám našiel" vyššie)

**Backend:** žiadne zmeny — celá funkcia je len prerozdelenie už stiahnutých dát na obrazovke.

**Verzia (8 miest):** ako vždy, všetkých na `2.0.59`.

## STOP

2.0.59 hotové — Events, Orders, Tickets a Sales majú teraz tab-prepínač presne v duchu toho, čo je už na
Dashboarde, s dohodnutými pravidlami (Cancelled→Completed, len Paid→Paid, Mixed→Pending) a Inventory
zostáva bez zmeny. Cestou som si sám všimol a opravil jednu nezrovnalosť v súhrnnom riadku na Sales (viď
vyššie) — appka teda ide von už opravená, nie s tým, čo som pôvodne napísal.

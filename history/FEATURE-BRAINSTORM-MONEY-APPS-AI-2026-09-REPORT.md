# TIQR Manager — 10 vecí z prieskumu peňažných appiek a AI, september 2026

Zadanie: prejsť si celý internet — nielen ticket reselling, ale celkovo ako
appky a ľudia narábajú s peniazmi, ako si zapisujú, akú majú automatizáciu —
a povedať, ako z toho spraviť lepšiu appku, vrátane toho, ako využiť už
existujúci AI (Anthropic) API kľúč na viac než len jednu vec.

**Vzťah k `FEATURE-BRAINSTORM-2026-09-REPORT.md`** (existujúci dokument,
20 návrhov): ten už dôkladne pokryl konkurenčné nástroje priamo pre ticket
brokerov (Lysted, Vendoo, Skybox, Automatiq, TicketFlipping, Stage Front,
Seller Ledger, Link My Books, Chargeback Gurus, Ticket Utils) — to som
zámerne nerobil znova. Tento dokument ide širšie, presne ako si žiadal:
appky pre bežné osobné financie a malé firmy (YNAB, Monarch Money, Copilot
Money, Quicken Simplifi, Rocket Money, plus AI/bookkeeping nástroje mimo
ticketingu úplne) — a dve položky nižšie (#1 a #3) vedome PREHLBUJÚ staré
návrhy #9 a #5 konkrétnou technikou, ktorú som predtým nemal — inak sa
neprekrývajú.

**Technický základ, z ktorého vychádzam**: prešiel som si `ai_categorize.rs`
— appka už dnes reálne posiela requesty na `api.anthropic.com` (Claude
Haiku, `claude-haiku-4-5-20251001`), kľúč je zabudovaný pri buildovaní z
GitHub Actions secretu `ANTHROPIC_API_KEY` (rovnaký vzor ako Google Sheets
service account), s tichým fallbackom keď kľúč nie je nastavený, retry na
prechodné chyby, a prísnym obmedzením na to, čo AI vôbec smie vrátiť
(len jednu z už existujúcich kategórií, nikdy vymyslenú). Toto je presne tá
infraštruktúra, o ktorej hovoríš — už funguje, už je odskúšaná, a dá sa
použiť na viac než kategorizáciu eventov bez toho, aby si musel riešiť
nový účet, nový kľúč, alebo nový spôsob platby.

---

## 1. "Opýtaj sa svojich dát" — reálny, overený bezpečný vzor (Wealthfolio)

Toto priamo prehlbuje starý návrh #9. Hľadal som, ako to rieši appka, ktorá
má rovnakú DNA ako TIQR — lokálna, open-source, bez cloudu (Wealthfolio,
portfolio tracker). Ich AI Assistant má presne taký tvar, aký by sedel aj
TIQR: AI nikdy nevidí surové riadky z databázy — má k dispozícii len úzku
sadu vopred definovaných, read-only funkcií (`get_holdings`,
`search_activities`, `get_performance`), preloží tvoju otázku na volanie
jednej z nich, appka funkciu spustí LOKÁLNE, a späť k modelu ide len
výsledok toho volania, nie surové dáta. Pre TIQR to znamená: "koľko som
zarobil na Ed Sheeranovi minulý mesiac" by AI preložilo na volanie niečoho
ako `finance::profit_by_event_name(name)` (funkcia, ktorá už dnes v
`finance.rs` prakticky existuje ako logika), nie na AI, ktoré si samo píše
SQL nad celou databázou. Bezpečnostne aj nákladovo najčistejší z celého
zoznamu.

## 2. AI zhrnutie mesiaca v ľudskej vete, nie len čísla

Copilot Money (jedna z najlepšie hodnotených appiek na AI kategorizáciu
podľa aktuálneho porovnania) generuje každý mesiac krátky odstavec:
kam išli peniaze, čo sa zmenilo oproti minulému mesiacu, čo by si mal
zvážiť zrušiť. TIQR's `finance.rs` už dnes presne tieto čísla POČÍTA
(idea #7 v starom dokumente to už navrhla ako tlačidlo/report) — toto je
lacné rozšírenie toho istého: namiesto samotných čísel poslať tie isté už
spočítané čísla jedným krátkym Haiku promptom a dostať späť 2-3 vety
po slovensky ("V auguste si zarobil X€, o Y % viac než júl, najviac na
[event]. Zvaž..."). Nulová nová AI infraštruktúra — presne ten istý
`send_anthropic_request` vzor, len iný prompt.

## 3. Fuzzy AI párovanie výplat (prehlbuje starý návrh #5)

Skutočné reconciliation nástroje pre finančné tímy dnes nepoužívajú len
presnú zhodu súm — používajú "fuzzy matching + učenie sa vzorov": platba,
ktorá príde o deň neskôr, alebo faktúra so skratkou namiesto celého mena,
sa aj tak správne spáruje, a systém si postupne pamätá, ako si podobné
prípady predtým vyriešil (jeden zdroj uvádza posun z ~70 % na 95%+
automaticky spárovaných prípadov, ako sa systém "učí"). Starý návrh #5
počítal skôr s presným/takmer presným párovaním; AI krok navyše by pomohol
presne v tých ťažších prípadoch, kde suma len približne sedí (zaokrúhlenie,
čiastočná dávka, časový posun) — appka by namiesto "nesedí, over ručne"
vedela navrhnúť "toto pravdepodobne sedí, lebo..." a ty by si len potvrdil.

## 4. Odfotená/prekopírovaná potvrdzovacia emailová správa → predvyplnený nákup

Viacero appiek na sledovanie výdavkov v roku 2026 kombinuje OCR (čítanie
fotky účtenky) a spracovanie prirodzeného jazyka tak, že vieš appke poslať
aj len prekopírovaný text potvrdzujúceho emailu, a ona z neho vytiahne
sumu, dátum, položky. Pre TIQR: keď kúpiš lístky (Ticketmaster/AXS/iný
predajca ti pošle potvrdenie), vieš odfotiť alebo prekopírovať ten email do
appky a AI z neho vytiahne názov eventu, dátum, sekciu/rad/miesto, cenu,
počet kusov — predvyplní formulár na pridanie do Inventory namiesto
ručného prepisovania. Dôležité: toto je o TVOJOM nákupe, nie o čítaní
marketplace stránky — nejde to proti tvojmu pravidlu "žiadne scrapovanie
marketplacov", lebo appka nič nenavštevuje, len číta text/obrázok, ktorý si
jej sám dal.

## 5. Tichá kontrola "nevyzerá to ako preklep" na vlastné záznamy

Malé firmy dnes bežne používajú AI len na jednu nenápadnú vec: upozorniť
na duplicitný záznam alebo sumu, ktorá vybočuje ("táto položka je 5x
vyššia než zvyčajne pre túto kategóriu"). Pre TIQR by to bola tichá
kontrola nad novými Finance záznamami/Orders - nie AI nutne, dá sa to aj
čisto v Rust bez volania von (priemer + odchýlka za posledných N záznamov
tej istej kategórie) - a keď niečo vybočí, len jeden riadok v Attention
Centri, presne v duchu toho, ako appka dnes upozorňuje na iné veci. Nikdy
nič samo neopraví, len upozorní.

## 6. Jeden "review queue" namiesto behania po stránkach

Copilot Money má týždenný "review" - jeden posuvný zoznam všetkého, čo
potrebuje tvoje rozhodnutie (nezaradená transakcia, atď.), namiesto
hľadania po rôznych obrazovkách. TIQR's Attention Center už dnes agreguje
presne "čo potrebuje pozornosť" naprieč 5 kategóriami - chýba mu len režim
"prejdi ich jeden po druhom": otvoríš prvú položku, jedným tlačidlom
vyriešiš alebo preskočíš, appka ťa posunie na ďalšiu. Žiadna nová dátová
vrstva, len nový spôsob prechádzania toho, čo tam už je.

## 7. Predpoveď najbližších 30 dní z tvojej vlastnej histórie (žiadna AI netreba)

YNAB aj Monarch Money teraz appke automaticky predpovedajú budúce bežné
výdavky/zostatok na základe histórie ("cashflow forecasting"). Toto je iné
než starý návrh #10 (ten pozerá DOZADU, kde si najviac zarobil) - toto
pozerá DOPREDU: appka z `finance.rs`'s vlastnej histórie (koľko si typicky
minul/zarobil za posledných 6 mesiacov, aké eventy máš nadchádzajúce) vie
odhadnúť "v najbližších 30 dňoch pravdepodobne minieš okolo X€ na nákupy
lístkov a zarobíš okolo Y€". Toto je čistá matematika nad dátami, ktoré
appka už má - žiadny nový AI call, najlacnejšia položka z celého zoznamu.

## 8. Priemysel sám potvrdzuje presne tvoje vlastné pravidlo: AI navrhne, človek potvrdí

Toto nie je návrh funkcie, ale stojí za to to vedieť: Rocket Money's "Rowan"
- AI agent, ktorého CELÝ biznis je automatizácia (ruší predplatné, vyjednáva
nižšie účty) - aj tak vyžaduje výslovné potvrdenie SMS-kou od používateľa
predtým, než čokoľvek spraví. Aj appka postavená na automatizácii sa
drží pravidla "AI navrhne aj s dôvodom, človek potvrdí, appka spraví".
Presne to, čo si sám opakovane trval pri Price Checkeri (žiadne automatické
scanovanie, žiadne automatické repricing) - toto potvrdzuje, že to nie je
len tvoja opatrnosť, je to aj to, kam smeruje celý odbor. Odporúčam držať
sa toho pri každom z návrhov vyššie.

## 9. "Local-first" nie je len technický detail - je to čoraz zriedkavejšia vlastnosť

Časť appiek na sledovanie financií (plain-text accounting, self-hosted
nástroje ako Firefly III/Actual Budget) rastie práve preto, že ľudia
čoraz viac chcú mať svoje peniaze OFFLINE, mimo cudzieho cloudu. TIQR toto
už dnes má natívne (lokálna SQLite, žiadny cloud, žiadny povinný účet) -
nič nemusíš stavať, len stojí za to vedieť, že to, čo appka už je, je samo
o sebe konkurenčná výhoda, nie len architektonické rozhodnutie spred
rokov.

## 10. Jeden zdieľaný AI klient namiesto piatich kópií

Praktická poznámka na záver, nie inšpirácia zvonku: ak by si sa rozhodol
spraviť čo i len 2-3 z návrhov vyššie (#1, #2, #4 všetky volajú Anthropic
API), najefektívnejšie by bolo vytiahnuť `send_anthropic_request` +
retry logiku z `ai_categorize.rs` do jedného zdieľaného
`ai_client.rs` modulu, ktorý si každá funkcia len zavolá s vlastným
promptom - rovnaký kľúč, rovnaký tichý fallback keď nie je nastavený,
rovnaký cenový profil (Haiku, malý max_tokens). Menej kódu na údržbu než
štyri samostatné kópie tej istej HTTP logiky.

---

## Keby som mal tipovať, od čoho začať

Rovnako ako v starom dokumente - nepýtal si si rebríček, ale mám ho v
hlave: **#7** (predpoveď z histórie) a **#9** (nič, len vedomie) sú
prakticky bez rizika a bez novej AI - odporúčam ich prvé, ak chceš rýchly
efekt. **#2** (AI zhrnutie mesiaca) je najmenší krok medzi tými, čo reálne
volajú Anthropic - doslova jeden nový prompt nad už existujúcimi číslami.
**#1** (opýtaj sa dát) je najhodnotnejší, ale aj najväčší kus práce -
vyžaduje si najprv navrhnúť presnú sadu "tool" funkcií, ktoré by AI smelo
volať, presne podľa Wealthfolio vzoru. **#3** a **#5** by som robil, len ak
sa najprv ukáže, že #5 zo starého dokumentu (obyčajné párovanie) v praxi
nestačí - inak je to predčasná komplikácia.

---

## Zdroje (skutočne použité pri prieskume)

- [Era vs. Monarch vs. Copilot vs. YNAB - 2026 comparison](https://era.app/articles/era-vs-monarch-vs-copilot-vs-ynab/)
- [Best AI Personal Finance Tools in 2026: YNAB vs Copilot vs Monarch vs Simplifi](https://www.techno-pulse.com/2026/04/best-ai-personal-finance-tools-in-2026.html)
- [AI Expense Tracker: How AI Automates Receipt Scanning, Categorization, and Financial Records](https://www.jenova.ai/en/resources/ai-expense-tracker)
- [How AI Is Transforming Small Business Bookkeeping In 2026 (GBQ)](https://gbq.com/how-ai-is-transforming-small-business-bookkeeping-in-2026/)
- [Wealthfolio - AI Assistant documentation](https://wealthfolio.app/docs/guide/ai-assistant/)
- [Best AI Reconciliation Tools for Finance Teams in 2026 (Optimus)](https://optimus.tech/blog/best-ai-reconciliation-tools-for-finance-teams-2026)
- [Rocket Money's Rowan AI Agent Cancels Subscriptions by Text](https://techjournal.org/rocket-money-rowan-ai-agent)
- [Open source budgeting software: 6 free, private tools](https://impause.com/blog/open-source-budgeting-software-6-free-private-tools-2026)
- [The Best Budget Apps for 2026 (NerdWallet)](https://www.nerdwallet.com/finance/learn/best-budget-apps)
- [13 best receipt scanner apps in 2026 (Bill.com)](https://www.bill.com/blog/best-receipt-scanning-app)

Toto je zámerne len prieskum a návrhy na zamyslenie - nič som z toho
neimplementoval. Ak niektorý z týchto 10 bodov chceš reálne postaviť, daj
vedieť ktorý a spravím to ako samostatnú úlohu, rovnakým spôsobom ako
doteraz (najprv `CURRENT_STATE.md`/`PROTECTED_AREAS.md`, cielené zmeny,
testy, report).

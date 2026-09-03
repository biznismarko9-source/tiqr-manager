# TIQR Manager — 20 návrhov na väčšie funkcie (september 2026)

Tento dokument vznikol na tvoju požiadavku: prešiel som si celý súčasný stav
TIQR Manager (`CURRENT_STATE.md`, `KNOWN_BUGS.md`, `PROTECTED_AREAS.md`, plus
priamo kód tam, kde bolo treba overiť detail), a potom som spravil poriadny
prieskum konkurenčných nástrojov pre ticket brokerov/predajcov a širšie
dashboard/SaaS produkty (crosslisting nástroje pre e-commerce, CRM, účtovný
softvér pre resellerov, inventory forecasting). Zoznam zdrojov, ktoré som
reálne použil, je na konci dokumentu.

Nie je to zoznam drobných vylepšení — to som zámerne vynechal, presne ako si
chcel ("väčšie veci"). Každý návrh nižšie je samostatná funkcia, ktorá by
znamenala nový model dát, novú obrazovku alebo aspoň novú backend logiku, nie
len úpravu existujúcej. Pri každom píšem aj to, na čo si dať pozor — buď preto,
že to naráža na niečo, čo je v `PROTECTED_AREAS.md` zámerne takto urobené, alebo
preto, že to je väčší architektonický krok, alebo preto, že potrebujem od teba
potvrdenie, či to vôbec sedí s tým, ako reálne pracuješ.

Zámerne som nedával späť veci, ktoré už existujú (napr. cross-marketplace
listing tabuľka, Market Analysis, Attention Center, Fulfillment Center,
notifikácie cez desktop/ntfy) ani veci, ktoré si už raz vyskúšal a vedome
zrušil (email notifikačný kanál — "email zatial odstranme", alebo Pushover
nahradený ntfy) ani veci, ktoré sú už explicitne odložené na neskôr (2FA,
overenie emailu pri registrácii). Ak niečo z toho nižšie pôsobí ako duplicita
niečoho existujúceho, pokojne to napíš — nemal som pri ruke teba, len kód a
dokumentáciu.

---

## A. Automatizácia listingov na marketplacoch

Toto je oblasť, kde konkurenčné nástroje (Lysted, Vendoo, Skybox od Vivid
Seats, Stage Front) idú výrazne ďalej ako TIQR dnes — a je to aj oblasť, kde
TIQR má už postavený správny základ (`ticket_listings` tabuľka, multi-
marketplace, bulk akcie z 2.2.4/2.2.5), takže tieto 4 návrhy naň priamo
nadväzujú, nie sú od nuly.

### 1. Automatické stiahnutie listingu zo VŠETKÝCH marketplacov, keď sa lístok predá na jednom

Toto je presne to, čím sa chvália Lysted aj Vendoo ako svojou hlavnou
hodnotou: keď sa lístok predá na jednom kanáli, systém ho automaticky
stiahne/označí ako predaný na všetkých ostatných, kde bol tiež vystavený —
aby nedošlo k double-sellu (predáš ten istý lístok dvakrát, lebo si zabudol
stiahnuť inzerát inde). TIQR dnes má `ticket_listings` presne pripravenú na
toto (per-marketplace status pre každý lístok), ale prepojenie "predané v
Sales → automaticky updatni status na všetkých ostatných listingoch toho
istého lístka" tam podľa všetkého chýba — je to manuálne. Toto by som
považoval za najprirodzenejšie prvé rozšírenie zo všetkých 20, presne preto,
že nepotrebuje žiadnu novú tabuľku ani externú integráciu, len prepojenie
dvoch vecí, ktoré už existujú.

### 2. Prepojenie Market Analysis → skutočná cena listingu jedným klikom

Price Checker dnes vypočíta odporúčanú cenu (Market Analysis, 2.2.0), ale je
to len číslo na obrazovke — aby sa premietlo do reálneho listingu, musíš ísť
ručne do Listings a cenu prepísať. Konkurenčné nástroje (Automatiq/Uptick)
toto robia ako plne automatický 24/7 repricing bot — to by som ale NEodporúčal
kopírovať doslovne, pretože by to šlo priamo proti tomu, prečo je Visible
Scanner postavený ako manuálny (marketplace captchas, nespoľahlivosť
automatizovaného scrapovania — presne dôvod, prečo bol starý hidden auto-check
v 2.1.9 zrušený). Namiesto plne automatického bota navrhujem len tlačidlo
"použiť odporúčanú cenu" priamo z Market Analysis obrazovky do listingu — človek
(ty) stále rozhoduje KEDY skenovať, len sa odstráni ručné prepisovanie čísla
potom.

### 3. Hromadné vytvorenie listingu na viacerých marketplacoch naraz

Dnes (podľa toho, čo je vidieť v `ticket_listings`) sa listing pridáva
marketplace po marketplace. Jednoduché, čisto frontendové rozšírenie: jeden
formulár, zaškrtneš 3 marketplace naraz (napr. Vivid Seats + Ticombo +
Viagogo), a vytvoria sa 3 samostatné listing záznamy s rovnakou cenou/
popisom naraz. Žiadna nová závislosť, žiadna externá integrácia — len ušetrí
opakované klikanie pri každom novom lístku.

### 4. Priama API integrácia s marketplacmi (Vivid Seats/Skybox, prípadne StubHub)

Toto je najväčší a najneistejší návrh z celého zoznamu, preto ho takto
zvýrazňujem. Skybox aj Automatiq stavajú celý svoj biznis na tom, že majú
partnerský/API prístup priamo k StubHub aj Vivid Seats — vedia automaticky
zistiť "je tento lístok ešte živý/predaný" bez toho, aby niekto otváral
viditeľné okno a skenoval. Pre TIQR by to znamenalo nahradiť časť Visible
Scanner + ručné listing-status-update za automatický pull stavu z API. Problém:
toto nie je len o kóde — vyžaduje to schválenie broker/partner API prístupu od
každého marketplace zvlášť, čo je mimo tvojej aj mojej kontroly a môže to byť
pomalé alebo nedostupné pre menšieho predajcu. Odporúčam to zaradiť ako "warto
preskúmať, začať jedným marketplace, ktorý má reálne prístupné API pre
bežného predajcu" — nie sľubovať to ako celok naraz.

---

## B. Účtovná/finančná hĺbka

TIQR má už dnes poriadny základ (`finance.rs` ako jediný zdroj pravdy pre
profit/margin, presné rozdelenie nákladov na cent v `insert_order_with_tickets`,
Finance modul so 4 tabmi). Prieskum reseller-špecifického účtovného softvéru
(Seller Ledger, My Reseller Genie) a nástrojov na marketplace payout
reconciliation (Link My Books) ukázal 4 veci, ktoré tieto nástroje majú a
TIQR zatiaľ nie.

### 5. Párovanie výplaty z marketplace s jednotlivými predajmi (payout reconciliation)

Marketplace ti nevyplatí každý predaj zvlášť — príde jedna súhrnná platba na
účet za viacero predajov naraz, po odpočítaní poplatkov. Podľa prieskumu (Link
My Books) je toto jeden z najbolestivejších manuálnych procesov pre
resellerov — treba ručne overiť, že súhrnná platba sedí s tým, čo si čakal.
Konkrétny návrh: nová obrazovka/report vo Finance, kde zadáš sumu a dátum
skutočnej platby na účet, a systém ti ukáže, ktoré Sales záznamy (podľa
očakávaného čistého výnosu po poplatkoch) túto sumu najpravdepodobnejšie
tvoria — a upozorní, ak niečo nesedí.

### 6. PDF faktúra/doklad k objednávke, s poľami pre EÚ

Dnes existuje len CSV export. Keďže si v EÚ, faktúra s náležitosťami (IČO/
DIČ, dátum, položky, mena) je často reálna povinnosť, nie len pekná vec navyše.
Toto by som staval ako nový, samostatný PDF generátor nad existujúcimi Order/
Sale dátami — nemení nič na finančnej logike, len pridáva výstupný formát.

### 7. Jednoklikový "daňový balíček" na konci roka

Rozšírenie existujúceho Finance → Reports tabu: jedno tlačidlo, ktoré
vygeneruje súhrn za zvolený rok/kvartál — celkový obrat, náklady (COGS),
poplatky, čistý zisk, rozpad podľa platformy — pripravené na odovzdanie
účtovníčke/účtovníkovi. Nie nová logika, len nová prezentácia toho, čo
`finance.rs` už dnes vie spočítať.

### 8. Sledovanie chargebackov/sporných platieb ako samostatný stav predaja

Ticket reselling má podľa prieskumu (Chargeback Gurus, Chargeflow) známy,
opakovaný problém: kupujúci dostane platný lístok, príde na akciu, a POTOM
zavolá do banky a nahlási chargeback ("friendly fraud") — čo je pre predajcu
čistá strata, lebo lístok už bol odovzdaný. `payment_status` má dnes len
pending/paid/refunded (refund je jednosmerný, iniciovaný tebou). Chýba stav
pre "kupujúci spustil spor v banke, čakám na výsledok" a súhrnný report, koľko
ťa to za rok reálne stálo — dnes to nemáš kde ani zapísať, natož sledovať.

---

## C. Inteligencia postavená na tom, čo už funguje (Claude Haiku klasifikátor)

Máš už dnes reálne fungujúci, lacný a overený vzor: `ai_categorize.rs`
posiela jednu krátku klasifikačnú otázku Claude Haiku modelu, len keď
jednoduché pravidlá zlyhajú, s tichým fallbackom keď AI nie je dostupná.
Toto sú dva návrhy, ktoré by ten istý, už overený vzor len použili na nové
otázky — nie nová AI infraštruktúra, len nové použitie tej istej.

### 9. "Opýtaj sa svojich dát" — jazykový asistent nad vlastnou databázou

TicketFlipping (konkurenčný nástroj) má "Ask-AI" — asistenta natrénovaného na
broker znalostiach. To pre teba nie je až tak zaujímavé (to je všeobecné rady,
nie tvoje dáta). Zaujímavejšie je niečo, čo tvoje dáta majú a generický
asistent nie: možnosť napísať bežnou vetou otázku ("koľko som zarobil na
Ed Sheeranovi minulý mesiac?", "koľko lístkov mi ešte ostalo nepredaných na
októbrové akcie?") a dostať odpoveď spočítanú z tvojej vlastnej lokálnej
SQLite databázy. Technicky: AI len preloží otázku na bezpečný, obmedzený
SQL/filter dotaz nad už existujúcimi read-only funkciami (dashboard, finance,
inventory) — nikdy nič nezapisuje, presne v duchu toho, ako `ai_categorize.rs`
už dnes obmedzuje, čo môže AI vôbec ovplyvniť.

### 10. Nákupná inteligencia — kde sa mi to doteraz najviac oplatilo

Automatiq's DataIQ predáva "analytics na základe overených dát, aby si vedel
kde nakupovať". TIQR už dnes POČÍTA presný zisk na objednávku (`finance.rs`)
— len ho nikde neagreguje spätne podľa platformy/miesta konania/kategórie/
tier, aby ti povedal "toto ti historicky nosí najviac peňazí, kupuj viac
takéhoto typu". Je to nová obrazovka/report nad existujúcimi dátami, žiadna
nová AI, len chýbajúci pohľad dozadu, ktorý by mal priamo ovplyvniť
rozhodnutia dopredu.

---

## D. Cenová inteligencia z TVOJICH vlastných dát

### 11. Archív vlastných skutočných predajných cien ako referencia pri cenení

Toto je vec, ktorú viacero konkurenčných nástrojov (Stage Front's "DataVue",
TicketFlipping's "Flare") predáva ako svoju hlavnú konkurenčnú výhodu:
skutočné PREDANÉ ceny sú oveľa spoľahlivejší signál než inzerované ceny
(ktoré vidí Price Checker dnes) — lebo inzerovaná cena môže byť nezmyselne
vysoká a nikdy sa za ňu nič nepredá. Tie nástroje to riešia agregáciou dát
naprieč všetkými svojimi klientmi — to TIQR nevie a ani by nemal
napodobňovať (bola by to úplne iná firma). Ale TIQR má niečo, čo tí ostatní
nemajú zadarmo: TVOJU VLASTNÚ históriu skutočných predajov. Návrh: pri
cenení nového lístka (v OrderDetail aj v Price Checker) ukázať "takto si
predal podobné lístky (rovnaká akcia/podobný tier/sekcia) v minulosti" —
čisto z tvojich vlastných Sales dát, žiadny nový externý zdroj.

---

## E. Vzťahová vrstva (zákazníci a dodávatelia)

CRM nástroje pre resellerov (aj mimo ticketingu — retail CRM prieskum) majú
jeden opakujúci sa vzorec: zmeniť rozptýlené polia (meno/email kupujúceho na
každom predaji zvlášť) na skutočnú, prvotriednu evidenciu osoby s históriou.
TIQR dnes toto nemá pre kupujúcich ani pre "pullerov" (Pulls given/received).

### 12. Adresár kupujúcich (Customer directory)

Namiesto toho, že meno/email kupujúceho žije len na jednotlivom Sale zázname,
mať skutočnú tabuľku zákazníkov, ktorá agreguje: koľkokrát u teba kúpil,
za koľko celkovo, aké typy miest preferuje, či platí načas, poznámky. Priamy
predpoklad pre návrh #14 nižšie (wanted-list), a inak štandardná vec, ktorú
má takmer každý CRM nástroj, ktorý som si pozrel.

### 13. Vzťahový prehľad pre pullerov (Pulls given/received)

Rovnaký princíp ako #12, ale pre druhú stranu — ľudí, od ktorých pulluješ
alebo ktorým pushuješ. Priebežná bilancia "kto mi čo dlží / komu ja dlžím",
namiesto prechádzania jednotlivých Pulls záznamov ručne.

### 14. Wanted-list — zoznam dopytu, keď nemáš skladom to, čo niekto chce

Bežný vzor na marketplace-style produktoch: keď zákazník chce niečo, čo
nemáš, zachytíš ten dopyt namiesto toho, aby si ho stratil. Konkrétne: keď ti
niekto napíše/spýta sa na lístky na akciu, ktorú nemáš (alebo už nemáš v
danom tieri), zapíšeš dopyt (viazaný na zákazníka z #12) — a keď sa ti neskôr
objaví zodpovedajúci lístok v Inventory, dostaneš upozornenie cez existujúci
Attention Center/notifikačný kanál (nie nový, ten istý, ktorý už funguje).

---

## F. Tím, prístup, platforma

### 15. Prístup podľa role (nie len "schválený/neschválený")

Dnešný Firebase approval gate rozhoduje LEN o tom, KTO sa vôbec dostane do
appky — nie čo tam potom smie robiť. Ak niekedy pribudne pomocník (napr. niekto,
kto má riešiť len Fulfillment/Sales, ale nemá vidieť Finance), toto dnes appka
nevie. Toto zámerne označujem ako vec, ktorá potrebuje poriadny dizajn skôr
než kód: TIQR je "local-first" appka s jednou SQLite databázou na jednom
počítači — skutočná viacpoužívateľská spolupráca nad ROVNAKÝMI dátami by
znamenala reálnu architektonickú zmenu (zdieľaný backend), nie len pridanie
"role" stĺpca. Ak ide len o "jeden ďalší človek pozerá appku na tom istom
počítači, ale nesmie vidieť Finance", to je oveľa menší, realistickejší kúsok
tejto väčšej myšlienky — stojí za to si najprv ujasniť, ktorý z tých dvoch
scenárov vlastne riešiš.

### 16. Odľahčený mobilný náhľad (nie plná appka)

Notifikácie (ntfy) ti dnes pošlú pushku na telefón, ale keď ju otvoríš, nemáš
kam kliknúť — appka existuje len na počítači. Namiesto stavania celej
natívnej mobilnej appky (veľký projekt) navrhujem len jednoduchý, mobilne
prispôsobený webový náhľad na Dashboard/Attention Center/Fulfillment Center
(read-only, prípadne pár najčastejších akcií ako "označiť ako doručené") —
menší krok, veľký rozdiel v tom, ako appku reálne používaš mimo počítača.

### 17. Automatická záloha, aj mimo tohto počítača

`backup_database`/`restore_database` dnes existujú, ale (podľa kódu) sú to
manuálne akcie — ty musíš spustiť zálohu a vybrať kam. Dve veci naraz: (a)
naplánovaná automatická záloha (denne/týždenne) bez nutnosti na to myslieť,
a (b) keďže appka už má uložený Google OAuth token pre Sheets, tá istá appka
by vedela tú zálohu rovno nahrať aj na tvoj Google Drive — takže aj keby sa
tomuto počítaču niečo stalo, dáta nie sú len na jednom mieste. Žiadna nová
dôveryhostiteľská hranica navyše (rovnaký Google účet, ktorý appka už dnes
používa).

---

## G. Hlbšie využitie toho, čo už existuje

### 18. Podpora sezónnych/balíkových vstupeniek (ak takto reálne nakupuješ)

Toto explicitne označujem ako neisté — potrebujem od teba potvrdenie, či sa
to vôbec týka toho, ako nakupuješ. Niektorí ticket reselleri kupujú celý
sezónny balík naraz (jeden nákup = veľa budúcich jednotlivých akcií počas
sezóny) a potom ho postupne predávajú/rozdeľujú po jednotlivých zápasoch.
Dnešný TIQR Order model je (podľa `insert_order_with_tickets`) postavený okolo
jednej akcie na objednávku. Ak toto reálne robíš, bola by to väčšia zmena
dátového modelu — ale ak nie, tento návrh jednoducho vynechaj, nie je to
univerzálne užitočné pre každého ticket predajcu.

### 19. Signál "toto sa mi oplatí dokupovať" z Inventory Intelligence

Inventory Intelligence (2.2.6) už dnes počíta aging buckets a rozpad podľa
tier/sekcie/marketplace. Chýba mu len jeden krok navyše: keď vidí, že
konkrétny tier/sekcia na podobných akciách opakovane vypredáš rýchlo (aging
0-7 dní) zakaždým, aktívne ti to povie ako odporúčanie, nie len ako
štatistiku, ktorú si musíš sám interpretovať. Inšpirované všeobecnými
inventory-forecasting nástrojmi (Inventory Planner, StockTrim) — tam je to
štandardná funkcia pre bežný retail; tu by šlo o rovnaký princíp nad dátami,
ktoré appka už zbiera.

### 20. Odpočet do termínu odovzdania lístka (in-hand date), naviazaný na existujúce upozornenia

Lysted vo svojej dokumentácii explicitne rieši "garantujeme doručenie pred
in-hand date" — čiže konkurencia to považuje za dosť dôležité na to, aby to
bolo jadro ich fulfillment automatizácie. TIQR má Fulfillment Center
(Pending/Awaiting Payment/Awaiting Delivery/Ready to Complete), ale
(z toho, čo je vidno) bez explicitného odpočítavania k termínu, dokedy MUSÍ
byť lístok odovzdaný pred akciou. Návrh: pridať tento termín ako pole na
sale/fulfillment položku a eskalovať ju cez existujúci Attention Center a
notifikačný kanál (rovnaký, ktorý už dnes rieši "upcoming events") — nie
nový systém, len nový, časovo kritickejší dôvod, prečo appka niečo zvýrazní.

---

## Keby som mal tipovať, od čoho začať

Nepýtal si si rebríček, ale keďže ho mám v hlave z toho, ako som to písal, tu
je stručne: najmenej riskantné a najviac naväzujúce na to, čo už funguje, sú
**#1** (auto-delist naprieč marketplacmi), **#2** (aplikovať odporúčanú cenu
jedným klikom) a **#17** (automatická záloha na Google Drive) — všetky tri
len prepájajú alebo automatizujú niečo, čo appka už dnes vie, bez novej
tabuľky, novej závislosti alebo novej externej integrácie. **#9** (opýtaj sa
svojich dát) je o niečo väčší kúsok práce, ale opakuje presne overený AI vzor
z `ai_categorize.rs`, takže riziko je nízke aj tam. Najväčšie a najneistejšie
veci na zozname sú **#4** (priame API na marketplace) a **#15** (skutočný
multi-user) — obe by som robil len s tvojím vedomím, že ide o poriadny projekt,
nie o jeden release.

---

## Zdroje (skutočne použité pri prieskume)

- [Skybox from Vivid Seats – čo to je](https://brokersupport.vividseats.com/support/solutions/articles/1000206849-what-is-skybox-)
- [Lysted – Listing and Fulfillment Automation](https://help.lysted.com/en/articles/6971922-listing-and-fulfillment-automation)
- [Automatiq – Solutions for Brokers (Uptick, Sync, DataIQ)](https://www.automatiq.com/brokers)
- [Ticket Broker Software Compared 2026 (TicketFlipping)](https://ticketflipping.com/blog/ticket-broker-software-compared-what-pros-actually-use-in-2026/)
- [TicketFlipping Toolbox – AI Assistant + Flare Dashboard](https://ticketflipping.com/ticketflipping-toolbox/)
- [Stage Front – The Next Generation of Ticket Broker Software](https://stagefront.com/news/the-next-generation-of-ticket-broker-software)
- [Essential Ticket Broker Tools Checklist (Daily Ticket Rankings)](https://dailyticketrankings.com/ticket-broker-tools)
- [Seller Ledger – Accounting Software for Resellers](https://sellerledger.com/reseller-accounting-software/)
- [Link My Books – Marketplace Payout Reconciliation](https://linkmybooks.com/blog/how-accountants-should-reconcile-marketplace-payouts-for-ecommerce-clients)
- [Vendoo – Crosslisting Software for Resellers](https://blog.vendoo.co/crosslisting-software-for-online-resellers)
- [Chargeback Gurus – Ticket Fraud and Chargebacks](https://www.chargebackgurus.com/blog/ticket-fraud-chargebacks)
- [Ticket Utils – Broker Software](https://www.ticketutils.com/Products)

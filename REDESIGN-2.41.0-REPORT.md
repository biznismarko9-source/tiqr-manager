# TIQR Manager 2.41.0 — svieti len položka, o tabuľku menej, panel preč

Zo zadania k novému zapisovaniu som zobral **to, čo je rozhodnuté a na ničom
nevisí**. Zvyšok čaká na dve tvoje odpovede — sú dole.

---

## 1 · V paneli svieti len položka

Skupina **Tickets** sa už nezvýrazňuje spolu s položkou. Svieti jedna vec: tá,
na ktorej si.

**Musím ti povedať jednu vec na rovinu:** toto je návrat k stavu pred 2.29.4 —
a tú zmenu si si vtedy vypýtal ty. V kóde je doslova tvoja veta *„ked mam
nieco vybrate v tickets tak tickets niesu oznacene"*. Teraz si povedal opak a
videl si pritom všetky tri možnosti vedľa seba, takže to beriem ako
rozhodnuté. Zapísal som obe rozhodnutia do `PROTECTED_AREAS.md`, aby to o pol
roka nikto „neopravil" naspäť podľa starého komentára.

## 2 · Detail eventu: len lístky

Tabuľka **Orders** z detailu eventu odišla. Objednávka je spôsob, akým si
zásobu kúpil; keď otvoríš event, zaujíma ťa, čo na ňom máš — a každý lístok aj
tak nesie kód svojej objednávky.

Tlačidlo **New order for this event** som presunul na hlavičku **Tickets**,
takže cesta dnu je tam, kam sa aj tak pozeráš. Objednávky sa nestratili:
zoznam `/orders` je nedotknutý a karta Listings ich stále dostáva.

## 3 · Pravý panel preč

„What this will create" zmizol z **novej objednávky** aj z **nového eventu**.
Prepisoval polia, ktoré si práve vyplnil, o stĺpec vedľa — a kvôli tomu bol
každý formulár dvojstĺpcový. Teraz je jeden stĺpec.

Zmazané je to naozaj: komponent `PreviewPanel` aj `preview` prop na modále,
nie len prestali byť volané.

## 4 · Event ukazuje aj dátum

V **novej objednávke** tam dátum bol, ale v surovom tvare `(2026-12-12)`.
Teraz je `· 12.12.2026`. A **filter Event v Sales** dátum nemal vôbec — teraz
ho má.

---

## Čo som NEurobil — a prečo

**Pulls z panela.** Vypýtal si si to, ale pozrel som sa do databázy: pull,
ktorý ťaháš **ty pre niekoho iného**, nikdy nevznikne ako objednávka. Tabuľka
`pulls` nemá `order_id` a jej vlastný komentár hovorí, že lístky platí karta
toho druhého a tvoj je len poplatok. Objednávku má len `pulls_received` — teda
to, čo potiahli **tebe**. Keby som obrazovku zrušil, polovica funkcie stratí
domov. **Povedz, kam s nimi**, a spravím to hneď.

**Riadkový formulár.** Kombináciu máš vybratú (celá stránka · tabuľkové riadky
· pridávanie tlačidlom · event raz hore · mena/pull/poznámka vždy viditeľné ·
bez pásu s rozdelením · normálna hustota). Chýbajú dve veci:

1. **Kam patrí dátum nákupu** — v tvojom zozname polí nie je, ale databáza ho
   vyžaduje. Navrhujem hore vedľa Eventu, predvyplnený dneškom.
2. Odpoveď na pully vyššie — formulár má prepínač *Objednávka / Predaj / Pull*
   a potrebujem vedieť, či ten tretí má vôbec existovať.

Je to prepis `OrderFormModal`, ktorý vytvára objednávku **aj jej lístky aj
rozdelenie nákladov**. To nechcem púšťať polospecifikované.

---

## Čo som overil

Node ani Rust tu nie sú, preklad je až CI. Overené:

- **Vyváženosť značiek a zátvoriek** v šiestich zmenených súboroch oproti
  balíku 2.40.0 — žiadny nový nepár, aj po vystrihnutí 53 riadkov z
  `EventDetail.tsx`.
- **Žiadny import sa nestal nepoužitým** (strojovo, porovnaním oproti 2.40.0).
  `Badge`, `IconPlus`, `navigate` aj `orders` v detaile eventu majú ďalších
  používateľov.
- **Žiadny zvyšok po `PreviewPanel`** — ani import, ani prop, ani komponent.
- Prázdny stav *„No orders for this event yet"*, ktorý v súbore zostal, patrí
  **karte Sales** (prepájanie Finance na objednávku), nie zmazanej tabuľke.

**Čo overiť nedokážem:** ako to vyzerá naživo. Pozri si hlavne, či ti
v detaile eventu nechýba prehľad objednávok — ak áno, vrátim ho ako stĺpec
alebo rozbaľovačku, nie ako druhú tabuľku.

---

**Verzia:** 2.41.0 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.

# TIQR Manager 2.2.4 — Event Workspace na 4 záložky + Listings ako skutočný systém

Reagujem na tvoju poslednú správu:

> *"Overview Inventory spoj do jedneho, Sales Market Finance spoj do jedneho, cize by mali ostat 4 overview,
> listings, sales a finance, co tym myslim spojit je ze odstranis to druhe a to info presunies do toho ktory
> ostava"*
>
> *"Chcem teraz prerobiť Listings z jednoduchého prehľadu na reálny listing systém. [...] chýba: Marketplace,
> Listing URL, Listing ID, Last checked/updated. Nechcem tieto údaje dávať priamo do tickets [...] Vytvor
> preto samostatný listing model, kde jeden ticket môže mať viac listingov [...] Naviaž to na existujúci
> Inventory/Ticket systém [...] Dôležité: jeden ticket môže mať viac marketplace listingov, žiadne duplicity,
> cena zostáva v INTEGER cents, mixed currencies bezpečne, existujúce tickets/inventory/sales/refund logiku
> nemen, nepridávaj zatiaľ automatické listingovanie, API ani repricing. Najprv skontroluj aktuálnu DB a
> architektúru [...] Pridaj potrebnú additive forward-only migráciu. Pridaj testy [...]"*

Postupne, po poriadku.

## 1. Záložky spojené — 4 namiesto 6

Presne podľa tvojho zoznamu na konci správy zostali tieto 4: **Overview | Listings | Sales | Finance.**

- **Overview** teraz obsahuje aj Inventory — tabuľky Orders a Tickets sú teraz dole pod pôvodnými kartami
  (tickets/sold/available/...), presne tak, ako si to popísal ("odstrániš to druhé a presunieš info do toho,
  ktorý ostáva" — Inventory zmizlo, jeho obsah je teraz v Overview).
- **Sales** teraz obsahuje aj Market — "Market vs. mine" aj "Potential Profit" sú teraz dole pod tabuľkou
  predajov.
- **Finance** som nechal úplne bez zmeny — vo svojom finálnom zozname si ho uviedol ako samostatnú položku.

**Jedna vec, kde som musel spraviť rozhodnutie sám, a chcem ti to priamo povedať:** vo vete "Sales Market
Finance spoj do jedneho" si tri veci spomenul spolu, ale hneď za tým si napísal finálny zoznam so 4
záložkami, kde **sales aj finance ostávajú ako dve oddelené položky**. Keďže Market je jediné meno, čo z
tohto zoznamu zmizlo, jeho obsah som presunul do Sales, nie do Finance — Market je o tom, čo by som za
lístok dostal teraz na trhu, čo je bližšie k predaju ako k účtovníctvu. Ak si to myslel opačne (Market →
Finance), napíš mi a presuniem to — je to jeden uzavretý blok, dá sa to prehodiť rýchlo.

## 2. Listings — skutočný multi-marketplace systém

Najprv som — presne ako si žiadal — skontroloval aktuálnu DB a architektúru, nie len uhádol. Pozrel som sa na
celú tabuľku `tickets`, existujúcu tabuľku `marketplaces` (tú, čo už používa Price Checker pre StubHub/
Vivid/Ticombo) a ako sú tam postavené podobné vzťahy (napr. `finance_entries` → `orders`).

**Nová tabuľka `ticket_listings`** (`migrations/022_ticket_listings.sql` — additive, forward-only, nič
existujúce nemení):
- `ticket_id` — na ktorý tiket sa listing viaže (jeden tiket môže mať viacero riadkov, presne ako si
  nakreslil: StubHub → Vivid → Ticombo, každý so svojou cenou/stavom/URL)
- `marketplace_id` — **použil som existujúcu tabuľku marketplaces** (tú istú, čo má Price Checker), nie
  novú — presne v duchu "nevytváraj duplicitný systém"
- `listing_id`, `listing_url`, `price_cents` (INTEGER, nikdy desatinné číslo), `currency`, `status`
  (`active`/`sold`/`removed`), `created_at`, `updated_at`

**Žiadne duplicity:** v DB je obmedzenie, že rovnaký tiket + rovnaký marketplace + rovnaké listing ID sa
nedá zapísať dvakrát. Keďže väčšinu týchto záznamov budeš zapisovať ručne a nie vždy budeš mať listing ID po
ruke, systém ti dovolí mať viac záznamov bez ID (len sa navzájom nepočítajú ako duplicity, kým naozaj
nezadáš rovnaké ID dvakrát).

**Mixed currencies bezpečne:** každý listing si drží svoju vlastnú menu, nič sa nikde neprepočítava ani
nemieša. Súhrn hore (Listed value/Lowest/Highest) ukáže sumu len vtedy, keď sú všetky aktívne listingy v
tej istej mene — inak to poctivo napíše, že sú zmiešané, presne ako to appka robí všade inde.

**Existujúce dáta/logika:** nič v `tickets`, objednávkach, predajoch ani refundoch som sa ani nedotkol.
Konkrétne `tickets.status` a `tickets.listingPriceCents` ostávajú presne také, aké boli — nová tabuľka ich
vôbec nečíta ani nezapisuje.

**Čo je teraz v záložke Listings:**
- Hore: Active listings / Listed value / Lowest price / Highest price (počíta sa len z aktívnych listingov)
- Dole: tabuľka VŠETKÝCH listingov (aj predaných/odstránených — vidíš stĺpec Status, takže má zmysel vidieť
  aj iné stavy než len "active"): ticket, marketplace, price, status, URL (klikateľné, otvorí marketplace
  stránku v novej záložke), last updated
- Tlačidlo "Add listing" a pri každom riadku Edit/Delete — takže listing vieš pridať, upraviť aj zmazať
  priamo tu, ručne (bez toho by táto tabuľka nemala ako sa naplniť)

**Zámerne (tvoje vlastné "Dôležité"):** žiadne automatické vytváranie listingov, žiadne nové API, žiadny
nový pricing/repricing systém. Všetko je manuálne zadávanie, presne ako Price Checker funguje dnes.

Jeden malý súvisiaci bezpečnostný fix, na ktorý som prišiel pri kontrole architektúry: mazanie marketplace
(napr. v Price Checkeri) už dnes odmietne zmazať marketplace, ktorý má uložené linky alebo históriu cien —
doplnil som do tejto istej kontroly aj nové listingy, takže zmazanie marketplace, ktorý má na sebe reálne
listingy, appka teraz tiež odmietne (predtým by to tie listingy potichu zmazalo spolu s marketplace).

## Čo som overil

```
cargo test --lib   -> 948 passed, 0 failed (+14 nových testov oproti 2.2.3: 13 pre ticket_listings, 1 pre
                       rozšírenú kontrolu pri mazaní marketplace)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.2.4 build" v hlavičke)
```

Nových 13 testov pre `ticket_listings` pokrýva presne to, čo si žiadal: jeden tiket s jedným listingom,
jeden tiket s viacerými marketplace naraz, úprava existujúceho listingu, zmazanie listingu, viac listingov
bez duplicít (aj konkrétne odmietnutie skutočného duplicitu), zmiešané meny na tom istom tikete (každá si
drží svoju vlastnú hodnotu), a existujúce dáta po migrácii (tiket vytvorený predtým ostáva presne taký istý,
nová tabuľka na začiatku prázdna).

## Zmenené súbory

**Backend:**
- `src-tauri/migrations/022_ticket_listings.sql` — nová tabuľka (additive/forward-only)
- `src-tauri/src/commands/ticket_listings.rs` — nový modul (list/create/update/delete + 14 testov)
- `src-tauri/src/commands/price_checker.rs` — rozšírená kontrola pri mazaní marketplace + 1 nový test
- `src-tauri/src/models.rs` — `TicketListing`, `TicketListingInput`
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs`, `src-tauri/src/db.rs` — registrácia nového
  modulu/migrácie/príkazov
- `src-tauri/src/commands/database.rs` — aktualizovaný počet migrácií v teste (21 → 22)

**Frontend:**
- `src/pages/EventDetail.tsx` — 4 záložky namiesto 6, Listings prerobené na skutočný systém
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy a API funkcie pre `ticket_listings`

**Verzia (9 miest v 7 súboroch):** `2.2.4`.

## STOP

2.2.4 hotové, otestované a zabalené. Skontroluj:

1. **Ktorýkoľvek Event → záložky hore** — over, že vidíš presne 4 záložky: Overview, Listings, Sales,
   Finance.
2. **Overview** — skontroluj, že dole pod kartami vidíš aj Orders a Tickets tabuľky (predtým Inventory).
3. **Sales** — skontroluj, že dole pod tabuľkou predajov vidíš aj "Market vs. mine" a "Potential Profit"
   (predtým Market). Ak si Market chcel radšej vo Finance, daj vedieť (bod 1 vyššie).
4. **Listings** — skús pridať listing (Add listing), zadaj marketplace/cenu/menu/status/URL, ulož, over že
   sa objaví v tabuľke aj v súhrne hore. Skús ten istý tiket zalistovať aj na druhý marketplace. Skús Edit a
   Delete. Skús kliknúť na URL — mala by sa otvoriť marketplace stránka.
5. Over, že **Inventory ani Sales/Ticket logika sa nezmenila** — pridanie/úprava listingu nemení stav tiketu
   (Available/Listed/Sold) ani nič v Orders/Sales.

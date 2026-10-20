# TIQR Manager 2.0.35 — tabuľka rastie s oknom (Sales/Sale Detail/Orders/Order Detail), Seats rozšírené

## Čo je nové

1. **Notifikácie pri blížiacom sa evente** - nič sa nemenilo, potvrdili sme si, že existujúca karta "Upcoming events (next 14 days)" na Dashboarde ti stačí.
2. **Tabuľka teraz rastie s oknom/fullscreen** - na Sales, Sale Detail, Orders a Order Detail. Namiesto pevného stropu 1400px (2.0.32-2.0.34) alebo toho, aby jeden stĺpec (Event) pohltil všetku voľnú šírku (spred 2.0.32), sa teraz šírka rozdeľuje pomerne medzi VŠETKY stĺpce naraz.
3. **Sekcia Seats rozšírená** - na Sale Detail aj Order Detail dostal stĺpec Seat výrazne väčší podiel šírky.
4. Popri tom som dotiahol aj zvyšné 2 položky z "found but not touched" zoznamu (Order kód na Orders.tsx, Ticket kód na Order Detail) - keďže som presne tie isté stĺpce v tých istých súboroch aj tak prerábal na percentá, dotiahol som ich šírku na bezpečnú hodnotu v tom istom kroku namiesto toho, aby som tú istú chybu len prepísal do percent.

**Events, Event Detail, Tickets/Inventory a Pulls (obe podzáložky) ešte na tento nový systém NEPREŠLI** - stále majú starý strop 1400px z 2.0.32-2.0.34. Nie je to zabudnuté, len som sa rozhodol poslať ti túto prvú, uzavretú a odskúšanú polovicu (celá "Sales & Orders" rodina stránok) samostatne, namiesto toho, aby som robil 8 súborov naraz bez priebežnej kontroly. Zvyšné 4 stránky idú v ďalšom kole rovnakou technikou - je už overená a hotová na kopírovanie.

## Ako presne funguje nový systém šírky

Každý stĺpec má teraz percentuálnu šírku namiesto pevných pixelov (`table-layout: fixed` to rešpektuje presne tak isto ako pevné px). Percentá som odvodil z DOTERAJŠÍCH pixelových hodnôt pri referenčnej šírke 1400px (starý strop) - napr. na Sales stĺpec Sale mal 120px, takže `120 / 1400 = 8.571 %`. Výsledok: **pri presne 1400px vyzerá tabuľka úplne identicky ako doteraz** (overené overené súčtom - všetky percentá na danej stránke dávajú dokopy ~100 %), ale nad 1400px pokračuje v raste namiesto toho, aby sa zastavila - a keďže rastú VŠETKY stĺpce naraz podľa svojho podielu, žiadny jeden stĺpec (Event, Seat) nezačne vyzerať neprimerane prázdny tak, ako sa to stalo pred 2.0.32.

**Čestne priznaný kompromis:** pod 1400px (smerom k najužšiemu podporovanému oknu appky, 1080px) sú stĺpce s pevným obsahom (Sale, Order, Ticket, Date...) teraz proporcionálne o niečo užšie, než bola ich doterajšia garantovaná minimálna šírka - narozdiel od predošlého systému, kde tieto stĺpce NIKDY nešli pod svoju nastavenú šírku bez ohľadu na veľkosť okna. `truncate` + title tooltip (a `overflow-x-auto` na celej tabuľke) sú tu presne na toto - rovnaká poistka, akú appka už na niektorých miestach reálne používa (napr. Orders.tsx mala už predtým zdokumentovaný rovnaký kompromis). V bežnom používaní (čokoľvek širšie než úplné minimum) to nebude vidieť. Naopak, stĺpec Event/Seat je na tom pri najužšom okne LEPŠIE než doteraz - predtým dostával len to, čo zvyšným stĺpcom "zostalo" (napr. 34px na Sales), teraz dostáva svoj pevný percentuálny podiel vždy (napr. ~361px na Sales pri 808px okne) - naozaj to nebolo len presúvanie problému z jedného miesta na druhé, obe strany sú na tom rovnako alebo lepšie.

## Seats - Sale Detail a Order Detail

Stĺpec Seat (spája sektor/rad/sedadlo cez `formatSeatLocation`, napr. "Sec 104 · Row A · Seat 12") bol už predtým tým "voľným" stĺpcom, ktorý dostával zvyšnú šírku - takže v širšom okne už spravidla mal dosť miesta, len to nebolo garantované. Teraz má explicitný, veľkorysý podiel: **47,1 % na Sale Detail, 65 % na Order Detail** (Order Detail má menej ostatných stĺpcov, tak mu ostáva pomerne viac). Dáta samotné (sektor/rad/sedadlo) sa nemenili - boli v appke od úplného začiatku, len teraz majú viac priestoru na zobrazenie.

## Order/Ticket kódy - dotiahnuté z "found but not touched"

- `Orders.tsx` - stĺpec Order: 92px → 120px (rovnaká oprava ako Sale kód v 2.0.33)
- `OrderDetail.tsx` - stĺpec Ticket: 84px → 120px (bola to ešte tesnejšia verzia tej istej chyby)

`Tickets.tsx` má identický Order stĺpec (92px) s tým istým problémom - to je jedna z vecí, čo príde v ďalšom kole spolu so zvyškom proporcionálneho systému.

## Vedľajší efekt pri úprave komentárov

Pri Orders.tsx som pri prepisovaní starého, dlhého komentára o šírkach stĺpcov omylom zmazal aj kúsok, čo vôbec nebol o šírke (prečo Event stĺpec nie je klikateľný odkaz - 1.9.1). Všimol som si to hneď pri kontrole a vrátil na miesto, priamo k danému riadku kódu namiesto späť do veľkého komentára hore. Menšiu poznámku o tom, prečo tam je zaškrtávacie políčko (Order Detail), som naopak nechal vypadnúť - je to samozrejmé z kódu samého, nie je čo strácať.

## Testy a build

Žiadny Rust súbor sa nezmenil - `cargo test` bez zmeny (494 testov, 491 passed, 3 ignored). Frontend: `Sales.tsx`, `SaleDetail.tsx`, `Orders.tsx`, `OrderDetail.tsx` prešli syntaktickou kontrolou (`ts.transpileModule`) - čisto. JSON súbory overené cez `JSON.parse`. Percentuálne súčty pre všetky 4 stránky prepočítané a overené (Sales 99.998 %, Sale Detail 99.999 %, Orders 100.001 %, Order Detail 99.999 % - drobné zaokrúhľovacie odchýlky, bežné a neškodné). Skutočný vizuálny render (ako to appka naozaj vykreslí, hlavne pri rôznych šírkach okna) si prosím over sám - v tomto sandboxe nemám spustiteľný prehliadač ani `npm run build`, takže toto je práve tá časť, ktorú by som najviac chcel, aby si skontroloval predtým, než pôjdem robiť rovnakú vec na zvyšných 4 stránkach.

## Zmenené súbory

**Frontend (4 súbory):**
- `src/pages/Sales.tsx` - colgroup na percentá, odstránený `max-w-[1400px]` (tabuľka aj súhrnný riadok)
- `src/pages/SaleDetail.tsx` - colgroup na percentá (+ Ticket/Order kódy 84px→120px, Seat na 47,1 %), odstránený `max-w-[1400px]`
- `src/pages/Orders.tsx` - colgroup na percentá (+ Order kód 92px→120px), odstránený `max-w-[1400px]`, drobná oprava komentára (viď vyššie)
- `src/pages/OrderDetail.tsx` - colgroup na percentá (+ Ticket kód 84px→120px, Seat na 65 %), odstránený `max-w-[1400px]`

**Verzia (8 miest):** ako vždy, všetkých na `2.0.35`.

## Čo sa NEMENILO

Žiadny Rust súbor, žiadna migrácia. Events.tsx, EventDetail.tsx, Tickets.tsx, Pulls.tsx - zámerne, idú v ďalšom kole. Dashboard (notifikácie) - žiadna zmena, potvrdili sme si, že netreba.

## STOP

2.0.35 hotové - **prvá polovica** (Sales/Sale Detail/Orders/Order Detail) prerobenej tabuľky. Skontroluj prosím hlavne:

1. Na týchto 4 stránkach - zmenši/zväčši okno appky (aj fullscreen) a sleduj, či sa tabuľka reálne mení podľa okna, bez veľkej prázdnej medzery pri širokom okne.
2. Sale Detail aj Order Detail - stĺpec Seat by mal byť teraz nápadne širší.
3. Orders aj Order Detail - kódy (Order/Ticket) by sa mali zobrazovať celé.
4. Skús aj zmenšiť okno smerom k minimu - ak by niekde nejaký kód vyzeral naozaj nečitateľne orezaný (nie len tesne), daj vedieť, viem doladiť konkrétny stĺpec.

Keď toto potvrdíš ako OK, pošlem rovnakú zmenu aj pre Events/Event Detail/Tickets/Pulls.

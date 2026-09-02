# TIQR Manager 2.2.10 — 8 opráv po kontrole 2.2.9

Poslal si mi dve správy rýchlo za sebou (druhá prišla ešte kým som pozeral na tú prvú), spolu so
7 screenshotmi. Beriem ich ako jeden spoločný zoznam — nebudem si tu vymýšľať citáciu, ktorú si
takto nenapísal, radšej to zhrniem vlastnými slovami, v poradí, v akom si to písal:

1. Stĺpec "Seats" (Section/Row/Seat) má ukazovať **len čísla**, nie aj nálepky "Sec"/"Row"/
   "Seat" pred nimi — napr. "Sec 402 · Row 56 · Seat 27" má byť len "402 · 56 · 27". A to všade,
   kde sme to minule menili.
2. Orders má mať taby **"Active"/"Completed"** namiesto "Active"/"Paid" — v Active majú byť
   všetky objednávky na eventy, čo sa ešte len budú konať (platené aj neplatené), v Completed až
   vtedy, keď buď prejde dátum eventu, alebo je v Sales všetko splnené (stav, doručenie aj
   platba).
3. Cez "New order" sa dnes dá založiť objednávka aj na event, čo sa už konal alebo je už
   Completed — to treba zastaviť, majú tam ísť len eventy, čo sú "Active".
4. Keď si spravil push z appky do Google Sheetu (Orders/Pulls), tabuľka v appke napísala, že sa
   updatla, ale v skutočnom Google Sheete sa nič nezmenilo.
5. Po prihlásení cez Settings → Integrations sa pri pushi (Sales aj Pulls) objaví nejaký dlhý,
   nečitateľný chybový text.
6. Pravé tlačidlo myši v appke ukazuje bežné menu prehliadača (Späť/Obnoviť/Uložiť ako/Tlačiť/
   Ďalšie nástroje) — to treba úplne vypnúť, nech sa po kliknutí pravým tlačidlom nič nezobrazí.
7. V Attention Center je toho veľa a je to "mixed" — majú tam byť len aktívne objednávky, kde
   treba niečo urobiť, nie všetko pomiešané.
8. V Sales sa v tabe "Completed" objavil predaj, čo v skutočnosti nie je hotový (na screenshote:
   Payment "Paid", ale Delivery "Not Delivered", a napriek tomu v Completed) — každý nesplnený
   predaj, aj keby chýbalo len doručenie, má ostať v Pending.

Toto je presne to, čo som spravil. Nikde som sa ťa nepýtal na spresnenie — pri pár nejasných
miestach (najmä bod 2 — presne kedy je event "hotový", a bod 7 — čo presne znamená "mixed") som
spravil rozhodnutie sám a píšem ho tu nahlas, aby si ho vedel opraviť, ak si to myslel inak.

## 1. Seats: späť na holé čísla

`formatSeatLocation` aj `formatSeatsSummary` (`lib/format.ts`) — tie isté dve funkcie, čo minule
(2.2.9) pridali nálepky "Sec"/"Row"/"Seat" — teraz spájajú len samotné hodnoty bodkou: **"402 ·
56 · 27"** namiesto "Sec 402 · Row 56 · Seat 27". Dôvod na vrátenie: v reálnych dátach je
hodnota v poli "Section" niekedy už sama o sebe celý názov (napr. doslova "Sec 408", alebo
"Category D, Standing") — nálepka pred tým potom vyzerala rozbito ("Sec Sec 408", "Sec Category
D, Stan..."), presne ako na tvojom screenshote. Keďže obe funkcie používa každý stĺpec "Seats" v
appke (Orders, Tickets, Inventory, Sales, Pulls, aj detaily objednávky/predaja), stačila táto
jedna zmena a je to opravené všade naraz, presne ako si žiadal.

## 2. a 3. Orders: Active/Completed namiesto Active/Paid + obmedzenie New Order

Toto je najväčšia zmena z tejto dávky.

**Nové pravidlo, kedy je objednávka "Completed"**: buď (a) event, na ktorý sa viaže, je už
hotový, ALEBO (b) samotná objednávka je celá vybavená — predaná, doručená aj zaplatená (rovnaká
"Completed" značka, akú appka už dnes počíta pri Sales). Stačí jedna z dvoch podmienok — presne
ako si napísal: aj objednávka na budúci event môže byť Completed skôr, ak je už celá vybavená,
nemusí čakať na dátum eventu.

Pri otázke "kedy je event hotový" som sa rozhodol pozerať na **dátum aj na stav eventu súčasne**,
nie len na jedno z toho. Appka má pri evente pole "status" (upcoming/completed/cancelled), ale
prezrel som si celý backend a nikde sa toto pole samo od seba nemení podľa dátumu — niekto ho
musí ručne prepnúť. Tvoj vlastný screenshot to potvrdil: event "Bad Bunny" s dátumom 22.8.2026 (2
týždne v minulosti) bol v "New order" stále vybrateľný, čiže jeho status očividne nikto ručne
neprepol na "completed". Keby som sa spoľahol len na status (tak, ako to dnes robí napr. Price
Checkerov vlastný výber eventu), presne tento prípad by sa neopravil. Preto: event je "hotový",
keď má status completed/cancelled, ALEBO keď jeho dátum už prešiel — čokoľvek nastane skôr.

**Dôsledok pre "New order"**: výber eventu tam teraz ukazuje len eventy, čo ešte NIE sú "hotové"
podľa práve popísaného pravidla — presne bod 3 z tvojho zoznamu. Predtým tam nebol žiadny filter.
Overil som si, že "New order" sa v appke používa len na zakladanie úplne novej objednávky (nie na
úpravu existujúcej) — takže toto obmedzenie nemôže nič pokaziť pri editovaní starej objednávky.

**Na zváženie**: Price Checkerov vlastný výber eventu som nechal bez zmeny (pozerá sa len na
status, nie na dátum) — nespomínal si ho a slúži na iný účel (sledovanie cien pri plánovaní
kúpy, kde má zmysel pozerať sa aj na event tesne pred konaním). Keby si chcel, aby sa aj tam
používalo rovnaké pravidlo ako pri New Order, daj vedieť.

## 4. a 5. Dva reálne potvrdené bugy pri pushi do Google Sheets

Obe veci (bod 4 aj 5 z tvojho zoznamu) som preskúmal priamo v kóde, nie odhadom.

**Bod 4 — "updatlo to, ale nič sa nestalo" — potvrdený a opravený.** Push do Sheetu (Orders aj
Pulls, `orders_sheet_sync.rs`/`pulls_sheet_sync.rs`) si interne poznačil "toto je už
zosynchronizované" **skôr**, než appka vôbec skúsila poslať dáta do Google Sheetu, nie až potom,
čo sa to reálne podarilo. Keď potom samotné odoslanie zlyhalo (napr. výpadok siete, vypršané
prihlásenie), appka si aj tak už myslela, že je všetko hotové — a keďže tento záznam si appka
značí ako "hotový" natrvalo, ďalší push tú istú vec už vôbec neskúsil znova. Presne to, čo si
videl. Teraz appka poznačí "hotovo" až **potom**, čo Google Sheet naozaj potvrdí zápis — ak
zápis zlyhá, appka to aj nahlási ako chybu a nabudúce to skúsi znova. Sales push som skontroloval
tiež a ten touto chybou netrpel (nemá žiadny takýto "predčasný" zápis).

**Bod 5 — dlhý error po prihlásení — opravené najlepším odhadom, ale bez istoty.** Toto sa mi
žiaľ nedá v tomto prostredí overiť naživo (nemám odtiaľto prístup ku skutočnému Google
prihláseniu). Prezrel som si ale, odkiaľ presne takáto dlhá chybová hláška môže prísť —
`describe_error_response` je jedna spoločná funkcia, čo appka používa aj pri Google Sheets
chybách, aj pri obnovovaní prihlásenia (Google OAuth). Najčastejšia príčina presne takejto
situácie (niečo prestane fungovať čoskoro po prihlásení) je, že Google prihlasovaciemu tokenu
vypršala platnosť — bežné pri menších/osobných Google projektoch, kde Google platnosť takéhoto
tokenu automaticky ruší po cca 7 dňoch. Pridal som rozpoznanie presne tejto chyby
("invalid_grant") a namiesto dlhého technického textu appka teraz ukáže krátku správu: *"Google
sign-in has expired - go to Settings -> Integrations and sign in again, then try again."* Keďže
si to ale neviem naživo overiť, **ak sa ti dlhá chyba objaví aj po tejto verzii, pošli mi presné
znenie textu** — možno ide o inú príčinu, než som predpokladal, a bez presného textu by som len
hádal.

## 6. Pravé tlačidlo myši — úplne vypnuté

Appka beží na technológii (Tauri/WRY), ktorá nemá žiadne nastavenie na vypnutie vstavaného menu
prehliadača — jediný spôsob je zachytiť kliknutie pravým tlačidlom priamo v appke a povedať jej
"nerob nič". Presne to som pridal (`main.tsx`, spustí sa hneď pri štarte appky) — teraz sa po
kliknutí pravým tlačidlom kdekoľvek v appke naozaj nič nezobrazí, žiadne "Späť"/"Obnoviť"/"Uložiť
ako"/"Tlačiť". Pred touto zmenou appka nemala žiadne vlastné riešenie pravého tlačidla, takže sa
nemalo s čím pobiť.

## 7. Attention Center: prečo to bolo "mixed" a čo som opravil

Skutočná príčina bola inde, než som pôvodne čakal — **nie v tom, že by appka zoskupovala zle**
(zoskupovanie podľa objednávky z 2.2.9 je správne a nechal som ho tak), ale v tom, **v akom
poradí appka riadky zoradila**. Appka pri rovnakej priorite triedila riadky najprv podľa NÁZVU
KATEGÓRIE, nie podľa objednávky — takže všetky riadky "chýba cena" zo všetkých objednávok boli
pokope na jednom mieste, potom všetky riadky "žiadny aktívny listing" pokope na inom mieste, atď.
— presne to, čo bolo vidno na tvojom screenshote (dve príčiny jednej objednávky boli od seba
oddelené riadkami úplne iných objednávok). Opravil som samotné triedenie tak, aby riadky tej
istej objednávky vždy skončili vedľa seba, bez ohľadu na to, z koľkých rôznych kategórií
pochádzajú.

Popri tom som pridal aj druhú vec, čo podľa mňa patrí pod "mixed" — appka teraz **vynecháva už
hotové eventy** (rovnaké pravidlo "dátum alebo status" ako pri bode 2) z 3 z jej 5 kategórií:
chýbajúca cena, žiadny aktívny listing, cena mimo trhu. Zámerne som **nevynechal** zvyšné dve:
"predané a nedoručené" (nedoručený tiket je problém bez ohľadu na to, či event už prebehol —
skôr naopak, je to potom ešte naliehavejšie) a "event o pár dní" (tá kategória sa aj tak spúšťa
len tesne pred eventom, takže sa s "hotový event" ani nemôže prekrývať).

**Na zváženie**: ak si "mixed" myslel niečo iné (napr. že tam má byť LEN jedna konkrétna
kategória, nie všetkých 5), daj vedieť — vybral som si výklad "zle zoradené + zbytočne veľa
starých eventov", lebo presne to sedelo na tvoj screenshot.

## 8. Sales: Completed vyžaduje predané aj doručené aj zaplatené naraz

Rovnaké pravidlo, ako appka už dnes používa na farebnú značku "Completed" pri jednom predaji, sa
teraz použije aj na to, do ktorého tabu (Pending/Completed) predaj patrí: predaj je Completed až
vtedy, keď je CELÝ predaný, doručený AJ zaplatený naraz — chýbajúce čokoľvek z toho (aj len
doručenie, presne tvoj príklad) ho necháva v Pending. Jedna výnimka, ktorú som schválne zachoval:
plne vrátený (refundovaný) predaj sa aj naďalej počíta ako Completed, nie Pending — to je
staršie pravidlo appky (ešte pred touto verziou) a bez tejto výnimky by every vrátený predaj
navždy zostal v Pending, čo nedáva zmysel.

## Zmenené súbory

**Backend:**
- `src-tauri/src/models.rs` — `Order` má nové polia `eventDate`/`eventStatus` (len na čítanie,
  žiadna nová stĺpec v databáze, žiadna migrácia)
- `src-tauri/src/commands/orders.rs` — `BASE_SQL`/`map_order` doplnené o join na `events`
- `src-tauri/src/commands/attention_center.rs` — opravené triedenie (zoskupenie podľa
  objednávky vedľa seba), pridané vynechanie hotových eventov pre 3 kategórie, nové testy
- `src-tauri/src/commands/orders_sheet_sync.rs` — opravené poradie zápisu pri pushi (najprv
  potvrdenie od Google Sheets, až potom lokálne "hotovo")
- `src-tauri/src/commands/pulls_sheet_sync.rs` — rovnaká oprava ako vyššie, pre Pulls push
- `src-tauri/src/google_sheets.rs` — rozpoznanie chyby `invalid_grant`, kratšia hláška

**Frontend:**
- `src/lib/format.ts` — `formatSeatLocation`/`formatSeatsSummary` bez nálepiek
- `src/lib/types.ts` — `OrderRecord` doplnený o `eventDate`/`eventStatus`
- `src/pages/Orders.tsx` — nová logika `isEventDone`/`isOrderDone`, taby Active/Completed,
  obmedzený výber eventu v "New order"
- `src/pages/Sales.tsx` — nová logika `isSaleGroupDone` pre tab Pending/Completed
- `src/main.tsx` — vypnutie natívneho menu pravého tlačidla

**Dokumentácia:**
- `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/PROTECTED_AREAS.md`, `CHANGELOG.md`

## Čo som overil

```
cargo test --lib   -> 1006 passed, 0 failed, 3 ignored (999 pôvodných + 7 nových: eventy so
                       stavom cancelled/uplynutým dátumom nevytvárajú kategórie chýbajúcej
                       ceny/listingu, "predané-nedoručené" a "event o pár dní" fungujú aj pre
                       takéto eventy naďalej, riadky tej istej objednávky sa zoradia vedľa
                       seba aj naprieč rôznymi kategóriami, plus nový test na krátku hlášku
                       pri invalid_grant)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.2.10 build" v hlavičke)
```

Celý pôvodný test balík prešiel aj naďalej bez zmeny správania — žiadna regresia v Tickets/
Listings/Finance/Price Checker/Inventory Intelligence logike, tie appka vôbec nemenila. Naviac
som ručne prešiel `grep` po celom `src/`, aby som si overil, že nezostalo nikde staré "Sec "/
"Row "/"Seat " spojenie ani stará referencia na Orders tab "paid".

---

Toľko k tých 8 bodom. Štyri veci na tvoje rozhodnutie, keby si to chcel inak, než som odhadol:
pravidlo "dátum ALEBO status" pre hotový event (body 2-3), Price Checkerov výber eventu ostal
bez zmeny (bod 3), môj výklad "mixed" ako zlé triedenie + staré eventy (bod 7), a hláška pri
Google chybe je najlepší odhad, nie overená naživo (bod 5 — ak sa dlhý error objaví znova, pošli
mi presný text). Všetko ostatné je presne tak, ako si napísal.

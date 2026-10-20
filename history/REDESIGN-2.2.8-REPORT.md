# TIQR Manager 2.2.8 — Global Attention Center

Reagujem na tvoju správu:

> *"Pome na ďalší focused task: TIQR Manager 2.2.8 — Global Attention Center. [...] Pridaj na
> Dashboard nový kompaktný blok 'Attention Center', ktorý zobrazuje veci zo všetkých eventov,
> ktoré aktuálne vyžadujú pozornosť. Použi existujúce dáta a logiku tam, kde už existuje.
> Kategórie: event do 48h + nepredané tikety; nepredaný ticket bez active listing; nepredaný
> ticket bez listing price; ticket výrazne mimo market ceny, iba ak pre daný event existujú
> uložené Price Checker dáta; sold ticket, ktorý ešte nemá dokončený delivery workflow, iba ak
> existujúce dáta spoľahlivo umožňujú takúto kontrolu. Žiadne automatické určovanie ani
> navrhovanie ceny, žiadne pricing podľa section/row, tier/level zatiaľ nepoužívaj na výpočet
> ceny. UI: zoskupiť podľa priority Critical/Attention/Info, pri každej položke Event/Ticket
> (ak relevantný)/dôvod/hodnotu-dátum, zoradenie podľa priority a najbližšieho eventu, klik
> otvorí existujúci event/ticket detail, žiadny veľký redesign. Jeden ticket nemá byť
> zobrazený viackrát pod rovnakým dôvodom; ak je alertov veľa, rozumný limit + Show all bez
> straty dát. Ak sa niečo nedá spoľahlivo vypočítať, nevymýšľaj fallback, vynechaj a napíš
> prečo. [...] STOP po tomto tasku."*

Toto je presne to, čo som spravil.

## 1. Odkiaľ berie dáta — 4 z 5 kategórií sú doslova to, čo už appka má

Namiesto písania novej "attention" logiky som znovu použil presne tie isté pravidlá, ktoré už
rok fungujú v Event Workspace → Overview → Inventory Intelligence (2.2.6): event do 48h s
nepredanými tiketmi, nepredaný ticket bez listing price, nepredaný ticket bez active listing,
a ticket výrazne (20%+) mimo priemernej trhovej ceny — posledné iba ak pre daný event už
existujú reálne Price Checker dáta.

Nový backend príkaz `get_attention_center` (`commands/attention_center.rs`) prejde všetky
eventy, ktoré majú aspoň jeden nepredaný tiket, a pre každý z nich rovno zavolá tú istú
funkciu, čo používa Inventory Intelligence (`get_inventory_intelligence_impl`) — žiadna druhá
kópia tejto logiky. Výsledok len "rozbalí" z jedného počtu na event na samostatné klikateľné
riadky. Výhoda: keď niekedy zmeníš prah (napr. 20% na inú hodnotu, alebo "48h" na iný počet
dní), zmení sa automaticky aj tu — nie je čo ručne synchronizovať medzi dvoma miestami.

## 2. Nová, piata kategória — sold ticket bez dokončeného delivery

Toto v appke ešte nebolo, tak som preveril, či sa to dá spoľahlivo vypočítať skôr, než by som
to buď vynechal, alebo si niečo vymyslel. Appka už od verzie 2.0.66 má reálny, používaný
"Completed" indikátor (na Orders aj Sales), ktorý presne takto počíta doručenie:
**`tickets.status = 'sold'` A `tickets.delivery_status` je doslova text `"Delivered"`** —
čokoľvek iné (prázdne, "Not delivered", alebo starý voľný text) sa počíta ako nedoručené.
Toto pravidlo je teda spoľahlivé a už dávno overené v praxi (aj bulk akcie "Mark Delivered" v
appke pracujú presne s touto dvojicou hodnôt) — **nevynechal som to, dá sa to počítať
naozaj presne.**

Keď sa ticket vráti refundom, appka mu status vráti späť na `available` (rovnaká logika ako
všade inde), takže refundnutý ticket z tejto kategórie automaticky vypadne sám — netreba
žiadnu extra podmienku.

## 3. Priority: Critical / Attention / Info — nové rozhodnutie, ktoré som musel spraviť ja

Zadal si 3 úrovne, ale nepovedal si, ktorá kategória kam patrí — toto je teda moje rozhodnutie,
píšem ho sem nahlas, aby si ho mohol opraviť, ak si to predstavoval inak:

- **Critical**: event do 48h (nedá sa to už nijako vrátiť späť, keď event prejde) a sold-bez-
  delivery **ak** je event tiež do 48h alebo už bol (nedoručený ticket tesne pred/po evente je
  vážny problém).
- **Attention**: nepredaný ticket bez listing price, nepredaný ticket bez active listing (reálne
  medzery, ktoré by si mal doplniť), a sold-bez-delivery keď je event ešte ďaleko.
- **Info**: ticket mimo market ceny — toto som zámerne dal nižšie ako Attention, pretože si
  explicitne povedal, že appka nesmie navrhovať ani naznačovať cenovú akciu; je to len
  postreh/informácia, nie vec, ktorú "musíš" riešiť.

Sold-bez-delivery bez akéhokoľvek dátumu eventu (event dátum nie je vyplnený) dostáva
Attention, nikdy nie Critical — keď sa nedá overiť naliehavosť, radšej nižšia úroveň ako
vymyslená vysoká.

## 4. Duplicity a limit zobrazenia

Jeden ticket sa pod tým istým dôvodom nemôže objaviť dvakrát (kľúč je vždy `dôvod:ID`).
Rôzne dôvody áno — napríklad ticket bez ceny AJ bez active listingu sa objaví ako dve
samostatné položky, presne ako si to povolil. Backend vždy vráti úplný zoznam (nič sa
nestráca), a UI ho zobrazuje po skupinách (Critical/Attention/Info), každá skupina zvlášť s
tlačidlom "Show N more" — presne ten istý mechanizmus, čo už roky používajú karty "Recent
events/orders/sales" na Activity tabe. Žiadny nový UI vzor, len ten istý znova použitý.

## 5. Klikateľnosť / navigation

Appka dnes má presne jeden spôsob, ako preskočiť na konkrétny ticket naprieč stránkami:
`Tickets.tsx?code=...` (predvyplní vyhľadávanie). Použil som presne toto — klik na položku s
ticketom otvorí Tickets stránku s tým ticketom. Klik na "event do 48h" (ktorý nemá jeden
konkrétny ticket, pozri bod 6 nižšie) otvorí ten event priamo (Event Workspace). Žiadny nový
routing systém.

## 6. Jedna dôležitá odchýlka od "Ticket/code ak relevantný": event-do-48h je jeden riadok za event

Pri "event do 48h" som spravil jeden riadok ZA EVENT (s počtom nepredaných tiketov v texte),
nie jeden riadok za každý tiket. Dôvod: tvoje vlastné zadanie hovorí "Ticket/code (ak je
relevantný)" — teda pripúšťaš, že pri niektorých kategóriách ticket nie je relevantný. Keby
mal event napríklad 40 nepredaných tiketov 1 deň pred konaním, dostal by si 40 takmer
identických riadkov, čo by presne porušilo tvoje "UI musí zostať prehľadné". Rovnaký spôsob
už dnes používa aj existujúci zoznam "Upcoming events" na Dashboarde — jeden riadok na event.
Ostatné 4 kategórie SÚ za jeden konkrétny ticket, presne ako si žiadal.

## 7. Čo som zámerne nechal bez zmeny

- **Existujúci Dashboard "Attention" blok a zvonček (alert bell)** hore vpravo — tie zostávajú
  presne také, aké boli. Riešia iné veci (pulls pred deadline, pending sales, missing listing
  price PODĽA OBJEDNÁVKY, upcoming events v 14-dňovom okne) a slúžia aj pre notifikácie na
  pozadí — nemenil som ich ani som ich nezlučoval s novým blokom. Nový "Attention Center" je
  samostatný, doplnkový blok.
- **Žiadna nová migrácia, žiadna nová závislosť.**
- **Tier/section/row** som nikde nepoužil ako faktor pri "outside market price" — hodnota, čo sa
  tam zobrazí, je vždy len ticketova VLASTNÁ, už predtým zadaná listing cena, nikdy nič
  vypočítané ani navrhnuté appkou.

## Zmenené súbory

**Backend:**
- `src-tauri/src/commands/attention_center.rs` — nový súbor, 1 nový príkaz
  (`get_attention_center`) + 10 nových testov
- `src-tauri/src/commands/inventory_intelligence.rs` — `EVENT_SOON_DAYS` zmenené z privátnej
  na `pub(crate)` konštantu (aby ju nový modul mohol znovu použiť) — žiadna zmena správania
- `src-tauri/src/models.rs` — nové DTO `AttentionCenterItem`
- `src-tauri/src/commands/mod.rs`, `src-tauri/src/lib.rs` — registrácia nového príkazu

**Frontend:**
- `src/lib/types.ts` — nové rozhranie `AttentionCenterItem`
- `src/lib/api.ts` — `getAttentionCenter()`
- `src/pages/Dashboard.tsx` — nový blok `AttentionCenterBlock` (+ `AttentionCenterGroup`,
  `AttentionCenterRow`) na Activity tabe, nad existujúcim "Attention" blokom; vlastný fetch pri
  načítaní stránky (nezávislý od filtra obdobia)

**Dokumentácia:**
- `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/PROTECTED_AREAS.md`, `CHANGELOG.md`

## Čo som overil

```
cargo test --lib   -> 995 passed, 0 failed, 3 ignored (+10 nových testov: event do/mimo 48h,
                       ticket bez active listing, ticket bez listing price, market alert iba
                       s reálnymi dátami, sold-bez-delivery sa spustí/vylúči doručené/vylúči
                       refundnuté, sold-bez-delivery priorita podľa blízkosti eventu, funguje
                       aj pre úplne vypredaný event, ten istý ticket pod 2 dôvodmi ale nikdy
                       dvakrát pod 1, správne zoradenie priority+dátumu)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.2.8 build" v hlavičke)
```

Celý existujúci test balík (985 testov pred týmto taskom) prešiel bez jedinej zmeny — žiadna
regresia v Orders/Tickets/Sales/Listings/Finance/refund-resell/Inventory Intelligence logike.

---

Teraz k tvojim siedmim bodom, presne v poradí:

**1. Zmenené súbory** — pozri zoznam vyššie (4 backend súbory, 3 frontend súbory, 3 stavové
dokumenty, plus tento report).

**2. Nové query/service/command logiky** — jeden nový príkaz `get_attention_center`. 4 z 5
kategórií sú priame volanie existujúcej `get_inventory_intelligence_impl` (žiadna nová
biznis logika, len nové poskladanie výstupu naprieč eventmi). Piata kategória (sold bez
delivery) je nová, ale postavená na existujúcom, už používanom `delivery_status = 'Delivered'`
pravidle (2.0.66).

**3. UI zmeny** — nový kompaktný blok "Attention Center" na Dashboard → Activity tab, nad
existujúcim "Attention" blokom. Zoskupené podľa priority (Critical/Attention/Info), každá
skupina s "Show N more" tlačidlom. Žiadny redesign zvyšku Dashboardu.

**4. Alert pravidlá** — 5 kategórií: event do 48h + nepredané tikety (Critical, jeden riadok
za event), nepredaný ticket bez listing price (Attention), nepredaný ticket bez active
listing (Attention), ticket 20%+ mimo trhovej ceny iba s reálnymi Price Checker dátami
(Info), sold ticket bez `delivery_status = 'Delivered'` (Critical ak je event do 48h alebo už
prešiel, inak Attention). Presné mapovanie na Critical/Attention/Info je moje rozhodnutie —
pozri bod 3 vyššie v texte.

**5. Navigation** — ticket-položky vedú na `Tickets.tsx?code=...` (existujúci deep-link),
event-položka (event do 48h) vedie na `/events/:id`. Žiadny nový navigačný mechanizmus.

**6. Test výsledky** — `cargo test --lib`: 995 passed / 0 failed / 3 ignored (+10 nových
testov). `tsc -b` aj `npm run build` bez chýb.

**7. Limity** —
- Priority mapovanie (Critical/Attention/Info) je moje rozhodnutie, nie tvoje presné zadanie —
  ľahko zmeniteľné, je to jeden `match` blok v `attention_center.rs`.
- "Event do 48h" je jeden riadok za event (s počtom tiketov v texte), nie jeden za tiket —
  zdôvodnené v bode 6 vyššie; ak by si to chcel radšej po tiketoch, je to malá zmena.
- Sold-bez-delivery sa počíta iba z `tickets.status`/`delivery_status` — appka nevie nič o tom,
  KTO má ticket doručiť ani AKO (žiadny kuriérsky/transferový tracking mimo tohto jedného poľa).
- Existujúci Dashboard "Attention" blok a zvonček zostávajú nezmenené a samostatné — nezlúčil
  som ich s novým blokom (pozri bod 7 vyššie v texte).

## STOP

2.2.8 hotové, otestované a zabalené. Ako si žiadal — žiadne ďalšie features, končím tu.

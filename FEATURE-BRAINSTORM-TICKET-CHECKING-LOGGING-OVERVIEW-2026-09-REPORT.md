# TIQR Manager — 10 vecí: kontrola lístkov, zapisovanie, prehľad (september 2026)

Zadanie: nielen financie — všetko okolo kontrolovania lístkov, zapisovania a
prehľadu, čo by appke pomohlo.

**Vzťah k existujúcim dokumentom** (nech sa nič neopakuje): `FEATURE-
BRAINSTORM-2026-09-REPORT.md` (20 návrhov) už pokryl auto-delist naprieč
marketplacmi (#1), in-hand date countdown (#20), wanted-list (#14),
customer directory (#12-13), payout reconciliation (#5), chargebacky (#8).
`FEATURE-BRAINSTORM-MONEY-APPS-AI-2026-09-REPORT.md` (predošlá odpoveď) už
pokryl "opýtaj sa dát", AI zhrnutie, review queue, predpoveď. Toto je tretí,
odlišný uhol - konkrétne kontrola pravosti/doručenia lístka, rýchlosť
zapisovania a to, ako appka ukazuje "čo je dôležité teraz".

Skontroloval som si aj `models.rs`'s `Ticket` štruktúru priamo v kóde: appka
už dnes má pole `ticket_type` (E-ticket/PDF/Mobile transfer/Physical/Will
call), ale nemá žiadne pole na barcode, súbor lístka, ani históriu zmien
statusu - viacero návrhov nižšie na to priamo nadväzuje.

---

## 1. Prečo "naskenuj a over lístok" nemôže byť jedna univerzálna funkcia

Zistenie, ktoré mení, ako by sa malo o "kontrole lístkov" vôbec rozmýšľať:
Ticketmaster's SafeTix (mobilné/rotujúce čiarové kódy) generuje nový kód
každých 15 sekúnd z časovo-obmedzeného kľúča a samotný kód sa dá vôbec
stiahnuť/zobraziť až približne 20 hodín pred akciou. Inak povedané - pri
mobile transfer lístku NEEXISTUJE stabilný barcode, ktorý by sa dal
naskenovať a "zapísať" vopred, nech appka urobí čokoľvek. Naproti tomu PDF/
Physical/Will call lístky majú barcode alebo kód, ktorý je stabilný od
momentu, čo ho dostaneš. Záver: akákoľvek budúca "sken a over" funkcia musí
vetviť podľa `ticket_type` (to pole appka už má) - nie jedna funkcia pre
všetko, ale dve úplne odlišné, každá pravdivo pomenovaná podľa toho, čo
reálne dokáže overiť.

## 2. Marcový 2026 mobile-only rollout od Ticketmastera - obchodné riziko, nie len feature nápad

Priamo súvisí s #1: Ticketmaster v marci 2026 spustil nový mobilný systém,
ktorý navyše blokuje screenshoty/nahrávanie obrazovky na telefóne a lístky
opisuje ako "permissioned credentials" - vydavateľ kontroluje, KEDY a AKO sa
dajú preposlať, nie ty. Toto priamo ohrozuje bežný spôsob odovzdania PDF
lístka, na ktorý sú resselleri zvyknutí. Nie je to niečo, čo appka vyrieši
kódom - ale appka ti vie ukázať tvoju vlastnú expozíciu: jednoduchý report
"koľko z tvojich aktuálnych/nadchádzajúcich lístkov je Mobile transfer vs.
PDF/Physical" - čistá agregácia nad dátami, ktoré appka už má, žiadna nová
logika.

## 3. Fulfillment checklist podľa spôsobu doručenia, nie jedno univerzálne "doručené"

Fulfillment Center dnes (Pending/Awaiting Payment/Awaiting Delivery/Ready to
Complete) je rovnaký pre každý `ticket_type`. V realite je to inak: Mobile
transfer potrebuje dva kroky s potvrdením (poslal si transfer → kupujúci ho
prijal), Physical potrebuje sledovanie zásielky, Will call potrebuje len
meno na zozname. Návrh: krok "Ready to Complete" sa rozpadne na malý,
podľa `ticket_type` iný checklist namiesto jedného tlačidla - stále ručné,
žiadna nová automatizácia, len presnejšie sedí na to, čo reálne robíš.

## 4. Odfotenie lístka - dve odlišné úlohy naraz

Pre lístky, kde reálne existuje súbor/obrázok lístka (PDF, Physical) - dve
odlišné veci, obe postavené na tom istom AI vzore z `ai_categorize.rs`:

- **Rýchlejšie zapisovanie**: odfotíš/vložíš lístok, AI prečíta čo je na
  ňom napísané (sekcia/rad/miesto/kód) a predvyplní formulár namiesto
  ručného prepisovania - rovnaký princíp ako bežné QR/barcode "scan to
  receive" appky v skladovom hospodárstve, len cez čítanie textu namiesto
  dekódovania kódu.
- **Kontrola presnosti**: appka porovná, čo je NAOZAJ napísané na lístku, s
  tým, čo si (alebo appka za teba) zapísala - a upozorní pri nezhode
  (preklep v rade/mieste). Čisto informatívne, nikdy nič samo neprepíše.

Toto je iné než návrh #4 v money/AI dokumente (ten čítal potvrdzovací
email o kúpe) - toto číta samotný lístok.

## 5. Poznámky o vstupe na akciu, viditeľné na jednom mieste

Bežná realita ticket resellingu: kupujúci sa pýta "čo potrebujem so sebou",
a odpoveď je iná pre každé miesto/akciu (občiansky doklad musí sedieť s
menom na lístku, zákaz tašiek, konkrétny vchod pre daný sektor...). Dnes to
nemá appka kde zapísať okrem voľných poznámok pri evente. Návrh: jedno
krátke textové pole na event/venue úrovni, "čo treba vedieť pred vstupom" -
viditeľné na day-of pohľade (#7 nižšie), aby si to nemusel hľadať v maile
od organizátora zakaždým znova.

## 6. "Čo potrebuje moju pozornosť dnes" - zoradené podľa dátumu, nie podľa kategórie

Attention Center dnes zoskupuje podľa kategórie (5 boxov). Skutočné
ticketing-ops nástroje zvyknú viesť s časovo zoradeným pohľadom naprieč
všetkými kategóriami naraz - event, ktorý potrebuje fulfillment zajtra, by
mal appku zaujímať viac než nezacenený listing na event o 3 mesiace, aj keď
sú v rôznych kategóriách. Návrh: voliteľné triedenie "podľa dátumu" nad tými
istými dátami, ktoré Attention Center už má - žiadna nová kategória, len
druhý pohľad na to isté.

## 7. Day-of pohľad - jeden event, jedna obrazovka, všetko na nej

Žiadna existujúca obrazovka dnes nedá "idem odovzdávať lístky na tento
event - ukáž mi k nemu všetko naraz": ktoré lístky, ktorí kupujúci, aký
spôsob doručenia, otvorené položky z Attention Centra, in-hand termín
(#20 zo starého dokumentu), poznámky o vstupe (#5 vyššie). Event Workspace
je blízko, ale je stavaný na priebežnú správu, nie na rýchly pohľad tesne
pred/počas akcie. Návrh: kompaktný, prevažne read-only "day-of" súhrn -
dobre sa dopĺňa aj s mobilným náhľadom zo starého dokumentu (#16).

## 8. História zmien statusu lístka - kto/kedy/prečo, nie len nová hodnota

Dnes sa `status` lístka len prepíše na novú hodnotu. Profesionálne
inventory/ops nástroje si vedú krátky audit log pri každej zmene statusu -
presne preto, že "prečo sa tento lístok zmenil z Active na Cancelled" je
presne otázka, ktorá príde na rad o mesiace neskôr pri spore alebo pri
účtovnej kontrole. Priamo nadväzuje na chargeback návrh (#8) zo starého
dokumentu. Malé rozšírenie schémy (nová tabuľka `ticket_status_history`),
nemení žiadnu existujúcu cenovú ani stavovú logiku.

## 9. Týždenné pripomenutie "sedí môj počet s realitou"

Sklad/inventory appky stoja na pravidelnej kontrole "systém vs. skutočnosť"
(odhaľuje straty/omyly skôr, než narastú). Pre TIQR: nenápadné, voliteľné
pripomenutie - "máš 12 Active lístkov na akcie tento týždeň, ešte
nedoručené - over, že ich máš reálne v rukách" - nič nové okrem
naplánovanej pripomienky nad dátami, ktoré appka už má (Active lístky +
blížiace sa dátumy). Len upozorní, nič samo nezmení.

## 10. Kontrola nech ostane úprimná: appka overí SEBA SAMU, nie pravosť lístka

Platí to isté pravidlo ako v predošlej odpovedi (AI navrhne, človek
potvrdí) - tu ešte konkrétnejšie: nič z vyššie uvedeného by sa nemalo
predávať ako "overenie pravosti lístka". Každý návrh vyššie porovnáva
TVOJE VLASTNÉ záznamy s tým, čo odfotíš/naskenuješ/zapíšeš - čo odchytí
tvoj vlastný preklep alebo medzeru, ale nevie (a ani nemôže) overiť, že
lístok je naozaj pravý tak, ako to vie len systém vydavateľa (pozri #1 -
presne to je teraz zámerne uzamknuté len na jeho stranu). Stojí za to mať
toto jasne povedané, nech žiadna budúca funkcia nesľubuje viac, než vie
splniť.

---

## Keby som mal tipovať, od čoho začať

**#6** (triedenie Attention Centra podľa dátumu) a **#2** (report expozície
voči mobile-only) sú takmer zadarmo - žiadna nová tabuľka, len iný pohľad
na dáta, ktoré appka už má. **#8** (história statusu) je malá, ale s
reálnou hodnotou pri budúcom spore. **#4** (odfotenie lístka) je najväčší
kus práce zo zoznamu a stojí na tom istom Anthropic kľúči ako predošlé
návrhy - dáva zmysel až po tom, čo sa appka reálne rozhodne pre aspoň jednu
z AI funkcií z predošlého dokumentu, nie ako prvý krok samostatne.

---

## Zdroje (skutočne použité pri prieskume)

- [Reverse Engineering TicketMaster's Rotating Barcodes (SafeTix)](https://conduition.io/coding/ticketmaster/)
- [Ticketmaster Touts "Enhanced Protections" in New Mobile-Only Ticket Rollout (TicketNews, marec 2026)](https://www.ticketnews.com/2026/03/ticketmaster-touts-enhanced-protections-in-new-mobile-only-ticket-rollout/)
- [How to use QR codes for inventory management (Jotform Blog)](https://www.jotform.com/blog/qr-code-inventory-management/)
- [The Essential Barcode Inventory Apps Guide (Descartes Finale)](https://www.finaleinventory.com/guides/barcode-inventory-app/)
- [Preventing Ticketing Fraud in 2026: Technology and Strategies (Softjourn)](https://softjourn.com/insights/prevent-ticketing-fraud)
- [How to Prevent Ticket Scalping and Fraud: The Ticketing Managers Guide (vivenu)](https://vivenu.com/blog/prevent-ticket-scalping-fraud-guide)

Zámerne len prieskum a návrhy, nič som neimplementoval. Ak chceš niektorý z
bodov (z ktoréhokoľvek z troch dokumentov) reálne postaviť, povedz ktorý a
spravím to ako samostatnú úlohu - najprv `CURRENT_STATE.md`/
`PROTECTED_AREAS.md`, cielené zmeny, testy, report, presne ako doteraz.

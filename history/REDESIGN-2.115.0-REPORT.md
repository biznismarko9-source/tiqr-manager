# TIQR Manager 2.115.0 — jeden počítač číta Gmail za oba, a mail podľa seba

Tvoje správy:

- *„mute nastaviť aj podľa objectu"*
- *„aj ten mail funguje na oboch zariadeniach rôzne, chcem to spojiť do
  jedného, lebo v kuse to je, že Google asked TIQR to slow down a chvíľu
  nerobí, potom chvíľu hej a zas nie a dookola"*
- a na otázku, ako to spojiť: *„najlepšie, ak to vedia robiť oba podľa toho,
  ktorý práve runnuje, ak naraz, tak len jeden a ten to bude posielať tomu
  druhému, aby to bolo obmedzené"*
- *„celkovo do mailu pridať viac funkcií, nastavení, aby všetko si človek
  vedel dať, ako chce"*
- *„taktiež menšie obrázky v mailoch sa nezobrazujú"*
- *„a keď je mail väčší, tak si vieš potiahnuť posuvníkom, aby sa zväčšila tá
  časť, kde je mail"*
- *„aj túto časť kus stiahnuť, nech je menšia a hneď viac sa toho zmestí"*
  (bočné menu)
- *„a pri new order nejde meniť kvantita manuálne"*
- *„aj orders sú problém, oprav"* (tabuľka lístkov v objednávke)

---

## 1. Dva počítače, jeden čitateľ Gmailu

### Prečo Google stále spomaľoval

Gmail dovolí jednému účtu prečítať zhruba 50 správ za sekundu. Jeden počítač
číta tesne pod touto hranicou. Keď čítali Mac aj Windows naraz, išli spolu
nad ňu. Google spomalil oba, oba čakali, potom začali znova naraz, a tak
dookola. Presne to bolo „chvíľu nerobí, potom chvíľu hej".

### Ako to funguje teraz

- **Beží TIQR len na jednom počítači?** Číta Gmail sám, ako doteraz.
- **Beží na oboch?** Gmail číta len jeden z nich. Ten druhý z Gmailu nečíta
  nič. Maily mu posiela ten, ktorý číta, po dávkach cez Firebase tvojho TIQR
  účtu (ten istý, cez ktorý ide živá synchronizácia).
- **Každú dávku druhý počítač uloží a hneď ju vo Firebase zmaže.** Maily tam
  teda neostávajú, len cez Firebase prejdú.
- **Ktorý číta?** Ten, ktorý bol spustený skôr. Keď ho zatvoríš, čítanie
  odovzdá hneď a druhý začne čítať Gmail sám do minúty. Keď počítač len
  zmizne (pád, zaklapnutý notebook), druhý to prevezme asi o tri minúty.
- **Prijímajúci počítač si každý mail prečíta sám**, ako keby prišiel
  z Gmailu: vlastné pravidlá a vlastný Automatic. Záznamy z mailov robí aj
  naďalej len počítač s nastavením „This computer makes the records", takže
  sa nič nevytvorí dvakrát. Na Discord nejde druhá správa „prišiel mail".
- **Čo cestuje:** len to, čo dal Gmail: odosielateľ, predmet, náhľad,
  dátum, štítky, prečítané a hviezdička, plus text, z ktorého TIQR číta
  čísla. Nikdy necestuje kategória, OTP kód ako taký ani to, kam bol mail
  v TIQR presunutý. Prijímajúci počítač kontroluje všetko, čo príde, ako
  nedôveryhodný vstup: zlé ID, dátum, štítok alebo iná schránka sa neuloží.
- **Zmena štítkov** (prečítané, hviezdička, priečinok v Gmaili) sa prenesie
  len vtedy, keď je novšia než to, čo už prijímajúci počítač má.

### Prvé spojenie

- **Windows** (16 000 zo 100 000) dostane od Macu všetko, čo mu chýba. Pri
  100 000 mailoch to bude asi pol hodiny, potom už len nové maily.
- **Opačne** (Mac, ktorý má prečítaný celý Gmail, dostáva od Windowsu) príde
  len to, čo sa zmenilo za týždeň pred jeho posledným čítaním, nie celá
  schránka znova.

### Kde to vidno

- **Mail → Settings → Mailbox → „Two computers, one reader".** Zapína a
  vypína sa naraz pre oba počítače. Pod tým je jedna veta, napríklad
  „This computer reads Gmail and sends the mail to your Windows computer -
  340 sent".
- **Obrazovka Mail hore:**
  - na prijímajúcom počítači: *„Your Mac reads Gmail and sends the mail here -
    12,345 received."*;
  - na čítajúcom: *„Sends to your Windows computer · 340"*.
- **Sync now na prijímajúcom počítači** poprosí čítajúci počítač, aby sa do
  Gmailu pozrel hneď.
- **Keď niečo nejde, veta to povie oranžovou** a počítač zatiaľ číta Gmail
  sám, ako doteraz. Napríklad *„Firebase refused - publish the new
  firestore.rules"*.

### Na čom to stojí

- **Nové pravidlá Firestore** (súbor `firestore.rules`) treba raz zverejniť
  v Firebase Console, pozri „Čo urobiť" nižšie. Bez nich každý počítač číta
  Gmail sám, ako v 2.114.3.
- **Živá synchronizácia** (Settings → Data) musí byť zapnutá na oboch.
- **Pozastavený mail** (Pause sync) sa nezapája: nečíta za druhý počítač a
  ani od neho nič nedostáva.
- **Hodiny na počítačoch sa nemusia zhodovať.** Či druhý počítač ešte žije,
  posudzuje každý počítač podľa toho, čo videl meniť, nie podľa cudzích
  hodín.
- **Spotreba Firebase:** keď bežia oba celý deň, približne 3 000 zápisov a
  5 000 čítaní denne. Bezplatný limit je 20 000 zápisov a 50 000 čítaní.
  Keby sa limit minul, veta to povie a každý počítač bude čítať sám, kým sa
  limit neobnoví.

## 2. Stíšenie podľa predmetu

**Mute** pri otvorenom maile má teraz dve časti: odosielateľ (ako doteraz) a
**slová z predmetu**.

- TIQR ponúkne predmet bez „Re:", „Fwd:" a bez čísel objednávok, napríklad
  „Your order is confirmed". Dá sa prepísať.
- Stíšia sa všetky maily, ktorých predmet tie slová obsahuje. Na veľkých
  písmenách, diakritike ani interpunkcii nezáleží.
- Doba je rovnaká ako pri odosielateľovi: deň, týždeň, mesiac alebo kým to
  nezrušíš.
- **Mail → Settings → Muted** ukazuje odosielateľov aj predmety. Dajú sa tam
  pridať aj zrušiť.
- Objednávky, predaje a výplaty sa ukážu aj tak, rovnako ako pri
  odosielateľovi.

## 3. Viac nastavení mailu

### Mail → Settings → Reading

- **Reading speed on this computer:**
  - *Full*: ako doteraz.
  - *Gentle*: menej naraz, s pauzami. Veľkú schránku číta niekoľkokrát
    dlhšie, ale Gmailu necháva veľa miesta, napríklad pre pomalý internet.
  - Platí len pre ten počítač, kde ju nastavíš.
- **An opened message counts as read:** hneď (ako doteraz), po 3 sekundách
  otvorenia, alebo len keď ju označíš. Pri šípkach (nižšie) sa tak pri
  rýchlom prechádzaní neoznačí všetko ako prečítané.

### Mail → Settings → Mail screen (nová časť)

- **Mail opens in:** Inbox, Important, Starred alebo „Where I left it"
  (posledný otvorený priečinok na tomto počítači).
- **After moving or deleting the open message:** späť na zoznam (ako
  doteraz), otvoriť ďalší starší, alebo ten nad ním (novší).
- **The list:**
  - *Preview line*: prvé slová pod predmetom;
  - *Tags*: Sale, Payout, Order;
  - *Compact rows*: menej miesta, zmestí sa viac mailov.
- **Keys** (dajú sa vypnúť):
  - ↓ alebo J: ďalší mail; ↑ alebo K: predošlý;
  - S: hviezdička; U: prečítané alebo neprečítané;
  - Delete alebo #: do koša;
  - /: hľadanie.

  Pri písaní ani v dialógu nefungujú. Delete mail vždy len presunie do
  koša, nikdy ho nezmaže natrvalo, a v Koši nerobí nič.

Všetko okrem rýchlosti čítania je rovnaké na oboch počítačoch.

## 4. Obrázky v mailoch

Na tvojom screenshote z Ticketmastera boli namiesto ikoniek a obrázka
prázdne rámčeky. Našlo sa niekoľko dôvodov, prečo obrázok nepríde:

- **Obrázok, ktorý sa pri prvom otvorení nepodaril, z uloženej kópie
  vypadol navždy.** Stačilo, aby server raz neodpovedal. Teraz si kópia
  pamätá každý obrázok a pri ďalšom otvorení sa chýbajúci skúsi znova (najviac
  raz za 5 minút).
- **Kópie uložené pred touto verziou**, v ktorých obrázok chýba, sa pri
  najbližšom otvorení raz znova stiahnu z Gmailu. Ak Gmail práve nie je
  dostupný, ukáže sa stará kópia.
- **Niektoré servery odmietajú program, ktorý sa nepredstaví ako
  prehliadač.** TIQR sa teraz pri sťahovaní obrázka predstaví ako Safari a
  pýta si len formáty, ktoré vie ukázať. Prijme aj AVIF, keď ho server pošle.
- **Skúšala sa len prvá adresa servera.** Keď to bola IPv6 adresa a internet
  doma IPv6 nemá, obrázok neprišiel. Teraz sa skúsia všetky, IPv4 ako prvé.
- **Najviac 40 obrázkov na mail.** Ikonky na konci dlhého mailu (ako
  „Important Event Info") nad touto hranicou už neprišli. Teraz je hranica
  150 a sťahuje sa 8 obrázkov naraz.
- **Adresy „//server/obrazok.png"** a niektoré obrázky vložené priamo
  v maile (`cid:` s inak zapísaným ID) sa nenašli. Teraz sa nájdu.

Obrázky stále sťahuje TIQR sám, nikdy okno s mailom. Sledovacie pixely
ostávajú vynechané a obrázky z webu sa dajú vypnúť ako doteraz.

## 5. Posuvník medzi zoznamom a mailom

**Okraj medzi zoznamom mailov a otvoreným mailom sa dá potiahnuť.** Doľava
zväčšíš mail, doprava zoznam.

- Pri prechode myšou sa ukáže úchyt. Ide to aj šípkami, keď je okraj
  označený.
- Dvojklik vráti pôvodnú šírku.
- Šírka sa pamätá na každom počítači zvlášť, lebo obrazovky sú rôzne.

## 6. Užšie bočné menu

- **Menu je užšie** (176 px namiesto 192 px).
- **Dole pribudlo „Fold the menu".** Zbalí menu len na ikony (56 px), takže
  sa na každú obrazovku zmestí viac. Názvy sa ukážu pri prechode myšou.
  Šípka vedľa ho zase rozbalí.
- Pamätá sa na každom počítači.

## 7. New order: množstvo sa dá písať

**Predtým:** keď bolo vyplnené Seats, stĺpec Qty bol len číslo a nedal sa
zmeniť.

**Teraz je to vždy políčko:**

- **Napíšeš sedadlá** a Qty sa podľa nich nastaví samo, ako doteraz.
- **Zmeníš Qty a sedadlá sú očíslované za sebou?** Upravia sa podľa neho od
  prvého sedadla. Napríklad sedadlo „23" a Qty 3 dá „23-25".
- **Sedadlá nie sú za sebou** (napríklad „A, B") a Qty nesedí? Formulár to
  povie: *„3 tickets but 2 seats - one seat per ticket, or leave the seats
  empty"*. Objednávka sa nevytvorí, kým to nesedí.

## 8. Tabuľky objednávok

Na tvojom screenshote z detailu objednávky sa „Edit" lámalo po písmenách,
stav ukazoval „Availal" a stĺpec Seat bol zbytočne široký. Šírky stĺpcov
tam boli percentá, takže malé stĺpce dostali len pár pixelov.

**Teraz majú pevné šírky**, rovnako ako ostatné tabuľky a od 2.113.1 aj
Finance. Platí to pre tabuľku lístkov v detaile objednávky aj pre zoznam
Inventory.

- Každý stĺpec je taký široký, ako treba na to, čo v ňom je.
- Seat, a v zozname Event, dostane zvyšok.
- Pri príliš úzkom okne sa tabuľka posúva do strany, namiesto aby sa
  stĺpce zmrštili.

## Čo urobiť

1. **Firebase Console → tvoj projekt TIQR → Firestore Database → Rules.**
   Nahraď všetko obsahom súboru `tiqr-manager\firestore.rules` z tohto zipu
   a stlač **Publish**. Bez tohto kroku sa dva počítače nespoja (každý bude
   čítať sám, ako doteraz). Staré pravidlá ostávajú v ňom bez zmeny.
2. Vydaj túto verziu (zip, `1-CLICK-UPDATE.bat`).
3. Nainštaluj aktualizáciu na **oboch** počítačoch.
4. Skontroluj, že živá synchronizácia je zapnutá na oboch
   (Settings → Data).
5. Otvor TIQR na oboch a pozri Mail → Settings → Mailbox. Na jednom má byť
   „This computer reads Gmail and sends the mail to your Windows computer",
   na druhom „Your Mac reads Gmail and sends the mail here". Ak je tam
   oranžová veta, pošli mi screenshot tej časti.
6. Hláška „Google asked TIQR to slow down" by sa už nemala opakovať dookola.
   Ak sa opakuje, pošli screenshot z oboch počítačov.

## Testy

- `cargo test --locked`: 1595 prešlo, 0 zlyhalo (4 ignorované, ako doteraz).
  Nové testy:
  - maily idú po dávkach a druhý počítač si každý prečíta sám (vlastné
    pravidlá);
  - novší pohľad na štítky sa prenesie a starší nikdy;
  - mail inej schránky alebo zlé ID, dátum či štítok sa neuloží;
  - prvé spojenie: schránka prečítaná do konca dostane len posledný týždeň,
    iná všetko;
  - pozastavený počítač sa nezapája;
  - stíšenie podľa predmetu nezávisí od veľkých písmen ani diakritiky;
  - obrázok, ktorý neprišiel, si drží miesto a ukáže sa po skopírovaní;
  - stará kópia so strateným obrázkom sa rozpozná;
  - `cid:` a `//` adresy sa nájdu, AVIF je obrázok;
  - nastavenia obrazovky Mail majú predvolené hodnoty a berú len skutočné
    voľby.
- `npm test`: 53 prešlo. Nové testy:
  - zámok a dávky;
  - hodiny, ktoré nesedia;
  - prítomnosť druhého počítača;
  - návrh slov na stíšenie;
  - sedadlá pri New order („23" a 3 dá „23-25").
- **Kontrola proti emulátoru Firestore so skutočnými `firestore.rules`:**
  13 kontrol, každá prešla. Dva počítače:
  - čítajú len raz a všetko prenesú po dávkach, žiadna dávka neostane;
  - nový mail aj hviezdička prídu hneď;
  - Sync poprosí druhý počítač;
  - zatvorenie odovzdá čítanie hneď;
  - zlá dávka sa odmietne a zmaže;
  - pozastavený počítač sa nezapája;
  - hodiny o 10 minút napred nič nepokazia;
  - zmiznutý počítač sa prevezme;
  - pravidlá odmietnu zlé dokumenty aj cudzí účet.
- Živá synchronizácia proti emulátoru: 9 kontrol, každá prešla, ako
  doteraz.
- `npm run build` prešiel. Hash pre zámok je napísaný bez `BigInt`, ktorý
  najstarší podporovaný Safari na Macu nepozná.
- **Obrazovky sú overené v náhľade** (bez skutočného Gmailu):
  - nastavenia;
  - posuvník (potiahnutie zmenilo šírku zoznamu z 260 na 456 px);
  - šípky prechádzajú zoznamom;
  - zbalené menu;
  - New order („23" a 3 dá „23-25", „A, B" a 3 ukáže chybu);
  - tabuľka objednávky pri šírke okna 1280 aj 1700 px.

## Čo sa tu overiť nedalo

- **Skutočný Gmail, skutočný Firebase projekt a dva skutočné počítače tu
  nie sú.** Spájanie je overené proti emulátoru Firestore so skutočnými
  pravidlami a s napodobeným Rust backendom. Rust strana (export a uloženie
  dávok) má vlastné testy. Ako rýchlo prejde prvých 84 000 mailov na
  Windows, ukáže až veta v nastaveniach.
- **Mail z Ticketmastera zo screenshotu tu nemám.** Opravené sú všetky
  dôvody uvedené vyššie, no ktorý z nich bol ten tvoj, sa tu zistiť nedá. Ak
  po aktualizácii obrázok stále chýba, otvor mail ešte raz o pár minút
  (chýbajúci sa skúsi znova), prípadne stlač „Download again" pod mailom. Ak
  ani potom nepríde, pošli mi, od koho mail je, a screenshot.

## Súbory

- **Rust:**
  - `src-tauri/src/commands/mail_share.rs` (nový: export, uloženie,
    začiatok, info);
  - `mail_sync.rs` (čitateľ čaká, kým číta druhý počítač; mierna rýchlosť);
  - `mail.rs` (stíšenie podľa predmetu, nastavenia obrazovky Mail);
  - `mail_view.rs` (obrázky);
  - `mail_facts.rs`, `shared_settings.rs`, `db.rs` (`tiqr_fold`),
    `mail_store.rs` (krok 6), `models.rs`, `lib.rs`, `Cargo.toml`.
- **Frontend:**
  - `src/lib/mailShareCore.ts`, `mailShareSession.ts`, `mailShare.ts` (nové,
    s testom);
  - `src/lib/seats.ts` (nové, s testom);
  - `mailMute.ts`, `api.ts`, `types.ts`;
  - `src/components/Layout.tsx`, `MailReader.tsx`, `MailSettingsPane.tsx`;
  - `src/pages/MailIntelligence.tsx`, `OrderRowsModal.tsx`, `Orders.tsx`,
    `OrderDetail.tsx`.
- `firestore.rules` (blok `users/{uid}/mail`) a
  `scripts/mail-share-emulator-check.ts` (nové).
- `CHANGELOG.md`, `PROJECT_STATE/*`, verzia v piatich súboroch.

Mailová databáza krok 6 (`labels_at` pre staré maily, index, tabuľka
`mail_share_cursors`), žiadna číslovaná migrácia (ďalšia je 047).

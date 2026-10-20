# TIQR Manager 2.110.0 — Mail nanovo: číta sám, od najnovšieho, bez limitu

> *„Google refused access … pri syncu tento error"*
> *„ani labels neukazuje, najskor treba important, starred, labels, finance a az potom vsetko ostatne"*
> *„napisalo ze fetched a nic tu nedalo"*
> *„chcem aby to citalo spravy od najmladsiej po najstarsiu po tych castiach a samo"*
> *„nechcem 200 message hold limit ale minimalne 100 000"*
> *„nech sa to iste aj webhooks zobrazuju na 2 zariadeniach"*
> *„urobit nejaky lock aby tie maily nemohli len tak zmiznut … ulozit na nejake miesto kde je to safe a kde to zerie najmenej info"*

---

## 1. Prečo to nefungovalo — štyri skutočné chyby, každá nájdená v kóde

**a) Limit 200, o ktorom nikto nevedel.** Obrazovka Mail si pýtala 2 000
mailov. Backend prijímal najviac 1 000 — a keď prišlo viac, potichu vrátil
**200**. Každý priečinok (Starred, tvoje labels, aj čísla vedľa nich) sa
potom filtroval z týchto 200 najnovších **v prehliadači**. Označený mail
starší ako 200 najnovších mohol byť v databáze a nikdy sa neukázal. Presne
to je tvoje „fetched a nič sa nedalo" a prázdny Starred. Na druhom
screenshote to bolo vidno priamo: NFL MEXICO má v databáze **4** maily
(číslo v ľavom paneli), a zoznam ukázal nič.

**b) „Google refused access" nebolo odmietnutie.** Keď Google chce, aby
appka spomalila, odpovie aj kódom 403 — rovnakým, aký používa pre „nemáš
povolenie". Appka každú 403 čítala ako odmietnutie: neskúsila znova, celý
beh skončil touto vetou, a maily, ktoré sa v tej chvíli nepodarilo stiahnuť,
vypadli z fronty. Šesť paralelných sťahovaní pritom sedelo presne na
Googlovom strope (asi 50 mailov za sekundu), takže sa to stávalo.

**c) Hviezdička a labels sa nikdy neaktualizovali.** Mail, ktorý už bol
uložený, appka viac nečítala — takže hviezdička alebo label pridaný v Gmaile
*po* tom, ako sa mail stiahol, sa do TIQR nedostal nikdy. A maily staršie
ako rok sa nečítali vôbec (okno bolo 365 dní), labels nevynímajúc.

**d) Chyba pri čítaní Starred sa potichu zahodila** — ak zlyhala, Starred
sa v ten beh jednoducho nečítal a nič to nepovedalo.

---

## 2. Ako to číta teraz — samo, po častiach, od najnovšieho

**Netreba stláčať Sync.** Appka číta sama, kým je otvorená, pre celú appku
(nie len keď máš otvorený Mail). Jeden krok trvá asi sekundu; kým je čo
čítať, ďalší ide hneď, potom kontroluje nové maily v tvojom intervale.

**Poradie je presne tvoje:**

| Poradie | Čo | Ako ďaleko dozadu |
|---|---|---|
| 0 | **nové maily** (čo práve prišlo) | — |
| 1 | **Important** — odosielatelia, ktorých sleduješ | všetko |
| 2 | **Starred** | všetko, bez dátumu, bez filtra |
| 3 | **tvoje labels** (NFL MEXICO, OASIS, PULLS …) | všetko, bez dátumu, bez filtra |
| 4 | **Finances** — PayPal, Stripe, Revolut, Wise, výplaty, refundy, faktúry, platby | všetko |
| 5 | **všetko ostatné** — Inbox | všetko |
| 6 | Promotions, Spam, Trash | všetko |

V každom priečinku **od najnovšieho po najstarší**. Kde skončil, je uložené
na disku — zavrieš appku, a pri ďalšom spustení pokračuje z tej istej
stránky. V hlavičke vidíš, čo práve číta: *„Reading Starred · 1 240 waiting"*.

**Hviezdičky a labels sa udržujú.** Pri prechode Starred a labels sa
označia aj maily uložené skôr, a keď prechod skončí, z mailov, ktoré už
v Gmaile označené nie sú, sa označenie zloží. Zmeny v Gmaile (nová
hviezdička, label, presun do koša) sa čítajú z Gmailovej histórie zmien.

**Keď Google povie „spomaľ":** appka si prečíta dôvod, všetky sťahovania na
chvíľu zastaví, dávka ostane vo fronte a pokračuje sama o ~20 sekúnd. Žiadny
mail sa nestratí. Paralelné sťahovania sú štyri namiesto šiestich, aby sa to
stávalo čo najmenej.

**Keď Google naozaj odmietne**, appka povie *prečo* — chýba povolenie na
mail, alebo je vypnuté Gmail API, alebo vypršalo prihlásenie — každé svojou
vetou, nie jednou spoločnou.

**Keď vypadne internet** (notebook stratí wifi): appka napíše *„Could not
reach Gmail"*, skúsi znova o 30 sekúnd a z fronty nevyhodí nič — mail sa
počíta ako „nedá sa stiahnuť" len vtedy, keď Google v tej istej dávke
ostatné maily normálne poslal. Keď sa internet vráti, pokračuje tam, kde
bola.

---

## 3. Priečinky — bez limitu

Priečinok je teraz **dotaz do databázy**, nie výrez z 200 riadkov. Načíta
sa 200 najnovších, a pri scrollovaní ďalších 200, a ďalších — bez konca.
Čísla v ľavom paneli počíta tá istá definícia, takže číslo a obsah
priečinka sa nemôžu rozísť.

**Ľavý panel**, v tvojom poradí:

- **Inbox** — všetky prijaté maily okrem spamu, koša a promotions
  (aj archivované; Social / Updates / Forums sú tu, nie zvlášť)
- **Important**, **Starred**, **Finances**
- **OTP codes**, **Needs review** (nechané — na Needs review posiela
  pravidlo „Send to Needs review")
- **Promotions**, **Spam**, **Trash**
- **tvoje labels** — všetky, aj prázdne
- **Settings** dole

Ak OTP alebo Needs review netreba, stačí povedať a dám ich preč.

**Otvorený mail** sa ukazuje celý (do 100 kB textu), s riadkami a
odstavcami. Doteraz bol každý otvorený mail jeden riadok s najviac 400
znakmi.

**Overené na 100 000 mailoch** (test v appke): stránka priečinka 1–2 ms,
ďalšia stránka 2 ms, čísla všetkých priečinkov 0,7 s v testovacom
(pomalšom) builde — vo vydanej appke rýchlejšie. Obrazovka ich počas
dlhého prvého čítania obnovuje najviac raz za 3 sekundy.

---

## 4. Lock a bezpečné miesto

**Maily majú vlastný súbor** `tiqr-manager.mail.sqlite3`, vedľa hlavnej
databázy. Čo to dáva:

- **Cloud sync sa ho nedotkne.** Na Drive ide len hlavný súbor — malý, ako
  bol pred mailom. Pri 100 000 mailoch by inak každý sync posielal ~100 MB
  a každý merge robil 100 MB zálohu.
- **Obnova zo zálohy ani prepnutie účtu ho neprepíšu.**
- **Lock:** databáza sama odmietne zmazať mail — nech sa o to pokúsi
  hocičo (sync, aktualizácia, chyba v kóde). Jediná cesta von je tvoje
  **Delete forever** v koši, a zmazaný mail si appka pamätá, takže ho
  ďalšie čítanie nevráti späť.
- **Najmenej miesta:** ukladá sa len hlavička (od koho, predmet, dátum,
  labels) a krátky náhľad. Telo mailu sa stiahne, až keď ho otvoríš, a
  neuloží sa. Zbytočné riadky logu (*„fetched"* pri každom maile) sa už
  nezapisujú. Zmerané: **100 000 mailov ≈ 85 MB** (asi 0,85 kB na mail).
- **Gmail ostáva originál.** TIQR má od Googlu len povolenie *čítať* — v
  Gmaile nevie nič zmazať ani presunúť. Preto mailový súbor nie je súčasťou
  zálohy (Backup) ani kópie na Drive: keby sa súbor niekedy stratil, appka
  si maily prečíta z Gmailu znova sama.

**Pri prvom spustení 2.110.0** sa existujúce maily presunú do nového súboru
(raz, pár sekúnd) a hlavný súbor sa zmenší. Potom appka prejde všetky
priečinky odznova — už uložené maily nesťahuje znova, len opraví ich
hviezdičky a labels — a dočíta to, čo chýba.

---

## 5. Dva počítače

**Na oboch počítačoch je rovnaké:** pripojenie Gmailu, pravidlá,
odosielatelia, filter, interval, a **webhooky** (Discord aj notifikácie).
Prejdú cez cloud sync; keď niečo zmeníš na oboch, platí novšia zmena —
aj odpojenie, takže sa nevráti späť.

**Maily samotné necestujú** — každý počítač si ich číta z Gmailu sám.
Preto je na oboch rovnaký obsah, a súbor na Drive ostáva malý.

**Na každom počítači zvlášť ostáva:** pauza, a čo smie TIQR vytvoriť samo
(Automatic). Dva počítače na Automatic by z toho istého mailu vytvorili ten
istý order dvakrát.

**Aktualizuj oba počítače.** Kým je jeden ešte na 2.109.x, nedávaj na ňom
ručne **Sync down** — stiahol by si hlavný súbor už bez mailov a jeho Mail
by do aktualizácie hlásil chybu (automatický sync to nerobí, ten len
spája).

---

## 6. Nastavenia v Maily — opravené

- **Pole pravidla bolo stlačené.** Text, ktorý pravidlo hľadá, bol omylom
  vnorený do 124 px širokého boxu výberu „contains" — preto sa tlačil pod
  neho. Teraz zaberá zvyšok riadku.
- Výber akcie ukazuje celý text („Send to Needs review" sa orezávalo).
- Pravidlá sa čítajú ako veta: *Subject contains „payout" → Send to Needs review*.
- Preč sú texty o „date window", ktoré už neplatia; „Keep mail for" sa
  volá **„Keep preview text for"**, lebo maže len náhľad, nie mail, a má
  tlačidlo **Save**.
- V sekcii Mailbox je vidieť, čo appka práve číta, koľko priečinkov je
  hotových, koľko mailov je na počítači a koľko miesta to berie.
- Ľavý panel: pri veľa labeloch sa už neprekrývajú so „Settings".

---

## 7. Čo je overené a čo nie

**Overené:**
- **1440 Rust testov, 0 červených** — z toho 54 nových: súbor a lock
  (presun, obnova starej zálohy, zamknuté mazanie), poradie priečinkov,
  fronta, prechody a zloženie hviezdičky, Gmail história, rozlíšenie
  rate-limit / bez povolenia / vypnuté API, výpadok internetu (nič nevypadne
  z fronty), priečinky a čísla z jednej definície, 100 000 mailov, dva
  počítače (novšia zmena vyhrá, odpojenie sa nevráti, Automatic necestuje).
  17 starých testov odišlo spolu s kódom, ktorý testovali (dátumové okno,
  starý zoznam chýb).
- Frontend: `npm run build` (TypeScript + Vite) prechádza.
- **Obrazovka vykreslená v Chromiu** s testovacími dátami, aj v menšom okne
  (1180×760) — z toho sa našlo a opravilo prekrývanie v ľavom paneli.

**Neoverené — treba povedať:**
- **Skutočná Gmail schránka.** Tu žiadna nie je; všetko je overené proti
  dokumentovanému správaniu Gmail API a testovacím odpovediam.
- Ako rýchlo sa reálne prečíta celá tvoja schránka — závisí od Google
  limitu (~30–40 mailov za sekundu), teda 100 000 mailov je rádovo hodina
  otvorenej appky, priečinky v tvojom poradí.
- Dva skutočné počítače naraz.

---

## Súbory

**Nové:** `src-tauri/src/mail_store.rs`, `src-tauri/src/commands/mail_sync.rs`,
`src-tauri/src/commands/shared_settings.rs`, `src/lib/mailSync.ts`

**Zmenené:** `commands/mail.rs`, `db.rs`, `lib.rs`, `models.rs`,
`commands/database.rs`, `commands/cloud_merge.rs`, `commands/notifications.rs`,
`src/pages/MailIntelligence.tsx`, `src/components/MailSettingsPane.tsx`,
`src/components/Layout.tsx`, `src/pages/Settings.tsx`, `src/lib/api.ts`,
`src/lib/types.ts`, `GMAIL-SETUP.md`

**Verzia 2.110.0.** Číslovaná migrácia nepribudla — mailový súbor má vlastnú
schému. **Ďalšia nová migrácia je stále 046.**

# TIQR Manager 2.113.0 — mail ako v poriadnom mailovom programe, Discord pri každom zázname, jeden sync, jednoduchší repozitár

Jedno veľké vydanie z jednej dlhej požiadavky. Všetko, čo v nej bolo, je
nižšie po častiach — čo sa zmenilo, prečo takto, čo bolo overené a čo nie.

---

## 1. Mail

### Ako sa mail zobrazí

*„kúsok pomeniť ako vyzerá ten obsah mailu, nech len nevypisuje mail, ale aj
ho pekne zobrazí"*

Mail sa ukáže tak, ako ho odosielateľ navrhol — farby, tabuľky, logá,
tlačidlá. Mail bez HTML sa ukáže ako text v odsekoch, citovaný text
(riadky s `>`) je odlíšený a odkazy aj adresy sú klikateľné.

**Dve steny, aby jedna chyba nestačila** (*„e-mailový obsah a prílohy treba
spracúvať ako nedôveryhodný vstup"*):

1. **Rust mail vyčistí skôr, než ho obrazovka vôbec uvidí**
   (`commands/mail_view.rs`, knižnica `ammonia`): žiadny skript, žiadny
   `onclick` a podobné, žiadny formulár, rámec ani objekt; odkaz len
   `http`, `https` alebo `mailto`.
2. **Obrazovka ho ukáže v okne, v ktorom nemôže bežať nič** (iframe so
   `sandbox` bez `allow-scripts`) a ktoré **nesmie nič načítať**
   (Content-Security-Policy `default-src 'none'`, obrázky len ako dáta). Do
   samotnej stránky appky sa mail nedostane nikdy.

Mail väčší ako 2 MB HTML sa ukáže ako text (taký mail nikto nenavrhol na
čítanie a len by spomalil obrazovku), text je najviac 200 000 znakov.

### Obrázky v TIQR

*„aj obrázky čo sú v maily tak prekopíruj do tiqr"*

Obrázky vložené priamo v maile (`cid:`) sa vezmú z mailu, obrázky z webu
stiahne **TIQR sám, nikdy nie obrazovka**:

- len verejné adresy (nie sieť doma ani v práci) a porty 80/443;
- len skutočné obrázky (PNG, JPEG, GIF, WebP) — podľa bajtov, nie podľa
  toho, za čo sa vydávajú;
- najviac 3 MB jeden, 40 v jednom maile, 15 MB spolu;
- **sledovacie pixely (1–2 px) sa vynechajú.**

Uložia sa do priečinka `<databáza>.mail-files` vedľa mailového súboru a pri
ďalšom otvorení sa berú odtiaľ. Odosielateľ tak môže vidieť nanajvýš prvé
stiahnutie obrázka, nie každé čítanie. Kto nechce ani to: Mail settings →
Reading → *Copy pictures from the web into TIQR*.

### Odkazy a kódy

*„linky ktoré sú v maily nech sú klikateľné a ukazujú sa tak ako v maily"*,
*„aby som to vedel aj jedným klikom skopírovať"*, *„OTP kódy nech sa
skopírujú na klik"*

- Klik na odkaz ho otvorí v prehliadači.
- **Keď text odkazu ukazuje inú stránku, než kam odkaz vedie** (napr. text
  „www.paypal.com", ale cieľ je iná doména — presne tak vyzerá phishing),
  appka sa najprv opýta a obe adresy ukáže. Rovnako pri každom odkaze z
  mailu v Spame alebo v Koši.
- **Myš nad odkazom** ukáže, kam vedie, s tlačidlami **Copy link** a
  **Open**.
- **OTP kód** sa skopíruje jedným klikom v zozname mailov, vo veľkej karte s
  kódom a aj priamo v texte mailu (kód je tam zvýraznený).

### Prílohy

*„pri každom maile kde je nejaký attachment tak ten tiež dať do tiqr, aby
sme si ho vedeli kuknúť normálne aj ho tam otvoriť bez problémov"*

Pri otvorenom maile je zoznam príloh. Príloha sa stiahne pri prvom použití
a ostane v TIQR.

- **View** — obrázky, PDF a text priamo v TIQR. Čo to je, rozhodnú bajty
  súboru, nie jeho meno; HTML alebo SVG príloha sa ukáže ako text, nikdy sa
  nevykreslí.
- **Open** — vo vlastnom programe, ale len druh, ktorý nemôže nič spustiť
  (PDF, obrázok, text, Office). Na Windows sa súbor označí ako stiahnutý z
  internetu, takže ho Office otvorí v chránenom zobrazení.
- **Save** — kamkoľvek, cez bežné okno na uloženie.

Program (`.exe`, `.bat`, `.js` a podobne) TIQR neotvorí, len uloží, a povie
prečo. Súbor, ktorý nie je tým, čím sa tvári (napr. „faktura.pdf", ktoré je
v skutočnosti archív), TIQR tiež neotvorí.

### Translate a Summary

*„built in translator"*, *„kľudne môžeme zakomponovať AI ako pomocníka, ale
to tiež len príležitostne na niektoré tasky v maily a tak, aby žral čo
najmenej usagu"*

Dve tlačidlá pri maile: **Translate** (do jazyka z Mail settings → Reading,
predvolene slovenčina) a **Summary** (pár viet). Aby to stálo čo najmenej:

- **len na klik** — bez kliknutia AI v maile nerobí nič;
- **najlacnejší model** Claude (Haiku), bez „premýšľania";
- dlhý mail sa pošle len po prvých 12 000 znakov (a odpoveď to povie);
- **každá odpoveď sa uloží k mailu** — druhý klik na to isté je zadarmo;
- v Mail settings → Reading je vidieť, koľko odpovedí a tokenov AI minula
  tento mesiac na tomto počítači.

Mail ide do AI ako materiál, nie ako pokyny — čo je napísané v maile, AI
nikdy nevykoná. Používa sa Anthropic kľúč zo Settings → Integrations → AI.
Translate sa neponúkne, keď je mail už v cieľovom jazyku.

### Presúvanie mailov kamkoľvek

*„aby som vedel dané maily presunúť tak kde chcem do iného miesta"*

- **Move to** pri otvorenom maile aj pre viac označených mailov.
- **Potiahnutie** mailu (alebo všetkých označených) na priečinok vľavo.
- Kam: Inbox, Finances, Promotions, ktorýkoľvek Gmail label a **vlastné
  priečinky**. Tie sa pridajú tlačidlom **+** pri *Your folders*, premenujú
  a zmažú ikonkami pri priečinku a **prenesú sa aj na druhý počítač**.
- **Back to where Gmail has it** vráti mail tam, kde ho má Gmail.
- Pravidlo vie nový mail rovno presunúť: Mail settings → Rules → *Move to
  a folder*.

Presun je vecou TIQR, rovnako ako prečítané/neprečítané: **Gmail sa nemení
nikdy** (TIQR má do Gmailu len čítací prístup). Zmazaním priečinka sa žiadny
mail nezmaže — vráti sa tam, kde ho má Gmail.

### Mail settings v sekciách

*„settings len v maily a to tiež rozdelil ako je to v primárnych settings"*

Vľavo v nastaveniach mailu sú sekcie ako v hlavných Settings:

| Sekcia | Čo v nej je |
|---|---|
| Mailbox | pripojenie, test, pauza, čo práve číta |
| Reading | ako často, ako dlho držať náhľady, obrázky z webu, jazyk prekladu, koľko minula AI |
| What is read | sledovaní odosielatelia, celý mail alebo len to dôležité, vlastný Gmail filter |
| Rules | pravidlá, nové aj s presunom do priečinka |
| Automatic | čo smie TIQR vytvoriť samo, ktorý počítač ho robí, a podľa čoho |

Mailové panely siahajú až po spodok okna.

---

## 2. Automatické orders, sales a finance

*„doladiť tie automatické orders, sales, finances"* a pravidlo k tomu:
*„AI nesmie rozhodovať naslepo — každý návrh musí mať zdrojové údaje, mieru
istoty a kontrolu duplicít. Nejednoznačné prípady patria na manuálne
potvrdenie."*

**Čítanie čísel nanovo** (`commands/mail.rs`):

- medzisúčet nikdy nie je celková suma;
- mena len ako celé slovo alebo symbol, číslo pri svojom symbole;
- jedna mena naraz — „94 EUR (2 350 CZK)" je 94 EUR;
- číslo objednávky nie je počet lístkov;
- pri nedávnych mailoch, ktoré môžu byť záznamom, sa číta celý text, nielen
  krátky náhľad z Gmailu.

**Každý návrh ukazuje:**

- každé číslo so slovami z mailu, z ktorých bolo prečítané, a mieru istoty
  (Certain / Check it / Unsure);
- **záznamy, ktoré už vyzerajú rovnako**: rovnaké číslo objednávky, rovnaká
  konverzácia, alebo rovnaký event, suma a mena v rozpätí ±3 dni;
- čo by urobil Automatic a prečo.

**Automatic vytvorí záznam len vtedy, keď:**

- mail prišiel až **po zapnutí** Automatic (staré maily nikdy; počítač, ktorý
  už mal Automatic zapnutý, počíta od tohto vydania);
- každé číslo je **isté**;
- event objednávky alebo lístky predaja sedia **presne**;
- **nič podobné** v TIQR ešte nie je;
- žiadne pravidlo neposlalo mail na kontrolu ani do koša.

Inak návrh počká na potvrdenie a napíše dôvod. **Review and edit** otvorí
krátky formulár s tým, čo bolo prečítané, na opravu pred vytvorením (event,
suma, mena, dátum, platforma, referencia, pri predaji konkrétne lístky).
**Create anyway** pri možnom duplikáte sa najprv opýta. **Not relevant** mail
odloží.

**Záznamy z mailov robí len jeden počítač** (Mail settings → Automatic →
*This computer makes the records* / *Make them here*). Dva počítače čítajúce
tú istú schránku by inak vytvorili každú objednávku dvakrát.

---

## 3. Discord

*„opraviť webhooky"*, *„send to discord pri každom evente, predaji,
finančnom zázname atď."*, *„krátke zhrnutia dôležitých mailov"*, *„nech to
vyzerá pekne, značka TIQR"*

**Prečo webhooky nechodili:**

- nový event, objednávka, predaj ani finančný záznam do Discordu nikdy nič
  neposlali — posielali len záznamy vytvorené z mailu a tlačidlo Test;
- čas v správe mal 9 desatinných miest, a ten Discord odmieta;
- posielanie bežalo, kým držalo databázu;
- kanál Sales bol mŕtvy, lebo predaj z mailu vždy skončil chybou.

**Teraz** (`commands/discord.rs`):

- **každý nový záznam sa ohlási sám** v svojom kanáli; každý druh (events,
  orders, sales, finance, mail) sa dá v Settings vypnúť;
- **dôležitý mail** (predaj, nákup, výplata, refund, zrušenie, zmena
  termínu) príde ako krátke zhrnutie — najviac 5 za jedno čítanie, OTP kód
  nikdy;
- tlačidlo **Send to Discord** pri evente, objednávke, predaji, finančnom
  zázname aj pri maile;
- správy majú logo a značku TIQR, sú escapované (nič v nich nespraví
  `@everyone`) a posiela ich jedno vlákno s opakovaním; posledných 20
  odoslaní je vidieť.

**Settings → Integrations → Discord:** celé nastavenie len prvýkrát, potom
jeden riadok na kanál, ktorý sa dá rozbaliť, s náhľadom najnovšej správy.

---

## 4. Jeden sync

*„jedna synchronizácia — živý sync plus záloha v Drive, aby sa nič nestratilo"*

Settings → Data → **Sync between your computers** je jedna karta s dvoma
časťami:

- **Live between your computers** — zmena na jednom počítači je na druhom
  o pár sekúnd (z 2.112.0, bez zmeny).
- **Backup in Google Drive** — plná kópia, ktorá sa ukladá sama pri spustení
  a zatvorení appky. Kým je živý sync zapnutý, hovorí ako záloha (*Kept*,
  *Saving*, *Saves on close*) namiesto „Conflict" či „Cloud changes" —
  to, že poslednú kópiu uložil druhý počítač, je v poriadku. Ručné
  tlačidlá (Sync, Combine both, Take theirs, Keep mine) sú pod **Advanced**.
  Bez živého syncu je karta Drive ako doteraz.

**Nič sa nestratí:**

- zmena urobená offline čaká, kým sa dá poslať;
- všetko, čo by prepísalo dáta, si najprv urobí kópiu (body obnovy);
- Google drží staršie verzie zálohy (Backup & restore);
- záloha, ktorú počítač pri zatvorení nestihol uložiť, sa po novom uloží pri
  ďalšom spustení, nie až pri ďalšom zatvorení.

Automatický sync naďalej beží len pri spustení a zatvorení (ako od 2.59.0),
žiadny časovač na pozadí. Text v karte, ktorý ešte hovoril o „každých 5
minútach", je opravený.

---

## 5. Bezpečnosť a súkromie

*„OAuth tokeny a citlivé údaje nesmú skončiť v logoch ani v nechcenom cloud
syncu."*

- **Prihlásenia mimo databázy** (`secrets.rs`): Gmail token a jeho
  oprávnenia, Google prihlásenie a Anthropic kľúč sú v súbore
  `<databáza>.secrets.json` vedľa databázy. Nejdú:
  - do zálohy v Google Drive,
  - do bodov obnovy,
  - do Sync down,
  - do živého syncu.

  Nedajú sa prečítať ani cez všeobecné nastavenia. Pri prvom spustení sa
  presunú samy a v databáze sa prepíšu.
- **Obnovená kópia** nikdy neprepíše prihlásenie tohto počítača.
- **Všeobecné príkazy na nastavenia** slúžia už len pre voľby obrazovky
  (téma, otvorené záložky), nie na čítanie nastavení s tajomstvami.
- **Mail a prílohy sú nedôveryhodný vstup**:
  - dve steny pri zobrazení (kapitola 1);
  - obrázky z webu sťahuje TIQR len z verejných adries, len ako obrázky
    podľa bajtov;
  - prílohy sa otvárajú len, keď nemôžu nič spustiť;
  - text mailu pre AI je materiál, nie pokyny;
  - maily zo Spamu a Koša TIQR nikdy nespracuje.

---

## 6. Ostatné z požiadavky

- **Bočný panel v troch skupinách:** Dashboard, Events, Inventory, Sales —
  Finance — Mail, Notes. **Pulls ostáva úplne dole**, lebo tam bol presunutý
  na výslovnú požiadavku v 2.98.0 (pull je láskavosť na cudzej karte, nie
  vlastný tovar) a nová požiadavka ho v zozname skupín neuvádzala.
- **Pulls rozdelené** na Active a Completed, v oboch zoznamoch.
- **Hlavičky tabuliek** ostávajú pri rolovaní hore a sú nepriehľadné — riadky
  už pod nimi nepresvitajú.
- **Finance:** karty a tabuľky už nie sú nalepené ani sa neprekrývajú.
- **Mailové panely** siahajú po spodok okna.

---

## 7. Repozitár a zip na vydanie

*„jednoduchšie súbory a zip len s pár vecami a jasným návodom ako začať"*

- **234 starých reportov, denníkov a dátovaných prehľadov je v `history/`**,
  s nezmenenými menami, takže každý komentár v kóde, ktorý ich spomína, ich
  stále nájde. Hore v repozitári ostalo 23 položiek namiesto 258.
- **Zip má len tri veci:**

  ```
  1-CLICK-UPDATE.bat   toto sa spustí
  AKO-ZACAT.txt        návod
  tiqr-manager\        celá appka (zdrojový kód) + release.ps1
  ```

- `1-CLICK-UPDATE.bat` a starý `PRECITAJ-PRED-VYDANIM.txt` už nie sú v
  repozitári — patria do zipu, mimo priečinka, ktorý sa zrkadlí do GitHubu.
- **Ochrana pred hromadným mazaním** v `release.ps1` počíta zmazané súbory
  až po `git add -A`, keď git presun vidí ako premenovanie. Inak by presun
  234 súborov vyzeral ako 234 zmazaní a vydanie by sa zastavilo. Skutočne
  zmazané sú v tomto vydaní dva súbory (`.bat` a starý návod); viac ako päť
  by vydanie stále zastavilo. Overené simuláciou: čerstvý klon, zrkadlenie
  ako robocopy, staré počítanie = 236, nové = 2.

---

## 8. Čo treba urobiť

**Nič povinné.** Firebase pravidlá ani migrácie sa nemenili. Prihlásenia
sa presunú samy.

**Odporúčané po aktualizácii:**

1. Settings → Integrations → Discord — skontrolovať, ktoré druhy sa majú
   ohlasovať samy (predvolene všetky, ak je Discord nastavený).
2. Mail → Settings → Automatic — skontrolovať, ktorý počítač robí záznamy z
   mailov.
3. Na druhom počítači treba Gmail pripojiť znova len vtedy, ak sa tam obnoví
   záloha z tohto počítača.

---

## 9. Testy

- `cargo test --locked`: 1546 prešlo, 0 zlyhalo, 4 vynechané.
- `npm test`: 33 testov, medzi nimi nové testy čistenia textu, odkazov, kontroly
  „odkaz ukazuje inú stránku" a stránky, v ktorej sa mail zobrazuje
  (`src/lib/mailText.test.ts`).
- Živý sync proti Firestore emulátoru so skutočnými `firestore.rules`: 9
  krokov prešlo.
- `npm run build` prešiel.
- **Obrazovky** boli vykreslené v prehliadači s napodobeným backendom, v
  tmavom aj svetlom režime:
  - mail v pieskovisku: v rámci nie je ani jeden skript, OTP kód je
    zvýraznený;
  - čip pri odkaze a otázka pri falošnom odkaze;
  - bežný odkaz sa otvorí rovno;
  - preklad, Move to, potiahnutie na priečinok;
  - všetkých päť sekcií Mail settings;
  - karta Sync between your computers so živým syncom aj bez neho.

---

## 10. Limity — čo overené nie je

- **Nič z toho nebežalo proti skutočnej Gmail schránke, skutočnému Discordu
  ani na skutočnom Windows.** Testy používajú napodobený Gmail a Discord;
  obrazovky boli v Chromiu, nie vo WebView2.
- **Náhľad PDF** používa prehliadač PDF vo WebView2 / WebKite. Keby sa PDF
  v TIQR neukázalo, ostáva **Open**.
- **Kancelárska spinka** v zozname sa objaví až po prvom otvorení mailu —
  dovtedy TIQR o prílohách nevie (stiahnuť každý mail celý by bolo veľa
  dát).
- **Obrázky z webu** sa sťahujú pri prvom otvorení; mail otvorený offline
  ich ukáže pri ďalšom otvorení po **Download again**.
- **Preklad** je od AI — mená, sumy a čísla treba overiť v origináli pod ním.
- Mail väčší ako 2 MB HTML sa ukáže ako text.

---

## Súbory

- **Nové:**
  - `src-tauri/src/commands/mail_view.rs`
  - `src-tauri/src/commands/discord.rs`
  - `src-tauri/src/secrets.rs`
  - `src/components/MailReader.tsx`
  - `src/components/MailProposal.tsx`
  - `src/components/Discord.tsx`
  - `src/lib/mailText.ts` a jeho test
  - `src/lib/clipboard.ts`
  - `src/lib/pullStatus.ts` a jeho test
  - `src/lib/useFillHeight.ts`
  - `docs/tiqr-logo.png`
  - `history/README.md`
- **Zmenené:**
  - `mail.rs`, `mail_sync.rs`, `mail_store.rs` (mailová schéma 3)
  - `models.rs`, `lib.rs`, `commands/mod.rs`
  - `notifications.rs`, `settings.rs`, `shared_settings.rs`
  - `events.rs`, `orders.rs`, `sales.rs`, `finance_entries.rs`
  - `backup.rs`, `database.rs`, `google_auth.rs`, `live_sync.rs`
  - `MailIntelligence.tsx`, `MailSettingsPane.tsx`, `Settings.tsx`,
    `LiveSyncCard.tsx`, `Layout.tsx`, `Pulls.tsx`, `EventDetail.tsx`,
    `OrderDetail.tsx`, `SaleDetail.tsx`, stránky Finance, `ui.tsx`,
    `icons.tsx`, `index.css`, `api.ts`, `types.ts`
  - `Cargo.toml` (nové `ammonia` a `encoding_rs`)
  - `RELEASE.md`, `GMAIL-SETUP.md`, `CHANGELOG.md`, `PROJECT_STATE/*`
  - `.gitignore`
- **Presunuté:** 234 súborov do `history/`.
- **Odstránené z repozitára:** `1-CLICK-UPDATE.bat`,
  `PRECITAJ-PRED-VYDANIM.txt` (sú v zipe).

**Migrácie:** 46 číslovaných, ďalšia je **047**. Mailový súbor má schému 3.

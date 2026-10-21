# Changelog

Short, append-only entries, newest at the top - one entry per completed
task under the TIQR development protocol (see step 8). This is not a
replacement for the detailed `REDESIGN-X.Y.Z-REPORT.md` / `*-REPORT.md`
files in `history/` (one per release, written for marko, in Slovak) -
those still get written for real releases. This log exists so a future
session can see recent activity at a glance without opening any of them.

(2026-09-01: this file merges two copies that grew independently in
different sessions after 2.0.80 - one bootstrapped at 2.0.80 with entries
back to 2.0.75, the other bootstrapped at 2.1.9 with entries from 2.1.9
onward. Versions 2.0.81-2.1.8 - Finance module, Price Checker auto-check
iterations - fall in the gap between the two bootstraps and are not
backfilled here, consistent with this file's own existing policy below;
read the matching `REDESIGN-X.Y.Z-REPORT.md`/`*-REPORT.md` for any of
those directly.)

## 2.114.1 - Pri prijatých pulloch suma, za ktorú sa lístky predali

**Pulls → Received → nový stĺpec *Sold for*** (hneď za Fee). Pri pulli,
ktorého lístky sa stali objednávkou a potom sa predali, je vidieť, za
koľko:

- súčet predajov lístkov z prepojenej objednávky; refundovaný predaj sa
  nepočíta, rovnako ako Revenue na detaile objednávky;
- keď je predaná len časť, za sumou je napríklad „· 1/4" (1 lístok zo 4);
- predaje v dvoch menách sa nesčítavajú ani neprepočítavajú, stoja vedľa
  seba („£360.00 + €120.00") a radšej sa zalomia, než by sa odrezali;
- „-" znamená, že sa zatiaľ nič nepredalo alebo pull nemá objednávku;
  bublina pri myši povie, ktoré z toho;
- v okne úpravy pullu je pod prepojenou objednávkou riadok „Sold for
  €200.00 · 1 of 4 tickets sold".

Suma sa nikam neukladá, počíta sa pri každom zobrazení z predajov, takže
sedí aj po neskoršom predaji, refunde alebo oprave ceny. Fee pre pullera
ostáva len informačný. Given pully sa nemenia.

Stĺpec dostal miesto zo stĺpca Event (9 % v širokej, 10 % v úzkej
tabuľke); súčty šírok ostali ako predtým.

Zip obsahuje aj všetko z 2.114.0 - ak 2.114.0 ešte nebolo vydané, stačí
spustiť tento.

`cargo test` 1579 prešlo (4 ignorované, ako doteraz), `npm test` 39, build v poriadku; obrazovka
overená v prehliadači s napodobeným backendom pri najmenšom okne (1080 px
s bočným panelom) aj pri širokom, bez vodorovného posúvania.

## 2.114.0 - Jednoduché webhooky, správne meno kupujúceho, hromadné mazanie, AI upratanie Promotions, viac nastavení mailu

**Meno vo webhooku.** Pri viagogo maili „Ticket Holder Details / Full Name:
Luis Somoza" Discord ukázal „Ticket Holder: Details": nadpis sa čítal ako
meno a skutočné meno sa už nezmestilo. Teraz platí:

- Riadky „Menovka: hodnota" (s dvojbodkou) sa čítajú prvé a majú prednosť
  pred riadkami tabuľky bez dvojbodky.
- Slová nadpisov (Details, Information, Summary...) nie sú nikdy hodnota.
- Výsledok pri tomto maili je „Full Name: Luis Somoza" a „Email Address:
  d1jedi@aol.com".
- Osobné údaje sa ukazujú len v maili, ktorý žiada poslať lístky niekomu,
  alebo pri menovke, ktorá kupujúceho priamo menuje (Ticket holder, Buyer,
  Recipient). Potvrdenie objednávky, ktoré ťa oslovuje menom, ich nemá.

**Jednoduché webhooky.** Každá správa v Discorde je nadpis a pár krátkych
riadkov, bez mriežky polí:

- Nákup: „2 tickets bought on Ticketmaster for €225.00", 📅 dátum a miesto,
  💺 sedadlá.
- Predaj: suma, zisk a ROI, dátum, sedadlá.
- Event: dátum a miesto, lístky (koľko, na sklade, predané), nakúpené a
  predané.
- Financie: suma a miesto, dátum, účet, Business/Personal, poznámka.
- Mail: odosielateľ · lístky · suma. Ďalej 📤 komu (spoiler), ⏰ dokedy,
  🎫 event, 💶 iná cena a 💺 sedadlá. Úvodné slová mailu sú tam len vtedy,
  keď nič lepšie nie je známe.
- Číslo objednávky je pri logu dole.

**Hromadné mazanie.** Nad zoznamom je *Select all*, potom *Select all 1,840*
pre celý priečinok (nie len načítanú stranu), a *Delete*. Delete presúva do
Koša a pri viac ako 50 mailoch sa najprv opýta. Z Koša *Delete N forever*.
Všetko sa robí v jednej transakcii, takže aj tisíce mailov idú hneď.

**AI upratanie Promotions.** V priečinku Promotions je *Clean up with AI*:

- Bez AI a zadarmo sa vždy ponechá mail s hviezdičkou, od predajcu lístkov,
  s kódom, rozpoznaný ako objednávka alebo predaj, alebo mail, ktorý už bol
  presunutý do iného priečinka.
- Zvyšok ide na najlacnejší model: doména odosielateľa (nie adresa),
  predmet a pár slov náhľadu, 60 mailov na dopyt, bez premýšľania, s
  najnižším úsilím. Odpoveď je jedno písmeno na mail.
- Každý mail sa kontroluje len raz. 1000 promotions stojí približne cent.
- Okno ukáže „isto junk" (zaškrtnuté) a „neviem" (nezaškrtnuté), so
  zhrnutím a cenou.
- Nič sa nepresunie bez tvojho kliknutia na *Move N to Trash*; z Koša sa
  dá všetko vrátiť.

**Viac nastavení mailu** (Mail → Settings):

- **What is read**: či TIQR vôbec číta Promotions, Spam a Kôš. Vypnutý
  priečinok sa nečíta a nové maily z neho sa nesťahujú; s hviezdičkou alebo
  v tvojom štítku áno.
- **Rules**: pole *Sender, subject or preview*, akcia *Star it* a počet
  mailov, ktoré by pravidlo chytilo, priamo pri písaní.
- **Muted senders**: zoznam, pridanie adresy alebo domény na 1/7/30 dní
  alebo navždy, *Unmute*.
- **Discord**: či sa dôležité maily posielajú samy a ktoré druhy (predaje,
  nákupy, výplaty a refundy, zrušenia a zmeny, aktualizácie objednávok).
  Ďalej čo správa nesie: kľúčové údaje, osobné údaje (vypnuté = nikdy sa
  nepošlú) a úvodné slová.

`cargo test` 1577 prešlo (4 ignorované, ako doteraz), `npm test` 38, build v poriadku; obrazovky (výber
všetkého, potvrdenie, AI upratanie, nové nastavenia) skontrolované v
prehliadači. Žiadna nová migrácia, ďalšia je 047.

## 2.113.1 - Opravy po 2.113.0 a mail: kľúčové údaje, stiahnutie z odkazu, stíšenie, hviezdičky, zmeny z Gmailu

**Finance → Transactions už nie je rozbité.** Stĺpce Category a Amount sa
zúžili na jedno písmeno na riadok („B iz ni s...", „- € 2 5 7"). Príčina: od
2.102.0 smie text v bunke tabuľky zalomiť kdekoľvek, aby nikdy nepretiekol do
susedného stĺpca - to je správne v tabuľke s pevnými šírkami stĺpcov, ale
tabuľky vo Finance si šírky rátali z obsahu, a tam to dovolilo zúžiť stĺpec
na jedno písmeno. Transactions a opakované výdavky v Accounts majú teraz
pevné šírky ako každý iný zoznam v appke (miesto/kto dostane, čo ostane, a
dlhý text skráti s celým textom po prejdení myšou), a v tabuľkách, ktoré si
šírky rátajú z obsahu, sa slovo zalomí len vtedy, keď by naozaj pretieklo.

**Pulls je hneď pod Sales** v bočnom paneli.

**Logo TIQR v Discorde.** Správy ukazovali namiesto loga rozbitý obrázok:
logo bolo na adrese GitHub Pages, ktoré pre tento repozitár nie sú zapnuté
(každá adresa `github.io` vracia 404). Teraz sa berie priamo zo súboru v
repozitári (`docs/tiqr-logo.png` na `main`), overené, že sa načíta. Staré
správy v Discorde ostanú, ako boli; nové už logo majú.

**Biela čiara v Inventory.** Medzi dvoma riadkami boli dve čiary naraz (okraj
tabuľky aj čiara riadka), a keďže riadky stáli na polovičných pixeloch, na
niektorých miestach sa spojili a jedna čiara vyšla oveľa svetlejšia. Teraz
je medzi riadkami len jedna čiara a riadky majú celé pixely - vo všetkých
zoznamoch.

**Kľúčové údaje z mailu - z celého obsahu, nie len z prvého riadku.** Pri
maili typu „please send your tickets" TIQR vyberie to podstatné: kam a komu
poslať lístky (meno, e-mail), dokedy, cenu a výplatu, event, dátum, miesto,
počet lístkov, sedadlá, číslo objednávky - každý riadok s menovkou, akú mu
dal sám mail („Total Proceeds", „Buyer's name"). Bez AI a bez hádania: hodnota
sa berie len vedľa menovky, ktorú mail napísal, alebo z vety, ktorá priamo
žiada poslať lístky na adresu. Čítajú sa menovky po anglicky, slovensky,
česky, nemecky a španielsky; pri citovanom staršom maili sa číta len nová
časť. Adresa odosielateľa a tvoja vlastná schránka nikdy nie sú „kam poslať".

- **V TIQR** je to karta *Key details* hore v otvorenom maili.
- **Osobné údaje** (meno, e-mail, telefón, adresa, komu poslať) sú skryté
  ako bodky. Ukážu sa až po kliknutí, hromadne cez *Show personal details*.
  Skopírovať sa dajú aj bez odkrytia.
- **V Discorde** je pole *📌 Key details*. Osobné údaje sú tam ako spoiler
  (ukáže ho až klik), aj v náhľade textu. V nadpise sú nahradené bodkami.
  Čo už hovoria polia Amount, Tickets a Reference, sa neopakuje.
- Pri príchode sa celé telo načíta aj pri aktualizácii objednávky, zrušení a
  zmene eventu (predtým len pri nákupe, predaji, výplate a refunde). Discord
  tak má kľúčové údaje z reálneho obsahu. Nová správa bez kľúčových údajov
  vyzerá ako predtým.

**„Download receipt" naozaj stiahne súbor.** Klik na odkaz, ktorý je súbor
(„Download receipt", „Download invoice", „Stiahnuť faktúru", „Rechnung
herunterladen", adresa končiaca na `.pdf` a podobne), stiahne súbor priamo do
TIQR. Súbor sa uloží k mailu medzi prílohy (s poznámkou *from a link*) a
obrázok či PDF sa hneď otvorí v náhľade. Odtiaľ ide *Open* aj *Save* ako pri
prílohe. Sťahuje Rust, nie obrazovka, a len z verejného internetu: každé
presmerovanie sa kontroluje, sú povolené len porty 80/443 a najviac 25 MB. Či
je to program, sa zisťuje podľa obsahu súboru, nie podľa mena, a program
TIQR neotvorí. Ak adresa vráti webovú stránku (prihlásenie, stránka faktúry)
alebo sa súbor stiahnuť nedá, odkaz sa otvorí v prehliadači - klik teda vždy
niekam vedie. Pri prejdení myšou má taký odkaz tlačidlá *Download* aj
*Open*. Ostatné odkazy (#1160703 v Zendesku, Concerts/Festivals/help centre
vo viagogo) sa otvárajú v prehliadači ako doteraz. Mail v Spame alebo Koši
nikdy nič nesťahuje.

**Stíšenie odosielateľa (Mute).** Odosielateľ, ktorý veľa spamuje, sa dá
stíšiť na 1 deň, 7 dní, 30 dní alebo natrvalo - jedna adresa alebo všetci z
jeho domény. Jeho maily ďalej chodia a ostávajú v TIQR, len sú skryté:

- nie sú v Inboxe, Important, Finances, OTP codes, Promotions ani v štítkoch;
- nerátajú sa medzi neprečítané;
- sú v novom priečinku **Muted** so zoznamom stíšených a tlačidlom *Unmute*.

Ďalšie pravidlá:

- Mail s objednávkou, predajom, výplatou, refundom, zrušením, zmenou eventu
  alebo aktualizáciou objednávky sa neskryje nikdy.
- Hviezdička, Needs review a tvoje priečinky ukazujú mail aj od stíšeného
  odosielateľa.
- Keď stíšenie skončí, nové maily sa ukazujú znova a staršie ostávajú v Muted;
  *Unmute* vráti všetky.
- Celú zdieľanú doménu (gmail.com, azet.sk...) stíšiť nejde, len konkrétnu
  adresu.
- Zoznam stíšených prejde aj na druhý počítač, ako pravidlá. Gmail sa nemení.

**Hviezdička v TIQR.** Tlačidlo *Star* v otvorenom maili, hviezdička priamo v
riadku zoznamu a *Star/Unstar* pre označené maily. Je to hviezdička TIQR
(prístup do Gmailu je len na čítanie), takže v Gmaile sa neobjaví. Ďalšia
kontrola Gmailu ju neprepíše, kým hviezdičku nezmeníš v Gmaile.

**Zmeny z Gmailu sa prenesú do TIQR.** Keď mail prečítaš v Gmaile (v mobile,
v prehliadači), je prečítaný aj v TIQR. Keď ho tam označíš ako neprečítaný,
je neprečítaný aj tu. Hviezdičky z Gmailu sa prenášali už predtým. TIQR
preberá len zmenu: mail, ktorý si prečítal v TIQR, ostane prečítaný, kým sa v
Gmaile niečo nezmení. Kontroluje sa to každých 20 sekúnd.

**OTP: kód z Ticketmastera.** Mail „John, Here's Your Authentication Code" s
kódom „1 3 8 8 8 0" je teraz v OTP codes. Kód sa hľadá pri každom výskyte
slova ako „authentication code", nie len pri prvom (to bolo v predmete), a
kód napísaný s medzerami („1 3 8 8 8 0", „138 880") sa číta ako jeden.
Telefónne číslo s medzerami ani rok kódom nie sú. Kódy mailov z posledných 60
dní sa pri prvom spustení prečítajú znova, takže sa tam presunie aj mail,
ktorý už máš. Zo Spamu a Koša kód nikdy.

**Drobnosti:** text z HTML mailov už správne číta písmená zapísané ako
`&Ntilde;`, `&uuml;`, `&Scaron;` (ESPAÑA, München, Škoda). Escape po kliknutí
na odkaz v maili zatvorí náhľad (klávesnica ostávala v rámčeku správy). Nad
mailom už nevisí bublina „Message".

Mailová databáza má krok 4 (stĺpec pre hviezdičku v TIQR); žiadna číslovaná
migrácia, ďalšia je 047.

`cargo test` 1568 prešlo (4 ignorované, ako doteraz), `npm test` 37, build v poriadku; obrazovky Finance a
Inventory skontrolované v prehliadači pri šírke 1000 aj 1440 bodov, čítačka
mailov (kľúčové údaje, skryté a odkryté, stiahnutie z odkazu, náhľad) v tmavom
aj svetlom režime, hviezdička, menu Mute a priečinok Muted.

## 2.113.0 - Mail ako v poriadnom mailovom programe, Discord pri každom zázname, jeden sync, jednoduchší repozitár

**Mail sa zobrazí tak, ako bol navrhnutý** - obrázky, tabuľky, tlačidlá -
a pritom bezpečne. Rust mail najprv vyčistí (žiadny skript, formulár ani
rámec, odkazy len http, https a mailto) a appka ho ukáže v okne, v ktorom
nemôže bežať nič a z internetu sa nenačíta nič. Obrázky z mailu sa
skopírujú do TIQR, aj tie z webu (len verejné adresy, len skutočné obrázky,
s limitom veľkosti). Sledovacie pixely sa vynechajú a obrázky z webu stiahne
TIQR len raz, pri prvom otvorení - odosielateľ tak môže vidieť nanajvýš toto
prvé stiahnutie, nie každé ďalšie čítanie. Kto nechce ani to, vypne v Mail
settings → Reading „Copy pictures from the web into TIQR".

**Odkazy sú klikateľné** a otvoria sa v prehliadači. Keď text odkazu ukazuje
inú stránku, než kam odkaz naozaj vedie (typický phishing), alebo je mail v
Spame či v Koši, appka sa najprv opýta. Po prejdení myšou nad odkaz sa
ukáže, kam vedie, s tlačidlom **Copy link**. **OTP kódy sa skopírujú jedným
klikom** - v zozname, v karte s kódom aj priamo v texte mailu.

**Prílohy:** pri každom maile zoznam s **View** (obrázky, PDF, text priamo v
TIQR), **Open** (vo vlastnom programe, len súbory, ktoré nemôžu nič
spustiť; na Windows označené ako stiahnuté z internetu) a **Save** kamkoľvek.
Program (.exe a podobne) TIQR neotvorí, iba uloží.

**Translate a Summary** na klik. AI pomocník beží na najlacnejšom modeli,
dlhý mail sa skráti a každá odpoveď sa uloží k mailu, takže ten istý klik sa
neplatí dvakrát. Bez kliknutia AI nerobí nič. Jazyk prekladu je v Mail
settings → Reading, spolu s tým, koľko AI tento mesiac minula.

**Presúvanie mailov kamkoľvek:** tlačidlo **Move to** pri maile aj pre viac
označených mailov, alebo potiahnutie mailov na priečinok vľavo. Cieľom môže
byť Inbox, Finances, Promotions, Gmail label alebo **vlastný priečinok**
(pridať, premenovať a zmazať priamo v zozname vľavo; prenesie sa aj na
druhý počítač). Pravidlo vie nový mail rovno presunúť do priečinka. V
Gmaile sa nikdy nič nezmení.

**Mail settings v sekciách** ako hlavné Settings: Mailbox, Reading, What is
read, Rules a Automatic.

**Automatické orders, sales a finance nikdy naslepo.** Každý návrh ukazuje
každé číslo so slovami, z ktorých bolo prečítané, a mieru istoty, plus
záznamy, ktoré už vyzerajú rovnako (rovnaké číslo objednávky, rovnaká
konverzácia, alebo rovnaký event, suma a dátum). Automatic vytvorí záznam
len vtedy, keď je všetko isté, mail prišiel až po zapnutí Automatic a nič
podobné v TIQR ešte nie je. Inak návrh počká na potvrdenie a povie prečo.
Záznamy z mailov robí len jeden počítač. Nejasný prípad sa dá opraviť vo
formulári ešte pred vytvorením a predaj z mailu vie vybrať konkrétne
lístky.

**Discord pri každom zázname.** Nový event, objednávka, predaj aj finančný
záznam sa pošle do svojho kanála sám (každý druh sa dá vypnúť), dôležitý
mail príde ako krátke zhrnutie a na každom zázname je tlačidlo **Send to
Discord**. Správy majú značku TIQR. V Settings sa Discord nastavuje celý len
prvýkrát, potom je to krátky zoznam kanálov s náhľadom.

**Jeden sync.** V Settings → Data je jedna karta: živý sync medzi počítačmi
a záloha v Google Drive, ktorá sa ukladá sama pri spustení a zatvorení
appky. Ručné tlačidlá sú pod Advanced.

**Bezpečnosť.** Prihlásenia (Gmail, Google) a Anthropic kľúč už nie sú v
databáze, ale v súbore `<databáza>.secrets.json` vedľa nej - nejdú do zálohy
v Drive, do bodov obnovy, do Sync down ani do živého syncu.

**Ďalej:** bočný panel v troch skupinách (Pulls ostáva úplne dole, ako od
2.98.0), Pulls rozdelené na Active a Completed, nepriehľadné hlavičky
tabuliek, Finance bez kariet nalepených na seba a mailové panely až po
spodok okna.

**Repozitár:** 234 starých reportov a denníkov je v priečinku `history/`.
Zip na vydanie má len tri veci: `1-CLICK-UPDATE.bat`, `AKO-ZACAT.txt` a
priečinok `tiqr-manager`.

Mailový súbor má schému 3; číslovaná migrácia nepribudla (ďalšia je 047).
Nové knižnice `ammonia` (čistenie HTML) a `encoding_rs` (kódovania mailov).
Testy: `cargo test` 1546 prešlo (0 zlyhalo, 4 vynechané), `npm test` 33, živý sync proti Firestore
emulátoru 9 krokov. Neoverené so skutočnou Gmail schránkou, skutočným
Discordom ani na skutočnom Windows.

## 2.112.0 - Živý sync: zmena na jednom počítači je o pár sekúnd na druhom

**Všetko ako jedna appka.** Objednávka, lístok, predaj, úprava, zmazanie,
poznámka aj mailové pravidlo sa na druhom počítači objaví do pár sekúnd,
kým sú oba zapnuté. Počítač, ktorý bol vypnutý, si všetko, čo zmeškal,
dotiahne hneď po otvorení. Doteraz išiel celý súbor cez Google Drive len pri
otvorení a zatvorení appky a **úprava** záznamu, ktorý mali oba počítače,
neprešla nikdy.

**Novšia úprava vyhrá**, na oboch počítačoch rovnako, a zmazanie platí vždy.
Rozhodujú o tom aj pravidlá vo Firebase, takže dva počítače, ktoré pošlú
zmenu v tej istej sekunde, si ju navzájom neprepíšu. Rovnaká platforma
napísaná na oboch počítačoch ostane jedna. Dve objednávky s tým istým
kódom dostanú dva kódy, na oboch počítačoch tie isté. Lístok, ktorý príde
skôr ako jeho objednávka, počká, kým objednávka dorazí.

**Obrazovky sa obnovia samy** (Dashboard, Events, Orders, Inventory, Sales,
Pulls, Finance, detaily, zoznam poznámok) a zaškrtnuté riadky ostanú. V
bočnom paneli je riadok so stavom (*Live*, *Sending 12*, *Offline · 4
queued*), v Settings → Data je karta **Live sync**.

**Zapína sa na každom počítači zvlášť** (jednorazová ponuka hore). Pred
prvým použitím treba raz vložiť nové `firestore.rules` do Firebase konzoly
(Firestore Database → Rules → Publish). **Prihlásenie do Gmailu cez Firebase
necestuje**: schránka sa pripája na každom počítači sama, pravidlá
a webhooky prejdú. Keď sú živé oba počítače, Google Drive slúži už len ako
záloha a pri spustení sa nepýta na zlučovanie. Po obnovení zálohy sa živý
sync na tom počítači pozastaví, aby obnovu hneď neprepísal, a appka to
povie.

Migrácia **046**, ďalšia je 047. Nových 23 Rust testov simuluje dva
počítače a Firestore s rovnakým pravidlom. Kontrola proti Firestore
emulátoru so skutočnými `firestore.rules` prešla v 9 krokoch. `npm test`:
21 testov. Nebolo to overené so skutočným Firebase projektom ani na dvoch
skutočných počítačoch.

## 2.111.0 - Mail podľa priority: nové, potom všetky Starred, potom všetky labels, až potom história

**Poradie podľa toho, čo mail je, nie aký je starý.** Najprv nové maily,
potom **všetky** maily s hviezdičkou (aj roky staré), potom **všetky** maily
v tvojich labeloch (aj archivované), a až potom ostatná história —
Important, Finances, Inbox, Promotions, Spam, Trash, každý priečinok od
najnovšieho. Hviezdička spred roka príde skôr ako obyčajný mail zo včera.
Important (sledovaní odosielatelia) je teraz súčasť histórie, hneď za
labelmi.

**Poradie drží fronta, nie náhoda.** Každý mail je vo fronte raz, v
najlepšom pruhu, v akom ho appka našla — mail z Inboxu, ktorý sa neskôr
nájde v Starred, sa posunie dopredu. Mail s hviezdičkou aj v troch labeloch
sa stiahne raz. Nové maily sa kontrolujú každých 20 sekúnd, aj uprostred
dlhého prvého čítania, a idú pred všetko ostatné.

**Nič nevypadne.** Mail, ktorý Gmail opakovane nevydá, sa už po piatom
pokuse nezmaže z fronty — odstúpi nabok a skúša sa znova (najprv o 30 s,
potom zriedkavejšie, nakoniec každých 6 hodín). Pri „spomaľ" od Googlu
appka čaká dlhšie s každým ďalším razom a nikdy menej, ako Google povie.
Keď Gmail zabudne históriu zmien, appka prejde všetky priečinky znova —
najviac raz za hodinu. Celý jeden krok sa zapisuje naraz, takže pád appky
nič nenechá napoly.

**Nad zoznamom mailov je riadok so stavom:** čo práve číta (*„Starred first
- the rest of your history follows"*), koľko ešte stiahne v každom pruhu
(číslo s **+** znamená, že ešte hľadá ďalšie — celkový počet sa
nevymýšľa), koľko čaká na ďalší pokus a kedy bol posledný sync. Zoznam sa
pri dlhom čítaní obnovuje bez preskakovania a prekresľuje len zmenené
riadky. Keď je čítanie pozastavené, tlačidlo **Sync now** ho obnoví.

Mailový súbor má novú verziu schémy (2); fronta z 2.110.0 sa prečísluje na
mieste, nič sa nestratí. 1461 Rust testov, 0 červených (29 nových, z toho
celý beh proti falošnému Gmailu), 11 testov obrazovky (`npm test`); s
naozajstnou Gmail schránkou to overené nie je.

## 2.110.0 - Mail nanovo: číta sám, od najnovšieho, bez limitu, na oboch počítačoch

**Prečo Starred a labels ukazovali „Nothing here".** Obrazovka si pýtala
2 000 mailov, backend potichu vrátil len najnovších **200** a každý
priečinok sa filtroval z tých 200 v prehliadači. Starší označený mail mohol
byť uložený a nikdy sa neukázal. Teraz je priečinok dotaz do databázy, bez
limitu, ďalšie maily sa načítajú pri scrollovaní.

**Prečo „Google refused access".** Google hlási „spomaľ" aj ako 403. Appka
to čítala ako odmietnutie, neskúsila znova a maily, ktoré sa práve nestiahli,
vypadli z fronty. Teraz sa číta dôvod v odpovedi, appka počká a pokračuje
sama; nič sa nestratí. Bez internetu napíše, že Gmail je nedostupný, skúša
znova každých 30 sekúnd a z fronty nevyhodí nič.

**Číta sám, v poradí: Important → Starred → labels → Finances → všetko
ostatné**, v každom priečinku od najnovšieho po najstarší, bez dátumového
limitu. Kde skončil, je na disku. Sync netreba stláčať.

**Maily vo vlastnom, zamknutom súbore** (`tiqr-manager.mail.sqlite3`).
Cloud sync, obnova zo zálohy ani prepnutie účtu sa ho nedotknú, a databáza
odmietne zmazať mail inak než cez **Delete forever** v koši. Ukladá sa len
hlavička a krátky náhľad, nikdy telo.

**Dva počítače:** pripojenie, pravidlá, odosielatelia, filter aj webhooky sú
rovnaké na oboch (platí novšia zmena). Maily si každý číta z Gmailu sám.
Automatické vytváranie ostáva per počítač.

**Ďalej:** priečinky Inbox (všetko okrem spamu, koša a promotions),
Important, Starred, Finances, OTP, Needs review, Promotions, Spam, Trash,
labels. Otvorený mail už nie je jeden riadok s 400 znakmi. V nastaveniach
opravené pole pravidla (bolo stlačené pod výberom), texty bez „date window".
1440 Rust testov, 0 červených (54 nových; 17 starých odišlo s kódom, ktorý
testovali); obrazovka vykreslená a skontrolovaná.

## 2.109.1 - Oprava: appka padala pár sekúnd po spustení syncu

**Príčina.** Pri ukladaní každého nového mailu sa z predmetu a náhľadu číta
počet lístkov. To čítanie sa pozrelo 12 *bajtov* pred slovo „tickets" - a
keď tam bolo emoji alebo diakritika (`🎟️ Your tickets...`), padlo to doprostred
znaku. Rust na tom spanikári a vydaná appka sa pri panike celá zavrie. Prvá
dávka 40 mailov to takmer vždy obsahovala, preto pár sekúnd.

**Rovnaká chyba ešte na dvoch miestach**, opravené naraz: skracovanie textu na
400 bajtov (každý náhľad pri ukladaní, a celé telo mailu pri otvorení) a
čítanie HTML entít ako `&amp;` (otvorenie HTML mailu). A jedno mimo mailu:
výsledok vyhľadávania vo Workspace pri znakoch ako `İ`, ktoré po zmenšení
písmen zmenia dĺžku.

**Nič iné sa nezmenilo** - pre každý text, ktorý predtým nepadal, vychádza
presne to isté. Pribudlo 7 testov, ktoré pred opravou padali presne s touto
chybou (vrátane celej cesty ukladania mailu na skutočnej databáze) a teraz
prechádzajú. Celkom 1404 Rust testov, 0 červených - prvýkrát reálne spustené,
nie len simulované.

## 2.109.0 - Mail: vlastné pravidlá, nastavenia v maily, starred vždy

**Nastavenia sú teraz v Maily, dole v zozname zložiek.** Presne tam, kde si
chcel - pri maily, nie tri obrazovky ďalej. V Settings → Integrations ostal
jeden riadok s odkazom, aby si ich tam zbytočne nehľadal.

**Pravidlá.** "Ktorú hlavičku rovno riešiť, ktorú rovno do koša." Pravidlo
je: pozri sa na *jedno* pole (odosielateľ / predmet / náhľad), hľadaj
*jeden* text, urob *jednu* vec. Štyri akcie: do koša, označiť prečítané,
poslať do Needs review, a **Keep** - ktorý zastaví zoznam, takže široké
pravidlo nemôže zjesť order confirmation. Idú zhora dole, prvá zhoda
vyhráva, a poradie sa dá meniť šípkami. Pri každom vidíš, koľkokrát už
zabralo. Tlačidlo "Run rules on mail I already have" ich pustí na to, čo už
máš - inak by platili len na budúce maily a nedalo by sa overiť, či robia,
čo si myslel.

**Nič z toho nesiaha do Gmailu.** Appka má read-only prístup. "Do koša"
znamená do koša TIQR-u, odkiaľ sa dá vrátiť. Žiadne pravidlo nikdy nemaže.

**Odosielatelia sa dajú upravovať.** Tých 17 domén bolo doteraz zadrôtovaných
v binárke. Teraz sa dajú prepísať, zmazať aj pridať, a je to jeden zoznam
namiesto dvoch (tvoje staré "also watch these" sa automaticky prenesú).
Keď zmažeš všetky, appka ťahá celú schránku - nevráti si potichu svojich 17.

**Starred konečne naozaj vždy.** Bola tam reálna chyba: starred aj bežné
maily sa listovali do jedného stropu 900, Gmail vracia od najnovších, takže
pri viac ako 900 označených sa tých istých 900 vracalo dookola a staršie
sa **nemali ako dostať dnu**. Teraz má starred vlastné listovanie až do
30 000, ide **prvý v poradí**, a neplatí naň ani dátumové okno ani tvoj
vlastný filter.

**Maily pribúdajú priebežne.** Zoznam ID sa uloží na disk (`mail_fetch_queue`)
a sťahuje sa po 40. Vidíš "ostáva ešte X", prvý sync prežije zavretie appky,
a kým sa dobieha pozadu, dátumové okno sa neposúva - inak by sa nestiahnuté
maily dostali mimo dosah navždy.

**Vlastný Gmail filter.** Napíšeš napr. `-category:promotions`,
`has:attachment`, `label:PULLS`. Pridáva sa k oknu, nikdy ho nenahrádza.
Nevyvážené zátvorky alebo úvodzovky sa odmietnu celé, nie polovične.

**Všetky záložky**, aj prázdne. Doteraz sa skrývali tie, z ktorých sa ešte
nič nestiahlo - čo pri prvom syncu znamenalo skoro všetky.

**Rust testy — všetkých 6 červených.** Tri kontrolovali `-in:spam` a nulové
`in:anywhere`, teda pravidlá, ktoré appka zámerne prestala dodržiavať, keď
si pýtal vidieť spam a kôš. Päť kontrolovalo 30-dňové okno, hoci sa pozerá
rok dozadu — tie si dátum teraz **odvodia z konštanty**, takže nevyhnijú.
Jeden čítal SQLite `dflt_value` ako číslo, pričom je to text. Pribudlo 26
nových testov (pravidlá, domény, filter, konštanta okna).

**Jedna reálna chyba chytená pred odoslaním:** nový SQL dotaz čítal stĺpec
`preview`, ktorý sa v databáze volá `snippet` - padlo by to až za behu.
Našiel to nový kontrolór SQL stĺpcov, ktorý som naňho napísal a najprv
overil na troch zámerne pokazených kópiách.

## 2.108.0 - Oprava padania + rýchlosť

**Prečo appka padala, a bola to moja chyba.** Na štyroch miestach som bral
okno okolo sumy obyčajným počítaním bajtov. V angličtine to prejde. Stačí
jedno euro, jedna dĺžeň alebo emoji pri cene a ten bajt padne doprostred
znaku - a Rust na tom **panikuje**. Dialo sa to **vnútri zámku databázy**,
takže sa zámok otrávil a padlo všetko ostatné s ním. **Jeden cudzí znak
v jednom reklamnom maili zabil celú appku.**

Nehádal som to - overil: dve z piatich bežných slovenských správ to zhodia,
vrátane úplne obyčajného "Ďakujeme za objednávku. Cena za lístok 40,00 EUR".
Je na to test presne s tými vetami.

Je to tá istá trieda chyby, akú som opravil v 2.101.0 pri čistení HTML - a
potom ju zaviedol inde. Pravidlo si tentoraz píšem natvrdo do kódu:
**nikdy nekrájať text počítaním bajtov.**

**Rýchlosť.** Predtým to ťahalo jednu správu na jeden HTTP request, za
sebou - 8 správ za sekundu. Teraz beží šesť naraz, s opakovaním pri
preťažení.

**Ale musím ti povedať strop na rovinu:** Google účtuje 5 jednotiek za
správu a dáva ich asi 250 za sekundu, čiže **~50 správ za sekundu je limit**,
nech to napíšem akokoľvek. Dvadsať vlákien by zarobilo len chyby. Niekoľko
tisíc za pár sekúnd teda nejde - 3000 správ je minúta, to je Googlova
matematika, nie moja lenivosť.

**Čo som spravil namiesto toho:** jeden beh je teraz malá dávka (~6 sekúnd),
uloží čo stiahol a hneď naväzuje ďalšia. Zoznam sa po každej dávke obnoví,
takže maily **pribúdajú vo vlnách a vidíš počítadlo**, namiesto toho aby si
minútu civel na prázdnu obrazovku. Rovnaký celkový čas, úplne iný pocit.

## 2.107.1 - Oprava buildu: tri vydania sa vôbec nepreložili

Mal si pravdu a je to horšie než spadnutý test: **2.105, 2.106 ani 2.107 sa
nikdy nepreložili**, takže sa celá sada testov zakaždým preskočila. Dve
chyby, obe moje:

- **`amount_cents_in` neexistovala.** Keď som prepisoval čítanie cien okolo
  totalu, nahradil som blok, v ktorom tá pomocná funkcia bola - a volajúci
  ju volal ďalej.
- **Nesúlad typov v `create_from_mail`**: sľuboval `MailProposal`, ale
  končil volaním, ktoré vracia `Option<MailProposal>`.

**Všetky moje kontroly hlásili v poriadku**, lebo ani jednu z týchto tried
nepokrývali - zátvorky sedeli, struct literály sedeli, importy sedeli,
príkazy boli zaregistrované. Dopísal som preto dve nové a **každú som
najskôr pustil na tú skutočnú chybu**, aby som vedel, že ju naozaj chytí:
volanie neexistujúcej funkcie (9599 volaní, čisto) a nesúlad návratového
typu (160 volaní, čisto).

Pri ich písaní som našiel **tri chyby v tých kontrolách samotných** - každú
tak, že som nález preveril namiesto toho, aby som mu uveril.

## 2.107.0 - Maily sa objavujú samy

- **Už nemusíš klikať Sync.** Kým máš Mail otvorený, appka sa pýta Gmailu
  sama a nové maily pribudnú do zoznamu bez toho, aby si čokoľvek urobil.
- **Interval si nastavíš** v Settings → Integrations → Email: 30 sekúnd,
  minúta, 3, 5, 15 minút, alebo „len keď stlačím Sync". Default je minúta.
  Platí aj pre kontrolu na pozadí, keď si v inej časti appky - predtým tam
  bolo natvrdo 15 minút.
- **V hlavičke vidíš, že to žije** - „Checking for new mail..." alebo
  „Checks every 1 min", takže nesedíš nad tým, či máš kliknúť.
- **Zoznam sa prekreslí len keď naozaj niečo prišlo.** Obnova, ktorá nič
  nezmení, by ti len preskočila zoznam pod kurzorom.
- Automatické behy sú **ticho** - hláška, ktorú si nepýtal, je horšia než
  žiadna. Skutočnú chybu aj tak vidíš na stavovom pruhu.

**Jedna vec na rovinu: je to opakované pýtanie sa, nie push.** Skutočný
Gmail push doručuje na verejnú HTTPS adresu a TIQR žiadny server nemá -
hovorí s Googlom priamo z tvojho počítača, takže nie je kam pushovať.
Interval je teda skutočný strop a kým je appka zavretá, nekontroluje sa nič.

## 2.106.0 - Z mailu sa dá urobiť záznam, po jednom, tvoje rozhodnutie

- **Pri každom rozpoznanom maile je tlačidlo.** Vidíš, čo z neho TIQR
  prečítal, a jedným klikom z toho vznikne finančný záznam alebo objednávka.
- **Nič sa nevymýšľa.** Objednávka potrebuje event, predaj potrebuje order -
  a tie sa **párujú** na to, čo už v TIQR máš. Event podľa mena v predmete,
  order podľa jeho vlastnej referencie. Len keď sedí **práve jeden**. Dva
  eventy s rovnakým menom nie sú voľba, ale blokér - appka ti povie prečo.
- **Dvakrát sa to stať nemôže.** Mail si pamätá, čo z neho vzniklo, a
  kontroluje sa to v tom istom zámku ako zápis. Druhý klik, druhý sync aj
  reštart uprostred behu narazia na to isté.
- **Používa existujúcu logiku**, nie kópiu - tá istá validácia a to isté
  rozpočítanie centov ako keď to píšeš ručne.
- **Auto / manuál / vypnuté, zvlášť pre každý typ**, v Settings. Default je
  **manuál** pre všetky tri. Auto zasiahne len tam, kde je všetko spárované.
- **Predaj sa z mailu zámerne nevytvára.** Mail nepovie, ktoré lístky sa
  predali, a vybrať to za teba podľa predmetu by bola lotéria. Appka ti
  namiesto toho **pomenuje ten order**, kde to zapíšeš dvoma klikmi.
- **Tvoje pravidlo o fees platí až do objednávky**: celok ide do ceny
  lístka, `fees` nesie len zvyšok po delení - 95,00 EUR na 3 lístky uloží
  31,66 x 3 + 2 a sedí to na cent. Je na to test.
- **Webhook do Discordu**, keď niečo vznikne - do Finance, Inventory alebo
  Sales podľa typu.

## 2.105.0 - Bližšie ku Gmailu: starred, záložky, OTP, kôš

- **Starred maily bez obmedzenia dátumu.** Sync beží teraz dvakrát - raz na
  okno, raz na `is:starred` úplne bez dátumu. Je naozaj jedno, aké sú staré.
- **Záložky sú tvoje vlastné z Gmailu**, pod skutočnými menami. Spam a Kôš
  sa dajú prezerať - ale **nikdy sa neklasifikujú**: spam, ktorý tvrdí, že
  si niečo predal, sa do Important nedostane a nič sa z neho nevyčíta.
- **OTP kódy majú vlastný priečinok a kód vidíš priamo v zozname.** Ťažšia
  polovica nie je nájsť číslice, ale odmietnuť ich: musí ich zaštítiť
  kľúčové slovo do 60 znakov, a štvorciferný rok sa odmietne.
- **Čítanie cien som prerobil a mal si pravdu.** Bral som posledné číslo pri
  mene, takže zo 40 € za kus sa stala cena objednávky, ktorá stála 94 €.
  Teraz hľadá **total s fees** a cenu za lístok z neho **vydelí**. Kde sú
  len časti a žiadny total, **radšej nepovie nič** - „cena 40 / poplatok 7"
  nehovorí, či je ten poplatok za kus alebo za objednávku.
- **Hromadné akcie a kôš.** Označíš viac mailov naraz. **Najskôr kôš, až
  potom mazanie** - a to pravidlo je vynútené v backende, nie v dialógu.
  Gmail ostáva nedotknutý, appka má prístup len na čítanie.
- **Číta toľko, koľko sa dá**: rok dozadu, 750 nových za beh. Ide to len
  preto, že beh preskakuje to, čo už má.
- Pri otvorení mailu vidíš **odkiaľ naozaj prišiel** - celú hlavičku From,
  komu, a kam by šla odpoveď. Ak sa odpoveď líši od odosielateľa, appka na
  to upozorní.

## 2.104.0 - Z mailu je normálny inbox

Žiadal si celý inbox. 2.100.0 ti dalo tabuľku toho, čo appka rozpoznala, bez
možnosti si čokoľvek **prečítať**, a 2.101.0 to ešte zúžilo len na
marketplace odosielateľov. Oboje odpovedalo na zlú otázku.

**Tie dve veci, čo si pýtal, si len zdanlivo odporujú.** Nevadilo ti, že
spam existuje - vadilo ti, že **zavadzia**. Takže sync znova ťahá celú
schránku a triedi to UI:

- **Záložky**: Important / All mail / Promotions / Social / Review.
  Promotions a Social sú **Gmailove vlastné kategórie**, nie pravidlo, čo
  som si vymyslel - teda presne to rozdelenie, čo už v Gmaile vidíš.
- **Dvojpanel**: zoznam vľavo, mail vpravo. Dá sa čítať.
- Neprečítané sú tučné s bodkou, prečítané tiché - ako v normálnom klientovi.
- V Settings si vieš prepnúť späť na „len títo odosielatelia", ak budeš chcieť.

**Sync ťahá len hlavičky, telo až keď mail otvoríš** - a neukladá sa.
Inak by sťahovanie celej schránky znamenalo megabajty reklamného HTML len
na to, aby sa vykreslil zoznam s odosielateľom a predmetom.

**Tri veci, ktoré odhalilo až to čítacie okno:**

1. Čistenie HTML zlučovalo aj nové riadky, takže skutočný mail bol **jedna
   stena textu**. Teraz sa odstavce zachovajú. Bunky tabuliek sa zámerne
   nelámu - reklamné maily sú poskladané z tabuliek a lámať každú bunku by
   dalo „Event:" a „Coldplay" na dva riadky.
2. Dekódovalo sa len **šesť** entít, takže mail písal „Good news &ndash;
   your listing has sold". Teraz je tam skutočný dekodér. `&`, ktoré nie je
   entita, ostáva nedotknuté - mailov plných „Tom & Jerry" a `?a=1&b=2` je
   dosť.
3. Dvojpanel potrebuje skutočnú výšku a Layout stránkam žiadnu nedáva
   (je to zapísané v PROTECTED_AREAS). `h-full` by sa ticho zrútilo.

**Prečítané/neprečítané je TIQR-ovo vlastné a appka to aj píše** - scope je
zámerne len na čítanie, takže v Gmaile sa nič neoznačí.

92 testov v `mail.rs`.

## 2.103.0 - Oprava po CI: `<scriptish>` nie je `<script>`

- **Chyba, ktorú chytilo CI.** Kontroloval som len to, či názov tagu
  *začína* na „script" - lenže názov tagu v HTML končí medzerou, `>` alebo
  `/`. Takže `<scriptish>` sa bral ako `<script>` a **zahodil sa celý zvyšok
  mailu** pri hľadaní `</script`, ktoré nikdy neprišlo. Opravené.
- **A druhá vec, opačná: nepravdu mal môj TEST, nie kód.** Rozšírený test
  tvrdil, že `<script/>after` nechá „after". Nenechá a ani nemá - `<script/>`
  sa v HTML **nezatvára sám**, prehliadač ho berie ako otvorený `<script>` a
  zhltne zvyšok. Pre funkciu, ktorej úlohou je nepustiť von zdroják skriptu,
  je zahodiť viac tá správna strana. Opravil som očakávanie, nie kód, a
  napísal to tam, aby to čítalo ako rozhodnutie, nie ako chyba.
- Chytil som to **pred CI** tým, že som si `strip_html` prepísal do Pythonu
  a prehnal ním všetkých dvadsať tvrdení - bez kompilátora sa očakávanie
  inak overiť nedá.

## 2.102.0 - Nečitateľné rozbaľovačky + dve chyby na detaile eventu

- **Rozbaľovačky sa konečne dajú prečítať.** Otvorený `<select>`, výber
  dátumu aj posuvník kreslí **prehliadač**, nie náš štýl, a jediné, čo mu
  povie tmavo/svetlo, je `color-scheme` - a ten v celej appke nebol nikdy
  nastavený. Prehliadač teda kreslil **biele** okno a text do neho dedil
  takmer bielu farbu z tmavého režimu. Biela na bielej - presne ten zoznam,
  čo si odfotil. Jeden riadok na tému opravil **všetkých 58 výberov**, každé
  pole s dátumom aj posuvníky naraz.
- **Na detaile eventu sa vypisovalo `&middot;` ako text.** JSX dekóduje
  entitu napísanú ako text, ale nie vnútri reťazca v zátvorkách. Opravené na
  skutočný znak. Zvyšok appky som **preveril** - tam boli správne.
- **Detail eventu mal nadpis „Sales" dvakrát** (obal + vlastný nadpis sekcie)
  - pozostatok z 2.90.0. Obal je preč.
- **Tabuľky: záruka namiesto pravidla.** Každá tabuľka má pevné šírky
  stĺpcov, takže jeden dlhý reťazec bez medzier sa vedel vytlačiť z bunky
  cez susedný stĺpec. Bunky teraz text zalomia - aj keby boli šírky zle,
  stĺpec bude **úzky, nikdy nie prekrytý**.
- **Checker na tabuľky som napísal a NEPOSLAL.** Dával falošné poplachy na
  zdravých tabuľkách (podmienené stĺpce, dva varianty, `<col>` spomenutý vo
  vlastnom komentári). Kontrola, ktorá kričí na zdravý kód, je horšia než
  žiadna.

## 2.101.0 - Oprava pádu pri Sync now + koniec so spamom

**Pád bol moja chyba, a boli to tri chyby naraz v jednej funkcii**
(`strip_html`, ktorá čistí HTML z mailu):

1. **Kvadratická.** Pri každom `<script>`/`<style>` si kopírovala **celý
   zvyšok dokumentu** a prevádzala ho na malé písmená. Reklamný mail s
   dvadsiatimi style blokmi = dvadsať kópií celého mailu. Sto takých mailov
   v jednom synchu = tisíce kópií. Tam to umrelo.
2. **`Vec<char>`** - 4 bajty na znak, takže 1 MB mail sa najprv nafúkol na
   4 MB.
3. **Bajtové posuny použité ako indexy znakov.** Pre čistú angličtinu to
   sedí - preto to prešlo všetkými mojimi testami. Stačila jedna slovenská
   diakritika alebo emoji a preskočenie skončilo na zlom mieste, takže
   **zdroják skriptu mohol prejsť do textu** - presne to, čomu má tá funkcia
   zabrániť.

A všetko to bežalo **so zamknutou databázou**, takže sa celá appka zasekla
skôr, než spadla. Teraz: jeden lineárny prechod po bajtoch, žiadne
alokovanie navyše, vstup obmedzený na 64 KB, a klasifikácia je **mimo
zámku**. 8 regresných testov vrátane slovenčiny a emoji.

**Druhá vec, ktorú si pýtal: už nesťahuje spam.**

Predtým sync pýtal od Gmailu **všetky** maily za posledný mesiac a triedil
ich až potom - teda sťahoval a parsoval haldu newsletterov, aby našiel štyri
skutočné. Teraz **filtruje Google, nie TIQR**: dotaz menuje tie
marketplace-y a platobné brány, ktoré appka pozná, plus čokoľvek si pridáš
sám. Spam, newslettery a súkromná pošta sa **nikdy nestiahnu, neparsujú ani
neuložia** - čo je rovnako výhra pre súkromie ako pre rýchlosť.

- V **Settings → Integrations → Email** je nový panel **„What TIQR looks
  at"** - vidíš zoznam sledovaných odosielateľov a vieš si pridať vlastné
  domény (napr. `oeticket.com, tickethub.sk`).
- Domény idú priamo do Gmail dotazu, takže čokoľvek, čo by mohlo ten dotaz
  zmeniť (úvodzovky, zátvorky, medzery), sa **zahadzuje, nie escapuje**. Je
  na to test s nepriateľskými vstupmi.

## 2.100.0 - Mail Intelligence (fáza 1 zo 4)

**Opravené po prvom CI behu (tá istá verzia, nevydaná):**

- **Migračná kanárka 39 -> 40.** `database.rs` má zámerne natvrdo zapísaný
  počet migrácií, ktorý treba zdvihnúť pri každej novej. Migrácia 040 ho
  zhodila presne tak, ako má. Nechávam ho natvrdo - keby sa počítal z toho
  istého zoznamu, ktorý kontroluje, vždy by sedel sám so sebou.
- **Dve karty písali do jedného configu a prepisovali si navzájom
  prepínače.** Settings -> Notifications (desktop + ntfy) a Settings ->
  Integrations -> Discord ukladajú ten istý `NotificationConfig`. Kým boli
  prepínače obyčajné `bool`, **každé uloženie z jednej karty vyplo to
  druhé** - uloženie Discordu potichu vyplo desktop notifikácie. 2.99.0 to
  obišlo v jednom smere (znovunačítanie stavu tesne pred zápisom); CI
  chytilo druhý smer ako chybu prekladu. Teraz je **každé pole v
  `NotificationConfigInput` voliteľné a vynechané pole znamená "nechaj tak"**
  - karta posiela len to, čo vlastní, a obchádzka je preč. Tri regresné
  testy na to.


Žiadal si, aby TIQR čítal mail a automatizoval z neho predaje, sklad a
financie. To je štvorfázová vec; toto je **fáza 1, dodaná celá** - nie štyri
fázy dodané na polovicu.

- **Hranica, ktorú treba vedieť:** nič v `commands/mail.rs` nezapisuje
  predaj, objednávku, lístok ani finančný záznam. Ani jeden `INSERT`, ani
  jeden `UPDATE`. Je na to test, ktorý spočíta riadky v piatich tabuľkách
  pred a po spracovaní mailu a overí, že sa nepohli. Párovanie a vytváranie
  záznamov je fáza 3.
- **Gmail, bez jedinej novej závislosti.** Appka už mala kompletný Google
  OAuth tok pre Sheets a `run_sign_in` berie scope ako parameter. Mailbox je
  teda tretí samostatný grant cez tú istú mašinériu: vlastný scope
  (`gmail.readonly` - TIQR nevie poslať, zmazať ani označiť nič), vlastný
  token, vlastný cancel. Odpojenie mailu ťa neodhlási zo Sheets.
- **IMAP tam NIE JE** - potreboval by nový crate a pravidlo znie žiadne nové
  závislosti. Rozhranie je však postavené tak, aby IMAP pribudol vedľa
  Gmailu, nie cezeň.
- **Kurzor je dátové okno, nie Gmail `historyId`** - zámerne. Gmail drží
  históriu asi týždeň; táto appka beží len keď je otvorená, takže zavretý
  notebook cez sviatky je bežný stav, nie výnimka. Okno sa prekrýva o deň a
  prekryv je zadarmo, lebo ukladanie je idempotentné.
- **Klasifikácia je deterministická** (doména odosielateľa + kľúčové slová),
  žiadny model. Preto "text v maile nevie appku nahovoriť na nič" je
  vlastnosť návrhu, nie nádej - a je na to test s prompt injection.
- **Tajomstvá:** stav pre frontend nesie booleany a adresu schránky, nikdy
  token. Telo Google odpovede sa nikdy nečíta. Telá mailov sa neukladajú -
  len orezaný, od HTML očistený a redigovaný výňatok, ktorý retention maže.
  Prílohy sa nesťahujú vôbec.
- **Nájdené renderom, nie čítaním:** figúrka "Failed" na Overview mohla
  ukazovať len 0 (vo fáze 1 nič ten stav nenastaví), tak je preč.

## 2.99.0 - Discord test webhook

Discord v tejto apke **doteraz vôbec nebol** - jediná zmienka bola komentár
vo `firebase.ts` o odloženom Discord prihlásení. Toto je teda celý kanál,
nie tlačidlo pridané k existujúcemu.

- **Settings -> Integrations -> Discord** (`DiscordCard`): tri webhooky
  (Sales, Finance, Inventory), každý s tlačidlom **Test webhook**, ktoré
  naozaj pošle POST na Discord a zobrazí, čo sa skutočne vrátilo.
- **Frontend nikdy nevidí ani neposiela URL webhooku.** Príkaz
  `test_discord_webhook` berie NÁZOV cieľa ("sales") a URL si nájde backend.
  `DiscordDestination::parse` odmietne čokoľvek iné než tie tri;
  `validate_webhook_url` pred každým odoslaním overí uloženú URL proti
  štyrom Discord hostom. Príkaz sa teda nedá použiť ako HTTP proxy.
- **`NotificationStatus` nesie len booleany** (`discordSalesSet`...), nikdy
  URL ani jej maskovanú podobu. URL webhooku má token v ceste - URL *je*
  to tajomstvo. Modul nelogguje nič, telo Discord odpovede sa **nikdy
  nečíta** (len `.status()` a hlavička `retry-after`) a žiadna chybová
  správa URL neobsahuje - je na to test.
- **`status` má tri hodnoty a UI ich rozlišuje.** `delivered` len pri 2xx,
  `failed` keď Discord odpovedal a odmietol, `unknown` keď sa nič nevrátilo.
  Timeout sa **nehlási ako chyba** a **nikdy sa automaticky neposiela
  znova** - Discord mohol správu prijať a druhý pokus by z jedného testu
  urobil dve správy v kanáli.
- Testovacia správa neobsahuje **nič z databázy** a nič nezapisuje - žiadny
  fiktívny predaj, pohyb ani položka skladu.
- 25 unit testov, bez siete a bez novej závislosti: rozhodovacia tabuľka je
  čistá funkcia `interpret_discord_status(u16)`.
- **Opravené pred vydaním:** pridanie polí do `NotificationConfig` a
  `NotificationConfigInput` nechalo **päť existujúcich test literálov**
  v `notifications.rs` nekompletných - `cargo test` by spadol (E0063).

## 2.98.0 - Päť zo siedmich

### Lístky v evente sú teraz objednávky

To bola **skutočná chyba**, nie vzhľad. Odkaz viedol na
`/orders?code=<kód LÍSTKA>`, ale Inventory hľadá kódy **objednávok** — preto
ťa klik hodil do Inventory a nič nenašiel.

Lístok nemá vlastnú stránku, preto ten odkaz mieril nabok. Objednávka ju má,
takže riadok **je** objednávka a klik ťa na ňu hodí.

Zoskupenie kľúčuje na `orderId`, nie na kód (kód je zobrazovací reťazec).
Mena sa nastaví **len keď sa všetky lístky v objednávke zhodnú** — jedna
objednávka môže niesť dve meny a vtedy sa vypíše „Mixed", nesčíta sa.

### Odstránené

- **Potential Profit** (detail eventu) — posledné miesto v appke, kde svietili
  peniaze, ktoré nikto nezarobil
- **Paid a Outstanding** (detail objednávky)
- **celý pás „Order summary"** — čím sa zoznam lístkov posunul hore pod hlavičku

Výpočty za nimi som odstránil tiež, nie nechal bežať naprázdno. Jediná
výnimka je `salesSummary`: stále sa načítava a nič ho nečíta — nechal som ho
zámerne, lebo je to jediné volanie, ktoré pozná skutočné tržby objednávky, a
je to prvá vec, ktorú bude niekto chcieť späť.

### Pulls na spodok navigácie

### Dve moje chyby, chytené pred vydaním

Do importu typov som pridal `SaleEntry`, **ktorý neexistuje** — to by
neskompilovalo. A nový výpočet `ticketsByOrder` mi pristál v `SalesTab`,
pritom sa používa v `OverviewTab`. Obe som našiel kontrolou, nie odhadom:
pri vkladaní na komentárovú kotvu treba overiť, v ktorom komponente to
skutočne skončí — tá kotva bola v susednej funkcii.

### Čo ostáva — a je to jedna práca, nie dve

Editačné okno v Pulls a zoznam podľa mena. **Skutočná príčina je
štrukturálna:** Events, Orders aj Sales majú každý svoju detail cestu
(`events/:id`, `orders/:id`, `sales/:id`). **Pulls nemá žiadnu**, preto sa
všetko tlačí do jedného vysokého okna. Riešenie je dať mu `pulls/:id` — a
zoznam podľa mena patrí do toho istého prechodu.

**Overené** renderom pred/po, vyvážením zátvoriek celého `src/`, kontrolou
scope (deklarácia aj obe použitia v tom istom komponente) a tým, že každý
importovaný typ naozaj existuje. **Nie je to kompilácia.**

## 2.97.0 - Inventory číta ako Events

Riadok v Inventory má teraz **kategóriový odznak vedľa názvu** a pod ním
**„venue, city"** — presne ako Events.

Väčšina toho tam už bola a nepoužívala sa:

- **Mesto pod názvom existuje od 2.60.0.** `Ticket.eventCity` má dokonca
  komentár „shown under the event name in Inventory".
- **`categoryName` a `categoryColorSlot` sú na tom riadku od 2.0.27** a
  odznak kreslia Orders, Events aj Sales. Inventory jediné nie. Čisté UI,
  žiadne nové dáta.
- **Venue bolo jediné naozaj nové pole.** `events e` už bolo v dotaze
  joinuté, takže je to jeden stĺpec navyše, jedno pole v štruktúre a jeden
  `row.get` — žiadna migrácia, žiadny nový join.

Pridanie poľa do Rust štruktúry bolo bezpečné preto, že `Order` sa skladá na
**jedinom mieste** (`orders.rs:60`). V 2.86.0 ma presne tento typ zmeny
zhodil E0063, lebo tá štruktúra sa skladala na viacerých — ten počet sa
oplatí overiť pred každým takým zásahom.

`eventPlace()` spája venue a mesto **rovnako ako Events.tsx**, takže sa tie
dve obrazovky nemôžu rozísť v tom, ako to formátujú. Event, čo nemá ani
jedno, nevykreslí druhý riadok vôbec — nie prázdny.

**Overené** renderom pred/po. Checkery som musel znovu postaviť (nová
session) a najprv ich pustil na úmyselne pokazený súbor — oba padli správne,
až potom som im veril. **Nie je to kompilácia.**

## 2.96.0 - Neon: sklo na nasvietenom podklade

Vybral si z labu **Neon** a jeho výrazné farby. Nasadené na celý TIQR.

| | hodnota |
|---|---|
| podklad | **#0a0618** (void) |
| akcent | **#c026d3** / 500 `#d946ef` |
| panel | **priehľadný** `rgba(255,255,255,.05)` |
| linka | `rgba(255,255,255,.12)` |
| rádius | **18 px** |

**Priehľadnosť funguje len vďaka podkladu.** `.dark body` nesie dva fixné
radiálne nasvietenia a každý panel nad nimi je `rgba`. Preto osem miest, čo
maľovali nepriehľadné pozadie appky, je teraz `dark:bg-transparent` — inak by
sklo nemalo čo ukázať a appka by vyzerala len vyblednuto.

### Chybu, ktorú si tá plošná náhrada spravila, som musel chytiť čítaním

Dve z tých ôsmich boli **stmavenia za dialógmi** (`dark:bg-slate-950/70`).
Náhrada ich zmenila na `bg-transparent/70` — čo nestmaví nič. Každý dialóg by
sa vznášal nad živým obsahom. Majú teraz `dark:bg-[#05030c]/70`.

Plošná náhrada cez názov triedy je presne taká bezpečná ako kontrola, ktorá
po nej nasleduje.

### `backdrop-filter` zámerne NIE je na `.card`

Každý panel v appke je `.card` a samotné Settings ich vykreslia desiatky.
Blur znamená **samostatný kompozičný prechod na každý prvok** — a účet za to
príde na notebooku. Sklo číta z priehľadnosti, ktorá je zadarmo. Blur je len
na `.card-glass` a na čomkoľvek so `shadow-overlay`, teda na paneloch, čo sa
vznášajú nad **obsahom**, nie nad pozadím. Pred rozšírením to odmerať na
tabuľke Inventory.

### Tabuľky majú hustejšiu výplň než karty

`bg-surface-muted` namiesto `bg-surface`. Dvanásťriadková tabuľka nad
nasvieteným prechodom je naozaj horšie čitateľná — a to sa rieši
nepriehľadnosťou, nie tmavším textom.

### Vedúce číslo svieti cez `currentColor`

Zisk je zelený, keď si zarobil, a červený, keď si stratil. Gradient by ten
význam prepísal, takže `.fig-glow` pridá svetlo **vo farbe, ktorú to číslo už
má**. `.fig-lit` (skutočný gradient) ostáva pre čísla, ktorých farba nič
neznamená.

**Overené** renderom s presnými hodnotami z kódu: nasvietený podklad, sklené
panely, fuchsiový akcent, svetielkujúci zisk, čitateľná tabuľka a opravené
stmavenie dialógu. Vyváženie zátvoriek celého `src/`, `index.css` 114/114,
`tailwind.config.js` 12/12, komentárové chyby 0. **Nie je to kompilácia.**

## 2.95.0 - Tri odstránenia

**Malé grafy pod KPI hodnotami — preč.** Druhýkrát. 2.88.0 ich odstránilo na
tvoju žiadosť, 2.94.0 ich vrátilo, lebo ich ukazovala referencia a brief ich
pýtal menovite. Pozrel si sa a povedal nie.

Zapísal som to do kódu aj do stavu projektu ako pravidlo: **referencia, na
ktorej sparkline je, nie je dôvod ju pridať — dôvod si len ty.** Komponent
`Sparkline` a prop `spark` ostávajú v `ui.tsx` nepoužité, presne ako medzi
2.88.0 a 2.94.0 — preto sa minule vrátili za pár minút a nič nestoja.

**„Profit & Loss" — preč z prepínača na Dashboarde.** Odfiltrované na mieste
volania (`DASHBOARD_METRICS`), nie vymazané zo zdieľaného zoznamu `METRICS` —
ten používa aj Recap a ty si ukázal na kartu Dashboardu. Profit na tej
stránke ostáva ako vlastná KPI karta.

**„Net" — preč z prepínača vo Finance.** Ostávajú Income a Expenses.
Cashflow, čo je prvá pilulka aj predvoľba, aj tak kreslí obe proti sebe —
čiže presne tú otázku, na ktorú Net odpovedal tretíkrát. S ním sa
zjednodušilo aj `headlineIsNet`: `metric === "profit"` sa po odstránení
pilulky už nedá dosiahnuť, takže tá polovica podmienky bola mŕtva.

**Overené** renderom: karty bez grafov nemajú po nich dieru, Dashboard
ponúka Revenue/Cost/Sales, Finance Cashflow/Income/Expenses. Vyváženie
zátvoriek celého `src/`, komentárové chyby 0. **Nie je to kompilácia.**

## 2.94.0 - Deep Black x Electric Purple

### Cashflow preč z Dashboardu

Mód **Cashflow** je z karty grafu odstránený, ako si pýtal. Karta ostala —
je to teraz čisto metrikový graf so štyrmi módmi (Profit & Loss, Revenue,
Cost, Sales). Samotný `CashflowChart` v `DashboardCharts.tsx` **zostáva**,
lebo ho vykresľuje aj Finance Overview a tá v zadaní nebola.

### Tokeny presne podľa tvojich hodnôt

| token | hodnota |
|---|---|
| Background | **#08090D** |
| Surface | **#11131B** |
| Elevated | **#171927** |
| Border | **#272938** |
| Primary purple | **#8B5CF6** |
| Soft lavender | **#C4B5FD** |

Tvoje dve fialové sú **500 a 300** rampy `brand` — takže rampa nie je môj
výklad tvojej farby, je to priamo tvoja farba.

**Plnené tlačidlo sa vracia na bielu na `brand-600`.** V 2.92.0 som ho
prehodil na tmavý text, lebo jasná tyrkysová bielu pri AA neunesie; #7c3aed
ju unesie (5,9 : 1). Rozhoduje kontrastná hranica, nie vkus.

### Sparkliny sú späť — a nie je to protirečenie

V 2.88.0 si ich kázal odstrániť. Tento brief ich pýta doslova („a restrained
sparkline… where appropriate") a referencia ich ukazuje. Takže sa vracajú.

Sú **skutočné**: každá je stĺpec z `data.revenueTimeSeries`, tie isté
koše, ktoré kreslí graf pod nimi. **ROI žiadnu nemá** — tá séria per-bucket
ROI neobsahuje a vymyslená čiara je presne to, čo táto appka nesmie.

### Tvary widgetov

- panely **12 px** rádius (2.90.0 ich dala na 8 px kvôli REGISTER-u, ktorý
  je touto referenciou nahradený)
- ikonové dlaždice **36 px, rádius 10 px** (boli 32/8)
- hodnota vedúcej karty **25 px**, ostatné 20 px
- KPI pás má podlahu **11 rem** namiesto 9 — päť kariet s ikonou, hodnotou,
  trendom aj sparklinou sa pri 9 rem rozdrví skôr, než sa riadok zalomí

### Farba

Vedúca KPI karta nesie **fialový gradient wash** namiesto svetlejšieho
prstenca. Prstenec v tejto appke znamená „vybrané" — a toto nie je vybrané,
je to len číslo, na ktoré sa pozeráš prvé.

### Čo z referencie ešte NIE je

Trojstĺpcový operačný riadok (Upcoming Events / Recent Activity / Ticket
Inventory + Pending Sales tak, ako ich referencia radí), celošírková tabuľka
Event Performance, a horný toolbar s globálnym hľadaním.

### Overené

Vyrenderované: KPI pás s ikonami a sparklinami, fialový wash, graf s
fialovou plochou, donut platforiem. Vyváženie zátvoriek celého `src/`,
`index.css` 110/110, `tailwind.config.js` 12/12, komentárové chyby 0.
**Nie je to kompilácia.**

## 2.93.0 - Mŕtvy stĺpec v Events + toolbar namiesto formulára

### 1. Tá medzera vpravo bola CHYBA, nie vzhľad

`colgroup` v Events mal **8 stĺpcov, hlavička 7**. V 2.76.0 som odstránil
stĺpec Stock z hlavičky aj z tela — a jeho `<col>` som nechal v **oboch**
colgroupoch. Tabuľka teda rezervovala **11,333 % šírky pre stĺpec, ktorý
nič nevykresľuje.** To je presne ten prázdny pás na tvojom screenshote.

Zapamätateľné: zablúdený `<col>` je pri čítaní kódu neviditeľný, lebo
nestojí nikde blízko `<th>`, ku ktorému patrí. Prejaví sa len ako šírka,
ktorá chýba.

Oba colgroupy sú teraz na 7 stĺpcoch so súčtom presne 100 %, a šírky som
prerezal, nie len jeden zahodil: Event si vzal miesto späť (dlhé názvy sa
skracovali ako prvé) a tri peňažné stĺpce sú rovnaké, čo drží stĺpec čísel
ako jeden stĺpec. Dátum tým prestal byť „12.09.2…".

### 2. `.toolbar` — nový layout primitív

Každá zoznamová obrazovka sa otvárala rovnako: päť alebo šesť ovládačov,
každý s viditeľným popiskom **nad sebou**, v zapustenej lište. To je
**formulár** — niečo, čo vypĺňaš zhora nadol. Zoznamová obrazovka nechce
nad tabuľkou formulár; chce **toolbar**, jeden riadok, do ktorého siahneš.
Tie popisky stáli celý pás tam, kde má byť tabuľka.

Popisky sa **nezmazali** — sú `sr-only`. Ostávajú v DOM aj v poradí pre
čítačku obrazovky, len sa nekreslia. Každý ovládač aj tak nesie placeholder
alebo prvú voľbu „All …".

Nasadené na **Events, Inventory, Sales a Pulls (2×)** výmenou **jednej
triedy na obrazovku** — markup vnútri ovládačov je nedotknutý. Preto sa
dalo bezpečne spraviť štyri obrazovky naraz.

Events dostal navyše preskupenie: **prepínač stavu ide prvý** (mení sa
najčastejšie a rozhoduje, na čo sa ostatné štyri filtre vôbec vzťahujú),
hľadanie berie zvyšok šírky cez `flex-1`, a ovládače majú skutočné
`aria-label`.

### 3. Čo priznávam

Mal si pravdu. Predošlé kolá boli vrstva tokenov — farby a povrchy —
nie kompozícia. Toto kolo mení rozloženie: jeden pás navyše preč zo štyroch
obrazoviek a tabuľka, ktorá konečne vypĺňa svoju šírku.

**Stále štrukturálne pôvodné:** Dashboard nad rámec riadku čísel, Finance
Transactions/Accounts/Reports, a všetky dialógy. To je ďalšia reálna práca,
nie formulácia.

### Overené

Vyrenderované pred/po s tvojimi dátami: mŕtvy pás (šrafovane) verzus
tabuľka dobiehajúca k okraju, a formulár verzus toolbar. Vyváženie zátvoriek
štyroch stránok aj `index.css`, obe triedy komentárovej chyby — 0.
**Nie je to kompilácia.**

## 2.92.0 - Premium Noir: tyrkysový akcent + audit obrazoviek

Podľa dodanej deväťobrazovkovej referencie.

**Akcent: violet → tyrkys.** Celá rampa `brand`. Každý stupeň si drží svoju
rolu, ale jeden dôsledok nie je kozmetický: **plnené tlačidlo už nemôže niesť
biely text.** Biela na `brand-600` meria 3,0:1, pod hranicou AA. Primárne
tlačidlo je teraz takmer čierny text na `brand-400` (9,4:1) — a presne tak to
ukazuje aj referencia.

**Povrchy: chladná čierna.** Podklad `#080b0e`, karta `#0e1317`, linka
`#1e262e`. Predtým to bola neutrálna sivo-čierna; referencia má modrastý
nádych a to je väčšina toho, čo oddeľuje „premium noir" od „tmavosivej appky".

**Navigácia:** aktívna položka nesie akcent aj v texte, nielen vo výplni —
samotný tieň pri tej šírke čítal ako „prejdené myšou". Settings sub-nav
dostal ten istý stav, aby sa tie dva railu nerozišli.

### Audit — čo som skutočne našiel

Žiadal si prísny audit a nie opis zámerov. Prečítal som kód a výsledok ma
prekvapil:

- **Notes už dvojpanelové je.** 300 px zoznam + editor (`Notes.tsx:212`).
  To JE ten dvojpanelový workspace z referencie.
- **Settings už ľavý sub-nav má.** `lg:w-56` vedľa obsahu
  (`Settings.tsx:890`). To JE rozloženie z referencie.
- **Inventory, Sales, Pulls** už majú lištu filtrov s popiskami nad
  ovládačmi, segmentový prepínač a tabuľku v `table-shell`. To JE
  architektúra z referencie.

Tie obrazovky teda nepotrebovali prestavbu — potrebovali ten vizuálny jazyk,
a ten práve dostali.

### Čo som ZÁMERNE nepostavil

- **Globálne hľadanie v hornom pruhu.** Referencia ho má, appka nie. Postaviť
  ho ako atrapu je presne to, čo mandát zakazuje: ovládač, čo vyzerá funkčne
  a nerobí nič.
- **Avatary interpretov v tabuľkách.** Appka žiadnu fotku interpreta nedrží.
  Domyslieť si ju je presne to, čo táto appka nesmie nikdy robiť.

### Overené

Vyrenderované: shell, navigácia, tlačidlá, riadok čísel aj tabuľka v novej
palete. Vyváženie zátvoriek celého `src/`, `index.css` aj `tailwind.config.js`,
obe triedy komentárovej chyby — 0 nálezov. **Nie je to kompilácia.**

## 2.91.0 - Prepínač grafu ostáva na riadku s nadpisom

marko poslal dva screenshoty toho istého grafu: pri **Cashflow** je prepínač
vpravo na riadku s nadpisom, pri **Profit & Loss** padne pod nadpis a odtlačí
celý graf dole. *„takto je dobre prvy graf ale ten druhy urobi toto ze tie
vybery da pod to."*

**Príčina bol nadpis, nie rozloženie.** „Balance cashflow" sa vedľa prepínača
zmestil; „Profit & Loss **over time**" nie. A tie dve slová nenesú nič —
karta má časovú os po celej spodnej hrane, čiže každý režim je *over time*.
Nadpis je teraz len názov metriky, takže najdlhší je kratší než ten, ktorý
sa už predtým zmestil.

| | zalomí sa pod |
|---|---|
| „Profit & Loss over time" (pred) | **622 px** šírky karty |
| „Balance cashflow" | 567 px |
| „Profit & Loss" (po) | **562 px** |

Takže Profit & Loss sa teraz zmestí všade, kde sa zmestí Cashflow — presne
to, čo si chcel.

**Tri zábrany okolo toho, v tomto poradí:**

- `min-w-[150px]` je **reálna podlaha**, zámerne nie `min-w-0`. Nulovú
  podlahu som najprv skúsil a vyrenderoval: blok sa zmenší tak, že číslo
  vyleze spod nadpisu a koliduje s prepínačom. To je horšie než zalomenie,
  ktoré to malo liečiť.
- Nadpis má `truncate` — je to tá polovica, ktorá má ustúpiť.
- Hodnota má `whitespace-nowrap`, lebo v 2.74.0 sa už raz peňažné číslo
  skrátilo na nulovú šírku.
- `flex-wrap` na riadku **ostáva** ako skutočná poslednná možnosť pre okno
  užšie než podlaha a prepínač dokopy.

**Overené** renderom na hranici: pri 580 px sa starý nadpis zalomí (jeho
screenshot) a nový drží na riadku. Vyváženie zátvoriek a obe triedy
komentárovej chyby OK. **Nie je to kompilácia** — CI je prvý kompilátor.

## 2.90.0 - THE REGISTER naozaj nasadený + Events dokončené

Prvý implementačný blok z kompletného redesign mandátu.

### Nález, ktorý zmenil poradie práce

**2.89.0 porušilo THE REGISTER.** Register hovorí doslova: *„vlasové linky,
8 px rádius, žiadne tiene, prechody ani ilustrácie"*. Ja som deň predtým
nasadil na každú kartu tieň aj gradient. Oboje je preč, spolu s mäkkým
nadvihnutím, ktoré tam pridalo ešte 3.0. Panel je teraz presne to, čo
register hovorí: povrch, vlasová linka, 8 px.

### Čo sa zmenilo globálne

- `--sh-card` je **len vlasová linka**, v oboch režimoch. Gradient preč.
- Karta má **8 px rádius** (bolo 12).
- Popisok je **polotučný**, ako register predpisuje — a pribudli `.field`
  a `.field-bar` (16 × 28 px), takže formulár si už nemusí riešiť medzery sám.
- **Mono čísla** (`.num`) — systémový mono stack, žiadny nový súbor, appka
  ostáva offline.

### Riadok čísel prestal byť riadkom kariet

Toto je tá odpoveď na „vsetky okienka ostali rovnake". V registri **riadok
hlavných čísel nie je päť kariet** — je to **jeden panel, v ňom čísla delené
vlasovými linkami**. Päť orámovaných krabíc dá piatim číslam rovnakú váhu ako
tabuľke pod nimi a čítajú sa ako päť objektov. Jeden panel so štyrmi
predelmi sa číta ako jeden súhrn, čím to aj je.

Nové `Figures`/`Figure` v `ui.tsx`. Prerobené: **Dashboard**, **EventDetail**
(3 riadky), **Finance Overview** (2 riadky). `StatCard` zámerne ostáva
exportovaný — `OrderDetail` a `SaleDetail` ho balia inak a prerobiť ich
naslepo v jednom kole je presne to, čím sa redesign rozbije.

### Events — hotové podľa usadeného dizajnu

`EVENTS-LAB-LOG.md` §07: **01 a 01**, usadené 30.09.

**Zoznam — *The Table*.** Tvojich sedem stĺpcov ostáva (Stock padol v 2.76.0,
ako si pýtal). Pribudlo: **deň pod dátumom** („in 2 days", „49 days ago",
„cancelled"), **mono čísla**, a nad tabuľkou **počet riadkov, ktoré sa nedajú
sčítať**. To posledné je dôležité: per-riadok „Mixed" fungovalo vždy, ale
nič ti nepovedalo, že sa to deje — musel si si to všimnúť v bunke. Chyba,
ktorú si táto appka stále opakuje, sa neprejaví ako zlé číslo, ale ako
**neprítomnosť**, a tá nemá pixely.

**Detail — *One Page*.** Záložky Overview a Sales sú preč, event je jeden
scroll. Lab to zdôvodňuje jednou vetou: *„boli to dve polovice jednej
otázky"*. `onSwitchTab` bol mŕtvy prop (deklarovaný, nikdy nevolaný).

### Dva konflikty — nehádal som, zapísal som ich

- **Pulls sú v dokumentácii zaparkované** („Pulls ostáva tak, ako je",
  30.09), mandát ich chce. Register pass na Pulls je nedorobok, funkcie
  z labu (Receipts atď.) som nestaval.
- **Syne/Instrument sa nedajú dodať.** Appka je offline-first a bundluje len
  dva Bricolage subsety; tie súbory nemám a stiahnuť ich neviem. Mono polovicu
  registra som dodal zo systému.

### Čo ešte NIE je spravené

Inventory · Sales · Pulls · Finance (Transactions, Accounts, Reports) ·
Notes (Two Pane, Pin to Record, Quick Note, Real Checklist, Templates) ·
Settings (Import Preview + podsekcie) · bankový výpis cez CSV ·
Restriction Stamp · Bulk Change Preview.

### Overené

Vyrenderované pred/po s jeho číslami (riadok čísel aj tabuľka Events).
Vyváženie zátvoriek 65/65 TS súborov a `index.css`, obe triedy komentárovej
chyby 0, všetky `<Figure>` použitia majú import. **Nie je to kompilácia** —
CI je prvý kompilátor.

## 2.89.0 - Onyx Deep: tmavšie okienka (a prečo predtým nešli stmaviť)

marko: „tie zmeny vobec neboli velke, som sklamany, tie okienka. vsetky
ostali rovnake, totreba zmenu, chcem ich tmavsie a taka krajsie".

Mal pravdu a dôvod bol štrukturálny, nie vec vkusu: **karta bola SVETLEJŠIA
než stránka** — `--surface` = slate-900 `#141415` na podklade slate-950
`#0b0b0c`. Preto každé okienko čítalo ako tá istá stredne sivá bez ohľadu na
to, čo sa na ňom menilo.

| | pred | po |
|---|---|---|
| podklad stránky | `#0b0b0c` | `#050507` |
| karta | `#141415` | `#0c0c10` |
| hrana karty | `#1f1f23` | `#32323d` |
| horná hrana | — | svetlo, 96 px nadol |

Karta je teraz približne tam, kde bolo **staré pozadie stránky**, a pozadie
kleslo ešte hlbšie. Dve veci bránia tomu, aby tmavšia karta na tmavšom
podklade zmizla: **svetlejší prsteň** v `--sh-card` (keď výplň prestane
kresliť kartu, musí ju nakresliť hrana) a **krátke svetlo po hornej hrane**
v `.card`. To svetlo je pevných 96 px, nie percento — vysoký panel s tabuľkou
tak dostane rovnakú hranu ako nízka KPI dlaždica, nie natiahnutý prechod.

### Čo bolo treba vyriešiť, aby sa to vôbec dalo

- **`--surface` už nesmie byť `theme("colors.slate.900")`.** Ten istý stupeň
  nesie text v light móde (`text-slate-900`). Keby som hýbal rampou, prefarbím
  text. Dostal vlastnú hodnotu.
- **`slate-950` v rampe sa zmeniť DALO** — overil som, že v celej appke nie je
  ani jeden `text-slate-950`, `border-slate-950` ani `ring-slate-950`, nesie
  výhradne pozadie stránky.
- **20 miest malo `dark:bg-slate-900` napísané natvrdo** vedľa `bg-surface`.
  Kým sa token rovnal slate-900, boli neškodné; po zmene by to boli svetlejšie
  fľaky — doslovná verzia toho „všetky ostali rovnaké". 17 z nich override
  stratilo a ide za tokenom. 3 majú priehľadnosť (`/60`, `/90`, `/95`), čo
  Tailwind cez `var()` farbu nevie, takže ukazujú na hex priamo.
- **`--line` som zámerne NEHÝBAL.** Niekoľko stoviek miest píše
  `dark:border-slate-800` natvrdo; token, ktorý by s nimi nesúhlasil, by
  spravil dve rôzne vlásočnice na jednej obrazovke.

### Overené

Vyrenderované pred/po s jeho vlastnými číslami, s **presne tými hodnotami,
ktoré sú v kóde**. Vybrané porovnaním piatich variantov, nie odhadom. Plus
vyváženie zátvoriek v TS aj v CSS/JS a obe triedy komentárovej chyby.
**Nie je to kompilácia** — CI je prvý kompilátor.

## 2.88.0 - Preč: donut v Sales by platform a sparkliny v KPI kartách

marko: „odstran to kolecko ten donut a tie male grafy odstran".

**Donut (`PlatformRing`) v „Sales by platform" je preč.** Zoznam pod ním bol
vždy ten skutočný rozpad — menuje platformu, sumu, percento aj počet kusov,
a dá sa rozbaliť. Krúžok to len zopakoval. Jeho stredové číslo bolo revenue
za obdobie, ktoré karta Revenue hore na Overview aj tak ukazuje, takže sa
odstránením nestratilo nič.

**Sparkliny v piatich KPI kartách sú preč** (Profit, Revenue, Purchase cost,
Tickets sold — ROI žiadnu nemala). Karty teraz nesú číslo, trend a
podriadok. Reálny graf je priamo pod tým riadkom, nezmenený.

`platformTotal` zostáva — ten ešte stále počíta percentá v zozname. Zmenené
sú len komentáre, ktoré hovorili o „arc above" a o stredovom čísle krúžku,
aby neklamali.

**Mŕtvy kód ponechaný zámerne:** `PlatformRing` v `DashboardCharts.tsx`,
`Sparkline` a prop `spark` na `StatCard` v `ui.tsx` v kóde zostávajú, len ich
už nič nevolá. Sú exportované, takže build to nerieši. Vymazať ich je väčší
zásah do súboru, ktorý importuje každá stránka — ak ich mám odstrániť nadobro,
povedz.

**Overené:** vyrenderované staticky s jeho vlastnými číslami pri šírke 1400 px
— päť KPI kariet v jednom riadku, každá hodnota čitateľná, zoznam platforiem
začína rovno pod hlavičkou karty. Plus vyváženie zátvoriek a obe triedy
komentárovej chyby na `Dashboard.tsx`. **Nie je to kompilácia** — na tomto
Macu nie je node ani cargo, CI je prvý kompilátor.

## 2.87.0 - Build fix: opravený build + 10 padajúcich Rust testov

### 1. TypeScript — moja oprava z 2.86.0 bola tiež chybná

V 2.86.0 som `{/* ... */}` správne nahradil blokovým `/* ... */`. Ale do
**textu** toho komentára som napísal, čo bolo predtým — vrátane `*/`.
Blokový komentár končí na **prvom** `*/`, takže sa uzavrel v polovici vety
a zvyšok prózy aj `}` zostali ako kód.

Presne to CI hlásil: `TS1109: Expression expected` na riadku, kde komentár
začína, a `TS1381` tam, kde mal skončiť — Overview.tsx 406/424,
Transactions.tsx 202/220, Reports.tsx 204/222.

Komentár je prepísaný tak, že ukončovaciu sekvenciu vo svojom texte
neobsahuje. Pravidlo: **text komentára nikdy nesmie obsahovať ukončovaciu
sekvenciu** — ak treba hovoriť o syntaxe komentárov, musí sa opísať slovami.

### 2. Rust: 10 padajúcich testov — všetky jedna príčina, ani jedna chyba v appke

`cargo test` je v CI od 2.73.0. Suite dovtedy reálne nebežal od posledného
stroja s Rust toolchainom, a medzitým 2.30.0 zaviedlo event-derived kódy
(`COLDPLAY-001` namiesto `PULL-000001`). Každý test, ktorý mal zadrôtovaný
fixný prefix, tým prestal platiť.

- **`a_brand_new_path_ends_up_fully_migrated`** — canary na počet migrácií
  stál na 26, kým pribudlo 027–039. Je to presne ten test, ktorý má padnúť
  pri zabudnutej registrácii migrácie, a padal — len to 13 vydaní nikto
  nevidel. Všetkých 39 je v `MIGRATIONS` a všetkých 39 sa aplikuje. → 39.
- **`search_finds_order_by_partial_ticket_code`** — `trim_start_matches("TKT-")`
  od 2.30.0 neodstrihával nič (kód je `TEST-001`), takže „podreťazec" bol
  celý kód. Teraz sa delí na separátore, čo funguje pre obe formy.
- **`pulls` / `pulls_received` kódy** — fixture má event „Coldplay Arena
  Show", takže prefix je COLDPLAY a počítadlo je scoped naň (tri číslice).
  Vlastnosť pod testom sa nemenila: dva zápisy na jeden event idú v poradí.
- **oba `*_sheet_sync` testy** — `starts_with("ORD-")` / `starts_with("PULL-")`
  už nemohli prejsť. Testy sa teraz pýtajú na to, čo majú v názve: marker
  zapísaný do sheetu **je kód toho nového záznamu**, nech je aký chce.
- **migrácie 004 / 007 / 008 / 024** — ich vlastné komentáre tvrdia, že
  `run_migrations` aplikuje „ONLY 004". Dnes dotiahne súbor na 039 vrátane
  029, ktorej prepis zámerne zmení každý kód (`SAL-000001` → `OLD-001`).
  `code` je preto z porovnania vyňatý a namiesto toho sa overuje skutočný
  kontrakt 2.30.0: starý kód sa presunul do `legacy_code`, nezahodil sa —
  čo je to, čo drží už nasynchronizovaný Google Sheet funkčný. 024 navyše
  hľadal riadok podľa starého kódu, teraz podľa `id`.

### 3. Čo z tohto kola NIE je opravené (zámerne)

- `codes.rs` nemá ani jeden test. Pravidlo `prefix_for_event` rozhoduje
  o každom kóde v appke a musí byť na oboch strojoch identické, inak merge
  vidí falošné kolízie. Pokryté je dnes len nepriamo, cez migračné testy.
- Tie štyri migračné testy padali **pred** svojím koncom, takže tvrdenia za
  opraveným riadkom na tejto schéme nikdy nebežali.

### Overené / neoverené

Na tomto Macu nie je node ani cargo — **CI je prvý kompilátor**. Overené
staticky, pričom každá kontrola bola najprv spustená na známe pokazenom
súbore a musela padnúť na tom istom riadku ako kompilátor: vyváženie
zátvoriek (65/65 TS súborov, 7/7 editovaných Rust súborov), obe triedy
komentárovej chyby (0 výskytov), a súhlas arity tuple ↔ počtu stĺpcov
v SELECT ↔ počtu `r.get()` (0 nezhôd). **Nič z toho nie je kompilácia ani
beh testov.**

## 2.86.0 - Build fix: dve nezávislé chyby, obe z CI

### 1. TypeScript — moja chyba z 2.80.0, dvakrát

`{/* ... */}` som umiestnil po `{period === "custom" && (`, čo je
**JavaScript expression** pozícia, nie JSX children. Tam sa to parsuje ako
objektový literál. TypeScript to ohlásil ako *"JSX element 'Card' has no
corresponding closing tag"* vo všetkých troch Finance súboroch.

Je to presne tá istá trieda chyby, akú som si sám chytil v 2.71.0 (`//`
v zozname JSX atribútov) a vtedy komentár preto presunul. Tu som ju spravil
znova, len na druhú stranu.

**Moja kontrola zátvoriek to nemohla vidieť**, lebo `{/*` a `*/}` sú
vyvážené. To je tá diera v mojom overovaní.

### 2. A horšie: nastylovaný bol nesprávny element

Ten div je **podriadok s vlastnými dátumami vnútri existujúcej `Card`**, nie
samostatná lišta. Finance toolbary už v `Card` boli — na rozdiel od
Events/Orders/Sales, ktorých voľné divy tá lišta z 2.76–2.78 riešila.

Čiže 2.80.0 „zjednotilo" niečo, čo zjednotené nemalo byť. Riadok je **vrátený
presne do pôvodného tvaru** vo všetkých troch súboroch.

### 3. Rust — hniloba v testoch, ktorú našiel job z 2.73.0

`ScanJsPayload` dostal pole `summary` a `ScanJsCandidate` pole `incomplete`
(2.9.0); tri testovacie pomocníky v `price_checker_scanner.rs` ich nikdy
nedostali, takže `cargo test` nekompiloval.

**Toto nie je moja chyba — je to presne to, na čo som ten `rust-tests` job
v 2.73.0 pridal.** Na prvom behu našiel reálnu hnilobu, ktorú `tauri build`
vidieť nemôže (preskakuje `#[cfg(test)]`).

Doplnené `summary: ScanJsSummary::default()` a `incomplete: false`.
Preaudítované: **5/5 literálov `ScanJsPayload` a 3/3 `ScanJsCandidate`** majú
svoje polia.

### Čo som skúsil a vzdal

Napísať všeobecný kontrolór — na JSX tagy aj na Rust štruktúrne literály.
**Trikrát produkoval šum**: JSX sa regexom parsovať nedá (šípkové funkcie
v props obsahujú `>`), a v Ruste sa mi `impl X {` a `-> X {` miešali
s literálmi (239 z 569 falošných).

Nedodávam kontrolu, ktorej sám neverím. Na túto triedu chýb je kompilátor,
a ten v CI práve dvakrát odviedol presne tú prácu, ktorú mal.

## 2.85.0 - Globálny prečesávací prechod (fáza G)

marko poslal master prompt na redesign celej apky. Väčšina už bola hotová —
Dashboard (2.74/2.75), Events (2.76), Inventory (2.77), Sales (2.78), Pulls
(2.79), Finance (2.80-2.82), Notes (2.83), Settings (2.84) — a bod 1.1 toho
promptu výslovne hovorí **nezačínať odznova**:
*"Preserve previous work that is already correct... Extend and refine them
instead of accidentally reverting them."*

Čo hotové **nebolo**, boli dve veci: globálny shell (fáza B) a jednotný
prečesávací prechod naprieč celou apkou (fáza G). Toto je ten prechod.

### Tri posledné ručne kreslené segmentované ovládače

Sken celej apky našiel tri miesta, kde pilulkový pruh nepoužíval zdieľané
`SEGMENTED_TRACK` + `segmentedItemClass` — a jedno z nich bolo prekvapivé:

- **`Finance.tsx`** — hlavný štvorzáložkový pruh sekcie Finance
  (Overview/Transactions/Accounts/Reports). Jeho **vlastné podstránky**
  zdieľaný ovládač používali, len jeho navigácia nie.
- **`finance/Reports.tsx`** — výber obdobia
- **`finance/Accounts.tsx`** — prepínač Personal/Business v opakovanom výdavku

Vo všetkých troch bol aktívny stav už predtým byte-identický
(`bg-brand-600 text-white`); menia sa dráha, neaktívne farby a focus ring.
Pribudlo `aria-pressed`.

### Prázdne stavy

20 z 27 `EmptyState` malo ikonu, sedem nie. Doplnených **šesť**.

Siedmy — `EmptyRow` na Dashboarde — ikonu **zámerne nedostal** a je to
zapísané priamo v kóde: je to generický riadok, ktorý používa veľa kariet
s rôznymi vetami, takže akákoľvek jedna ikona by bola pre väčšinu z nich
nesprávna. Dokumentovaná výnimka, nie prehliadnutie.

Jeden z tých šiestich si zaslúži zmienku: `Recap.tsx` používa `EmptyState`
na zobrazenie **chyby**, nie neprítomnosti. Dostal výstražnú ikonu, aby to
bolo vidieť skôr, než sa veta prečíta.

### Globálny shell: nedotknutý, a zámerne

Bočný panel má zdokumentované iterácie z 2.6.0, 2.29.0 a 2.43.0 — aktívny
tint s akcentovým prúžkom, focus ring, uvážené rozmery pre 192px rail.
Je to jedna z najstarostlivejšie navrhnutých častí apky. Prerábať ho by
bola churn, nie zlepšenie.

## 2.84.0 - Settings: Data a Integrations prestavané do skupín

marko menoval Data a Integrations ako miesta, kde *"many actions and status
indicators currently compete for attention"*. Mal pravdu a audit ukázal
presne prečo.

### Integrations: štyri karty boli v skutočnosti dve integrácie

`GoogleSignInCard` + karta Pulls + karta Orders & Sales sú **jedna**
integrácia — jeden Google účet a dva hárky pod ním. Každá z nich pritom
ukazovala vlastný odznak „Connected", takže stránka čítala ako tri služby.
AI kľúč je genuínne druhá, nesúvisiaca.

Teraz sú to dve pomenované skupiny: **Google Sheets** (účet + dva hárky)
a **AI** (kľúč). Pridané boli len obaly; karty sú nedotknuté.

### Data: štyri karty bez toho, aby bolo vidieť, ktoré hryzú

CSV, Check my data, Sync a Backup & restore stáli v jednom nerozlíšenom
stĺpci. Dve z nich sú bežný nástroj, dve vedia prepísať celú databázu.

Teraz: **Everyday** (CSV, Check my data — *"Nothing here can overwrite your
data"*) a **Keeping your data safe** (Sync, Backup & restore — *"the one
action that can replace it"*).

Karty sa preusporiadali, ale ich vzájomné poradie v rámci skupiny ostalo —
Sync je stále priamo nad Backup & restore, ako to žiadal komentár z 2.12.0
(*"the same concern - moving this database between machines"*).

### Nová `SettingsGroup`

Nadpis, voliteľná veta pod ním, deti nezmenené. Lokálna v `Settings.tsx`.

### Zvyšných päť podsekcií sa nezmenilo — a je na to dôvod

Lookups, Notifications, Software, Account a Support sú **po jednej karte**
(alebo jednej komponente). Nemajú čo zoskupovať; vytvárať v nich skupiny by
znamenalo vymýšľať štruktúru tam, kde je jedna vec.

### Overenie presunu

Presúvalo sa ~600 riadkov v 4 352-riadkovom súbore. Overené: `<Card>` 17/17,
`<SettingsGroup>` 4/4, zátvorky 0/0/0, a **každý handler sedí 1:1**
(`setImportOpen`, `doDownloadTemplate`, `setExportConfig`, `runDataChecks`,
`cloudSyncPush`, „Backup database", „Restore from backup").

## 2.83.0 - Notes: štítok sa konečne zobrazuje

Audit Notes. Výsledok je ako pri Pulls — zadanie pýtalo veľa vecí, ktoré už
existujú.

### Čo audit zistil

**Nie je to contentEditable editor.** Žiadny `execCommand`, žiadne
`dangerouslySetInnerHTML`. Je to **blokový editor**: `NoteBlock[]`, jedna
textarea na blok, bloky `text`/`bullet`/`num`/`check`/`image`/`rule`
s veľkosťou (h1/h2/p/small), šiestimi farbami, bold, italic a polovičnou
šírkou. Formátovanie je teda blokové, nie inline HTML.

**Všetko toto už existuje a funguje:** pripínanie (aj s triedením
`ORDER BY pinned DESC` v backende), archív, duplikácia, šablóny, kontextové
menu, drag-and-drop preradenie, vyhľadávanie naprieč podstránkami, náhľad
obsahu v zozname, dátum poslednej úpravy, počet podstránok a obrázkov,
odkazy na objednávky/predaje, podstránky ako spodné záložky.

Bod 4 zadania navrhuje pripínanie, obľúbené, štítky, lepšie náhľady, rýchle
vytvorenie, duplikáciu a kontextové akcie. **Sedem z ôsmich už tam bolo.**

### Jediná skutočná medzera: štítok

`Note.tag` je v type, backend ho vyberá, editor ho píše pri `onBlur` —
a `Notes.tsx` ho **nerenderoval ani raz**. Napísal si štítok a zmizol.

Teraz je v riadku zoznamu, ako prvý v metadátach, lebo to je presne to, na
čo štítok je — rozoznať dve poznámky na prvý pohľad.

Pozn.: marko má dnes jednu poznámku a žiadny štítok, takže **zmenu zatiaľ
neuvidí**. Dátová cesta bola mŕtva a už nie je.

### Šípky preradenia

`↑` a `↓` boli holé znaky, ktoré sa vykresľovali v akejkoľvek veľkosti
a hrúbke, akú písmo zvolilo. Teraz `IconChevronUp`/`IconChevronDown`.

### Čoho som sa nedotkol

Editora. Bod 7 hovorí, že stabilita je priorita, a blokový editor s 1 295
riadkami, vlastným draft/dirty/save mechanizmom a `fieldRefs` je bez
kompilátora to posledné, čo sa oplatí prerábať kvôli vzhľadu.

## 2.82.0 - Finance dokončené: Transactions, Accounts, Reports, dialógy

Zvyšok sekcie Finance. Dashboard sa nedotkol.

### Audit najprv - a dvakrát ma zastavil

**Tabuľkové triedy.** Finance používa `.th`/`.td`, nie `.th-c`/`.td-c`. Skoro
som to „opravil" - a bola by to chyba: `.th`/`.td` je legitímna rozvoľnenejšia
konvencia (px-3 py-2.5), ktorú používa aj Dashboard, Settings a EventDetail.
Nechal som ju.

**Rozlíšenie príjmu a výdavku.** Zadanie žiada, aby nestálo len na farbe -
a už nestojí: každý riadok má odznak so slovom *aj* znamienko `+`/`−`.
Nemenil som to.

### Čo som naozaj zmenil

**Transactions**
- Suma výdavku je **rose**, nie neutrálna sivá. Príjem bol emerald a karta
  Cashflow Forecast na Overview používa pre výdavky rose od 2.1.0 - v jednej
  sekcii mala tá istá vec dve farby.
- Prepínače Type a Scope vo formulári na zdieľané `segmentedItemClass` -
  ten istý ťah, aký dostal Pulls v 2.79.0. `SEGMENTED_TRACK` sa tu **ne**použil
  zámerne: nesie `w-fit`, a tieto dva majú vyplniť bunku mriežky.
- Prázdny stav dostal ikonu.

**Reports**
- Pruhy v rozpade nosia farbu kategórie, rovnako ako na Overview.
- Prázdny stav dostal ikonu.

**Accounts**
- Prázdny stav opakovaných výdavkov dostal ikonu. Druhý ju už mal.

### `FormGroup` povýšený do `ui.tsx`

Žil ako **bitovo zhodná** kópia v `Orders.tsx` aj `Pulls.tsx`. Ich vlastný
komentár hovoril, že ostáva lokálny, lebo *"still only a handful of forms want
this kind of sectioning"*. Tretí záujemca mení tú kalkuláciu.

Markup je nezmenený, takže Orders aj Pulls renderujú presne to, čo predtým -
presunula sa definícia, nič sa nereštylovalo. Overené: jeden export, žiadna
lokálna kópia, oba súbory importujú, **9 použití stále platí**.

### Čo som NEVNÚTIL

`FormGroup` do transakčného formulára. Vnucuje `grid-cols-2`, kým ten
formulár má vlastné mriežky (`1fr 110px` pre sumu a menu). Vnútiť ho by
rozbilo fungujúce rozloženie kvôli zoskupeniu, ktoré ten formulár aj tak už
má cez `Field` a `space-y-3`.

Povýšenie tým nestráca zmysel - odstránilo duplikát - ale dôvod, ktorý ma
k nemu viedol, nevyšiel. Hovorím to rovno.

## 2.81.0 - Finance Overview dokončený (etapa 1)

marko mi dal stupňovanie, ktoré som sám navrhol: Overview, Transactions,
Accounts, Reports — a *"finish and verify one stage before starting the
next"*. Toto je **etapa 1**, Overview celý.

Aj férová kritika 2.80.0: *"Do not redesign only the top of the page and
leave the remaining content visually inconsistent."* Presne to som vtedy
spravil. Toto to dopĺňa.

### Expenses by category

**Pruh teraz nosí farbu svojej kategórie.** Predtým bol každý fialový, hoci
štvorček vedľa názvu mal vlastnú farbu — štvorček hovoril jedno, pruh druhé.
Nová exportovaná `financeCategoryFill` vracia **tú istú triedu**, akú používa
štvorček, takže sa nemôžu rozísť.

**Pribudli percentá**, ktoré zadanie žiadalo. Počítajú sa zo súčtu riadkov
toho zoznamu, nie z karty Expenses nad ním — zoznam je to, čoho sú tie
percentá podielom, a cudzí menovateľ by spôsobil, že by si na vlastnom
zozname nesadli do 100 %.

### Chyba, ktorú našlo pozeranie

Vykreslil som ten zoznam s markovými vlastnými kategóriami a dve z nich —
Zdravie (25,00 €) a Doprava (3,90 €) — ukazovali **„0%"**. To číta ako „nič",
hoci o stĺpec vedľa stojí reálna suma.

Riadok s nenulovou sumou, ktorý zaokrúhľuje pod jedno percento, teraz ukazuje
**„<1%"**.

### Cashflow Forecast

Štyri popisky (`<dt>`) prešli na `.section-title`, štýl malých nadpisov,
ktorý má zvyšok apky.

Odznak „Forecast" mal `bg-brand-50` a `uppercase tracking-wide`. Tá výplň je
presne tá sýta levanduľa, ktorú `StatCard` vo vlastnom komentári z 2.47.4
označil za chybnú v light mode (*"marko: na light mode je tam chybna farba"*),
a veľké písmená malé popisky stratili v 3.0. Teraz nesie tónovaný prstenec,
rovnaký idiom ako `METRIC_ACCENTS`, ktorý funguje na oboch podkladoch.

### Nedotknuté

Žiadny výpočet, filter ani dátový tok. Graf z 2.80.0 funguje ďalej.
`Dashboard.tsx` aj `DashboardCharts.tsx` sú stále nedotknuté.

## 2.80.0 - Finance: Dashboard cashflow graf znovupoužitý

Priorita č. 1 zo zadania: *"the Finance chart must use the same chart design
and visual implementation as the Cashflow chart in Dashboard"*.

### Čo audit našiel

Finance graf **už znovupoužíval Dashboard komponentu** — ale tú staršiu.
2.45.0 na markovu vlastnú žiadosť (*"vo finance ako je income vs expense
widget tak urobit taky isty graf ako ktory je na dashboarde"*) postavil
Finance graf na `MetricChart`.

Lenže Dashboard medzitým dostal druhý graf: `CashflowChart` v 2.74.0 — dve
plynulé krivky, monotónna interpolácia, gradientové výplne. **To je ten,
ktorý marko myslí.**

### Riešenie je symetrické

Karta grafu vo Finance teraz nesie **oboje**, presne ako Dashboard:

- pilulka **Cashflow**, predvolená, vykreslí `CashflowChart`
- tri pilulky **Income / Expenses / Net** na `MetricChart`, nedotknuté

Je to **skutočná komponenta**, importovaná z `DashboardCharts`, nie napodobenina
v CSS. Jej API je od 2.74.0 dátovo agnostické (`points` + `formatValue`
vlastný volajúcemu), takže **nebolo treba Dashboard vôbec refaktorovať** —
ani `Dashboard.tsx`, ani `DashboardCharts.tsx` sa nezmenili.

Adaptér je triviálny: jeden `SeriesBucket` → jeden `CashflowPoint`. Nič sa
neprepočítava — `incomeCents`/`expenseCents` sú tie isté hodnoty, aké kreslí
metrický graf pod tým, a `bucketLabel` je Dashboardov vlastný exportovaný
formátovač, takže oba grafy píšu mesiac rovnako.

Nadpis v režime Cashflow ukazuje **net za obdobie** — tú istú hodnotu, akú
má karta „Net Cash Flow" nad ním, podaná zhora, nie sčítaná z vykreslených
bucketov (`buildSeries` strope „All time" na 24 mesiacov, takže súčet
bucketov by s kartami nesedel).

### Karty a filtre

Päť `StatCard`-ov vo Finance Overview dostalo ikony a akcenty: Current
Balance emerald, Owed to you orange, Income blue, Expenses orange, Net Cash
Flow teal.

Tri filtračné lišty (Overview, Transactions, Reports) do vnorenej lišty.
Po tomto vydaní má **osem líšt naprieč siedmimi súbormi** rovnaký tvar.

### Čo už bolo v poriadku

Finance bolo na tom lepšie než Inventory a Sales boli. `StatCard` už
používalo, ručne kreslené karty tam neboli žiadne, zastarané popisky tiež nie
(okrem nadpisu grafu, ktorý zámerne kopíruje Dashboard).

## 2.79.0 - Pulls zarovnané so zvyškom

Redesign sekcie Pulls. Žiadna iná sekcia sa nedotkla a **žiadna zdieľaná
komponenta sa nemenila** — takže nebolo čo kontrolovať na regresiu inde.

### Čo obhliadka našla — menej práce, než zadanie čakalo

Zadanie pýtalo zoskupiť polia formulárov a zarovnať detail stránku. Ani jedno
nebolo treba:

- **`FormGroup` sa už používa** v oboch modáloch, päť skupín: „Pull",
  „Seats & details", „Fee & tracking", „Pull", „Fee & link". Presne to
  zoskupenie, aké bod 6 žiadal.
- **Pulls nemá detail stránku.** Žiadna `/pulls/:id` neexistuje —
  `PullFormModal` sa otvára priamo zo zoznamu a pull nemá žiadne podradené
  entity. Bod 7 teda odpadá.
- **Zastarané popisky s `uppercase` tam nie sú ani jedny** (0 výskytov),
  na rozdiel od Inventory a Sales.

### Prepínač Given / Received

Bol ručne kreslený — vlastný rám, vlastné veľkosti — kým zvyšok apky
segmentuje cez `SEGMENTED_TRACK` + `segmentedItemClass` (Dashboard taby,
prepínač grafu, `TabSwitcher`).

Prevedený na zdieľané tokeny. **Aktívny stav bol už predtým byte-identický**
(`bg-brand-600 text-white`), takže sa menila len dráha a neaktívne/focus
štýlovanie. Tie isté dve tlačidlá, ten istý handler, ten istý stav. Pribudlo
`aria-pressed`.

### Filtre

Obe vetvy — Given aj Received — dostali tú istú vnorenú lištu. Po tomto
vydaní má **päť filtračných líšt naprieč štyrmi súbormi** rovnaký tvar:
Events 1, Orders 1, Sales 1, Pulls 2.

### Zachované

Rozdiely medzi Given a Received (iné polia, filtre aj stĺpce) ostali
nedotknuté — zadanie výslovne hovorilo ich nezjednocovať. Rovnako stavy,
výpočty poplatkov, pravidlá dokončenia, výber riadkov aj hromadné mazanie.

## 2.78.0 - Sales zarovnané s Inventory a Events

Redesign sekcie Sales. Žiadna iná sekcia sa nedotkla.

Zadanie bolo jasné: *"Do not create a new design language or give Sales a
different layout philosophy"*. Takže žiadne nové komponenty — len dotiahnutie
Sales k tomu, čo Inventory a Events už majú.

### Päť ručných kariet na `StatCard`

`SaleDetail` kreslil Revenue, Fees, Cost, Profit a ROI ručne cez `<Card>`
s vlastným popiskom a hodnotou — presne to isté, čo mal Inventory pred
2.77.0. Prevedené na `StatCard`.

Profit si cez `tone` drží presne to isté emerald/red/neutral pravidlo.
`formatMoneyOrMixed` ostalo, takže „Mixed" pri predaji cez viac mien sa
stále zobrazí. **Žiadna hodnota, formát ani podmienka sa nezmenila.**

Akcenty podľa zadania: revenue blue, fees a cost orange, profit emerald,
ROI teal.

### Informačný panel

Šesť popiskov (Platform, Sale date, Currency, Refunded, Paid, Outstanding)
zjednotených na `.section-title`. Používali `uppercase`, ktoré tento štýl
stratil už v 3.0 — rovnaká oprava, akú dostal Inventory panel v 2.77.0.

### Filtre

Do jednej vnorenej lišty. Po tomto vydaní majú **všetky tri zoznamy** —
Events, Inventory, Sales — tú istú lištu (overené: jeden výskyt v každom).

### Čo už bolo v poriadku

Súhrnný pruh výsledkov (Results, Tickets, Revenue, Profit, Paid,
Outstanding, Refunded) už používal `SummaryStat`, čo je presne tá kompaktná
súhrnná komponenta, akú zadanie žiadalo. Nedotknutý.

## 2.77.0 - Inventory: dvanásť ručných kariet na StatCard

Redesign sekcie Inventory. Žiadna iná sekcia sa nedotkla.

### Hlavný nález

`OrderDetail` si dvanásť metrických kariet kreslil **ručne** cez `<Card>`
s vlastným popiskom a hodnotou, kým Dashboard aj Event Detail používajú
`StatCard`. To je zdroj tej nekonzistencie, ktorú bolo vidieť.

Všetkých dvanásť prevedených na `StatCard`. Jedna komponenta znamená jedny
odsadenia, jednu typografiu a jedno pravidlo orezávania - metrická karta
vyzerá rovnako všade.

**Žiadna hodnota, formát ani podmienka sa nezmenila.** Vrátane „Partial",
ktoré sa stále zobrazí presne vtedy, keď sú `orderPaidCents` /
`orderOutstandingCents` null (model objednávky nemá pole, ktoré by nieslo
skutočné číslo - viď audit z 1.9.0), a stále jantárovo.

### `StatCard` dostal tón `warning`

Kvôli tomu „Partial". Čisto aditívne - žiadny existujúci volajúci ho
neposiela, a `SummaryStat` si svoju trojhodnotovú úniu ponechal nedotknutú.

### Akcenty podľa významu, nie pre pestrosť

Hlavička objednávky: violet pre počet, blue pre vstupy na kus, orange pre
náklad, emerald pre skutočne vybrané peniaze.

Order summary: tri karty s počtami violet, Purchase cost orange, Sales
revenue blue, Realized profit teal - a ten si cez `tone` drží presne to isté
emerald/red/neutral pravidlo, aké mal predtým.

### Zoznam objednávok

Šesť filtrov (Search, Category, Platform, From, To, Sort) v jednej vnorenej
lište, rovnako ako Events v 2.76.0, nech sa oba zoznamy čítajú rovnako.
Tie isté vstupy, handlery aj šírky.

### Panel Platform / Notes / Currency / Finance

Štyri popisky zjednotené na `.section-title`. Používali `uppercase`, ktoré
tento štýl stratil už v 3.0 - boli to posledné štyri veľké písmená v tom
paneli. Dva ďalšie výskyty mimo panelu (riadok detailu, pole v modáli)
ostali zámerne nedotknuté, lebo tento prechod je zúžený na Inventory panel.

### Čo som NEPRIDAL

Súhrnný pruh nad tabuľkou z inšpiračného obrázka. Znamenal by počítať nové
agregáty, čo je funkcia, nie vzhľad - a zadanie hovorí neduplikovať
informácie a nevymýšľať funkcionalitu.

## 2.76.0 - Events: doladenie, Stock stĺpec preč

Redesign sekcie Events. Žiadna iná sekcia sa nedotkla.

marko v priebehu práce: *"pamataj ze vsetko robime v podobnom designe, toto su
len navrhy a inspiracia"* - takže referenčné screenshoty slúžili ako smer, nie
ako predloha, a všetko ostalo v jazyku apky (existujúce tokeny, zdieľané
komponenty).

### Stock stĺpec odstránený

marko: *"ten stock cely odstran"*. Stĺpec aj komponenta `StockBar` sú preč.

Nič sa pre ten stĺpec nepočítalo zvlášť - čítal `stats.purchasedTickets` a
`stats.availableTickets`, oboje backend naďalej posiela a detail eventu
naďalej zobrazuje. Žiadny dotaz, typ ani výpočet sa nemenil.

Hlavička aj riadok majú po odstránení **8 buniek** (overené počítaním), a
v tejto tabuľke nič nepočíta stĺpce podľa indexu - žiadny `colSpan` ani
`rightAlign`, takže sa nemalo čo rozsypať.

### Zoznam eventov

Filtre (Search, Category, From, To, Sort) sú v jednej vnorenej lište
(`surface-sunken`) namiesto piatich voľne stojacich ovládačov. Tie isté
vstupy, tie isté handlery - lišta len spôsobí, že sa čítajú ako jedna skupina.
Prepínač Upcoming/Completed ostáva pripnutý vpravo.

### Detail eventu

Desať `StatCard` dostalo ikony a akcenty cez voliteľné props z 2.74.0.
Pravidlo: **tri karty s počtami nosia brand violet, peňažné berú mapovanie
z Dashboardu** (cost orange, revenue blue, profit teal, ROI emerald). Jeden
pohľad oddelí "koľko lístkov" od "koľko peňazí".

Trojica Potential Profit nosí tie isté tri akcenty ako jej dvojička na
Dashboarde, takže odhad vyzerá rovnako na oboch obrazovkách.

Štyri prázdne stavy dostali ikonu. `EmptyState` ikonu aj akciu podporoval už
predtým - Events ich len neposielalo, takže zdieľaná komponenta sa nemenila.

### Pozn. k overovaniu

Pri kontrole `StockBar` v prehliadači vyzeralo, že výplň pruhu nie je vidieť.
**Chyba bola v mojej testovacej stránke**, nie v apke: zabudol som na
`display:block` na vnútornom pruhu, ktorý appka má (`block h-full`). Bez neho
má inline span nulovú výšku. Píšem to sem, lebo inak by som ohlásil chybu,
ktorá neexistovala - a stĺpec aj tak medzitým padol.

## 2.75.0 - Dashboard dokončený: platformy, Financials, Activity

Dokončenie redesignu Dashboardu. Events sa nedotklo, podľa zadania.

### Sales by Platform zapojené bez straty

`PlatformDonut` z 2.74.0 nahradený **`PlatformRing`** - samotný prstenec, bez
vlastnej hlavičky a legendy. Pridaný **nad** existujúci zoznam v
`SalesByPlatformCard`, nie namiesto neho: ten zoznam je jeho legenda, už
menuje každú platformu s tržbou aj počtom lístkov a už sa rozbaľuje.

Riadky zoznamu dostali bodku a pruh vo farbe svojho výseku plus percento.
Obe strany berú farbu z `platformFill(index)`, takže sa nemôžu rozísť;
backend už triedi podľa tržby zostupne (má na to test).

### Chyba, ktorú našlo pozeranie: zmizli sumy

2.74.0 dalo hodnotu a sparkline na jeden riadok, ako referencia. Sparkline má
pevných 64px, hodnota `min-w-0 truncate` - v úzkej karte sa hodnota orezala
**na nič**. Zmerané (`scrollWidth > clientWidth`):

```
orezany text: ANO - €3,539.61 | €7,473.78 | €3,934.17 | 41
```

Sparkline presunutý na vlastný riadok pod hodnotou, cez celú šírku. Po
oprave pri najužšej karte, akú `.summary-bar` dovolí (152px): bez orezania.

Pozn.: prvý beh testu používal `minmax(0,1fr)` namiesto skutočného
`minmax(9rem,1fr)` z `index.css`, čiže staval karty užšie než apka kedy
postaví. Test opravený na reálne pravidlo.

### Financials

**Bar chart "Revenue vs Purchase Cost" v apke neexistuje** - tri nezávislé
vyhľadávania. Je len na referenčnom screenshote. Nič sa teda neodstraňovalo.

Doplnené ikony a akcenty ôsmim peňažným kartám. Štyri karty s počtami ostali
bez ikon zámerne - zadanie žiada striedme farebné kódovanie, nie farbu všade.

### Activity

`AlertCard` dostal voliteľnú ikonu. Podklad jantárový pri nenulovom počte,
neutrálny pri nule - rovnaké pravidlo, aké má už samotné číslo. Karta s nulou
je dobrá správa a nemá vyzerať ako výstraha.

### Neoverené

Node ani cargo tu nie sú: **žiadny build, lint, type-check ani testy, appka
nespustená.** Overené sú komponenty vykreslené v prehliadači na reálnych
dátach, nie zložený Dashboard.

## 2.74.0 - redesign, fáza 3-4: KPI karty a Balance Cashflow

marko dodal master prompt na kompletný vizuálny redesign a referenčné
screenshoty. Toto je prvý skutočný kus: zdieľané tokeny, KPI riadok a hlavný
graf. Fázy 5-9 (Financials, Activity, ostatné sekcie) nasledujú.

### Čo sa ukázalo pri analýze

Dashboard **už má** Overview/Financials/Activity (perzistované v
`app_settings`), segmentovaný prepínač, `StatCard`, `SalesByPlatformCard`
aj rozloženie "graf + platformy vedľa". A `DashboardData` už nesie
`cashflow`, `revenueTimeSeries`, `salesByPlatform` aj `previousPeriod`.

Čiže všetko, čo referencia žiada, **už existuje ako dáta a ako štruktúra**.
Je to naozaj vizuálna práca, presne ako marko napísal.

### StatCard: ikona, akcent, sparkline

Tri **voliteľné** props. Každý existujúci volajúci (Event Detail, Sales,
Pulls, Finance) renderuje presne to, čo predtým - pridáva ich len Overview.

Akcenty: teal/blue/orange/emerald + `violet`, ktorý ukazuje na **brand ramp**,
nie na Tailwindov vlastný violet. Tú rampu si marko vybral sám z živého
náhľadu a `PROTECTED_AREAS` hovorí nesiahať na ňu bez opýtania - tak ju
redesign používa namiesto toho, aby vedľa nej postavil druhú fialovú.

Sparkliny sú **reálne**: každá je stĺpec z `revenueTimeSeries`. ROI žiadnu
nemá - na tom rade per-bucket ROI nie je a vymyslená čiara je presne to, čo
táto apka robiť nesmie.

### Balance Cashflow

Nová komponenta v `DashboardCharts.tsx`, ručné SVG (projekt nemá grafovú
knižnicu a pridávať ju pre dva grafy sa neoplatí - rovnaký argument má
`MetricChart` vo vlastnom komentári).

Income = `revenueCents` z bucketu. Expenses = `cogsCents + sellingFeesCents`.
Oboje z rovnakých dát, aké kreslí Revenue karta nad tým. Headline berie
`data.period.profitCents`, nie prepočet z bucketov, takže graf a KPI karta
sa nemôžu rozísť.

**Pridané ako ŠTVRTÁ možnosť** na existujúci prepínač metrík, nie namiesto
neho. Revenue/Profit/Cost sú reálna funkcia od 1.7.5 a redesign ich nemazhe
kvôli screenshotu. Cashflow je východzí, ostatné sú jeden klik ďaleko.

### Chyba, ktorú našlo až pozeranie

Prvá verzia používala Catmull-Rom interpoláciu. Na markových vlastných
štyroch mesiacoch (Apr 700, Aug 2 494, Sep 3 771, Oct 509) **vrchol krivky
pristál medzi augustom a septembrom**, nie na septembri - prudký pád do
októbra ťahá predchádzajúci kontrolný bod. Kto by z toho grafu čítal najvyšší
mesiac, dostal by zlú odpoveď.

Nahradené **Fritsch-Carlson monotónnou interpoláciou**: kde dáta rastú,
krivka rastie, a nikdy neopustí interval medzi dvoma susednými hodnotami.

Overené meraním na **skutočne vykreslenej ceste** (`getPointAtLength` nad
400 vzorkami): najvyšší mesiac na x=513.3, vrchol krivky na x=512.8,
odchýlka 0.5px. Plus: žiadne NaN, všetky Y v ploche, segmenty donutu v
súčte presne obvod, percentá 100 %.

Pri tom istom pohľade sa ukázalo aj to, že osové popisky 9px vo viewBoxe
760px sa renderujú ako ~5px. Zväčšené na 11px, vodiacich čiar 5 -> 3.

### PlatformDonut

Postavený a overený (matematika dasharray sedí), ale **zatiaľ nezapojený**.
`SalesByPlatformCard` má rozbaľovanie, ktoré by sa výmenou stratilo - to je
na ďalší krok, nie na tichú stratu funkcie.

## 2.73.0 - sync dokončený, testy v CI, "Check my data"

marko si vybral všetky tri veci, ktoré som navrhol.

### 1. `decide_auto` vracia Merge, nie Pull

Vetva `(false, true)` - "druhý stroj má novšie dáta a tento nemá nič
neodoslané" - vracia **Merge**.

Presne táto vetva ho 1.10.2026 pripravila o tri objednávky, šesť lístkov a
dva predaje: `local_dirty` tvrdil `false`, hoci deň práce čakal neodoslaný,
a Pull prepísal celý súbor staršou kópiou z Drive.

Merge to spraviť nemôže. Aplikuje **tie isté** tombstony, ktoré by niesol
Pullov súbor, takže skutočné zmazanie na druhom stroji stále dorazí - ale
nechá riadky, ktoré táto strana má a druhá nikdy nevidela. Keď je príznak
**správny**, sú obe rovnocenné: bez neodoslaných zmien niet lokálnych riadkov,
ktoré by Merge zachovával, takže dôjde k tej istej databáze.

`decide_auto` už `Pull` nevracia vôbec. Variant ostáva pre ručné "Sync down".

Príčina (prečo je príznak nepravdivý) je **stále otvorená** v `KNOWN_BUGS.md`.
2.68.0 a 2.73.0 spolu dosiahli to, že jeho nepravdivosť je prežiteľná, nie
fatálna.

### 2. `cargo test` v CI

Nový job `rust-tests` v `release.yml`. Tieto testy existovali celý čas a nikdy
ich nikto nespustil: `tauri build` kompiluje bez `#[cfg(test)]`, takže testový
modul, ktorý sa ani nepreloží, vyzerá ako zelený build - a vyzeral, štyri
vydania.

Beží na macOS, nie na ubuntu: Tauri potrebuje na Linuxe webkit2gtk a kontrola,
ktorá padne na chýbajúcom systémovom balíku, nikoho nič nenaučí.

**Zámerne zatiaľ nie je podmienkou `release`.** Tie testy nikto nevidel prejsť
mesiace, takže spraviť z nich bránu pred prvým zeleným behom by riskovalo, že
markovi zablokujem vydanie kvôli niečomu, na čo sa ani jeden z nás nepozrel.
Keď job zozelenie, pridá sa `needs: rust-tests`.

Frontend vlastný job nepotrebuje: `npm run build` je `tsc -b && vite build`.

### 3. "Check my data" (Settings -> Data)

Deväť integritných kontrol, ktoré som tento týždeň robil ručne, ako tlačidlo.
Nový modul `commands/data_health.rs`.

Obsahuje presne tie dve, ktoré našli reálny problém: mena, ktorá nie je
trojpísmenový kód (`US $`, kvôli nej nefungoval convert), a zaplatená
objednávka s nulovou cenou (NFL-001/002/003, šesť lístkov).

**Nikdy nič nemení.** Každá kontrola je `SELECT`. Opraviť ktorúkoľvek znamená
rozhodnúť, aká hodnota je tá správna, a to je markovo rozhodnutie o jeho
peniazoch, nie pravidlo pre tlačidlo. Žiadne "oprav všetko".

Kontrola, ktorá nič nenašla, sa vracia tiež (`count: 0`), a panel to povie
nahlas - "pozrel som a nič tam nie je" je odpoveď, ktorú potrebuje najčastejšie,
a prázdny panel ju dať nevie.

**Overené naživo:** všetkých deväť dotazov som pustil proti kópii jeho
databázy. Bežia bez chyby a vrátia 2 (`YELIVE-001 (US $)`) a 3
(`NFL-001,NFL-002,NFL-003`); zvyšných sedem nula. Plus tri unit testy vrátane
jedného, ktorý overuje, že všetkých deväť SQL príkazov vôbec zbehne nad
zmigrovanou schémou - presne ten druh preklepu, ktorý by inak objavil až
marko stlačením tlačidla.

## 2.72.0 - restrictions po anglicky, sale picker len na budúce eventy

marko: *"restrictions by mali byt napisane v anglictine, a mali by obsahovat
len zakladne restrictions ake mozu byt"* a *"pri sales by sa mali ukazovat len
tie events a orders ktore este len budu"*.

### Restrictions

Celý zoznam prepísaný do angličtiny a skrátený na to, čo reálne nastáva:

| kód | label | tón |
|---|---|---|
| `rv` | Restricted view | warn |
| `18` | 18+ | warn |
| `16` | 16+ | warn |
| `id` | ID required | warn |
| `aw` | **Away fans only** (nové) | warn |
| `nr` | No re-entry | info |
| `st` | Standing | neutral |
| `wc` | Wheelchair space | neutral |

**Bezpečné prepísať naraz, lebo to bolo overené:** v databáze nemal ani jeden
lístok nastavené restriction (0 riadkov s neprázdnym `restrictions_json`),
takže vypadnutím kódu sa nič neosirotilo. `clean_restrictions` v Ruste nemá
vlastný allow-list - len trimuje, zmenšuje a dedupuje - takže zoznam žije na
jednom mieste a je to čisto frontendová zmena.

`ao` (maloletý s dospelým) vypadol: 16+/18+ už vekovú otázku nesú a marko
pýtal základný zoznam, nie úplný. `aw` je nový a je to tá futbalová, ktorá na
resale reálne bolí - miesto v sektore hostí predané domácemu fanúšikovi je
refund a otočený kupec.

Popisky polí `"Obmedzenia"` -> `"Restrictions"` v oboch editoroch.

### Sale picker: len to, čo sa ešte neodohralo

`SaleRowsModal` filtruje zoznam objednávok na eventy s dátumom dnes alebo
neskôr. Dve veci sa **zámerne neskrývajú**:

- Objednávka **bez dátumu** (TBD event) nemá dátum, ktorý by mohol byť
  v minulosti - ostáva. Nikdy neskrývať niečo preto, že údaj chýba.
- Keď marko **napíše hľadanie**, zobrazia sa zhody bez ohľadu na dátum.
  Skryť to, na čo sa práve spýtal menom, by bolo presne tá nepriehľadnosť,
  ktorú hlási - a je to zároveň jediná cesta, ako zapísať oneskorený predaj,
  ktorý by inak filter znemožnil.

Záchrana predvolenej objednávky (tlačidlo "Add sale" na detaile objednávky)
hľadá v **nefiltrovanom** výsledku, takže funguje aj pre event, ktorý sa už
odohral.

Porovnanie dátumov je textové - obe strany sú ISO `YYYY-MM-DD`, ktoré ako text
triedi v poradí dátumov.

## 2.71.0 - restrictions ako dropdown + oprava konverzie mien

marko: *"tie restrictions su nepriehladne strasne ... chcem, aby ked nato
klikneš tak sa ti ukaze taky vyber ako je pri platforme"* a *"taktiez convert
mien nefunguje"*.

### Restrictions: osem písmen -> dropdown so slovami

Nová komponenta **`MultiSelect`** (`ui.tsx`) - `Select` pre hodnotu, ktorá je
zoznam. Zatvorená vyzerá ako Platform vedľa nej a číta sa "Výhľad, 18+";
otvorená vypíše všetkých osem **plným slovenským názvom** s fajkou na
vybraných. Sú to tie isté názvy, ktoré ukazuje per-lístkový editor v
`Tickets.tsx`, takže obe obrazovky nemôžu rozísť.

Natívny `<select multiple>` by to nevyriešil - renderuje sa ako trvalo
otvorený list box, ktorý sa do bunky tabuľky nezmestí.

Panel ide cez **portál**. `.table-shell` je `overflow-auto` (index.css), takže
panel umiestnený v bunke by scroll kontajner odstrihol, hneď ako je vyšší než
riadok - čo s ôsmimi možnosťami je vždy. Scroll a resize panel zatvárajú,
nepresúvajú ho.

Poradie ostalo: štyri, ktoré stoja peniaze, sú stále prvé. `RESTRICTION_GLYPH`
zmizol, existoval len pre tie prepínače.

### Konverzia mien: `US $` nie je mena

404 z kurzovej služby nebol výpadok. **Objednávka YELIVE-001 má menu uloženú
ako `US $`**, nie `USD` - AI import to tak prečítal zo screenshotu a
`fx::normalize_currency` ten tvar nepoznala, takže ho `to_uppercase()` nechal
tak a poslal sa do URL. Overené naživo proti službe:

| dotaz | odpoveď |
|---|---|
| `from=GBP&to=EUR` | 200, kurz 1.1814 |
| `from=US $&to=EUR` | **404 `{"message":"not found"}`** |
| `from=EUR&to=EUR` | 422 `{"message":"bad currency pair"}` |

Čiže endpoint aj sieť boli v poriadku; chybná bola tá jedna hodnota.

**Dve opravy v `fx.rs`:**

1. `normalize_currency` pozná symbol s predponou krajiny - `US$`, `A$`, `C$`,
   `NZ$`, `HK$`, `S$`, `R$`, a to aj s medzerou vnútri (medzery sa najprv
   vyhodia). Neznámy kód stále prepadne na obyčajný veľký tvar, ako predtým.
2. Kód, ktorý nemá tri ASCII písmená, sa **odmietne pred odoslaním** a chyba
   pomenuje konkrétnu hodnotu, namiesto preposlania holého 404. Kontroluje sa
   PRED skratkou `from == to`, inak by dve kópie tej istej nepoužiteľnej
   hodnoty ticho vrátili kurz 1.0.

**Jeho existujúci riadok to neopraví** - normalizácia beží na vstupe, uložená
hodnota ostáva. Opravuje si ho sám, dáta sú jeho.

### Kontrola dát (marko: *"skus pokukat vsetky chyby ktore vies najst"*)

Pätnásť integritných dotazov nad jeho živou databázou. Čisté: meny lístkov vs
objednávok, quantity vs počet lístkov, osirelé predaje, `sold` bez predaja,
`available` s predajom, záporné sumy, duplicitné kódy, objednávky bez eventu,
`restrictions_json` (platnosť aj neznáme kódy), statusy.

Jediný nález je tá mena vyššie. Falošný poplach: osem `batch_id` tvaru
`SAL-000xxx`, ktoré nezodpovedajú žiadnemu kódu predaja - to je stav z čias
pred 2.30.0, keď sa kódy prečíslovali na event-ové a `batch_id` ostalo.
`GROUP_BASE_SELECT` s tým vedome počíta (zobrazovaný kód berie z `MIN(id)`
riadku), zoskupovanie funguje správne - overené.

## 2.70.0 - build fix: chýbajúce `restrictions` v piatich literáloch

CI (Windows aj Mac) zastavilo build 2.69.0 na mojej chybe z 2.67.0:

```
error[E0063]: missing field `restrictions` in initializer of `OrderInput`
  --> src/commands/csv_import.rs:250
  --> src/commands/orders_sheet_sync.rs:1015
```

2.67.0 pridalo `OrderInput.restrictions` a doplnilo ho do 12 literálov.
Literálov je **18**. Dva zostali a kompilátor ich našiel.

**Doplnené `restrictions: None`** - oba zdroje žiadne obmedzenia nenesú,
takže `None` je pravda, nie výplň:

- `csv_import.rs:250` - import nemá stĺpec pre restrictions.
- `orders_sheet_sync.rs:1015` - hárok ho tiež nemá; rovnaký precedens ako
  `tier: None` priamo nad tým.

**Plus tri, ktoré CI nemohlo vidieť.** `tauri build` preskakuje
`#[cfg(test)]`, takže tieto by padli až na `cargo test`:

- `sales.rs:1502` - tiež `OrderInput`, tiež z 2.67.0, moja chyba.
- `tickets.rs:893` a `tickets.rs:924` - `TicketUpdateInput`, chýbajúce od
  **2.60.0**. `cargo test` teda nekompiloval štyri verzie, a žiadny release
  build to nemohol povedať. (`:936`/`:937` sú v poriadku, používajú
  `..blank()`.)

**Ako viem, že už nič nechýba.** Namiesto ručného prechádzania som si napísal
audit: nájdi v `src/` každý literál každej štruktúry, ktorá v `models.rs`
deklaruje `restrictions` (`OrderInput`, `Ticket`, `TicketUpdateInput`),
a over, či ho má - alebo či používa `..` syntax. Výsledok po oprave:
**0 chýbajúcich z 18 + 2 + 5**. To je kontrola, ktorú som mal spraviť v
2.67.0 namiesto počítania po pamäti.

## 2.69.0 - "Record 2 sales" klamalo, je to jeden predaj

marko: *"ked tam mas nejaku order, kliknes na nu a das add its tickets tak sa
to rozdeli na 2 sales, ale v skutocnosti to je len jeden"*.

**Dáta boli celý čas správne.** Overené, nie predpokladané:

- `SaleRowsModal` posiela všetky riadky jedným volaním `createSalesBatch`
  (jedno `lines: rows.map(...)`), nie jedno volanie na riadok.
- `create_sales_batch` dá celej dávke **spoločný `batch_id`** (kód prvého
  riadku), a to len vtedy, keď má viac než jeden riadok - jeden lístok ostáva
  `NULL`, čiže samostatný predaj. Presne to, čo marko chcel.
- `GROUP_KEY_EXPR` = `COALESCE(s.batch_id, 'single:' || s.id)` zoskupuje to
  späť **v Sales liste, na dashboarde, v kalendári aj v CSV exporte**.
- V jeho vlastnej databáze: `OASIS-001 | 2 | OASIS-001,OASIS-002` - jedna
  skupina, dva riadky. Tiež `UEFA-001 | 2`, `ENGLAND-001 | 8`,
  `SAL-000145 | 4`. Samostatné predaje (`CELINE-003`, `CELINE-004`,
  `CELINE-009`) majú `batch_id` NULL.
- `OrderDetail` ukazuje len súhrn (revenue/profit), žiadny zoznam riadkov.

Čiže nikde v apke sa to nerozdelilo na dva predaje. Jediné, čo to tvrdilo,
bolo **tlačidlo v tom okne**.

**Zmenené** (`src/pages/SaleRowsModal.tsx`, dva reťazce):

- `submitLabel` už nie je `Record ${rows.length} sales`, ale natvrdo
  **`Record sale`**. Tento formulár nikdy nevytvoril N predajov - vždy jeden.
  Modal nemá edit režim a volá sa z jediného miesta (`Sales.tsx:996`), takže
  "vždy jeden" platí bez výnimky.
- Súhrn dole začína **`One sale · `** - fakt, ktorý marko potreboval, je
  povedaný presne v momente potvrdzovania.

**Nedotknuté:** `batch_id`, `create_sales_batch`, `GROUP_KEY_EXPR`, schéma,
peniaze. Toto bola chyba v texte, nie v modeli, a opravil sa text.

## 2.68.0 - automatický Pull a Merge sa pýtajú

marko: *"ten sync nefunguje spravne, stracam udaje"* -> *"oprav to aby sa to
uz nestalo"*.

**Čo sa stalo.** 1.10.2026 o 16:47 automatický sync stiahol z Drive staršiu
kópiu a prepísal ňou celú lokálnu databázu. Preč boli 3 objednávky
(ESPAA-002/003/004), 6 lístkov a 2 predaje (OASIS-001/002, 640 €). Doložené:
`deleted_rows` sa cez tú stratu nezmenil (79->87 bolo o dva dni skôr, iná
udalosť) a živá databáza bola striktná podmnožina zálohy - podpis prepisu
súboru, nie mazania riadkov. V Settings boli **štyri** body obnovy s názvom
"Before Sync Down".

**Prečo.** Záloha `pre-restore-20261001-164651520.sqlite3` má zapísané
`cloud_sync_local_dirty = false` a `cloud_sync_last_sync_at = 2026-09-30
13:23`, pričom tie objednávky vznikli 30.09. o 14:36 - hodinu PO poslednom
syncu. Príznak, ktorý má presne tomuto brániť, bol v tej chvíli nepravdivý,
`decide_auto` z neho vyrobil `(false, true) -> Pull`, a `Layout.tsx` ten Pull
vykonal bez otázky.

**Oprava.** `Layout.tsx`: vetvy `pull` a `merge` v päťminútovom ticku už
nevykonávajú nič. Prácu zaparkujú do `pendingRunRef` - telo je nezmenené,
len o jeden `async () =>` hlbšie - a otvoria `ConfirmDialog`. Push ostáva
automatický: iba odosiela a vlastnú poistku proti prepísaniu druhého počítača
už má.

Ďalej: "Nie" sa pamätá na celú session (`syncDeclinedRef`), takže sa otázka
neopakuje každých 5 minút - namiesto toho ostane banner. História syncu
zapisuje `"asked"`, nie `"pull"`, keď sa len pýtalo; `plan.action` by tvrdil
stiahnutie, ktoré sa nestalo.

Opravený aj komentár z 2.16.0, ktorý tvrdil, že merge *"nothing is deleted"*.
Nebola to pravda - `totalDeleted` priamo pod ním ráta riadky, ktoré merge
zmaže podľa tombstonov z druhého stroja. Dialóg to teraz hovorí nahlas.

**Neopravené, vedome.** Prečo bol `local_dirty` nepravdivý, ostáva otvorené
(`KNOWN_BUGS.md`). Správna oprava - neveriť príznaku, ale porovnať obsah -
je v `cloud_sync_auto` drahá: `content_hash` beží nad celým snapshotom,
a ten príkaz je dnes zámerne read-only a lacný. Dialóg robí zlyhanie
nedeštruktívnym bez ohľadu na to, prečo je príznak zlý.

## 2.67.0 - restriction sa dá vybrať pri novej objednávke

marko: *"do new order pridat moznost vybrat restriction"*.

Nový stĺpec **Restrictions** v mriežke novej objednávky, osem prepínačov po
jednom písmene (plný slovenský názov je v `title`). Štyri, ktoré stoja peniaze
keď sa prehliadnu - obmedzený výhľad, 18+, 16+, doklad - sú prvé, lebo tak ich
radí `TICKET_RESTRICTIONS`.

Uložia sa **len na lístky**, nie na objednávku. Presne vzor `tier` z 2.2.7:
nastaví sa raz pri vytváraní, skopíruje na každý vygenerovaný lístok, a dá sa
potom meniť po jednom. Druhá kópia na objednávke, ktorú by čítal filter alebo
súčet, by bola ten istý tvar chyby ako riadky s `€`.

Obmedzenia sú súčasťou tvaru riadku, takže dva riadky, ktoré sa líšia len nimi,
sa nezlúčia do jednej objednávky - razítko sa aplikuje raz na objednávku.
A prenášajú sa do ďalšieho riadku rovnako ako typ lístka, lebo druhý riadok je
skoro vždy ďalší kus toho istého bloku.

Backend: `OrderInput.restrictions` v TS aj v Ruste, `restrictions_json`
pribudlo do INSERTu lístkov, čistené cez `clean_restrictions` - tú istú funkciu
používa editor lístka. Prázdny zoznam dá `"[]"`, čo je presne DEFAULT toho
stĺpca. Pole doplnené do dvanástich miest, kde sa `OrderInput` skladá ručne.

## 2.66.0 - tri tiché chyby v Settings

Všetky tri overené priamo v kóde, nie prevzaté z auditu.

**Upozornenia sa pri prvom zapnutí neuložili.** `configured` sa počítalo
z `desktopEnabled || ntfyEnabled`, čo sú **rozpísané** checkboxy formulára.
Na čistej inštalácii prvé kliknutie prehodilo `configured` na true, kým
`editing` bolo stále false, takže `configured && !editing` vymenilo formulár
za zhrnutie - aj s tlačidlom Save. Odznak ukázal „Enabled" a neuložilo sa nič.
Teraz sa `configured` číta zo `status`, teda z uloženého stavu. To opravuje
všetky tri použitia naraz: odznak, prepínanie formulár/zhrnutie aj tlačidlo
Cancel. `AiFeaturesCard` to nikdy nemala, lebo volá `setEditing(!c)`
z načítanej hodnoty.

**Obnova staršej kópie z Drive nemala potvrdenie.** Jeden klik prepísal celú
databázu a o 900 ms reštartoval, pričom obnova zo súboru hneď vedľa sa vždy
pýtala. Pridaný ConfirmDialog s rovnakým znením a rovnakou vážnosťou, vrátane
dátumu tej revízie a zmienky o bezpečnostnej kópii.

**„Combine both" tvrdilo, že sa nič nemaže.** Nie je to pravda od 2.20.0, kde
pribudlo spracovanie tombstonov - zlúčenie **odstráni riadky, ktoré si zmazal
na druhom počítači**, a presne to počíta `MergeOutcome.totalDeleted`. To pole
sa navyše nikde nezobrazovalo, takže zlúčenie mohlo mazať úplne ticho. Text
opravený, `totalDeleted` pribudlo do výsledného panela aj do toastu, a rovnaké
falošné tvrdenie v doc komentári typu je opravené tiež.

## 2.65.0 - v predaji vidno, ktorá objednávka to je, a z objednávky sa dá rovno predať

Dve veci, obe na marka vlastnú žiadosť, obe bez zmeny backendu.

**Rozbaľovačka objednávok v novom predaji** ukazovala `KÓD · Event · N voľných`,
čo nestačí na rozlíšenie dvoch objednávok na ten istý event - a dve objednávky
na jeden event sú normálny prípad, nie okrajový. Doplnený **dátum eventu**
a **sektor/rad/sedadlá**. Obe polia už cestujú na `OrderRecord` (`eventDate` od
2.2.10, `seats` od 2.0.38), takže to nepotrebovalo nový príkaz ani nové pole -
je to čisto popisok.

Sedadlá sú **v zátvorke**, a to nie je ozdoba: `formatSeatsSummary` spája
sektor/rad/sedadlo tým istým `" · "`, ktorým popisok oddeľuje svoje polia, takže
bez zátvoriek by sa `104 · 8 · 1-2` čítalo ako tri samostatné údaje. Overené
prepisom oboch formátovačov do Pythonu a spustením na piatich prípadoch vrátane
objednávky cez dva bloky a objednávky bez dátumu aj bez sedadiel.

**Order Detail dostal tlačidlo "Add sale"**, ktoré otvorí formulár predaja
s tou objednávkou už vybranou. Ide cez ten istý `location.state`, ktorý Sales
už používalo pre `openCreate`. Tlačidlo sa nezobrazí, keď na objednávke nie je
nič predajné, aby nikdy neviedlo do slepej uličky.

Stojí to vedome proti 1.9.1/2.45.0, ktoré z Orders/Tickets/Sales odstránili
odkazy typu "táto zmienka ma prehodí inam". Tamto boli mimovoľné odkazy, ktoré
navigovali bez toho, aby si o to požiadal; toto je tlačidlo, ktoré stlačíš
zámerne, a je to jediná cesta, ktorú dáta naozaj implikujú - objednávka plná
nepredaných lístkov existuje na to, aby sa stala predajom.

## 2.64.1 - oprava buildu

2.64.0 sa nezostavilo. Dátum eventu v predaji som napísal proti políčku, ktoré
na type `Sale` neexistovalo, a moja kontrola pred odoslaním sa pýtala len na to,
či sa slovo `eventDate` niekde v súbore vyskytuje. Vyskytovalo sa, ale na inom
type.

- **dátum eventu v predaji teraz existuje naozaj** — v Ruste, v dotaze, aj v
  TypeScripte
- overené spustením dotazu proti skutočnej schéme: vráti dátum, a pri evente bez
  dátumu vráti prázdno, čo appka ukáže ako `TBD`
- **všetko ostatné z 2.64.0 je nezmenené**

## 2.64.0 - chýbajúce peniaze, Listings preč, dátum eventu

### Opravené: dve objednávky ti chýbali v číslach

Dashboard písal **„Convert to EUR: € (2)"**. Prevádzať eurá na eurá nedáva
zmysel, ale to nebola tá škoda.

V tej kolónke bol **symbol `€`** namiesto kódu `EUR`. Appka sa všade pýta „je
to EUR?" a `€` nie je `EUR`. Tie dve objednávky, ich lístky aj ich predaje
preto **vypadli zo všetkých súčtov** na Dashboarde. Tvoje peniaze boli neviditeľné.

- **opravené aj uložené dáta** vo všetkých 14 tabuľkách, čo nesú menu
- **opravený aj vstup**, takže sa to už nemôže vrátiť
- `kr` sa naschvál nepremieňa — je švédska, nórska aj dánska a hádať by
  znamenalo vymýšľať si o tvojich peniazoch

### Events

- **Listings je preč.** Ostali dve zložky, Overview a Sales
- tabuľka a príkazy v pozadí ostali, rovnako ako pri Sheets

### Dátum eventu

- **v objednávke** medzi Ticket a Seat
- **v predaji** tiež, a číta sa pre každý riadok zvlášť, takže pri predaji cez
  viac eventov vidíš pri každom lístku jeho vlastný dátum

## 2.63.0 - Noir

Len vzhľad. Žiadna funkcia sa nezmenila, žiadna migrácia, žiadny nový príkaz.

Vybral si to sám v tom živom prepínači štýlov: **Noir, fialová `#7c5cf0`,
Bricolage Grotesque, Slight rohy, Cosy hustota, plošný graf.**

### Farby

- **celá appka je neutrálna** — sivá stratila fialový nádych, tmavý režim
  sedí na `#141415` a `#0b0b0c`
- **akcent je tvoja fialová** `#7c5cf0`
- tlačidlá sú o odtieň tmavšie (`#6e4ce4`), lebo biely text na `#7c5cf0`
  nemá dosť kontrastu

### Písmo

- **Bricolage Grotesque, priložené v appke** — dva súbory, 107 kB
- funguje **aj bez internetu**, nič sa nesťahuje
- má aj **č š ž ť ď ň ľ ĺ ŕ**, nie len holú latinku

### Tvary

- **ostrejšie rohy** — ovládacie prvky 4 px, kontajnery 6 px

### Čoho som sa nedotkol

tieňov, šírok stĺpcov v tabuľkách, stavových farieb, ani jednej stránky

## 2.62.0 - poznámky sa dajú používať

Bez migrácie. Schéma sa najskôr pozrela a nič nové nepotrebovala.

### Priradenia

- **štítok je odkaz** — klikneš naň a otvorí sa to, k čomu je podkarta
  priradená
- objednávka, event a predaj majú vlastnú stránku; **lístok otvorí svoju
  objednávku**, **pull otvorí zoznam pullov**, **financia otvoria Financie** —
  tieto tri stránky v TIQR neexistujú a nové sa nerobili

### Kopírovanie

- **Duplikovať podkartu** a **Duplikovať poznámku** — vždy sa spýta, či ísť
  *len s obsahom*, alebo *aj s priradeniami*
- kópia sa volá „… copy", druhá „… copy 2" — nikdy sa nezrazí s existujúcim
  názvom
- kópia celej poznámky si berie **aj svoje obrázky**

### Presúvanie

- **ťahaním myšou** sa presúvajú riadky, podkarty aj poznámky — kam to pustíš,
  tam to je
- šípky ↑ ↓ ‹ › ostávajú

### Písanie

- **odrážky** a **číslovanie** — čísla sa prepočítajú samé, keď niečo pridáš
  alebo zmažeš
- **dva bloky vedľa seba** — „⬓ Na polovicu" funguje na všetkom, aj na
  obrázkoch a čiarach
- **posuvník šírky** — nastavíš si, aká široká je plocha, kde píšeš (pamätá si
  to tento počítač, nikam sa to neposiela)
- **Ctrl+F** — nájde v poznámke a postaví kurzor rovno na nájdené slovo,
  ‹ › preskakuje medzi nálezmi

### Obrázky

- klikom sa otvoria **na celú veľkosť**
- majú **vlastný popis** pod sebou

### Zoznam a menu

- pri zaškrtávacom zozname vidíš **koľko z neho je hotových**
- podkarta má menu **⋯** — premenovať, duplikovať, posunúť, zmazať
- poznámka má menu **⋯** — duplikovať, pripnúť, archivovať, zmazať
- **Zmazať poznámku sa presunulo z panela nástrojov do tohto menu** — bolo to
  hneď vedľa formátovania
- nová poznámka si vie vybrať **šablónu** (Event, Objednávka, Pull, Predaj) —
  šablóna len pripraví podkarty, nič do nich nenapíše

## 2.61.0 - opravy v poznámkach, Sheets preč

### Priradenie

- **funguje po podkartách** — každá podkarta má svoje vlastné priradenia, nie
  spoločné pre celú poznámku
- **Inventár sa vyberá cez objednávku** — ukáže sa celá objednávka, rozklikneš
  ju a vyberieš **všetky lístky alebo len niektoré**
- **Financie sú konečne čitateľné** — keď záznam nemá popis, názov sa poskladá
  z kategórie, miesta, objednávky alebo účtu, a vždy je tam suma a dátum

### Podkarty

- nová podkarta si pýta názov **v okne appky**, nie v systémovom
- dajú sa **posúvať** doľava/doprava
- premenovanie dvojklikom

### Písanie

- **Enter ťa hodí rovno do nového riadku** (predtým si musel kliknúť)
- riadky, texty aj **obrázky sa dajú posúvať** hore/dole
- **Späť (Ctrl+Z)** — vráti poslednú zmenu, až 60 krokov dozadu

### Bezpečnosť

- **mazanie sa pýta** — poznámka, podkarta aj obrázok
- **štítok a dátum sa dajú odstrániť** krížikom

### Poznámky

- dajú sa **posúvať** v zozname a ostanú tam, kde ich dáš

### Sheets

Celá sekcia hárkov je **preč z appky**, aj ten odkaz „Staré hárky".

**Dáta z hárkov som nezmazal** — sú stále v databáze a synchronizujú sa, len
ich appka nikde neukazuje. Mazanie je nevratné, tak to spravím až keď povieš.

## 2.60.0 - obmedzenia na lístkoch, mesto pod eventom

### Obmedzenia

Pri úprave lístka je nové pole **Obmedzenia** — naklikáš, čo platí:

**Obmedzený výhľad · 18+ · 16+ · Na meno/doklad · Bez opätovného vstupu ·
Maloletý s dospelým · Na státie · Bezbariérové miesto**

Dá sa ich zapnúť **viac naraz** (sedadlo môže byť aj za stĺpom aj 18+).

V **detaile objednávky** ich vidíš ako farebné odznaky **pri sedadle** — tam,
kam sa pozeráš, keď riešiš, ktoré miesto to je.

Vieš ich nastaviť aj **hromadne** pre označené lístky naraz — celá objednávka
býva rovnaká a preklikávať ich po jednom je istý spôsob, ako ich nenastaviť
vôbec.

### Mesto pod názvom eventu

V **Inventory** aj v **Sales** je teraz pod názvom eventu **mesto**, menším
sivým písmom.

### Širšie stĺpce

Stĺpec Event dostal viac miesta, aby sa tam tie dva riadky zmestili:
Inventory **26 → 32 %**, **44 → 48 %** a **33 → 39 %** (tri rôzne šírky podľa
toho, ako široké máš okno), Sales **16 → 22 %**.

## 2.59.0 - sync na pozadí, len pri zapnutí a vypnutí

### Tak, ako si to chcel

- **5-minútový časovač je preč.** Automaticky sa syncuje **pri zapnutí** a
  **pri zatváraní** appky.
- **Nič už appku nezamkne.** Sťahovanie aj zlučovanie beží na pozadí, appka
  medzitým funguje.
- **Ručne kedykoľvek** cez Settings → Data. Každá zmena si sama poznačí, že je
  čo poslať, takže ručný sync je vždy správny.

### Opravil som chybu, ktorú som ti spravil v 2.58.0

Sync odkladal sťahovanie, kým „niečo píšeš" — ale stačilo, aby bol kurzor v
akomkoľvek poli, **aj prázdnom**. Nový editor poznámok je celý z takých polí,
takže kým si mal otvorenú poznámku, **automatický sync sa odkladal donekonečna**.
Vyzeralo to presne ako keby sync nefungoval. Teraz sa pýta na to podstatné:
je tam rozpísaný text, o ktorý by si prišiel?

### Zatváranie appky

Pri zatváraní sa ešte pošle, čo je neposlané — **maximálne 8 sekúnd**, potom sa
appka zavrie tak či tak. Nič sa nestratí: čo neprešlo, ostáva označené a pošle
sa pri najbližšom spustení.

**Appka sa musí dať vždy zavrieť** — preveril som všetkých 6 možných priebehov
(nič na poslanie, prešlo, odmietnuté, zaseklo sa, nevie sa rozhodnúť, offline).

### Čo som overil a nemenil

Rozhodovanie syncu aj porovnávanie verzií z Drive sú **správne**. A označovanie
zmien funguje aj pre nové tabuľky poznámok.

## 2.58.0 - Poznámky

Vybral si si návrh **02** a je hotový.

### Ako to funguje

Vľavo zoznam poznámok, vpravo tá otvorená. Hore lišta, dole **podkarty**.

**Každý riadok si nastavíš zvlášť** — to je na tom to hlavné:
- **veľkosť** (veľký nadpis / nadpis / text / malý)
- **farba** (6 farieb + základná)
- **tučné, kurzíva**
- **checkbox** — z riadku spravíš odškrtávacie políčko
- **obrázok** — vložíš ho presne tam, kde stojíš
- **čiara** na oddelenie

Enter spraví nový riadok, a ak si bol na checkboxe, nový je tiež checkbox.

### Podkarty

Dole v poznámke sú **podkarty ako v Sheets**. Napr. poznámka „Oasis kódy" a v
nej podkarty `ABC123`, `DEF456`… a v každej si píšeš o tom kóde zvlášť.
**+** pridá, dvojklik premenuje, **×** zmaže.

### Priradenie k záznamu

Tlačidlo **⧉ Priradiť** — poznámku pripojíš k **objednávke, eventu, inventáru,
predaju, pullu aj finančnému záznamu**. Aj k viacerým naraz. V zozname aj v
poznámke to potom vidíš ako štítok.

Keď ten záznam neskôr zmažeš, **poznámka ostane** — zmizne len to prepojenie.

### Obrázky

Vyberieš súbor a appka ho **sama zmenší** (max 1400 px, JPEG), aby ti
nenafukoval to, čo sa nahráva pri každom syncu.

### Ukladá sa samo

Po písaní, pri prepnutí podkarty aj pri odchode z poznámky.

### Staré hárky

Sheets **nikam nezmizli** — len už nie sú v paneli. Dole v zozname poznámok máš
odkaz **„Staré hárky"**, kým tam nejaké máš.

### Čo v tom NIE JE

**Zmena syncu**, ktorú si pýtal (nech beží v pozadí). Sync je chránená časť a
nechcel som ju narýchlo prilepiť sem — spravím ju zvlášť.

## 2.57.0 - ťahanie, spájanie, výplne, kalendár

### Veľkosti sa ťahajú myšou

Chytíš **okraj písmena stĺpca** a ťaháš — stĺpec sa zužuje/rozširuje živo.
To isté pri **čísle riadku**, spodný okraj mení výšku. Uloží sa to až keď
pustíš.

### Farby sú sýtejšie + výplne

Text je výraznejší v oboch režimoch. Pribudla **výplň bunky** (8 možností),
**podčiarknutie**, **preškrtnutie** a **zarovnanie** vľavo/na stred/vpravo.

### Spájanie buniek

Na lište **Merge: 2 · 3 · 4 · all · split**. Spojíš bunku s ďalšími, alebo celý
riadok do jednej — hodí sa na nadpisy. `split` to rozdelí späť **aj s textom**,
nič sa nestratí.

Keď stĺpec presunieš alebo zmažeš, spojenie ide s tým, čo si spojil.

### Kalendár a čas

**Insert → Date… / Time… / Date and time…** — vyberieš z kalendára a hodí ti to
do bunky.

### Čo kedy sa deje

**Coming up** (na lište aj v Insert) prečíta **celý hárok**, nájde všetky
dátumy a vypíše ich **od najbližšieho**, aj s tým, čo je vedľa nich v riadku.
Pripočíta aj alerty. Klikneš na riadok a skočí ti na tú bunku.

Nehľadá to v jednom „dátumovom stĺpci" — hľadá to všade, lebo stĺpce tu nič
nediktujú.

### Ďalšie funkcie

- vložiť riadok **nad/pod** kurzor
- **duplikovať riadok** aj s farbami a spojeniami
- **vymazať formátovanie** bunky
- **zmraziť 1–3 riadky** hore (View), ostanú visieť pri rolovaní
- dole vedľa hodnoty bunky: **koľko je v stĺpci vyplnených a ich súčet**
  (rozumie aj `1 234,50`)

## 2.56.0 - farby, veľkosti, viac miesta

### Číslovanie je maličké

Písmená stĺpcov aj čísla riadkov sú teraz **miniatúrne** a majú vlastnú
veľkosť — ostanú malé, aj keď si riadky zväčšíš.

### Farba textu

Nad tabuľkou je lišta: **7 farieb**, **B** (tučné), **I** (kurzíva). Klikneš na
bunku a na farbu. Druhý klik na tú istú farbu ju zruší.

Farba **drží sa bunky** — keď stĺpec presunieš alebo zmažeš iný, farba ide s
tým, čo si ofarbil. Nikam sa neposunie.

### Veľkosti

- **Výška riadku** — `−` / `+` na lište, alebo `reset` späť na východziu
- **Šírka stĺpca** — `−` / `+` na lište
- **Výška všetkých riadkov** naraz — Format → All rows 20/24/30/40 px

### Viac riadkov a stĺpcov

Prázdnych riadkov je teraz **60** namiesto 40. Na lište sú **+ 50 rows** a
**+ column**, v menu Insert aj **+200 riadkov** a **+5 stĺpcov** naraz.

### Čo sa nezmenilo

Tvoje hárky, riadky ani text. Migrácia len **pridáva** štyri stĺpce s
východzími hodnotami — čo si mal, vyzerá presne tak ako predtým, kým sám niečo
neofarbíš.

## 2.55.0 - Sheets: tmavý, voľný, s alertmi

Vybral si si návrh **01 Classic** a chcel ho tmavší. Tu je — aj s tým ostatným,
čo si k nemu napísal.

### Hlavička už nič nediktuje

Hore sú **písmená A, B, C**, presne ako v Sheets. Názov stĺpca je len
**nepovinný štítok** v druhom riadku hlavičky — môže byť prázdny.

Stĺpec už nehovorí, čo do neho patrí. **Píšeš čo chceš a kam chceš.**

Nový hárok = **14 prázdnych stĺpcov bez názvov** a prázdna mriežka. Štítok si
ktorémukoľvek stĺpcu dáš (alebo zmažeš) cez **Format → Label for column**.

### Taby dole

Zoznam hárkov je **dole**, ako v Sheets. Klikneš = prepneš, **+** = nový,
dvojklik = premenovať.

### Alerty

**Insert → Reminder…** alebo zvonček vpravo hore. Napíšeš čo, vyberieš **kedy**
(dátum aj čas), prípadne poznámku — a vieš to **pripnúť ku konkrétnej bunke**,
tá potom má oranžový rožtek.

Keď príde čas, dostaneš **systémové upozornenie** aj hlášku v appke. Zvonček
ukazuje, koľko ich je po termíne.

**Hovorím rovno:** funguje to, **kým máš appku otvorenú**. Keď je zavretá, nič
nevyskočí — čo medzitým dobehlo, dostaneš pri najbližšom spustení. Je to
napísané aj priamo v tom okne.

### Menu hore

**File · Edit · View · Insert · Format · Data** — tak ako si chcel. Sú v nich
tie základné veci: nový/premenovať/zmazať hárok, zmazať riadok/stĺpec, zoom,
pridať riadky/stĺpec, dnešný dátum, alert, štítok stĺpca, posun stĺpca,
triedenie.

### Riadky

Mriežka vždy ukazuje **aspoň 40 riadkov**, aj keď je hárok prázdny. Napíšeš do
ktoréhokoľvek a vznikne — aj všetky nad ním, aby ti nič neposkočilo hore.

### Čo som nechytal

Synchronizáciu, Dashboard, Events, Inventory, Sales, Pulls, Finance,
prihlásenie, nastavenia, tému, navigáciu. Databáza sa len **rozšírila** o
tabuľku alertov — nič existujúce sa nemaže.

## 2.54.1 - oprava buildu 2.54.0

2.54.0 sa nezostavilo — moja chyba. Pri premenovaní jednej premennej v mriežke
mi ostal jeden starý názov (`isEditing`), a TypeScript to zastavil.

**Obsahovo je to to isté ako 2.54.0** (Sheets), len sa to teraz zostaví.

Pri tom som si všimol ešte jednu vec: build najprv spúšťa TypeScript a až
potom Rust. Keďže TypeScript padal, **Rust sa nikdy nekompiloval** — ani v
2.52.0, ani v 2.53.0. Tak som ho prešiel a dve miesta prepísal tak, aby si
Rust nemohol sťažovať.

## 2.54.0 - Sheets

Povedal si, že by si to radšej mal **ako naozajstný Google Sheets, úplne
jednoducho**. A keď som sa spýtal, vybral si si **len tabuľky**.

Tak je to tak. Sekcia sa volá **Sheets**, poznámky (voľné písanie) sú preč.

### Tabuľka sa konečne správa ako tabuľka

Doteraz bolo v každej bunke políčko na písanie. Fungovalo to, ale vyzeralo to
ako formulár so stovkou okienok. Teraz:

- **bunka je obyčajný text** — rámček sa objaví len v tej jednej, v ktorej si
- **klávesnica**: šípky posúvajú, **Enter** otvorí a ide dole, **Tab** doprava
  (Shift+Tab doľava), **Esc** zruší, **Delete** vymaže
- **začneš písať a bunka sa rovno prepíše** — nemusíš nič otvárať
- **čísla riadkov vľavo**, mriežka okolo každej bunky
- **dole vždy čaká prázdny riadok** — napíšeš doň a riadok vznikne. Žiadne
  tlačidlo „Add row".

Ukladá sa to samo, po každej bunke, tak ako doteraz.

### Ostalo

Triedenie (klik na stĺpec), filtrovanie, pridávanie/premenovanie/mazanie/
presúvanie stĺpcov, mazanie riadkov, hľadanie cez všetky hárky naraz.

### Poznámky z 2.52/2.53

Ak si si do nich niečo stihol napísať, appka ti hore ponúkne **„Import as a
sheet"** — spraví z nich hárok so stĺpcami Title / Text / Date / Tags.
**Nič nemaže**, originál zostáva v databáze. Ak si tam nič nemal, nič sa
neukáže.

### Čo som nechytal

Synchronizáciu, Dashboard, Events, Inventory, Sales, Pulls, Finance,
prihlásenie, nastavenia, tému, navigáciu. **Žiadna nová migrácia** — databáza
je nezmenená.

## 2.53.0 - Notes

Workspace z 2.52.0 bol na teba prikomplikovaný, tak som ho **zjednodušil na to,
čo si pýtal**: jedno čisté miesto, kam si zapíšeš čokoľvek dôležité.

Volá sa **Notes**, je to vlastná položka v paneli, s Finance nemá nič spoločné.

### Sú tam už len dve veci

- **Note** — normálne písanie, čokoľvek.
- **Table** — tabuľka so stĺpcami, ktoré si sám pomenuješ.

Records, Tasks, kategórie, štatistiky a Upcoming sú **preč**. `+ New` ti ponúkne
len Note alebo Table.

### Note

Veľké pole na písanie. Nad ním lišta: **nadpis, tučné, kurzíva, odrážky,
číslovanie, checkbox, odkaz**. Tlačidlo **Preview** ti to ukáže vysádzané a
**checkboxy sa dajú rovno odkliknúť**.

Nie je to žiadny komplikovaný editor — je to obyčajný text, takže to, čo napíšeš,
tak presne to sa aj uloží.

**Ukladá sa samo** — chvíľu po tom, ako prestaneš písať, keď klikneš preč, aj
keď odídeš späť na zoznam. Žiadne tlačidlo Uložiť.

Každá poznámka má názov, dátum (nepovinný), tagy a vieš si ju **pripnúť hore**.

### Table

Funguje ako doteraz (bunka sa uloží sama), ale pribudlo:

- **triedenie** — klikneš na stĺpec, druhý klik otočí, tretí zruší
- **filtrovanie** — políčko nad tabuľkou, píšeš a riadky sa zúžia
- **presúvanie stĺpcov** — šípky vľavo/vpravo v hlavičke

**Opravil som aj chybu z 2.51.0/2.52.0:** keď si zmazal stĺpec v strede,
na obrazovke ostali staré hodnoty pod novými hlavičkami. Dáta v databáze boli v
poriadku, zobrazenie nie. Už je to správne.

### Hľadanie

Jedno políčko hore, hľadá naraz v **názvoch poznámok, texte poznámok, názvoch
tabuliek aj v bunkách tabuliek**. Výsledky sú rozdelené na Notes a Tables.

Tabuľka dostane **jeden riadok** vo výsledkoch, aj keď jej sedí dvadsať riadkov —
zvyšok ti len spočíta („Code · 3 more rows").

### Usporiadanie

**Pinned → Recent → Tables → All notes**, plus triedenie zoznamu podľa
**naposledy upravené / najnovšie / najstaršie / abecedne** a filtrovanie podľa
tagov.

### Čo som nechytal

Synchronizáciu, Dashboard, Events, Inventory, Sales, Pulls, Finance,
prihlásenie, nastavenia, tému, navigáciu. **Databázu som nemenil** — žiadna nová
migrácia. Čo si prípadne stihol napísať v 2.52.0, ostáva a otvorí sa.

## 2.52.0 - Workspace

Nová vlastná položka v paneli: **Workspace**. Miesto, kam si zapíšeš všetko,
čo potrebuješ mať poruke — komu si čo predal, aký kód, aký nick, heslá k
účtom, plány, úlohy s termínom — a hlavne to celé vieš zas nájsť.

Finance sa vracia na svoje štyri záložky. Workspace je samostatná sekcia,
nie je to súčasť Finance.

### Štyri veci, ktoré si vieš vytvoriť

- **Note** — obyčajná poznámka, text.
- **Record** — poznámka s vlastnými políčkami (Nick, Kód, Cena, čokoľvek).
- **Task** — úloha s termínom a stavom (Open / Done).
- **Table** — tabuľka s vlastnými stĺpcami (to sú tie hárky z 2.51.0,
  nezmenené, aj s tým, čo si do nich napísal).

**Nemusíš vopred vedieť, čo z toho to bude.** Napíšeš rýchlu poznámku a
kedykoľvek ju v editore prepneš na Record alebo Task — nič sa nestratí, je to
stále tá istá položka.

### Rýchly zápis

Hore je jeden riadok: napíšeš a dáš Enter. Hotovo. Nič sa neotvára, nič
nevypĺňaš. Detaily doplníš, keď budeš chcieť.

### Nájsť to

Vyhľadávanie prehľadá **názvy, text, políčka, tagy aj názvy tabuliek** naraz
a výsledky poukladá podľa typu. Vidíš kúsok textu okolo toho, čo si hľadal.

Ďalej: **tagy**, **kategórie**, **pripnutie** hore, **archív** (nemažeš, len
odložíš), **Upcoming** s najbližšími termínmi a prehľad naposledy upraveného.

### Heslá

Ak políčko pomenuješ ako heslo (password, heslo, pin, token, 2fa…), jeho
hodnota sa **automaticky skryje na bodky** — v editore aj vo vyhľadávaní — a
odkryje sa až keď klikneš Show.

**Dôležité a hovorím to na rovinu:** toto je len o tom, aby ti heslo
nesvietilo na obrazovke. **Nie je to šifrované.** Databáza je ten istý
obyčajný súbor ako Sales a Finance. Workspace nie je správca hesiel.

### Čo sa nezmenilo

Dashboard, Events, Inventory, Sales, Pulls, Finance, prihlásenie,
synchronizácia, nastavenia, téma, navigácia — nič z toho som nechytal.
Synchronizácia funguje presne tak ako doteraz, len navyše prenáša aj
Workspace.

## 2.51.1 - Notes je teraz pod Finance

Presunuté z vlastnej položky v paneli na **piatu záložku vo Finance**:
Overview · Transactions · Accounts · Reports · **Notes**.

**S dátami sa nestalo nič** — tie isté tabuľky, tá istá migrácia, to isté
synchronizovanie. Čo si stihol napísať v 2.51.0, tam ostáva.

## 2.51.0 - Notes: miesto na všetko dôležité

Nová položka v paneli: **Notes**. Hárky, ktorým si **sám pomenuješ stĺpce** —
ako v Google Sheets, ale vnútri appky a synchronizované medzi oboma počítačmi.

### Ako to funguje

Vytvoríš si hárok a píšeš riadky. Stĺpce si kedykoľvek **pridáš, premenuješ
alebo zmažeš**. Každá bunka sa **uloží sama**, hneď ako z nej klikneš preč —
žiadne tlačidlo Uložiť, takže nikdy nie je nič „ešte neuložené".

### Aby to nebol prázdny papier

Nový hárok vieš začať zo **šablóny**:

- **Buyers** — Nick · Ticket code · Event · Paid · Contact · Note
- **Accounts** — Platform · Account · Email · Note
- **Plans** — What · By when · Status · Note
- **Blank** — jeden stĺpec, pomenuj si ho

Stĺpce sú len začiatok, hárok je potom tvoj.

### Nájdeš v tom všetko

Hore je **jedno hľadanie cez všetky hárky naraz**. Napíšeš nick, kód lístka
alebo kus mena — a vypíše ti to riadok, **v ktorom hárku** je a **v ktorom
stĺpci** sa to našlo. Klikneš a si tam.

### Synchronizuje sa

Zápisky idú medzi Macom a Windowsom rovnako ako objednávky — vrátane zlučovania,
keď si na oboch počítačoch napísal niečo iné, aj mazania.

## 2.50.1 - sync sa už nespúšťa pri každom kliknutí na okno

Moja regresia z 2.48.1. Vtedy som pridal, že sa syncuje aj keď sa vrátiš na
okno — aby bolo prepnutie medzi počítačmi okamžité. Fungovalo to až príliš
dobre: **každé alt-tabnutie spustilo sync** a zakaždým to o sebe dalo vedieť
v hlavičke.

Preč. Zostáva **jeden sync pri spustení appky** a potom ticho na pozadí každých
5 minút.

Ten časovač som nechal naschvál — keby som zrušil aj jeho, vrátila by sa presne
tá vec, na ktorú si sa sťažoval predtým (že sa počítače nespoja, keď necháš
appku otvorenú). Na rozdiel od toho focusu je ale **neviditeľný**, kým naozaj
nie je čo preniesť.

## 2.50.0 - euro sa už nekonvertuje na euro + preferovaná mena

### Tá chyba

V databáze bol uložený **symbol `€`**, nie kód `EUR`. A kontrola „je to už
v eurách?" porovnávala doslova s textom `"EUR"` — `"€"` sa mu nerovná, takže
objednávka zadaná v eurách išla konvertovať **z eur na eurá**. A služba na
kurzy, celkom správne, žiadnu menu `€` nepozná. Odtiaľ to 404.

Teraz appka rozpoznáva symboly ako kódy: **€ = EUR, $ = USD, £ = GBP**, aj
`Kč`, `zł`, `Ft`, `lei`, `лв`, `₺`. Čo je už v cieľovej mene, sa **nikdy
neponúkne na konverziu**.

`kr` som naschvál nechal tak — je to švédska, nórska **aj** dánska koruna.
Tipovať by znamenalo prepočítať ti peniaze zlým kurzom.

### Preferovaná mena

**Settings → Lookups → Preferred currency**: na výber **EUR, USD, GBP**.

Všetko „Convert to…" v celej appke sa riadi podľa toho — Dashboard, obe
tlačidlá pri objednávke, Sales aj Finance. Keď to prepneš, **popisky sa zmenia
naraz všade** a nemôžu sa rozísť s tým, čo konverzia naozaj spraví.

Default je EUR, takže ak to nikdy neotvoríš, appka sa správa presne ako doteraz.

## 2.49.2 - povolenie sa dá dať jedným klikom priamo z hlášky

Preveril som ešte dve veci, ktoré by boli naozajstné chyby v kóde — a **obe sú
čisté**: appka si pri obnove tokenu nepýta užší rozsah, a to druhé prihlásenie
(„Continue with Google") ti ten prvý token neprepisuje.

**Zostáva to, čo sa z mojej strany opraviť nedá.** Aké povolenia token má, sa
rozhodlo v momente, keď si ho schválil — a **žiadny kód nevie do tokenu pridať
povolenie, ktoré mu nikdy nikto nedal.** To je pravidlo Googlu, nie chyba appky.
Takže úplne „bez zmien" to nejde a nebudem ti tvrdiť opak.

**Čo som spravil:** zrušil som to hľadanie. V tej červenej hláške je teraz
tlačidlo **„Allow Google Drive access"** — klikneš, otvorí sa Google, schváliš,
a **sync sa rozbehne hneď**. Žiadne chodenie do Settings.

Spolu s 2.49.1 je to uzavreté: ak by si pri tom schvaľovaní niektoré políčko
odškrtol, appka to **odmietne uložiť** a povie ti to — takže sa to nemôže
potichu zopakovať.

## 2.49.1 - to 403 bolo chýbajúce povolenie

Tá chyba, čo si poslal, je jednoznačná: **`insufficientPermissions`**. Nie je to
vypnuté API ani expirovaný token — **tvoj prihlasovací token nemá povolenie na
Drive**.

Ako sa to stane: na tej Google obrazovke má **každé povolenie vlastné
zaškrtávacie políčko**. Keď jedno odškrtneš, Google ti aj tak vráti úplne
platný token — len bez toho povolenia.

**A appka sa nikdy nepozrela, čo Google naozaj povolil.** Uložila taký token ako
úspešné prihlásenie a každé ďalšie volanie na Drive padlo na 403. Bez stopy,
bez vysvetlenia.

Teraz sa to kontroluje: ak pri prihlásení chýba Drive (alebo Sheets),
**prihlásenie rovno zlyhá a povie ktoré** — namiesto toho, aby sa uložilo
a lámalo sa to až potom.

**A tá hláška ťa posielala na zlé miesto.** Začínala tým, že „Drive API nie je
zapnuté v Google Cloud projekte" — čo nebol tvoj problém. Teraz sa číta
odpoveď od Googlu a podľa nej sa povie buď „prihlás sa znova a nechaj všetko
zaškrtnuté", alebo „zapni API, tu je odkaz".

**Čo s tým máš spraviť ty:** Settings → Integrations → prihlásiť sa Googlom
znova a na tej obrazovke **nechať zaškrtnuté všetky políčka**.

## 2.49.0 - dátum sa dá písať, mena sa vyberá

### Dátum: píš alebo klikni, oboje

Ono to nebolo pokazené — **ono sa doň nikdy nedalo písať**. To políčko bolo
v skutočnosti tlačidlo, ktoré len otváralo kalendár.

Teraz je to **normálne políčko**. Klikneš, píšeš číslice a bodky si doplní samo:
`21092026` → `21.09.2026`. Kalendár zostáva — je to tá ikonka vpravo v políčku.

Nezoberie dátum, ktorý neexistuje: **31.02.2026** je osem správnych číslic a aj
tak to nie je deň. Priestupný rok sedí — 29.02.2024 áno, 29.02.2026 nie.

Rozpísané `21.0` sa do formulára nedostane. Keď odklikneš preč bez dokončenia,
vráti sa posledná platná hodnota.

### Mena sa vyberá zo zoznamu

Pri **order, sale aj pull** je mena rozbaľovačka s tými istými 13 menami, aké
ponúka zvyšok appky. Keby si mal niekde menu mimo zoznamu, zostane ti —
appka ti ju ticho neprepíše na EUR.

## 2.48.1 - a toto bola tá chyba

Našiel som ju. **Zlučovanie a sťahovanie smelo bežať len pri jednom jedinom
tiku — hneď po otvorení appky.** A práve vtedy ešte zvyčajne nie je pripravené
prihlásenie do Googlu, takže ten jediný pokus vrátil „vypnuté" a **minul sa,
hoci nikdy žiadnu šancu nedostal**. Každý ďalší tik už právo zlučovať nemal —
len ukázal pruh a čakal. Donekonečna.

Preto sa ti tie dva počítače nikdy nespojili.

**Teraz to smie každý tik.** Jediné, čo to odloží, je keď máš **otvorené okno
alebo rozpísané políčko** — zlúčenie totiž do databázy iba pridáva riadky a tá
nie je v ohrození, ale obnovenie stránky po ňom by ti zmazalo rozrobené. Vtedy
ti to napíše a dokončí sa hneď, ako dopíšeš.

**Plus: syncuje sa aj vtedy, keď sa na okno vrátiš.** Predtým si po prepnutí
z druhého počítača čakal aj päť minút — a práve to je ten pocit „nie je to
automatické".

**Nezacyklí sa to:** po zlúčení sa uloží verzia a databáza zostane „špinavá",
takže ďalší krok je nahranie spojených dát hore a potom pokoj.

## 2.48.0 - autosync konečne povie, čo robí

Prešiel som celú cestu autosyncu a **v samotnej logike som chybu nenašiel** —
príkazy sú zaregistrované, názvy sedia, rozhodovacia tabuľka je správna,
príznak „mám neodoslané zmeny" sa nastavuje pri každom zápise a zámok sa
nemôže zaseknúť.

**Čo bolo naozaj pokazené:** autosync **prehĺtal každú chybu**. V kóde bol
prázdny `catch {}`. Takže počítač, ktorému sa upload každých 5 minút odmietal,
vyzeral úplne rovnako ako počítač, ktorý nemá čo poslať — **na oboch stranách,
koľko chcelo dní**.

Teraz:

- **Červený pruh hore** s konkrétnou chybou, keď autosync zlyhá.
- **Každý pokus sa zapíše** — aj ten, čo nemal čo robiť — a v **Settings → Data**
  vidíš posledných 8 pokusov aj s časom a dôvodom, zvlášť na každom počítači.

Ten existujúci riadok „Last synced" ti to povedať nevedel: počíta aj ručné
synchronizácie, takže mohol hlásiť „pred hodinou", kým automatika medzitým
zlyhala dvanásťkrát.

## 2.47.5 - štyri slovenské slová, ktoré sken nevidel

Mal si pravdu. Môj sken v 2.47.3 hľadal slovenčinu **podľa diakritiky** — takže
slovo bez dĺžňov a mäkčeňov mu prešlo popod ruky. Štyri také tam boli:

| Slovo | Kde |
|---|---|
| `nie` | prepínač Pull v **New Order** |
| `Kupec` | popis poľa v **New Sale** |
| `Ks` (2×) | hlavička stĺpca v zozname **Pulls** |
| `e.g. Doprava` | placeholder v Settings |

Teraz je to `no`, `Buyer`, `Qty` a `e.g. Transport`.

**Ako som to našiel poriadne:** nie lepším zoznamom slov, ale tak, že som
vypísal **všetkých 733 textov**, ktoré appka zobrazuje, a tých 399 krátkych
(nadpisy, tlačidlá, popisy polí) som prečítal.

## 2.47.4 - Profit karta už nie je fialová

Na light mode svietila karta Profit levanduľovo medzi štyrmi bielymi — vyzerala
ako označená, a zelené číslo na nej sa s tou farbou bilo.

Boli to **dve chyby v jednom riadku**:

- `border-brand-300` **nikdy nič nenakreslilo** — karta má nastavené
  `border: 0`, takže farba rámika nemala čo zafarbiť. Jediné, čo bolo vidieť,
  bola tá výplň.
- `brand-50` je sýta levanduľa. Ako jediná zafarbená dlaždica v rade bielych
  pôsobila ako výber, nie ako dôležitý údaj.

Teraz je tam **neutrálny tenký krúžok** a číslo zostáva väčšie (24 px oproti
19 px) — nájdeš ho hneď, ale nebije sa to.

Ostatné fialové miesta v appke (lišty výberu, čipy, oznamy) som nechal — tie
majú byť fialové, sú to akcie.

## 2.47.3 - celá appka je po anglicky

Vypĺňovače boli od 2.45.0 po slovensky, lebo vznikli z toho slovenského
preview. Teraz je všetko po anglicky.

**127 reťazcov v 8 súboroch** — štyri vypĺňovače, spodná lišta a ikonky
v `ui.tsx`, dve hlášky v AI paneli, jeden placeholder v Settings a jeden
príklad mena vo Finance.

Našiel som ich tak, že som preskenoval **každý reťazec a každý text v celom
`src/`** na slovenskú diakritiku a slová — a po oprave som ten istý sken
spustil znova. Vrátil nulu.

**Čo zostalo po slovensky: komentáre v kóde.** Tie citujú tvoje vlastné
zadania, prečo je niečo tak, ako je — a nikto ich v appke nevidí.

## 2.47.2 - vypĺňovač je širší a stĺpce sa roztiahli

Okno malo **napevno 1152 px** bez ohľadu na to, aký veľký máš monitor — preto
Typ ukazoval len „—" a Sektor s Radom boli užšie než ich vlastné nadpisy.

Teraz je **až 1560 px**, respektíve 94 % šírky okna, keď máš menšie. Rozpočet
na stĺpce narástol z 1112 na **1520 px** a najviac dostali tie, čo boli
najstlačenejšie:

| Stĺpec | Predtým | Teraz |
|---|---|---|
| Typ | 110 px | **150 px** |
| Sektor | 92 px | **130 px** |
| Rad | 64 px | **90 px** |
| Sedadlá | 108 px | **150 px** |
| Platforma | 130 px | **190 px** |
| Poznámka | 126 px | **230 px** |

Platí to na všetkých štyroch — order, pull, sale aj event.

## 2.47.1 - dátum, Ks a rozpoznávanie mien

### Dátum nebol pokazený, bol orezaný

Kalendár sa otváral ako `absolute` vnútri tabuľky, ktorá sa posúva — a taký
kontajner absolútne umiestnené okno **oreže**. Takže sa otvoril do neviditeľna.

Teraz je `fixed` a počíta si vlastnú pozíciu. **Opravené naraz pri order, pull
aj event** — je to jeden komponent, bola to jedna chyba.

### Ks: šípky preč, políčko väčšie

`type="number"` kreslí šípky **dovnútra** políčka a pri 56 px sadli rovno na
číslo — presne ako na tvojej fotke. Teraz je to obyčajné políčko, píšeš doň len
číslice a je širšie (order 56 → 72 px, pull 50 → 70 px). Šírku som vzal
z Poznámky, takže sa stále všetko zmestí.

### AI: meno eventu a platformy sa konečne trafí

Porovnávalo sa **presne znak po znaku**. Takže „Karpatské Chalupy 2026"
z fotky nenašlo event „Karpatské Chalupy" a „TICKETPORTAL.SK" nenašlo
„Ticketportal" — pole ostalo prázdne a nikto ti nepovedal prečo.

Teraz sa ignoruje diakritika, veľkosť písmen aj medzery navyše, a skúsi sa aj
začiatok a obsiahnutie. **Keď sedia dve možnosti, nevyberie ani jednu** —
tipovať medzi dvoma eventmi je horšie než prázdne pole.

### Dátum nákupu z fotky sa už nezahadzuje

AI ho z účtenky čítala vždy, formulár ho ignoroval a dal dnešný. Už nie.

## 2.47.0 - fotka vyplní viac objednávok naraz, rohy sú ostrejšie

Vybral si si **mriežku** — riadková tabuľka zostáva na všetkých štyroch.

### Fotka teraz vytvorí toľko riadkov, koľko skupín na nej je

Doteraz appka zo screenshotu prečítala všetky bloky sedadiel, ale **dala ti
vybrať jeden** a o zvyšku napísala, že si druhú objednávku máš spraviť ručne.
Preč.

Teraz **každá skupina = jeden riadok**. A čo je jedna objednávka a čo dve,
rozhodne **to isté pravidlo, aké platí, keď riadky píšeš sám**: musí sedieť
sektor, rad, cena, mena, typ, platforma aj pull — **a sedadlá musia ísť tesne
za sebou**. Inak vzniknú samostatné objednávky.

Takže screenshot s troma blokmi:

| Na fotke | Riadky | Objednávky |
|---|---|---|
| A/12 14–15 · A/12 16–18 · VIP/1 22 (rovnaká cena) | 3 | **2** — A/12 sa spojí na 14–18, VIP zvlášť |
| A/12 14–15 @201 · A/12 16–18 **@185** | 2 | **2** — iná cena sa nezlučuje |
| A/12 14–15 · A/12 **19** | 2 | **2** — sedadlá nie sú vedľa seba |
| A/12 · **B/3** | 2 | **2** — iný sektor vždy zvlášť |

**Prázdny formulár sa nahradí**, rozpísaný sa **doplní** — čo si už napísal,
sa nestratí. A fotka bez detailu lístkov ti riadok **nezmaže**, len doplní to,
čo z nej vyčítala.

### Fotka je teraz na všetkých štyroch

Sales bol jediný bez nej. Riadky v predaji sú **skutočné lístky**, takže tie zo
screenshotu vyčarovať nejde — ale vyplní sa dátum, platforma, kupec, stav
platby, a **cena aj poplatok sa predvyplnia do riadkov, ktoré ešte žiadnu
nemajú**. Cenu, ktorú si napísal ty, neprepíše.

### Ostrejšie rohy

Z **13 / 20 / 24 px** na **6 / 8 / 10 px**. Jedna zmena v jednom súbore, takže
to chytí celá appka vrátane Finance — netreba sa nikde inde hrabať.

Guličky, avatary a pilulky (`rounded-full`) som nechal okrúhle. Bodka s rohom
nie je ostrejší design, to je chyba.

## 2.46.1 - riadky sa zmestia a Ks je zase číslo

Mal si pravdu dvakrát.

### Ks už nie je zhasnuté políčko

Keď napíšeš sedadlá, počet z nich vychádza — ale v 2.46.0 som to ukazoval v
**read-only políčku**, čo vyzerá ako niečo, čo ťa odmieta. Teraz je tam
**samotné číslo**, zarovnané doprava. Keď sedadlá nenapíšeš, políčko na
písanie počtu je tam normálne ďalej.

### Naozaj sa to nezmestilo — zmeral som to

Okno má ~1112 px využiteľnej šírky. Stĺpce mali:

| Formulár | Predtým | Teraz |
|---|---|---|
| Inventory | 1296 px | 1112 px |
| **Pulls** | **1426 px** | 1112 px |
| Events | 1206 px | 1112 px |
| Sales | 966 px | 1054 px |

Pull pretekal o **314 px**. Teraz sedí každý.

### Ostrejší design

- **Hustejšie bunky** — prepnuté na kompaktný krok, ktorý appka už má
  (`px-2 py-2` namiesto `px-3 py-2.5`). Nič nového som nevymýšľal, len som
  použil to, čo tu bolo. Tým sa aj získala tá šírka.
- **Čísla a ich hlavičky sú zarovnané rovnako** — doprava. Číslo vpravo pod
  nadpisom vľavo je klasická známka tabuľky, ktorú nikto poriadne nenastavil.
- Užšie číslo riadku (28 px) a užší stĺpec s ikonkami (64 px), drobnejšie
  ikonky.

## 2.46.0 - vyplňovače vedia presne, čo je zle, a graf má os

### Chyba svieti na políčku, nie vo vete dole

Doteraz ti formulár povedal prvú chybu ako vetu a zvyšok zamlčal. Teraz
**každé zlé políčko zčervenie** — tým istým červeným krúžkom, aký má zvyšok
appky — a dole je napísané, koľko vecí treba opraviť a kde začať.

**Kedy to zasvieti:** ak si niečo napísal zle (cena „201,0x"), hneď. Ak niečo
len chýba, až keď klikneš Vytvoriť — prázdny formulár ešte nie je chybný.

### Ďalšie drobnosti, ktoré to robia rýchlejším

- **Čísla riadkov** — chyba vie povedať „riadok 3" a ty ho nájdeš bez počítania.
- **Hlavička zostáva** pri rolovaní dlhého zoznamu.
- **Duplikovať riadok** — šesť miest toho istého turné alebo štyri lístky, čo
  sa líšia sedadlom, sú rýchlejšie skopírované ako prepísané.
- **Kurzor ide do nového riadku**, keď ho pridáš.
- **⌘↵ / Ctrl+↵ vytvorí** odkiaľkoľvek z formulára.

### Ks a sedadlá si už neodporujú

Keď napíšeš sedadlá, počet kusov **z nich vychádza** — políčko to teraz ukáže
a nedá sa prepísať. Predtým tam ticho ostávala jednotka, ktorú appka aj tak
ignorovala.

### Finance: graf už nie je bodka

Default je „This month" a ten mal presne **jeden mesačný stĺpec** — takže graf
vyzeral prázdny. Teraz sa veľkosť kroku riadi podľa obdobia: **do 92 dní po
dňoch, nad to po mesiacoch**, rovnako ako to robí graf na Dashboarde. „This
month" je tým pádom ~30 bodov, teda skutočná krivka.

**Čo som zámerne neurobil:** nerozťahujem graf mimo zvoleného obdobia, len aby
bola čiara dlhšia. Mesiace mimo obdobia by sa kreslili ako nula, hoci v nich
reálne peniaze boli — to by bol graf, ktorý klame. Prázdne dni vnútri obdobia
sú naozajstné nuly.

## 2.45.0 - jeden vyplňovač všade, a otvára sa NAD zoznamom

Tvoja požiadavka mala dve časti a obe sú hotové.

### 1. Rovnaké riadky na všetkom

**Events, Inventory, Sales aj Pulls** majú teraz ten istý vyplňovač: tabuľka
riadkov, pod ňou tlačidlo „pridaj ďalší", dole jedna lišta, ktorá hovorí, čo
vznikne, a vedľa nej Zrušiť/Vytvoriť.

- **Inventory** — 11 polí, gulička na pull, dátum nákupu automaticky. Čo
  nesedí sektorom, radom, cenou, menou, typom, platformou či pullom — alebo
  nemá sedadlá tesne za sebou — je vlastná objednávka.
- **Pulls** — jeden riadok = jeden pull. Event sa píše (nie je to tvoja
  objednávka) a ďalší riadok si ho aj s dátumom prevezme.
- **Sales** — riadky sú skutočné lístky, takže „ďalší riadok" znamená
  „prihoď lístky z ďalšej objednávky". Zisk sa ráta hneď v riadku.
- **Events** — jeden riadok = jeden event, takže turné napíšeš na jeden raz.
  Ďalší riadok prevezme kategóriu a krajinu.

### 2. Žiadne vlastné okno — otvára sa nad zoznamom

New Order už nie je stránka. Všetky štyri sú okno **nad** svojím zoznamom a
pod ním vidíš rozmazané ostatné riadky — presne ako to robili Sales a Pulls
predtým. `/orders/new` zmizlo; stará linka ťa pošle na Inventory.

Editovanie sa nemenilo: event upravuješ na jeho detaile, pull v jeho okne,
oba pôvodné formuláre sú nedotknuté.

### 3. Finance: Income vs Expenses je graf z Dashboardu

Dvojité stĺpce sú preč. Je tam **ten istý graf ako na Dashboard → Overview** —
tá istá plynulá krivka, to isté zvýraznenie pri prejdení myšou, tie isté
prepínače. Tri: **Income · Expenses · Net**. Čísla sú tie isté, čo kreslili
stĺpce; veľké číslo nad grafom je súčet za obdobie z kariet nad ním, aby si
nikdy neukazovali dve rôzne sumy.

### Prečo 2.45.0 a nie 2.44.0

Zip s názvom `tiqr-manager-2.44.0.zip` už máš v Downloads a má iný obsah
(finance migrácia, ktorú sme opustili). Aby ti v Downloads neležali dva rôzne
súbory s tým istým názvom, táto verzia ide o krok ďalej.

## 2.43.0 - bočný panel je plochý

Skupina **Tickets** z bočného panela odišla. **Events, Inventory, Sales a
Pulls sú vždy vidieť** — nič sa neotvára a nezatvára, nič sa neskrýva.

Panel má teraz šesť riadkov v poradí, v akom práca beží: Dashboard · Events ·
Inventory · Sales · Pulls · Finance. Settings zostáva dole, kde bol.

**Čo to upratalo popri tom:** s rámčekom zmizol aj rozbaľovací stav, výpočet
„som niekde v tejto skupine" a nepoužitý typ riadku-nadpisu, ktorý nemal ani
jednu položku od 2.36.0. Vykresľovanie panela je jedna vetva namiesto troch.

Otázka, či zvýrazniť skupinu spolu s položkou (2.29.4 → 2.41.0), tým padá
sama: skupina už nie je.

## 2.42.0 - nová objednávka je stránka a riadky

Presne tá kombinácia, ktorú si vybral v preview: **celá stránka**, tabuľkové
**riadky**, pridávanie tlačidlom, **event raz hore**, mena/pull/poznámka
viditeľné.

### Jeden riadok = jedno miesto

Vyberieš event a pridávaš riadky. **Jedenásť polí, nič viac** — ks, typ,
sektor, rad, sedadlá, platforma, cena za kus, mena, pull, poznámka.

**Pull je gulička.** Zhasnutá = nie. Rozsvietená = pullnuté a vedľa vyskočí
políčko na meno. Meno prežije vypnutie.

### Čo nesedí, je vlastná objednávka

Dva riadky sa zlúčia do jednej objednávky **len vtedy**, keď sa zhoduje
všetko — sektor, rad, cena, mena, typ, platforma, pull — **a sedadlá idú tesne
za sebou**. `14-15` a `16` je jedna objednávka 14-16; `14-15` a `19` sú dve.
Cena sa porovnáva ako **peniaze**, nie ako text, takže „201,00" a „201.00" je
tá istá cena.

Tlačidlo dole rovno povie, koľko objednávok vznikne.

### Dátum nákupu sa nepýta

Stavia sa dnešným dátumom, opečiatkovaný raz pri otvorení stránky. Prepísať sa
dá naďalej na detaile objednávky.

### Čo sa už nepýta

**Poplatky a „ostatné náklady"** formulár nezbiera — cena, ktorú napíšeš, je
celý náklad na kus. **Stav platby** zostáva „paid", ako bol predvolený od
2.0.70. **Poplatok za pull** sa zadáva tam, kde vždy patril — na detaile
objednávky.

### Kam to vedie

`/orders/new`. Tlačidlá **New Order** v Inventory aj **New order for this
event** v detaile eventu smerujú sem; starý odkaz so `state` sa presmeruje,
takže nič nekončí naslepo.

Stará modálka `OrderFormModal` zostáva v súbore **nepoužitá, jednu verziu**,
aby sa dalo porovnať. Povedz a zmizne.

## 2.41.0 - jedna vec svieti, o tabuľku menej, panel preč

Štyri veci z tvojho zadania, ktoré sú rozhodnuté a nezávisia na ničom
otvorenom. Všetko frontend — žiadna migrácia, žiadny Rust.

### Svieti len položka

V ľavom paneli sa už nezvýrazňuje skupina **Tickets** spolu s položkou.
Svieti jedna vec: tá, na ktorej si.

**Toto je návrat k tomu, čo bolo pred 2.29.4** — vtedy si písal opak
(„ked mam nieco vybrate v tickets tak tickets niesu oznacene"). Napísané v
`PROTECTED_AREAS.md`, aby to o pol roka nikto „neopravil" späť podľa starého
komentára v kóde.

### Detail eventu: len lístky

Tabuľka **Orders** z detailu eventu odchádza. Objednávka je spôsob, akým si
zásobu kúpil; keď otvoríš event, zaujíma ťa, **čo na ňom máš** — a každý
lístok aj tak nesie kód svojej objednávky.

Tlačidlo **New order for this event** sa presunulo na hlavičku Tickets, takže
cesta dnu je tam, kam sa aj tak pozeráš. Objednávky sa nikam nestratili —
zoznam `/orders` je nedotknutý a karta Listings ich stále dostáva.

### Pravý panel preč

„What this will create" zmizol z **novej objednávky** aj z **nového eventu**.
Prepisoval polia, ktoré si práve vyplnil, o stĺpec vedľa — a kvôli tomu bol
každý formulár dvojstĺpcový. Formulár je zase jeden stĺpec. Komponent
`PreviewPanel` aj `preview` prop na modále sú zmazané, nie len nepoužité.

### Event ukazuje aj dátum

V **novej objednávke** bol dátum v surovom tvare `(2026-12-12)` — teraz je
`· 12.12.2026`. A **filter Event v Sales** dátum nemal vôbec; teraz ho má.
Dva koncerty tej istej šnúry sa inak v zozname nedajú rozoznať.

## 2.40.0 - tichšie stavy, čitateľnejší text, skeletony, návrat na riadok

Prvá dávka z tvojho výberu tridsiatich. Všetko je **frontend** — žiadna
migrácia, žiadny Rust, žiadna zmena v peniazoch.

### 17 · Badge je teraz bodka a text

Stav prestal byť plná farebná pilulka. Zostala **farebná bodka a obyčajný
popis** — bez výplne, bez rámika. V tabuľke, kde má každý riadok stav, tie
pilulky prekričali čísla vedľa seba.

Farby sa nemenili ani o odtieň: `STATUS_TONES` je stále jediný zdroj pravdy,
len sa z neho pri vykreslení vyhodí výplň a rámik (`quietTone`). Bodka je o
stupeň väčšia a v plnej sile, keďže identitu teraz nesie sama. **Ani jedno
volanie sa nemenilo.**

`InlineStatusSelect` si plnú pilulku **necháva zámerne** — je to ovládací
prvok a ten musí vyzerať ako ovládací prvok.

### 18 · Oprava kontrastu — 260 miest

Sivý text v tmavom režime bol na hrane čitateľnosti, a bolo to **prehodené
naopak**: tmavší odtieň na tmavšom podklade. Zmerané **3,16 : 1**; po oprave
**4,16 : 1**.

Zámena `text-slate-400 dark:text-slate-500` → `text-slate-500
dark:text-slate-400` prebehla na **260 miestach v 24 súboroch**. Overené, že
ani jeden výskyt nemal nalepenú predponu (`hover:`, `group-hover:`), takže
zámena nemohla nič iné zasiahnuť. Po nej: starý pár **0×**, správny **350×**.

### 16 · Skeletony namiesto „Loading…“

Zoznamy to vedia od 2.6.0. Dorobené tam, kde ešte zostávalo koliesko nad
prázdnom: **Dashboard** a **všetky štyri karty Finance**. Pribudli dva tvary —
`StatsSkeleton` (riadok kariet s číslami) a `PanelSkeleton` (karta textu alebo
grafu). Stránka si drží výšku a neposkočí, keď dáta dorazia.

V modáloch a krátkych čakaniach vnútri panelov `LoadingBlock` **zostáva** —
tam je koliesko správna odpoveď.

### 20 · Riadok, na ktorom si bol

Prilepená hlavička tabuľky **už v appke bola** (od 2.6.0, `.table-shell thead
th`). Druhá polovica tej karty nie — a tú robí táto verzia: keď sa vrátiš z
detailu, **riadok, z ktorého si odišiel, sa na chvíľu rozsvieti**.

`lib/lastRow.ts`, rovnaký dohovor ako `lastFilters` v Sales: modul-level,
platí do reštartu, databázy sa netýka. Značka sa pri čítaní **spotrebuje**,
takže riadok bliká raz po návrate, nie pri každej návšteve. Zapnuté na
**Events, Inventory a Sales** — v Pulls nie, tie vlastný detail nemajú.
Rešpektuje `prefers-reduced-motion`.

## 2.39.0 - Orders a Inventory sú jedna obrazovka; Margin zaniká

### Jedna obrazovka, volá sa Inventory

Orders a Inventory boli **dva zoznamy nad tými istými riadkami** — obe volali
`api.listOrders`, obe vykresľovali objednávky, obe viedli na `/orders/:id`.
Líšila sa len sada stĺpcov. Teraz je to jedna položka v menu, **Inventory**.

Prežil `/orders` (bohatšia stránka — New Order, úpravy, hromadné akcie) a
dostal názov Inventory. `/tickets` naň **presmeruje aj s query stringom**,
takže staré odkazy fungujú — vrátane `?code=` z detailu eventu, ktoré Orders
nájde, lebo jeho vyhľadávanie matchuje aj kódy lístkov.

Čo sa prepojilo: odkazy z Dashboardu (×2), z detailu eventu (×2), spätný odkaz
z detailu objednávky (už nevetví — vždy „Back to inventory") a prehliadka.
Prehliadka mala **dva kroky na `/tickets`**; jeden by teraz ukazoval tú istú
stránku dvakrát za sebou, takže sa zlúčil do kroku o objednávkach (veta o
listing price sa presunula doň). Zostáva 11 krokov a Guide si to číta sám.

`pages/Tickets.tsx` **nie je zmazaný** — SaleDetail a OrderDetail z neho
importujú `DELIVERY_STATUS_OPTIONS`, `RESALE_STATUS_OPTIONS` a
`TicketEditModal`. Odišla len jeho route a položka v menu.

### Margin zaniká

Marko: „margin vsade kde je uplne ju odstranme aj s widgetov proste ako keby
zanikla." Odišla odvšadiaľ, kde ju bolo vidieť:

- **Dashboard → Financials:** karta „Margin", aj podriadok pri Profite (ten
  teraz ukazuje len ROI).
- **Dashboard → porovnanie období:** riadok Margin.
- **Detail eventu:** karta Margin.
- **Detail predaja:** karta Margin (súhrn je päť kariet namiesto šiestich,
  mriežka sa zúžila s ním, nie je tam diera) aj výpočet.
- **Texty**, ktoré o nej hovorili — prehliadka (dva kroky) a Guide v Settings —
  hovoria o zisku, nie o marži.

**ROI zostáva.** Backend `margin` ďalej počíta a posiela, len to už nikto
nečíta — meniť `finance.rs` kvôli zobrazeniu by znamenalo siahať na chránený
finančný modul bez možnosti to tu preložiť.

`components/Recap.tsx` má maržu tiež, ale ten je od 2.38.0 nedostupný (nič ho
neimportuje), takže na obrazovku sa nedostane.

## 2.38.0 - riadok filtrov, vlastný kalendár, tenšie Settings

### Prepínač kariet ide do riadku filtrov

`Upcoming / Completed` (a `Pending / Completed`) už nesedí na vlastnom
riadku nad filtrami — je **vpravo v tom istom riadku, kde je Search a
Sort**, presne ako to má Pulls. Platí na **Events, Orders, Inventory aj
Sales**. `TabSwitcher` si prestal nosiť vlastný `mb-4`; tie dve miesta,
ktoré stoja samostatne (detail eventu), si ho píšu samé.

### Sales

- **Currency filter preč.** Dátumový rozsah **Od / Do** sa posunul presne
  na jeho miesto.
- **Refund status preč úplne** — a s ním aj tlačidlo „More filters", pod
  ktorým to bola jediná vec.
- **Jeden status na riadok.** Badge `Paid / Pending` zmizol, **guličky
  zostali** (Sold · Deliv. · Paid) — riadok hovoril to isté dvakrát. Riadok
  „N/M refunded" sa presunul pod guličky, nezmizol: je to jediné miesto,
  kde zoznam vôbec povie, že sa niečo vrátilo.
- Stĺpcov je desať namiesto jedenástich, obe `colgroup` prepočítané na 100.

### Inventory

Stĺpec **Sold preč**, zostáva **Total** a **Available**. Nič sa
neprepočítavalo — Total je stále počet kusov, Available stále
`available + listed`, takže predané je čitateľné ako rozdiel medzi nimi.
Obe `colgroup` prepočítané (8 stĺpcov naširoko, 6 nasúzko) a **Event pustil
kus zo svojich 43,5 %**, aby sa Seats a Purchase date zmestili celé.

### Vlastný kalendár, všade

`<input type="date">` doteraz otváral **kalendár prehliadača** — biely
panel, vlastné písmo, o tmavom režime appky nevie nič. Je to chrome
prehliadača, CSS sa k nemu nedostane. Takže je nakreslený vlastný:
mesiac + šípky, pondelok prvý, šesť riadkov (výška panela sa nemení),
dnešok zvýraznený, `Today` a `Clear` dole, otvára sa hore alebo dole podľa
toho, koľko je miesta. **Žiadna nová knižnica.**

`Input` posiela `type="date"` doň sám, takže **ani jedno z ~30 miest sa
nemenilo** — stále posielajú `value` ako `YYYY-MM-DD` a stále čítajú
`e.target.value`. Tri miesta, ktoré si kreslili `<input>` ručne
(Dashboard ×2, detail objednávky), idú teraz cez `Input`.

### Settings

- **Insights preč úplne.** Bola to jediná cesta k Recapu, takže z appky
  odchádza aj Recap; `components/Recap.tsx` zostáva na disku nedotknutý,
  len ho už nič neimportuje. Zmazať ten súbor je samostatné rozhodnutie.
- **Import CSV + Export CSV = jedna karta „CSV"** s dvoma riadkami.
  Handlery sú do písmena tie isté.
- **Guide prerobený.** Gradientový hero preč — bol to najhlasnejší prvok v
  Settings a hovoril najmenej. Zostalo to isté, čo niesol: tlačidlo na
  prehliadku a **tri veci, ktoré si nový používateľ pomýli**, teraz ako tri
  krátke bloky vedľa seba. Počet krokov sa berie z prehliadky samotnej
  (`TOUR_STEP_COUNT`), nie z čísla napísaného v texte.
- **Posledný krok prehliadky** mieril na `/settings/insights`. Neznáma
  sekcia ticho spadne na prvú, takže by prehliadka skončila na Lookups a
  rozprávala o recapoch, ktoré už neexistujú. Končí na Support, odkiaľ sa
  spúšťa.

### Čo to znamená pre refundy

Po 2.35.0 (tlačidlo Refund) a tejto verzii (filter Refund status) **už v UI
nie je ani jedna cesta k refundu — ani ho spraviť, ani si ich vyfiltrovať.**
Dáta aj logika sú nedotknuté, refundované kusy sa stále počítajú v riadku
predaja aj v súhrne dole. Píše sa to sem, aby to nebolo prekvapenie.

## 2.37.0 - dizajnový jazyk 3.0, prvá vrstva

Schválené na celoapkovom náhľade. Toto je **vrstva tokenov** — komponenty a
texty idú ďalej.

### Hĺbka tónom, nie rámikom

Tmavý koniec sivej rampy je prerezaný: **950 `#08080b`** je podklad appky,
**900 `#101016`** karta, ktorá nad ním sedí, a **800 `#1e1e27`** vlásočnica
namiesto rámika. Povrchy v `index.css` to nasledujú.

Karta dostala späť **veľmi jemný tieň** — jednu vrstvu, pätinu toho, čo tam
bolo pred 2.29.2. Vtedy som tiene odstránil, lebo appka vyzerala 3D. Opačný
problém — všetko rovnako ploché — rieši toto.

### Hlavičky tabuliek stratili verzálky

`.th`, `.th-c`, `.th-c-narrow` aj `.section-title` idú do normálneho textu,
váha zo `semibold` na `medium`.

**Rozmery som nechal.** `PROTECTED_AREAS.md` hovorí, že `px-1` a `text-[11px]`
boli merané proti reálnym dátam v troch jazykoch, aby nikdy nevyskočil
vodorovný posuvník. Zrušenie verzálok text **zužuje, nikdy nerozširuje**,
takže tá záruka platí ďalej.

### Kontrast — nameraný, ale neopravený

`slate-400` sa posunul zo **4,44 na 4,56** na bielom. Krok pre tlmený text tak
prvýkrát prechádza hranicou 4,5 vo svetlom režime.

**Tmavý režim sa v rampe opraviť nedá.** Žiadna hodnota nevyhovie na bielom aj
na `#101016` naraz — musela by byť zároveň tmavšia aj svetlejšia. Pár teda
musí byť rôzny podľa režimu. A tu je problém:

**Prevládajúci zápis v appke to má obrátene.** `text-slate-400
dark:text-slate-500` je na **258 miestach** a na tmavšie pozadie volí tmavší
krok — vyjde **3,16:1**. Správny tvar `text-slate-500 dark:text-slate-400` je
na 85 miestach a dáva 4,16:1.

Oprava je mechanická výmena 258 výskytov naprieč všetkými stránkami. To si
zaslúži **vlastnú verziu**, nie aby sa to prišilo k zmene tokenov.

### Čo z 3.0 ešte príde

`Badge` z vyplnenej pilulky na bodku + text (bodka tam už je a dedí farbu),
prekreslenie `Skeleton` a `EmptyState`, texty prázdnych stavov po stránkach,
a tá kontrastná výmena.

## 2.36.0 - Pulls patria k lístkom, akcent je fialový

Oboje si schválil na interaktívnom náhľade skôr, než som sa dotkol jediného
súboru.

### Navigácia

**Pulls sú v skupine Tickets**, piate za Sales. A nadpis **„Market & money"
zanikol** — po presune Pulls by stál nad jedinou položkou, čo nie je kategória,
ale ozdoba.

Finance je samostatná položka najvyššej úrovne a **pod Tickets zámerne
nepatrí**: pokrýva peniaze, ktoré s lístkami nemajú nič spoločné — nájom,
poplatky, osobné výdavky — a má vlastné štyri taby, kategórie aj účty.

**Žiadna routa sa nezmenila.** `/pulls` je stále `/pulls`, takže staré odkazy,
krok sprievodcu aj návraty z detailov fungujú ďalej. Zmenil sa jeden súbor:
`Layout.tsx`.

### Farba

Ružová z 2.34.0 je preč, rampa sa presunula na odtieň 253 — tá levanduľová
rodina z Onyxu, o stupeň hlbšie. Presne to, čo si vyklikal v náhľade.

**Dva kroky som nedopočítal, ale doladil — a oba kvôli kontrastu, nie vkusu:**

* **600** nesie po celej appke biely text (`bg-brand-600`) a má **9,1:1**.
  Predtým mala ružová 4,6:1, čiže toto je výrazne lepšie čitateľné.
* **400** je v tmavom režime farba odkazov a akcií (`dark:text-brand-400`).
  Keby som len posunul svetlosť, vyšlo by **4,05:1** na povrchu karty — pod
  hranicou 4,5. Nastavil som ho na **5,3:1**.

Sivá rampa ostala nedotknutá — už predtým mala fialový nádych a k novému
akcentu sadá.

### Čo som nerozhodol za teba

Tri veci z design review v náhľade, ktoré stále čakajú:

1. Skupina sa volá **Tickets** a je v nej položka **Inventory** — dve
   lístkové mená v jednom rail-i. Premenovať skupinu?
2. **Zabalenie skupiny teraz schová Pulls.** Predtým bol vždy viditeľný.
3. **Inventory stále vypisuje aj predané a refundované** lístky — filter
   odišiel s `/inventory` v 2.35.1.

## 2.35.1 - Inventory Intelligence preč, dve stránky lístkov sa zlúčili

### Inventory Intelligence

Celý blok zo stránky eventu je preč. **A s ním aj filter lístkov** — každé
kliknutie, ktoré ten filter zapínalo, bolo vnútri toho bloku, takže by sa už
nedal zapnúť a pás „Showing: ..." by sa nikdy neukázal. Tabuľka lístkov pod
tým zase vypisuje všetky.

Backend som nechal: `get_inventory_intelligence` aj `inventory_intelligence.rs`
existujú a sú zaregistrované. Zmizla len obrazovka.

### Tickets a Inventory sú jedna položka

Boli to vždy tá istá stránka — Inventory bol `TicketsView` so zamknutým
filtrom. Teraz je v sidebare jedna položka **Inventory**, a routa, ktorá
prežila, je `/tickets`. Preto ti fungujú staré odkazy aj návrat z detailu
objednávky.

Prerobil som **všetky** odkazy na `/inventory`, inak by viedli do prázdna:
krok sprievodcu, dlaždicu „Missing listing price" na Dashboarde (aj riadok, aj
kartu) a vetvu návratu v detaile objednávky vrátane popiskov.

### Jedna vec na rozhodnutie

Keďže `/inventory` zmizol, **nikto už neposiela `lockedStatus`** — a tým sú
tie dve vetvy z 2.35.0 nedostupné: filter „available + listed" a tých sedem
stĺpcov, ktoré si pre Inventory vypísal.

Nezmazal som ich. Ak má tá premenovaná stránka ukazovať len predajný sklad a
nie všetky lístky vrátane predaných, je to jeden prop a vrátim to. Povedz.

## 2.35.0 - stĺpce, gulôčky a opravené selecty

### Stĺpce podľa tvojich zoznamov

**Sales:** Sale, Event, Platform, Event date, Seats, Tix, Cost, Revenue,
Profit, Status, Completed. Fees a Margin/ROI sú preč, Cost je pred Revenue —
riadok sa číta náklad → čo to prinieslo → čo zostalo.

**Detail predaja:** Ticket, Order, Seat, Cost, Sale price, Profit a tri stavy.
Fees preč, Cost prvý.

**Inventory:** Event, Purchase date, Seats, Total, Available, Total cost,
Status. Order a Sold vypadli **len tam** — Inventory a Tickets zdieľajú jeden
komponent, takže som to podmienil cez `lockedStatus`, na ktorom ten súbor už
aj tak rozlišuje. Tickets ostalo nedotknuté. Dátum a sedadlá sa v Inventory
prestali skrývať pri úzkom okne, lebo po dvoch stĺpcoch menej je miesto.

### Tie polia na výber na Macu

Príčina nebola šírka stĺpca. `InlineStatusSelect` je `inline-flex` okolo
`<select>` s `appearance-none` — a taký select sa roztiahne na **najširšiu
položku**, nie na tú vybranú. Keďže každá bunka má `whitespace-nowrap`,
„Not delivered" vytlačilo celú pilulku cez susedný stĺpec. Preto sa ti tri
prekrývali.

Teraz má obal `min-w-0 max-w-full` a select `w-full min-w-0 truncate` —
zmestí sa do bunky a v najhoršom prípade sa oreže tromi bodkami. Stĺpce som
rozšíril tiež, ale to bol príznak; toto je príčina.

### Gulôčky sú teraz všade rovnaké

Vytiahol som ich do jedného zdieľaného komponentu `StatusDots`. Berie **ten
istý zoznam podmienok**, ktorý si každá stránka už aj tak stavia pre odznak —
takže bodky a odznak nemôžu tvrdiť nič rozdielne.

**Pulls** má dve (Paid, Transferred) a dajú sa klikať. **Sales** má tri (Sold,
Delivered, Paid) a klikať sa nedajú, lebo sú odvodené — kliknutie by bolo
klamstvo. Vzor je rovnaký, počet sa riadi dátami.

**Ako sa to číta bez hoverovania:** hlavička stĺpca ich menuje v poradí
(„Paid · Done", „Sold · Deliv. · Paid"), zelená znamená hotové, sivá nie.
Hover vypíše každú slovom.

### Refund

Odstránil som ho, ako si chcel. **Ale bol to jediný spôsob, ako sa v appke
dalo refundovať** — `OrderDetail.tsx` to má priamo v komentári. Všetko za ním
(dialóg, príkaz, stav „refunded", blok Refunded) ostáva nedotknuté, zmizlo len
tlačidlo. Vrátiť ho je ten jeden blok.

### Finance

Taby sú vedľa nadpisu, nie pod ním. O riadok nižšia hlavička.

## 2.34.1 - font naspäť

Mono si videl a nechcel. Stack je **presne ten, čo tam bol** — bajt po bajte
rovnaký ako vo všetkých verziách po 2.33.0 vrátane. Overil som to diffom proti
2.33.0 zipu, nie od oka.

Ružová (Vapor) a graf s plochou ostávajú, tých si sa netýkal.

Jedna vec z 2.34.0 platí ďalej a nechal som ju napísanú v kóde aj v
`CURRENT_STATE.md`, nech ju niekto o pol roka nehľadá znova: **`"Inter"` na
čele toho stacku sa nikde nenačítava.** Appka vždy kreslila systémovým písmom.
Nechal som ho tam, lebo jeho odstránenie by na obrazovke nezmenilo nič.

## 2.34.0 - Vapor

Vybral si si: Vapor, Spline Mono hrubší, graf Plocha, Finance Mix. Tri zo
štyroch sú vonku, štvrtá nebola treba.

### Celá appka je ružová

Levanduľa je preč. Prepísal som dve rampy v `tailwind.config.js` — `brand` na
ružovú, `slate` z modrosivej na fialovosivú pri rovnakých stupňoch svetlosti —
plus štyri hexy v `index.css`. Nič iné som nechytal: každý `bg-brand-600` a
`text-slate-400` naprieč appkou si nové hodnoty vezme sám.

`brand-600` som **nedal na najkrajšiu ružovú, ale na tú s kontrastom** — biely
text na nej má ~4,6:1. Sýty koniec rampy je na 400/500, kde ho tmavý režim
používa ako text na tmavom, nie ako text na akcente.

### Font: zlá správa a dobrá

**Zlá:** Spline Sans Mono do appky poslať nejde. TIQR neťahá žiadny webový font
a nemá zabundlovaný ani jeden — stiahnuť ho z Googlu by rozbilo offline chod.

**Dobrá:** pri hľadaní som zistil, že stack viedol `"Inter"`, ktorý **nikde
nie je**. Žiadny `@font-face`, žiadny súbor, žiadny odkaz. Appka celý čas ticho
padala na systémové písmo. Takže si doteraz Inter nikdy nevidel.

Teraz je tam **systémový mono** — SF Mono na Macu, Consolas na PC. Vyzerá
takmer ako Spline Mono a nepotrebuje sieť.

### Graf je plocha

`MetricChart` kreslí pod krivkou výplň s prechodom. Používa **tú istú cestu**
ako čiara, takže výplň nikdy nemôže ísť inokade než ťah nad ňou. Základňa je
nula — na P&L grafe je to tá čiara, na ktorej záleží.

### Finance som nechal tak, a je to zámer

Vybral si Mix. `finance/Overview.tsx` **ten tvar už má**: KPI pás, druhý KPI
pás, pod tým dvojstĺpec s rozpadom kategórií a grafom príjem/výdaj. To je
presne poradie z Mixu.

Rozdiel oproti ukážke sú proporcie, nie štruktúra. Prestavovať 591 riadkov
naslepo, bez možnosti zbuildovať, za kozmetický zisk sa mi nezdalo. Je to
odložené, nie prehliadnuté — povedz a spravím to ako ďalšiu verziu.

## 2.33.0 - menej obrazoviek, menej stĺpcov

### Events má osem stĺpcov

Event, Date, Status, Tickets, Stock, Cost, Revenue, Profit. **Days, Available,
Margin a ROI z tabuľky zmizli** — čísla nezanikli, sú ďalej na stránke eventu.
Preč je aj **farebný pruh kategórie**, ktorý pridala 2.32.0 na ľavú hranu
riadku; odznak kategórie vedľa názvu ostáva. Obe colgroupy sú teraz rovnaké —
keď zmizli Margin a ROI, širokému režimu nezostalo čo pridať.

### Price Checker, Ticket Center a Calendar sú preč z appky

Routy, položky v sidebare aj tri súbory stránok sú zmazané. S nimi aj všetko,
čo na ne ukazovalo:

* celá karta **„Market vs. mine"** a odkaz „Open in Price Checker" na stránke
  eventu,
* tlačidlá **„Check prices"** na detaile objednávky aj predaja,
* vetva návratu na **`/ticket-center`** z detailu objednávky,
* box **MARKET ATTENTION** na Dashboarde,
* dva kroky sprievodcu (zo 14 na 12) — a dva ďalšie som preformuloval, lebo
  sľubovali kalendár, ktorý už neexistuje.

### Z databázy som nezmazal nič

**Toto si prečítaj, je to jediné rozhodnutie, ktoré som spravil za teba.**
Povedal si „kompletne odstrániť" — odstránil som **obrazovky**, nie dáta.
Rust príkazy (`calendar.rs`, `price_checker.rs`, scanner aj analysis) ostávajú
a sú ďalej zaregistrované, `api.ts` ich ďalej ponúka, a tabuľky
`price_checks`, `price_check_tiers` a `event_marketplace_links` držia každý
riadok, ktorý držali.

Dôvod je jednoduchý: história skenov sa po zmazaní už nedá vrátiť. Vrátiť
obrazovku naopak stojí jednu routu a jednu položku v sidebare. Ak naozaj
chceš zmazať aj dáta, povedz to zvlášť a spravím migráciu.

## 2.32.0 - what the table shows you

Z tvojich 31 vybraných návrhov mala táto verzia priniesť trinásť čisto
zobrazovacích. Sedem z nich je vonku.

### Päť si už mal

Toto nie je výhovorka, je to výsledok čítania kódu pred písaním:

* **Miesto pod názvom eventu** — `Events.tsx` už vykresľuje `[venue, city]`
  ako druhý riadok pod menom.
* **Poznámka v Pulls** — „More info" je vlastný stĺpec už dávno.
* **Farba podľa druhu v kalendári** — `KIND_ACCENT` pokrýva všetkých sedem
  druhov.
* **Čipy filtrov v kalendári** — `activeKinds` a prepínacia lišta existujú.
* **Minulé/budúce v Events** — `EVENT_TABS` s `upcoming`/`completed`.

### Čo pribudlo

**Events** dostali stĺpec **Days** („3d", „today", „passed"), **pruh kategórie**
po ľavej hrane riadku a **Stock** — predané voči kúpeným ako pásik. Pruh berie
farbu z tej istej palety ako existujúci odznak kategórie, takže kategória nikdy
nemôže byť jednou farbou ako pilulka a druhou ako pruh.

**Pulls** majú Paid a Done v **jednom stĺpci ako dva body**. Obidva ostávajú
klikateľné — bodka má 10 px, ale tlačidlo okolo nej má normálnu veľkosť, lebo
toto odškrtávaš často. Tabuľka tým vrátila šírku textovým stĺpcom.

**Price Checker** označí sken starší než 7 dní jantárovo. Relatívny čas tam bol
aj predtým, ale čítal sa rovnako po hodine ako po mesiaci.

**Kalendár** má vedľa názvu obdobia súčet — „4 events · 7 deadlines". Počíta z
`visibleEntries`, takže keď vypneš druh vo filtri, číslo sa zmení tiež.

**Riadky tabuliek nabiehajú zhora nadol.** Jedno CSS pravidlo na `.table-shell`,
osem krokov a potom už nie — pri 200 riadkoch by si inak čakal na animáciu dlhšie
než na dáta. Systémové „obmedziť pohyb" ju vypína úplne.

### Nebezpečnú zónu som nepostavil

V appke nie je čo do nej dať. Neexistuje príkaz na zmazanie všetkého ani
továrenský reset. Jediné, čo prepisuje dáta, je Obnova a Sync down — a obidve
už majú vlastný `danger` potvrdzovací dialóg. Postaviť tú sekciu by znamenalo
buď prelepiť záchrannú akciu ako hrozbu, alebo dorobiť mazacie tlačidlo, ktoré
si nikdy nechcel. Ani jedno.

**Seats ostávajú ako boli** — pásik sedadiel (`t1`) si zrušil, vypadol z plánu
úplne.

## 2.31.0 - a pull is paid AND transferred

Marko: "do pulls taktiez mali by byt 2 veci dane co sa daju checknut a to je
payment ci uz zaplatili a + transfer ale ten tam uz je."

### The second checkbox

Pulls (Given) has a **Paid** column next to **Done**, tickable straight from
the list exactly like the transfer one. It answers the other half of a pull:
has the buyer actually paid the fee? The two are independent on purpose - they
finish in either order, and "transferred but not paid" is the state worth
seeing. Both are also correctable from the edit form.

`paid` is its own column with its own `paid_at` stamp (migration 030), shaped
exactly like `transfer_done`/`transfer_done_at`. One helper now applies the
timestamp rule for both: stamp when it really flips off->on, clear when it
flips back, and never touch it on a plain re-save.

### Completed now means both

**Every pull you already have will read "Not paid" until you tick it** -
including ones showing "Completed" today. That is the new meaning, not lost
data: nothing knew about payment before this version, so there was nothing to
carry over. The hover on the badge spells out which half is missing.

### The Google Sheet

Untouched. It has a `Transfer` column and no `Paid` one, so the sync neither
reads nor writes payment - it carries the app's own value through instead,
which is what stops a sync from quietly un-ticking a paid pull. Adding a real
`Paid` column would mean changing your live sheet, so that is your call.

**Not included, say if you want them:** a Paid filter next to the existing
Transfer filter, and payment tracking on Pulls (Received) - that tab has no
transfer flag either, so there was nothing to pair one with.

## 2.30.1 - Settings says less

Marko: "zjednodusti celu data sekciu a jednodusit vsetko co je v nastaveniach
nech je to minimalisticke."

### Twenty blocks of prose, cut

Nothing moved, nothing was removed, nothing changed what it does - only the
text around the controls. Cloud sync went from four sentences to one line. The
CSV importer no longer lists sixteen column names on screen; the template file
it tells you to download is where those actually get read. Export, Backup,
Drive revisions, restore points, the two Google sign-in notes, the Sheets URL
note, the Anthropic key, ntfy, Notifications, the guide header, Suggest a
change, both category pickers and all eight section descriptions are each a
line or two now.

What was deliberately KEPT, because each one changes what you would do:

* an import is all-or-nothing;
* notifications only fire while the app is running, one per category per day;
* an ntfy topic is a shared secret - anyone who knows it reads your alerts;
* duplicates are never flagged on import;
* a restore or a sync down replaces this computer's data, and takes a backup
  first;
* both-computers-changed: combining keeps everything, the other two throw one
  side away.

### The counter bug 2.30.0 left behind

Found while writing this up, fixed here, and it is the more important half of
this release. `reconcile_counter` exists so a merge can never leave a code
counter behind the codes the table actually holds - the failure it prevents is
silent for days and then stops order creation dead on a UNIQUE constraint.
2.30.0 started minting from per-event counters (`order:CELINE`) and did not
teach that function about them, so the exact failure it was written to prevent
came back one counter row over: `CELINE-004` arrives from the other machine,
this machine's `order:CELINE` still reads 3, and the next Celine order collides.

It now raises every event counter a table's codes imply, in one statement, and
there is a test named for that scenario. `counters` stays out of the merge on
purpose - each machine owns its own numbering.

**Known and left alone:** when two machines mint the *same* code for one event
at once, the arriving one is still re-numbered with the old fixed prefix
(`ORD-000092`), not the event's. It is rare, it is harmless, and making it
event-derived means a different event lookup per table - not something to do
inside a text-simplification release.

## 2.30.0 - CELINE-001: codes that say what they are

Marko: "ked je to celine dion tak celine-001 ... bolo by to lepsie zmapovatelne
nez to ako to je teraz", and "nech je vsade rovnaky - nie je jedno miesto ma 91
a druhy order 0091".

### One format, everywhere

`shortCode` is gone. 2.23.0 rendered ORD-000014 as `#14` in three list views
and nowhere else, which is exactly the "91 in one place, 0091 in another" he
reported. Every screen now shows the same string.

### Codes come from the event

`ORD-000084` becomes `CELINE-001`. Orders, tickets, sales, pulls and received
pulls all mint from the event they belong to - a sale through its ticket, a
pull off the event name it stores. Each kind counts from 1 within an event, so
an event's second order is CELINE-002.

The prefix rule is pure and deterministic, because **both machines mint
independently from synced counters** - if it ever answered differently on two
machines they would produce two codes for one record. It keys on the part
before the app's own " · " separator, so two Oasis nights share a run of
numbers; folds accents (Letná -> LETNA); and caps at eight characters. Run
against marko's real event names: OASIS, MILEY, CELINE, ENGLAND, COLDPLAY,
EAGLES, GARTH, ACDC, VBV08 and VBV10 - no collisions.

**An event whose name yields nothing usable keeps its old code.** No invented
prefix - `code` is NOT NULL UNIQUE everywhere, so "no prefix" can never mean
"no code".

### Existing records are rewritten too

Migration 029 adds `legacy_code`; the rewrite itself runs in Rust, tied to that
migration so it happens exactly once, on the same all-or-nothing path as every
other schema change. Numbering is by `id` within each prefix, so both machines
rewriting their own copy of the same data land on identical codes.

**Your Google Sheet keeps working.** It matches rows back by the code in its
"TIQR ID" column, and every one of those is now a stale ORD-000091 - without a
fallback the sync would reject the lot as "does not match any order in the
app". The matcher now tries the current code first and falls back to
`legacy_code`.

### The one thing this needed inside protected logic

A sale group took its identity from `MIN(s.code)`. That is the same row as
`MIN(s.id)` for every code this app has ever minted, because ids and codes
ascended together - but a batch spanning two events (which the
`COUNT(DISTINCT t.event_id) = 1` guard exists to handle) would have shown
COLDPLAY-003 as the group's code while `batch_id` said OASIS-001: identified by
one string, displayed under another. The group now takes its code from the
MIN(id) row. Verified against a real mixed-event batch - the old expression
returned COLDPLAY-003, the new one OASIS-001, matching batch_id. `batch_id`
itself is untouched.

### Verified by running it

The prefix rule and the whole backfill were executed against a real SQLite
database, not read: accents folded, per-event numbering correct, the unnamed
event left alone, `legacy_code` preserved, and the counters advanced so the
next Celine order is CELINE-003 rather than a collision.

**Before you update, sync both machines.** The rewrite is deterministic on
identical data; if the two have drifted they will each renumber their own copy
and the merge will reconcile by `uid` afterwards, which works but is noisier
than it needs to be.

## 2.29.5 - Ticket Center stays put, and Finance explains itself

### Ticket Center

The search row and the four category tiles now stay on screen and only the
table scrolls. `.table-shell` already scrolls inside itself under a sticky
header - it was just using the default 13.5rem inset, which assumes ordinary
page chrome. Ticket Center has a filter row AND four tiles above the table, so
the shell ran past the bottom of the window and the PAGE scrolled instead,
taking the tiles with it. Measured against the real chrome.

### Finance: the thing that looked like bad arithmetic

I audited it by running the real queries against a real database with edge
cases rather than reading them, and **the arithmetic is correct**: a
future-dated entry is correctly excluded from a balance, a transfer is
subtracted from one account and added to the other from a single row, inactive
and non-EUR accounts stay out of the EUR totals. Every figure came out exact.

What is NOT obvious is this: **an entry's account is optional**, and the three
screens then quietly use three different populations of the same rows.

- Overview's *Income / Expenses* count **every** EUR entry
- **Account balances** only move for entries that **have** an account
- **Reports** skips account-less entries outright

Each is right for the question it answers, and nothing on screen said so — so
Expenses and the balances underneath simply refused to reconcile, which reads
exactly like a miscount.

Fixed by **naming the difference, not by changing which rows count** — altering
that would silently move a number marko has been reading. Overview now prints,
in the same style the non-EUR note already uses: how many entries in the period
have no account, and the exact net difference they cause between that block and
the accounts above.

## 2.29.4 - Seven fixes from marko's own screenshots

- **Orders**: the *Tier / Level* field is gone from the New order form, and
  *Supplier* is gone from its preview. `tier` is still sent (null unless the
  AI import panel fills it) and `tickets.tier` still exists, so nothing
  downstream changed shape - only the field he never filled in.
- **Events**: *Artist / team* removed from the preview. It was showing a row
  for a field the form does not even offer.
- **Sidebar**: when a page inside the Tickets group is open, the **group now
  marks itself** too, so the rail shows both where you are and which group it
  belongs to. It was only opening, never highlighting.
- **Sidebar and segmented tracks are darker.** New `--surface-sunken` token:
  darker than the page, for chrome that sits UNDER the content rather than on
  top of it. Both were using the lighter muted surface.
- **Price Checker**: the whole selection mechanism is gone - the "Selected: N"
  bar, "Select all", and the per-row checkbox. It never led anywhere a single
  click did not: the scanner opens one visible window that marko drives
  himself, so "Check selected" could only ever open the FIRST one anyway.
- **Finance transactions now sort by when things actually happened.** Sorting
  on the date alone returned 0 for everything on the same day, and a
  comparator that returns 0 leaves rows in whatever order the two lists were
  concatenated - every entry, then every transfer. A morning transfer sat
  below an evening entry, and the order shifted whenever either list grew.
  Broken by `id` descending (the closest thing to a time this data carries -
  the date columns have no clock), with a final stable tiebreaker.

Removing the Price Checker selection also orphaned a `toggle` helper that
still called the deleted `setSelected` - caught and removed before shipping;
that one would have failed the build.

## 2.29.3 - BUILD FIX (a comma)

`tsc` stopped on `Events.tsx(24,1): Identifier expected`. My script added
`PreviewPanel` to an import list that already ended in a trailing comma, so the
file got `Textarea,` followed by `, PreviewPanel }` - a double comma. Both
import lists are now written out properly, one name per line, like the rest of
the file.

**The check that was missing now exists.** Brace and JSX balance both sail
straight past this: it is a token-level error, not a structural one, and the
file balances perfectly while failing to parse. There is now a pass that reads
every named-import list in `src` and requires each specifier to be a plain
identifier - it reports zero across the tree.

Version stays **2.29.3**: the failed build published no release, so the updater
has never seen this number. Same call as 2.16.0, 2.24.0 and 2.27.0.

## 2.29.3 - The layouts, not just the paint

Marko was right: 2.29.0-2.29.2 changed how the app is COLOURED and never
changed how the create forms and Settings are ARRANGED. The preview he approved
had split-preview forms and a sections rail; the app still had plain dialogs and
a tab strip.

### Split preview

`Modal` gained an optional `preview` slot. When a form passes one, the dialog
becomes two columns: the form on the left, **what it is about to create** on the
right, updating as he types. The preview column is sticky, so on a long form the
result stays on screen while he works down it. Under `lg` it stacks, where
side-by-side would squeeze both halves.

Optional on purpose - every one of the app's other modals renders exactly as it
did, and only the create forms opt in.

**Wired so far: New order and New event.** Both read the form's OWN state -
the order preview uses `summary`, the same memo the cost bar already used - so
the panel cannot disagree with what `submit()` actually sends. The order preview
also states the per-ticket cost, which is the thing that split is for.

**Not wired yet: New sale, New ticket, New pull, finance entries.** Each needs
its own few lines against its own state; the slot is there and the pattern is
set, but I am not claiming work I have not done.

### Settings: sections down the left

The tab strip across the top becomes a rail on the left, as in the preview.
Routes are untouched (`/settings/:section`), so every existing deep link - the
sidebar's "Account settings", the Dashboard's update pill - still lands exactly
where it did. Under `lg` it falls back to a horizontal scroller rather than
eating half a narrow window.

### Also

`border-line` and `bg-line-soft` exist now: with a hairline drawing every edge
in the flat design, that was worth a token instead of repeating
`border-slate-200 dark:border-slate-800`.

## 2.29.2 - Flat: the lighting comes off

Marko: "odstranme tu taku 3d svetlo co je za tym, nech to je ako keby bez
efektov jednoliate... tie farby take tmavsie aby mali lepsi kontrast."

### The extrusion is gone

Every surface was lit by a two-shadow pair. That pair is now **a 1px ring**.
One substitution in `--sh-card` / `--sh-raised` / `--sh-inset` flattens every
card, table shell, dialog and field in the app at once — because once the light
stops drawing an edge, the edge still has to exist, and a hairline does it
without adding depth.

**One exception, on purpose:** `--sh-overlay` keeps a real drop shadow. A
dialog genuinely floats over a dimmed page, and with no shadow at all the modal
and the backdrop merge into one dark mass.

The gestures that only made sense while things were extruded went with it: the
sidebar's current item is **tinted** rather than pressed in, the chosen tab is
**filled with the accent** rather than lifted, and a selected card gets a brand
**ring** rather than a dent.

### Darker, with the contrast put back

With the lighting gone, every bit of separation has to come from the fills and
the lines themselves, so the ramp was rebuilt for that job rather than for
softness: the dark ground drops to `#0d0e12`, the surface steps well clear of
it at `#15171d`, and `slate-800` is **lifted** so a hairline is actually visible
against a panel — that hairline is now what draws every edge in the app.

Light mode goes back to a white surface on a light grey ground, which is the
highest-contrast pairing there is and no longer contradicts anything: the "one
sheet of material" rule that made white wrong in 2.29.1 was a property of the
soft design, and the soft design is what just came off.

### Not changed

No logic, no schema, no migration (next is still 029), no new dependency. Every
figure on every screen is the same figure.

## 2.29.1 - Onyx, second pass: the surfaces the first pass missed

Marko, on 2.29.0: the colours are not nice in the layout, New order / pull and
the settings are not changed, and it looks different in the real app than in
the preview. All three had the same root cause, and my 2.29.0 note that this
was "a two-file change" was wrong.

### `bg-white` was painted in 39 places

`ui.tsx` does not use `.card` — **Modal, ConfirmDialog, the shared panel, the
secondary button, the tab strip and the empty state each define their own
surface inline**, as `border border-slate-200 bg-white`. So every dialog in the
app — New order, New pull, New sale, every Settings dialog — kept its old flat
white box while everything around it went soft. Twenty-five more literals sat
across fifteen page files.

Fixed properly rather than one class at a time: a semantic **`surface`** colour
now resolves to the same `--surface` variable index.css already sets per theme,
and every `bg-white` became `bg-surface`. `text-white` and
`ring-offset-white` were deliberately left alone — those are genuinely white.

That is also the whole "looks different in the app" answer: on the new grey
ground a pure white card is the exact opposite of one sheet of material.

### The ground and the surface are no longer the same colour

2.29.0 made them identical, which is soft-UI orthodoxy and reads **washed out**
in light mode. The card is now a step LIGHTER than the page — which is what a
surface tilted toward a top-left light actually does — and every shadow pair
was re-measured against the card's own colour rather than the page's.

### More pigment in the accent

The first lavender was so desaturated that a primary button read as another
grey panel. The accent is the one saturated thing on screen and has to earn it.

### Also

Modal header and footer bands, tab strips and the empty state now use the
material's own gestures — a band is a muted surface, a chosen tab presses in —
instead of borders and tints. The page header's hairline rule went: in a design
with no borders it was the last stray line.

Checked in a browser against the real token values, in both themes, before
shipping. No logic, no schema, no migration (next is still 029).

## 2.29.0 - Onyx: the app becomes one soft material

Marko picked it out of a design lab of eleven, then out of ten siblings of the
one he liked: **Onyx** — a dark, near-neutral soft UI with a single lavender.

### It is four files, not twenty-three thousand lines

The 2.6.0 redesign put the whole visual language in four places on purpose, and
this release is the proof that it worked. Every `bg-slate-N`, `text-slate-N`,
`shadow-card` and `rounded-xl` already spelled out across the pages picks the
new look up **with no page edit**:

- **`tailwind.config.js`** — the `slate` ramp is retuned to near-neutral greys
  and deliberately FLATTENED at the dark end (900 and 950 now sit two values
  apart, not eight), because in this material the card and the ground are one
  sheet. `brand` becomes the lavender. `borderRadius` grows.
- **`src/index.css`** — the surface tokens, and the component classes.

### The shadow is the whole design

A soft surface is lit by **two** shadows derived from the ground it sits on:
one darker, one lighter. That pair cannot be a single fixed value — light and
dark need their own measured pairs — so `shadow-card`/`shadow-raised`/
`shadow-overlay` now resolve to `--sh-*` variables set per theme. Both themes
got measured pairs; the light half on a dark ground has to be a genuinely
lighter grey rather than a translucent white, which is the thing most dark
neumorphism gets wrong and why it usually looks muddy.

That also retires the 2.6.0 top-highlight trick: on a dark ground the light
half of the pair IS the highlight.

### What lost its border

`.card` and `.table-shell`. A soft surface that also carries a 1px outline
reads as two design languages arguing — the shadow pair already says where the
card ends. `.input` went further: a field is now a dent pressed into the sheet
rather than a box on it. The focus halo is unchanged and simply stacks on the
inset, so a focused field still reads instantly.

The sidebar's current item is **pressed in** instead of tinted — the one
gesture this material has that a flat one does not. The accent bar 2.6.0 added
stays, because the dent alone is quiet in a narrow rail.

### One reversal, stated plainly

`borderRadius` grew: `lg` 8px → 13px, `xl` 10px → 20px. That **reverses
marko's own 2.6.0 instruction** about "obrovské rounded cards". He chose the
preview that has them, and the reason is structural rather than fashion: a
soft extruded surface at a 10px radius reads as a mistake, because the corner
has to be round enough for the two shadows to travel around it.

### Not changed

No page file was touched. No logic, no schema, no migration (next is still
029), no new dependency, and not one number on any screen moves.

## 2.28.0 - Market Map removed

Marko: "odstranme tu mapu kompletne lebo aj tak to vbc nejde." Gone, not
hidden - both files deleted, every reference with them. `grep` for MarketMap,
market_map, marketMap or price_checker_map across `src` and `src-tauri/src`
returns nothing.

**Removed**
- `src-tauri/src/commands/price_checker_map.rs` (module, `compute_market_map`,
  13 tests) and its registration in `commands/mod.rs` + `lib.rs`. Commands are
  back to **181**.
- `src/components/MarketMapView.tsx`.
- The `MarketMap*` structs in `models.rs` and the matching interfaces in
  `types.ts`.
- `computeMarketMap` in `api.ts`.
- The event-level map state, the `scanTotal` memo, its effect and its render in
  `PriceChecker.tsx`.

**Deliberately KEPT** - none of this was the map, and all of it still works:
- **The automatic scan run** (`start_price_scan_run`): one press reads the
  whole page, on a background thread, while TIQR stays usable. Its notification
  and toast now say **"Price scan finished"** instead of advertising a map that
  no longer exists.
- **The viagogo reader label fix** in `price_checker_scan.js` - a real scanner
  bug found during the map's reader audit. Viagogo pages were labelling every
  listing `"generic"`; that stays fixed.
- **The truncated macOS user agent fix** ("An outdated browser...").
- **Scans saving themselves to history**, and the stripped-back marketplace
  card.
- **The `grid-cols-5` collision fix** (`EUR1,815.0064`).

**Version bumped FORWARD to 2.28.0**, not reused - marko's own rule for a
revert, because the updater rejects a repeated number.

## 2.27.0 - BUILD FIX (a comment I wrote, twice over)

The 2.27.0 build failed in `tsc` before anything else ran. Six errors, all in
`MarketMapView.tsx`, all cascade from one:

**`{/* ... */}` is only legal as a CHILD of a JSX element.** I put one directly
in a ternary's branch - `) : ( {/* ... */} <div>` - which is expression
position, where it is a syntax error. Fixed.

And the first fix was wrong in a second way worth recording: I rewrote it as a
block comment whose TEXT quoted the JSX comment syntax, so the comment
terminator inside my own explanation closed the comment early and broke the
file again. It is line comments now, which cannot be closed by their own
contents.

Both are the same shape as 2.24.0's `"Mixed events"` inside a non-raw Rust
string: **a comment whose content is also syntax**. Scanned the whole `src`
tree for the JSX case afterwards - zero remaining.

Version deliberately stays **2.27.0**: the failed build published no release,
so the updater has never seen this number. Same call as 2.16.0 and 2.24.0.

## 2.27.0 - One press reads the whole page, and the glitching is gone

### The glitching had two causes, both removed

1. **The map was unmounted on every refresh.** The map is recomputed after each
   pass of a scan, and the panel swapped itself for a loading box each time:
   the map vanished, a short box took its place, everything below jumped up,
   then the map came back and everything jumped down - several times a minute.
   Now a refresh is invisible except for the word "updating" in the header; the
   big loading state is only for the FIRST build, when there is genuinely
   nothing to show.
2. **The `zoom` control.** `zoom` is a non-standard CSS property that re-lays
   out the whole subtree, and it was re-applied on every render. It is gone -
   marko asked for the map to be "nehybne". Blocks are a fixed readable size,
   the area scrolls, and the panel has a fixed minimum height so adding a
   section no longer resizes it. Hover and colour transitions on the blocks
   went too.

### One press now reads the whole page, in the background

`start_price_scan_run`: scan, scroll, scan, until the page stops giving
anything new. It returns immediately and the run continues on a backend thread,
so **leaving the Price Checker page - or putting TIQR behind another window -
does not interrupt it**. Each pass broadcasts the same scan-result event a
manual scan always did.

**This is not the Live Market Monitor coming back.** There is no schedule, no
timer, no polling, and nothing runs unless marko pressed a button on a window
he opened himself. A run is finite by construction and ends on: an exhausted
page (three passes with nothing new - three, not one, because a lazy-loading
page routinely needs a beat), a 60-pass cap, a 300-second cap, a failure, or
Stop. When it ends it is over; nothing re-arms it.

The termination logic was executed against scripted page behaviours - a normal
page, an infinite feed, a dead page, a mid-run failure, a closed window, Stop,
and a page with a two-pass lazy gap. Every one terminates, for the right
reason.

### It tells you when it is done

An OS notification ("Market map is ready - N listings read") reaches marko with
TIQR behind another window, and an in-app toast fires wherever he is in the app
- the listener lives in the Layout, not on the Price Checker page he has
already left. A run he stopped himself announces nothing.

The card also reports **why** the run ended in the backend's own words, rather
than just "done" - a run that hit a cap says so.

### The "outdated browser" warning

macOS only, and it was a TRUNCATED user agent, not an old engine. WKWebView's
default string stops after `AppleWebKit/605.1.15 (KHTML, like Gecko)` with no
`Version/… Safari/…` suffix, so a site's browser check finds no version and
falls through to "outdated". The scanner window now sends the suffix the engine
leaves off - it says Safari/WebKit, which is exactly what renders the page.
Windows is untouched: WebView2 already reports a current version.

It defeats nothing - challenge pages are still detected and reported honestly,
never bypassed.

### Verified

`WebviewWindowBuilder::user_agent` and `WebviewWindow::eval` are both new to
this codebase, so both were checked against the real tauri 2.11.5
documentation rather than assumed.

## 2.26.1 - Price Checker stripped back to what marko actually reads

His screenshots, his words: the screen was unreadable. Three marketplace cards
side by side, each stacking a listings table, Min/Max filters, a CSV export, a
"Save to history" button, a Market Map, a Market Analysis panel and a "Compare
a specific ticket" tool underneath it.

### The manual "Check Prices" form is gone

Button and modal both. It opened a form asking him to type in the lowest,
median, average and highest price - **numbers the scanner had just read for
him**. So scanning is now the only way a price check is recorded, and **a
finished scan records itself**.

The auto-save waits for the tier breakdown so history keeps its "1 tier" line,
and it is guarded on the scan NUMBER rather than a boolean, so one scan can
never be written to history twice. **No currency, no save** - a price check
whose currency had to be guessed is worse than none, and the card says so in
one line instead of saving a guess.

### A marketplace card is now a scan button and its history

Exactly the last screenshot he sent: name, URL, Visible Scanner, Latest check,
the five figures, the history table. Everything else went - 766 lines of UI
that is no longer reachable was deleted, not just hidden.

### The Market Map moved out and became ONE map

Marko: "mapa by mala byt niekde inde nie tam dole a mapa by mala byt pre vsetky
platformy rovnaka a tie listingy sa spoja."

So `compute_market_map` is keyed on the **event** now, not on one scanner
session: every open session for that event contributes its listings to one map,
and it is drawn **above** the marketplace cards rather than inside one. Scan
Viagogo, then Ticombo, and section 102 shows both marketplaces' rows together.

Deliberately NOT deduplicated across marketplaces - the same seat listed on two
sites is two real offers at two real prices, and the scanner has no cross-site
listing identity that could tell a genuine duplicate from two different sellers
who happen to match.

### The collision in the screenshot was a real bug

`EUR1,815.0064` - the Highest figure running straight into the Listings count.
The stats row was `grid-cols-5`: five FIXED columns no matter how wide the card
is. These cards sit three-across, so a fifth of a third of the page is not
enough for a four-figure price. Now the column COUNT follows the width
(`auto-fit` + a `6.5rem` minimum), so the figures wrap to a second row and
every one of them is fully readable. Same class of bug as `.summary-bar` in
2.19.0, same fix.

## 2.26.0 - Price Checker Market Map

A second VIEW of a scan session that already happened. **No new scanner, no new
table, no migration, no background work, no polling** - `compute_market_map` is
the same shape as `compute_market_analysis` next door: it folds the listings a
manual scan already accumulated together with marko's own unsold tickets for
that event, and returns a read-only structure the frontend draws.

### The reader audit came first, and it changed the design

`price_checker_scan.js` is ONE generic DOM reader, not three per-marketplace
ones - the three `read*` functions differ only in the label they stamp. Per
listing it can produce price, currency, section, row, tier, quantity, listing
id and marketplace; everything except price and marketplace is best-effort and
routinely absent. It produces **no seat numbers, no venue identity, no
coordinates or map geometry, and no per-listing URL** - none of those exist
anywhere in the extraction.

So the map stops at tier -> section, section detail shows Seats as a COUNT, and
the only URL offered is the event's own marketplace link. A geometric replica
would have had to be invented.

### Bug found by the audit and fixed

**`hostFamily` had no viagogo branch.** Viagogo replaced StubHub as the seeded
marketplace in migration 017 and StubHub was removed outright in 020, but this
file was never updated - so every scan of a viagogo page fell through to
`"generic"` and every listing it produced was labelled `"generic"`. Since the
three readers differ only in their label, the fix is the label (three lines):
a `readViagogo`, a `hostFamily` branch and a dispatch arm. Nothing else in the
scanner was touched.

### Normalization: safe only

`"Sec 102"`, `"Section 102"` and `"102"` become one block. Trim, collapse
whitespace, upper-case the grouping KEY, and drop leading zeroes ONLY when what
is left is entirely digits - so `"0102"` joins `"102"` while `"0A"` stays
`"0A"`. Every group keeps both a `key` and the first SOURCE value as its
`label`, which is what is drawn. **Tier wording is never mapped**: "Level 100"
and "Tier 1" stay separate groups, because nothing here can know whether they
mean the same thing.

### What the map shows

Tier bands of section blocks, tinted in four steps by listing count (not a
continuous ramp - it is "more here than there", not a measurement). Marko's own
tickets overlay the same blocks in emerald and are counted separately, never
folded into the market count. Click a block for the detail panel: counts,
lowest / median / highest, then Row | Seats | Marketplace | Price | Mine, with
his own tickets first and each linking to the EXISTING order detail.

Filters: each marketplace, My tickets, and tier. Zoom 70-160%, scroll contained
inside the map area.

### Honest states rather than invented ones

- A scan with prices but **no sections at all** says so and draws nothing.
- Listings with no section go to one explicitly labelled bucket **outside** the
  grid, never scattered into blocks.
- A section whose listings span **more than one currency** reports no
  lowest/median/highest at all rather than a blend.
- Section order is numeric-then-alphabetical - a stable way to find a section,
  never a claim about where it physically sits.

### No pricing, anywhere

Lowest/median/highest describe what is listed in one block. Nothing compares
one section to another, interpolates between them, or recommends a price.

## 2.25.0 - A recap you watch, a guide that points, and the CSV gaps

Seven things marko asked for in one pass.

### The Recap is a story now

It OPENS as a sequence: a title beat, then one idea per screen, each arriving
with its own effect and advancing on its own, ending on a card that hands you
the full report. "Skip to the full report" reaches the old view at any point,
and "Replay" in the header starts it again.

Not one new figure. Every slide reads a field the report already shows, from
the same single `get_dashboard` call, so a slide and the report cannot
disagree. A slide with nothing true to say is never built - an empty period
says so instead of showing zeroes, and the two numbers the app genuinely does
not hold (what unpaid orders total, per-event profit inside a period) say so
on the slide.

### All time, and it is the default

Marko: "ked som chcel aby mi vsetko ukazalo tak som musel dat last 6 months. a
realne pracujem na tiqr mesiac max dva." With one or two months of history the
recap opened on a near-empty month and the only preset that showed the whole
business was a six-month window. `period_bounds` already had an `all` arm, so
this is the same two sentinel dates it produces and no backend change at all -
`previous_period_bounds` already special-cases them, which is why the summary
table correctly says there is nothing to compare against.

### The guide walks the app instead of describing it

The seven-paragraph list is gone. In its place is a 14-step tour that
NAVIGATES to each real page, finds a real element, dims everything else and
puts the explanation next to the thing it is talking about - the Dashboard's
headline numbers, the period picker, then events, orders, tickets, sales,
inventory, pulls, finance, price checker, calendar, sync and the recap.

Elements opt in with a `data-tour` attribute. Four files carry them and every
one is a single attribute with no logic: `PageHeader` in ui.tsx (which gives
every page in the app an anchor in one edit), the sidebar in Layout.tsx, and
two on the Dashboard. A step whose anchor is not on screen still runs, just
centred and without a spotlight - the tour never points at nothing and never
waits for an element that is not coming.

### Insights and Support moved to the very end of Settings

They are the two things you go looking for on purpose; everything above them
is something you came to Settings to change.

### The restore lists close behind you

The Drive revision list had no collapse at all - it loaded every version
Google still holds and left them all on screen. It now shows two with "Show N
more", both lists have a real **Hide**, and leaving the Data section puts them
back the way they were found. Switching Settings sections does not unmount the
page, which is why this is keyed on the section rather than left to unmount.

### CSV: three columns that existed in the database and reached no export

- **Orders** gained `event_date` and `external_reference`. The reference has
  been on orders since migration 009; the event's own date is the one 2.23.0
  made the visible date in the Orders list, while the export still carried
  only `purchase_date`.
- **Tickets / Inventory** gained `event_date`.
- **Sales** gained `event_date`.

All **appended at the end**, so every column an existing sheet or script
already reads keeps its position. A TBD event still exports a blank date -
never a fabricated one.

**Checked and deliberately NOT changed:** Events CSV is complete (its
`category` column is kept in step with `category_id` by `resolve_category_name`,
so it is not the stale legacy value it looks like). **Not built, and marko
should say if he wants them:** there is no Pulls export, no Finance export and
no ticket-listings export at all, and Sales still does not carry `batch_id` -
that one is protected and was left alone rather than touched in passing.

### Suggestions can carry a picture

One optional image, resized on a canvas before it is sent - no image library,
no Firebase Storage, no new dependency. It is stored in the suggestion
document itself, which is why the ceiling is 600k base64 characters: Firestore
allows 1 MiB per DOCUMENT and the text and metadata share it. The quality
steps down until it fits rather than refusing a big photo. The admin inbox
renders it, and only a `data:image/` URL is ever put into an image element.

**`firestore.rules` must be pasted into the Console again** - the new rule caps
the image field there too, because a rule that trusts the app to have resized
the picture is not a rule.

## 2.24.0 - BUILD FIX (three compile errors, no behaviour change)

The 2.24.0 build failed on both Windows and macOS. Three independent errors;
the other 34 lines of the log were cascade from the first.

1. **`commands/sales.rs` - a `"` inside a `&str` constant.** The 2.23.0
   `event_date` change added an SQL `--` comment containing `"Mixed events"`,
   `"no single date"` and `"-"` INSIDE `GROUP_BASE_SELECT`, which is a plain
   (non-raw) Rust string literal. The first quote closed the string and the
   rest of the SQL was parsed as Rust, which is why all 17 `sales::*` commands
   then "could not be found" by `generate_handler!`. Quotes removed from the
   comment text; the SQL itself is byte-for-byte unchanged.

2. **`commands/cloud_sync.rs` - `AtomicBool` was never imported.** Line 77 read
   `use std::sync::atomic::Ordering;` while `SYNC_IN_PROGRESS` (2.18.0) needs
   both. Widened to `{AtomicBool, Ordering}`.

3. **`commands/cloud_merge.rs` - `db_path` was used but never bound.**
   `cloud_merge_pull` called `merge_inner(&mut conn, db_path)` without the
   `let db_path = state.db_path.lock().unwrap().clone();` line its twin
   `cloud_sync_pull` has. Added, in the same lock order (db first, then
   db_path) so the two entry points cannot deadlock against each other.

**Errors 2 and 3 date from 2.18.0, not from Recap** - which means every build
from 2.18.0 onward failed the same way and 2.17.0 was the last version that
actually produced installers.

Version deliberately stays **2.24.0**: the failed build published no release,
so the updater has never seen this number and reusing it is not a repeat.
Same call as the 2.16.0 build fix.

## 2.24.0 - TIQR Recap: Ticket & Finance

**No schema change, no new dependency, and - the point of the whole thing -
NOT ONE NEW BUSINESS CALCULATION.** Reached from **Settings → Insights**, never
the sidebar.

### One existing call does all of it

Every figure comes from `get_dashboard` with a concrete range. That command
already returns the period's `FinanceSummary`, the equal-length previous
period, the cashflow split, the inventory potential, the per-platform
breakdown and the time series - so the Recap makes **one round trip**, not the
"dozens of separate queries" the brief warned against, and every number is the
same number the Dashboard shows under the same definition.

**Caught before shipping:** `period_bounds` in `dashboard.rs` matches on the
period NAME first and only reads `from`/`to` under `"custom"`. Passing the
dates alone falls into the `None` arm, which returns **today → today** - every
recap would have shown a single day's figures under a month's heading, silently
and plausibly. Checked against that function rather than assumed.

### Realized / Pending / Potential, kept apart by construction

Three bands, each with its own heading, colour and **one-line definition on
screen**, because without it "pending" and "potential" read as the same kind of
number:

- **Realized** - money that actually moved, inside the period.
- **Pending** - owed either way and not moved yet. *Unpaid orders show a COUNT
  only, and say so: the app tracks which orders are unpaid but never totals
  what is owed, and adding that sum would be a new calculation.*
- **Potential** - what unsold stock might be worth. Never called profit.

### The summary table

Compares against the previous period with the app's own `computeTrend` /
`computeTrendPoints`, so **percent and percentage points are distinguished**:
money and counts move by %, ROI and margin are already percentages and move by
**pp**. "Previous period" is the app's own definition - the equal-length window
immediately before - and the table says so underneath rather than leaving it to
be assumed.

### Charts and share

The chart is `MetricChart`, the app's own hand-rolled SVG component, unchanged.
Purchased/Sold/Remaining is a composition bar and deliberately **not** a time
series: nothing in the existing aggregation reports how many tickets were
*bought* per bucket, so that line would have to be invented.

**Share** produces a PNG, laid out by hand as SVG and rasterised on a canvas -
so it reads as a report rather than a screenshot of the app, and needs no
screenshot library (this project carries no UI dependencies). Saved through the
normal save dialog; `save_png_file` decodes base64, **verifies the PNG
signature** and writes bytes. No logic, no database, no cloud.

### Deliberately absent rather than invented

Biggest single sale, fastest-selling event and best tier: none exists as an
aggregation today, and building one would make this a reporting engine. The
best-event line is labelled **all time**, because all-time is the scope the app
already keeps per event.

### Verified

- **Date ranges executed, not reasoned about**: end of March → Feb 1–28; leap
  year → Feb 1–29; January → December of the previous year; rolling 3/6-month
  windows; custom keeps what was typed.
- 3 tests on `save_png_file` (signature accepted, non-PNG refused without
  writing a file, bad base64 is a clear error not a panic).
- 180 commands still match `api.ts` ↔ `lib.rs` in both directions.

## 2.23.0 - Event dates in lists, readable codes, the wheel works, and a Support section

**No schema change, no new dependency.** Five things marko asked for.

1. **Lists show the EVENT date, not the purchase/sale date** (Orders, Sales,
   Pulls). The list is scanned to find *what is coming up*; a purchase date
   answers a question nobody asks while scanning. Detail screens still show
   when it was bought or sold, and in the list it moved into the tooltip so
   nothing was lost. Sales needed `event_date` added to its grouped query -
   carried under the same "only when every line's event agrees" guard the event
   name already used, so a mixed-event group and a TBD event both honestly show
   "-" instead of a made-up date.
2. **Codes are readable in lists**: `#14`, not `ORD-000014`. Six zero-padded
   digits behind a prefix were the first thing the eye landed on in every
   table, and the table is already called Orders. **Only the display changed** -
   the stored code keeps its full form because a connected Google Sheet shows
   it and the merge parses its numeric tail to keep both machines' counters in
   step (PROTECTED_AREAS 2.16.0). Full code stays in the tooltip and on every
   detail screen.
3. **Finance → Transactions can be scrolled with the wheel again.**
   `.table-flush` is `overflow: auto` **plus** `overscroll-behavior: contain`,
   and this wrapper had no height - so it could never scroll itself while
   `contain` still stopped the wheel reaching the page behind it. Only dragging
   the window's scrollbar worked, exactly as reported. **Measured both ways in
   a real browser before fixing:** without a height cap the wheel moved the
   page 0px; with one the table moved 500px. The class's own doc comment
   already says it belongs on a box that scrolls - this usage never gave it
   one. **The same bug was in the Reports and Accounts tabs** and is fixed
   there too.
4. **Restore points show the last two**, with "Show N more". Each one that can
   be paired with a merge from the same minute now says what actually arrived
   ("then: 3 orders, 4 tickets") - taken from the merge log, and left blank
   rather than guessed when nothing matches (a whole-file sync down has no
   merge entry at all).
5. **Settings → Support**: a short guide in the order the app wants to be used,
   and a suggestion box. The box posts one document to Firestore and reads
   nothing back - a postbox, not a forum, because a reply thread that never
   gets answered is worse than none. Only an admin can read the collection, and
   `admin` is a field only the Firebase Console can set, so the app can never
   promote itself. The inbox appears in Settings for that account and nowhere
   else.

**Two manual steps for #5, and nothing in the app can do either:** publish the
updated `firestore.rules` in the Firebase Console, and set `admin: true` on
your own `users/{uid}` document. Until both are done, sending a suggestion
fails and the inbox stays hidden - the safe way round. The failure message says
so in plain words rather than showing a raw Firebase permission code.

## 2.22.0 - 2.21.0 reverted at marko's request

**The dashboard is back to exactly what it was in 2.20.0.** marko looked at the
preview and said so plainly: *"okej toto ani nejdem stahovat je to strasne
zabudnime na tuto verziu"*. He never installed it, so nothing needs migrating
back - the code is simply gone.

Removed: `OperationsSnapshot` and its five structs, the six aggregates in
`dashboard.rs`, the six Overview cards, and profit on the Sales-by-platform
card. `PROTECTED_AREAS.md`'s 2.21.0 section went with them - it documented
invariants for code that no longer exists, and a stale invariant is worse than
none.

**Untouched:** everything from 2.14.0 through 2.20.0. Automatic sync, the
merge, tombstones, merge history, AI cost, Drive revisions, the `.summary-bar`
grid fix, the busy indicator - all still there. So is `PeriodComparisonCard`
(DSH-L), which marko picked himself.

Version goes FORWARD, not back to 2.20.0: the Tauri updater compares version
numbers directly and will not accept a repeat. Same lesson as the 2.3.0 ->
2.3.1 revert (see `CURRENT_STATE.md`).

*What was actually wrong with 2.21.0 is not recorded here, because I do not
know yet - "it's terrible" could be too many blocks, the wrong metrics, or the
wrong idea entirely. Whatever replaces it starts from an answer, not a guess.*

## 2.21.0 - The dashboard starts answering questions

**No schema change, no new dependency, no API wired up - that last one was
marko's only constraint, and it turned out not to cost anything: nearly
everything a reseller needs was already in the database and simply never asked
for.**

### Six new aggregates in `dashboard.rs`, all from data already stored

The metric set is the one this trade actually uses, checked against published
inventory/ticketing KPI guidance rather than invented:

1. **Sell-through** (sold / bought) - that guidance calls it the single most
   important number in a stock business. `None` when nothing has been bought:
   a rate with no denominator is unanswerable, not 0%.
2. **Days to sell**, median *and* mean, with the sample size shown. The median
   because one ticket that sat for a year drags a mean into uselessness; the
   sample size because an average of three is not the same claim as an average
   of three hundred. A sale dated before its own purchase is left out - that is
   a typo, not a fast sale.
3. **Ageing stock** in 0-6 / 7-29 / 30-89 / 90+ day buckets, with the money in
   each, plus the five oldest tickets by name.
4. **Money at risk**: events in the next 14 days with unsold stock, ranked by
   euros and days left, and how many of those tickets have **no price at all**.
   The Attention tiles count things; this says which one costs the most to
   ignore. Same window and same `upcoming` scope as those tiles, so the two can
   never disagree.
5. **Realized profit per event**, sold tickets only - never blended with the
   potential profit of unsold stock.
6. **Where stock comes from**, per supplier. Only the SOLD tickets' cost counts
   against realized revenue, or every batch still in stock would show as a
   loss; and tickets bought with no supplier keep their own row rather than
   being folded into a neighbour, which would make every puller look worse than
   they are.

### On the screen

Six new cards on Overview, ordered by the question they answer: what happened
to the money, what the stock is doing, what needs doing today, and only then
what worked.

- **Where the money went** - revenue split into stock cost / platform fees /
  profit. It says out loud when no fees are recorded, because then the margin
  shown is a best case rather than a fact (marko's recorded fees are currently
  zero against €5,093 of revenue).
- **Sales by platform now shows profit beside revenue.** `profitCents` has been
  sent with every dashboard load since 2.0.47 and was never displayed, so the
  card ranked channels by turnover instead of by what they earn.

### How the numbers and colours were checked

- **All six SQL statements were extracted verbatim from the Rust source and run
  against a real seeded SQLite database** before being trusted - 6/6, with the
  figures hand-checked.
- The three bar colours (brand / amber / emerald, the app's own ramp - no new
  palette) were checked for colour-blind separation by **computing OKLab
  distance under simulated deuteranopia and protanopia**, not by eye: worst pair
  9.3 against a target of 8, normal-vision floor 23.8 against a floor of 15.
  Every segment is also directly labelled, so identity never rests on colour.
- 5 new Rust tests for the parts that live in Rust rather than SQL.

### Deliberately absent, not approximated

Conversion rate, cart abandonment, show-up rate and on-sale velocity all need
data from the marketplace's side of the transaction. This app does not have it
and will not invent it.

## 2.20.0 - Deletions travel, the merge keeps a record, the AI has a price tag, and Drive has yesterday

**Migration 028. No new dependency, no business logic touched.** SYN-5/6/7/8.

### SYN-5 - Tombstones (migration 028)

The last hole in the merge, and marko named it: delete an order on the Mac and
it stays on the PC - then the next merge, seeing a record the Mac "has never
seen", copies it straight back. The deletion undoes itself.

A merge cannot notice the absence, because **a row that is merely not there is
indistinguishable from one that has not arrived yet.** Absence carries no
information. So the deletion has to leave something behind: `deleted_rows`,
written by an AFTER DELETE trigger on all 17 syncable tables.

- **Verified against real SQLite, not assumed:** an AFTER DELETE trigger DOES
  fire for rows removed by `ON DELETE CASCADE`, with or without
  `recursive_triggers`. That matters - deleting an order cascades to its
  tickets, and without it those tickets would leave no tombstone.
- Deletions are applied **children first** (reverse merge order), because
  `tickets.event_id` is `ON DELETE RESTRICT` and an event cannot go while a
  ticket holds it. A deletion the schema still refuses is reported, never
  forced, and never abandons the rest of the merge.
- Tombstones travel too, so a third copy cannot resurrect what was removed.
- **A tombstone beats an edit.** No per-row history exists to decide otherwise,
  and the alternative is resurrecting a record its owner deliberately removed.
- Probed against two real databases across 4 scenarios (9 assertions) before
  any of it was trusted, then covered by 3 Rust tests.

### SYN-6 - What came from the other computer

The merge already reported what it did; the report left with the toast. Now the
last 20 are kept - per table, with removals - and the sync card shows the last
three. JSON in `app_settings` rather than a table: nothing queries it, and
`app_settings` is a bookkeeping table, so writing the log cannot itself look
like data that needs syncing.

### SYN-7 - What the AI costs

The Messages API reports token usage on **every** response and this app was
throwing it away. Each screenshot import now counts towards a monthly total,
with an estimate at Claude Opus 5's published list price - **$5 / $25 per
million tokens**, read off Anthropic's pricing page today rather than recalled -
in integer cents, in USD because that is what Anthropic bills. Shown in the
existing API-key card. This does not contradict that card's long-standing note
about not showing a balance: a balance needs an Admin key and has no endpoint;
these tokens are measured, not guessed.

### SYN-8 - Earlier versions in Google Drive

The restore points are all on this machine, and the one copy that isn't gets
overwritten by every sync. It turns out the history was already there: **Drive
keeps a revision of every upload**, and `revisions.list` accepts
`https://www.googleapis.com/auth/drive.file` - the narrow scope this app
already holds, verified against Google's own reference. So point-in-time
recovery from off the machine, retroactively, with no new storage, no new
service and no new consent screen. Restoring one goes through the same
`restore_database_impl` as every restore (validation, safety backup, rollback)
and then pushes the older version to the other machine too, so the rescue
travels. `keepForever` is deliberately not set - pinning every sync would
multiply Drive usage by the number of syncs, so this shows what Drive kept and
says so.

5 new tests (3 tombstone, 2 cost). 179 commands still match `api.ts` <-> `lib.rs`
in both directions.

## 2.19.0 - The boxes stop breaking when the window isn't full screen

**No schema change, no new dependency, no business logic touched.**

marko: *"ked neni dashboard na full screene tak ze nebudu bugovat okienka"* -
and it was measurable, not a matter of taste.

1. **`.summary-bar` is a grid, not `flex flex-wrap`.** With `flex-1` the cards
   that land on the LAST row stretch to fill it. Measured in a real browser
   before and after, at the app's minimum window width (1080px -> 832px of
   content) and at its default (1400px -> 1152px):
   - 6 cards at 832px, before: five at 157px and **one at 832px** - a single
     box across the whole row.
   - 9 cards at 1152px, before: seven at 154px and **two at 570px** - so Price
     Checker and Event Detail did it on a normal window, not just a small one.
   - After, both cases: every card the same width, tidy rows.
   - 6 cards at 1152px: 182px each, one row, **identical before and after** -
     the common case is untouched.
   Resizing used to re-flow that stretch continuously, which is the "skákanie".
2. **A long figure can no longer push the row off the page.** A grid item
   defaults to its content width, so `StatCard` gained `min-w-0` and a
   truncating value. The 9rem column floor is what keeps a 22px money figure
   readable at the minimum window width.
3. **12 CSV export/import commands and the restore-point listing moved off the
   main thread** - the same fix the sync family got in 2.17.0. A large export
   was reading files and querying whole tables on the thread that draws the
   window. The Price Checker scanner was deliberately left alone: it creates
   windows through the `AppHandle`, and its logic is protected.
4. **Buttons that could error out.** 2.18.0's one-sync-at-a-time guard meant
   pressing Sync while the timer happened to be uploading came back "a sync is
   already running" - which reads as a bug, not a queue. Every sync button now
   disables for a sync started anywhere, and the panel stops saying "Syncing"
   by itself instead of waiting to be navigated away from.
5. **A recorded conflict now has a button.** "Combine both" only existed inside
   the prompt that appears after a push is refused, so a conflict recorded by a
   merge showed the state and offered nothing to do about it.
6. **Two stacking mistakes of my own, from 2.17.0.** The busy pill was
   `fixed bottom-4 right-4` - the same corner as the toast stack at `z-[100]`,
   so every toast hid the one thing meant to say the app was busy. And the
   blocking overlay sat at `z-50`, below `ConfirmDialog`'s `z-[60]`, so a
   confirm dialog could be clicked into a database being merged underneath it.

Also checked and clean: all 175 Tauri commands match between `api.ts` and
`lib.rs` in both directions (a typo'd `invoke` name is a button that silently
does nothing), every `api.X()` the UI calls exists, every `<Icon*>` and every
`ui.tsx` import resolves, and no literal-union state mismatch remains - the
class that broke the build twice this week.

## 2.18.0 - Sync gets safer, faster and harder to confuse

**No schema change, no new dependency, no new module, no new screen.** Every
change lands in `cloud_sync.rs` / `cloud_merge.rs` and in the sync card that
already existed in Settings.

1. **One sync at a time.** A process-wide `SyncGuard` - the only guards before
   were per-screen (`Layout.tsx`'s timer ref, Settings' disabled buttons) and
   neither knew about the other. 2.17.0 made that genuinely concurrent by
   moving these commands off the main thread: two uploads racing decide the
   winner by whichever request finishes last, then record a `version` for a
   file the other already replaced. Every entry point takes it - push, pull,
   merge - and the timer answers `idle` instead of an error, because "nothing
   to do right now" is the truth when it fires every five minutes anyway. It
   releases on drop, including on an early `?` return and on a panic.
2. **Bounded retries.** A dropped connection, a 429 or a 503 gets two more
   goes with 1s and 3s between. A 401/403/404 or an `invalid_grant` is
   returned immediately - those are answers, not hiccups, and retrying them
   makes the user wait longer for the same message. Never endless. The pause
   is only safe because 2.17.0 moved these commands off the main thread.
3. **Pointless uploads are gone.** The app records a hash of the bytes it last
   uploaded, so a database that was WRITTEN TO but does not actually DIFFER
   from Drive's copy is no longer shipped across. Running the migrations on
   launch marked it dirty, so every app update used to push several megabytes
   Drive already had, byte for byte. A download records the same hash, so a
   freshly pulled database is not immediately pushed back. Taking the snapshot
   is the cheap half of a push; this skips the slow half. The hash is FNV-1a
   plus the byte length, no dependency, and it fails in the safe direction:
   the same data laid out differently reads as "changed" (one extra upload),
   while the opposite needs a 1-in-2^64 collision.
4. **One short word for the state.** `synced` / `syncing` / `localChanges` /
   `cloudChanges` / `conflict` / `offline` / `failed`, decided by a pure
   `summarize_state` so the panel can never drift from what the timer decided,
   shown with the last successful sync and one readable sentence for the last
   failure. In the existing card - no Sync Center, no new tab.
5. **A conflict the merge could not settle now persists.** Skipped rows or
   `legacy-N` identity clashes set a conflict flag that only a clean merge
   clears - an ordinary push no longer hides it behind a green tick while the
   two machines still disagree about those records.

15 new tests: the guard (including that it frees on drop), the retry
classification both ways, the retry limit, the hash, all eight states, and the
error shortener.

## 2.17.0 - The window stops freezing while data moves

**No schema change, no new dependency.**

marko: *"su tam nejake nereagujuce spravy, nejake dlhe nacitavanie"*. Not a slow
query - a threading mistake, and it was in my own code.

1. **Tauri runs a SYNCHRONOUS command on the main thread.** Verified against
   the v2 docs, not memory: *"Commands without the async keyword are executed on
   the main thread unless defined with `#[tauri::command(async)]`."* Every
   Cloud Sync command was synchronous, so pushing a multi-megabyte database
   over the internet blocked the event loop and the OS drew "Not responding"
   over the window. Sheets sync and AI import never did this - they were
   already `async fn`, which is why only the sync family froze. 2.14.0's
   5-minute timer made it recurring and 2.16.0's merge made it long.
2. **16 commands that reach the network or move the whole database are now
   `#[tauri::command(async)]`**: the cloud sync/merge family, backup, restore,
   validate, `switch_active_database`, the Sheets connection tests, Google
   sign-in status and sign-out, currency conversion, the desktop-notification
   test. Their `State` argument gained the explicit `'_` lifetime that this
   codebase's already-async commands (`test_ntfy_notification`,
   `check_and_send_notifications`) already spell out.
3. **Something to look at while it works.** An app that is silently busy for
   eight seconds still looks broken even when it is perfectly responsive. A
   background upload gets a corner pill it can be ignored in; a download or a
   merge gets the screen, because both end in a restart or reload and a restart
   with no warning reads as a crash.
4. **The one silent failure the merge could still have, caught and reported.**
   Machines that drifted apart BEFORE migration 027 each counted up from the
   same place, so the Mac's 7th order and the PC's 7th order both call
   themselves `legacy-7`. The merge would see that identity already present,
   skip the arriving order as "already have it", and report nothing at all - a
   real order would simply never arrive. A shared identity whose `code` differs
   is now counted and named, together with the one manual sync that fixes it.
   Deliberately NOT repaired automatically: deciding which `legacy-7` is which
   is a human's call. 2 tests, and the detector was run against two real
   databases first - including the negative, so a genuinely shared record never
   trips it.

## 2.16.0 - The two machines add up instead of one replacing the other

**No schema change (027 did that), no new dependency.**

marko: *"ked ma jedna strana nieco ine a druha a das sync tak sa to zachova len
z jednej strany"*. Correct, and by design - push and pull move the whole
database FILE, so whichever direction runs, one machine's copy replaces the
other's. `commands/cloud_merge.rs` makes them add up.

1. **Records only one side has are copied in; records both sides have are left
   exactly as they are here.** So a merge cannot lose anything. It deliberately
   does NOT carry an edit or a delete across: an insert has one obvious correct
   outcome, an edit has two plausible ones, and guessing between them is the
   data loss this was written to end. Deletes need tombstones (a row that
   merely "isn't there" is indistinguishable from one that hasn't arrived yet)
   and that is its own migration.
2. **Ids are rewritten on the way in.** The other machine's order #12 becomes a
   different number here, so every foreign key is translated remote id ->
   `uid` -> local id, parents before children.
3. **Three collisions found by running the real thing, not by reading it.** The
   algorithm was executed against two actual SQLite databases before any of it
   was trusted:
   - **`code` is UNIQUE and both machines mint the same ones** from their own
     `counters` row, so `ORD-000007` exists twice. Arriving records whose code
     is taken get the next free one, and the count is reported.
   - **A lookup matched by name would have silently dropped its orders.**
     "Ticketmaster" typed on both machines is linked, not duplicated - which
     means this machine holds no row carrying the other's `uid`, so every
     arriving order pointing at it looked like an orphan. `translate_id` falls
     back to the name for exactly this reason.
   - **An arriving code from ahead poisoned the counter.** `ORD-000009`
     arriving while this machine's counter reads 5 breaks nothing that day -
     and then order creation starts failing on a UNIQUE constraint once the
     counter climbs to 9, with nothing on screen connecting it to a sync.
     Counters are now dragged up to the highest code present, before and after
     each coded table.
4. **Anything that still clashes is skipped, counted and named** (the same
   ticket sold on both machines), never forced and never allowed to abandon the
   rest of the merge. The whole thing runs in one transaction on top of a
   safety backup, which appears in Settings -> Data's restore points.
5. **Automatic sync no longer asks.** `decide_auto`'s two manual cases - both
   sides changed, and a machine that has never synced finding data in Drive -
   now answer `merge` instead of `ask`. Merging at launch reloads the page
   rather than relaunching the app: rows were added, the database file was not
   swapped. Mid-session it stays a banner, same rule as pull. Settings' old
   Take theirs / Keep mine prompt now leads with **Combine both**.

8 tests in the new module, all of them a scenario that was first run for real
against two databases: the platform-name drop and the counter poisoning are
both in there.

*Build fix, same version:* `syncBusy` is a fixed set of strings (`up`, `down`,
`toggle`) and the new Combine-both handler set it to `"merge"`, which `tsc -b`
rejected before the Rust ever compiled. The union now carries `"merge"` -
deliberately still a closed union rather than `string`, because every button in
that card disables itself on `syncBusy !== null`, so a missing value should stay
a compile error rather than a button that stays live mid-operation.

## 2.15.0 - Migration 027: the identity a merge is built on

**Schema change (migration 027), no new dependency, no behaviour change yet.**

marko chose merge-over-Drive: the two machines should combine what each has
instead of one overwriting the other. Merging needs one thing that did not
exist - a way to say "this row here and that row there are the same row".
Every primary key in this app is a per-machine `INTEGER AUTOINCREMENT`, so the
Mac and the PC both mint id 5 for two different orders. That is the exact
reason 2.12.0 shipped whole-file sync instead of a merge.

1. **`uid` on the 17 tables that can travel between machines** - events,
   orders, tickets, sales, pulls, pulls_received, ticket_listings,
   event_marketplace_links, payments, finance_entries, accounts, transfers,
   recurring_expenses, platforms, suppliers, event_categories,
   finance_categories. Not on `marketplaces` (seeded identically by these same
   migrations), not on the price-checker/`market_*` tables (per-machine scan
   history), not on the bookkeeping tables (the same list `is_bookkeeping_table`
   already keeps out of the sync dirty-flag).
2. **Existing rows get `legacy-<id>`; new rows get a random 128-bit uid.**
   Both databases descend from the same file, so a row that exists on both has
   the same id on both and therefore ends up with the same uid on both -
   without the machines talking. Shared history becomes shared identity for
   free.
3. **The uid is assigned by a trigger, not by Rust.** No existing INSERT
   changed - not the order writer, not the CSV import, not Sheets sync, not the
   AI import - and an insert site added later gets one automatically. The
   trigger fires only `WHEN NEW.uid IS NULL`, so a row arriving from the other
   machine keeps the identity it came with.
4. **One last whole-file sync is needed after both machines update.** If they
   had diverged first, their post-divergence rows carry colliding `legacy-N`
   uids. Push from whichever machine is current, pull on the other, once.
   After that hand-off nothing has to be overwritten again. Not automated on
   purpose: picking which side is "current" is the one judgement no timer may
   make.

Nothing reads `uid` yet - the merge engine itself is the next step. 5 tests.

5. **Dashboard: "This period vs. previous" (DSH-L), marko's pick, built wider
   than the mock he chose from.** The six KPI cards already carry this period's
   figure and a small "+62% vs. previous" label; what they never show is WHAT
   it is being compared to - the previous period is computed, sent, and thrown
   away after producing one adjective. This puts both columns side by side, and
   adds the line no card has: **profit per ticket**, which is the whole
   difference between selling more and selling better (revenue and ticket count
   can both rise while it falls). A table rather than the paired bars in the
   preview - the row above is already two cards of bars, and what was missing
   here was the numbers, not another shape. Cost is deliberately uncoloured,
   the same call `trendColored={false}` already makes on its card. No backend
   change: `previousPeriod` has been sent since 2.0.47, and every figure goes
   through the same `computeTrend`/`computeTrendPoints` the cards use, so a
   number here cannot disagree with the card above it. A range with nothing
   before it ("All time", a custom range with no start) says so in one line
   instead of rendering six dashes.

## 2.14.0 - Cloud sync runs by itself, both directions

**No schema change, no migration (next new one is still 027), no new
dependency** - `rusqlite` gains its `hooks` feature flag, which is the same
crate already in use, not a new one.

1. **The hand-off no longer needs a click.** With sync on, unsent work goes up
   every few minutes and anything new comes down when the app opens. marko asked
   for exactly this ("obe smery automaticky"), which retires 2.12.0's "nothing
   syncs automatically" rule - see `PROTECTED_AREAS.md`'s new 2.14.0 section for
   the rules that replace it.
2. **The new command decides and does nothing.** `cloud_sync_auto` makes one
   Drive metadata request and returns a verdict; `Layout.tsx` acts on it by
   calling the same `cloud_sync_push` / `cloud_sync_pull` the Settings buttons
   call. No second destructive path: every download still runs through
   `restore_database_impl`'s validation, safety backup and rollback. The policy
   itself is `decide_auto`, a pure function with the whole table on one screen
   and 11 tests over it.
3. **Two cases stay manual, on purpose.** Both machines changed since the last
   sync, and a machine that has never synced finding data already in Drive.
   Whole-file sync must pick a winner and neither case has an answer that isn't
   a guess, so both stop and ask. An automatic push never passes `force`, and an
   unreachable Drive is never read as "unchanged" - it does nothing and waits.
4. **Deleting something now counts as a change.** "Has this machine changed
   anything?" is answered by SQLite's own `update_hook` instead of a scan over
   `updated_at`: 17 of the 29 tables have no such column (`transfers` and every
   lookup list among them) and no table records a DELETE at all, so a timestamp
   scan would have missed whole classes of edit and let them be overwritten. The
   hook ignores the app's own bookkeeping tables, so the 30-minute notification
   tick can never trigger an upload.
5. **A machine closed before it could push still knows.** The flag is persisted
   to `app_settings`, written through on every check, on exit (`try_lock`, never
   blocking a close) and when accounts switch - so unsent work is not silently
   pulled over on the next launch.
6. **Restore points** (Settings &rarr; Data). marko asked for a way back "ak by
   ten sync nebol spravny". These files are not new - every destructive restore,
   cloud-sync downloads included, has always taken one first - but the only
   place their path ever appeared was a toast that disappears. `list_restore_points`
   reads both folders they land in (sync-down puts its own next to the active
   database, manual restore in `safety-backups/`), newest first, capped at 20.
   Read-only: it creates nothing and deletes nothing, and restoring one goes
   through the ordinary `restore_database`, which validates, takes its own
   backup first and rolls back - so undoing a bad sync is itself undoable.
7. **Dashboard: the two cards are one row again.** `items-start` let each end at
   its own content height, leaving a step; grid items stretch by default, and
   the platform card now fills the extra height from the inside (`flex-1` list)
   instead of showing dead space under its last row.

## 2.13.4 - One chart, profit back in the row, a scanner for Pulls

**No schema change, no migration (next new one is still 027), no dependency
change.**

1. **Profit rejoins the card row.** 2.13.2 lifted it out as a headline above
   the others; marko wanted all six on one line. It is a card again, just a
   louder one - brand-tinted border, larger figure, margin and ROI as its
   sub-line. Every card narrowed so six fit without wrapping.
2. **The second chart is gone.** Every series it could draw came from the same
   sales, so revenue and ticket count traced the same curve - the second frame
   repeated the first rather than adding to it. **Sales by platform** takes
   that slot: already on this tab below the fold, already the same
   `data.salesByPlatform` at the same period scope, and it answers *where* the
   money came from, which a line over time cannot. The **Cost** metric added in
   2.13.2 stays on the remaining chart.
3. **Pulls gets the screenshot scanner** Events, Orders and Sales already had.
   A new `"pull"` kind for the AI import fills event, date, section, row and
   seats off an image, reusing the `ticketGroups` seat-range expansion the
   `"order"` kind already describes. It deliberately does **not** read the
   buyer name or the pull fee: neither is ever printed on a ticket, and asking
   for either would invite exactly the guessing the feature forbids.

**Not built here:** no toolchain on the machine this was written on.

## 2.13.2 - Separate cards again, editable order prices, two charts

marko's own list after running 2.13.1. **No schema change, no migration (next
new one is still 027), no dependency change.**

1. **Back to one card per figure.** 2.13.1 put every summary figure in a
   single bordered bar; held up against the Attention Center row, marko wanted
   separate boxes. `StatCard` is a card again and `.summary-bar` is now just
   the row that holds them - one component change plus one CSS line, so all 13
   wrappers, Ticket Center's filters and the Calendar tiles followed without
   being touched. `SummaryStat` survives from 2.13.1 for Sales' own results
   strip, which genuinely is one line of text.
2. **An order's purchase price is editable after creation.** Unit price, fees
   and other costs, re-split across the order's tickets with the exact
   `allocate_cents` the create path uses - **including tickets already sold,
   which does retroactively change the profit reported on those sales.**
   marko's explicit choice over blocking the edit; the dialog says so whenever
   the order has sold tickets. 5 new tests.
3. **Fixed before it shipped**: the first version of that form used
   `parseFloat`, and `parseFloat("12,50")` silently returns `12`. It now uses
   `decimalStringToCents`, which the app already had and which handles the
   comma marko actually types. An empty unit price errors instead of zeroing
   the order.
4. **Fixed**: `orders_sheet_sync.rs` also builds an `OrderEditInput`, which the
   new required fields broke. It now reads the three price columns out of the
   database and passes them through, the same way it already did for
   `supplier_id` and `payment_status` - the sheet does not carry a price, and
   filling them with 0 would have zeroed every synced order's price.
5. **Pulls**: the Delete / New Pull buttons sit on the filter row instead of a
   right-aligned row of their own, which had left a band of empty space.
6. **Dashboard: two charts instead of one wide one**, each with its own metric
   switch rather than one being chosen for him. A fourth metric, **Cost**,
   joins Profit / Revenue / Sales - `cogs_cents` has been on every bucket since
   1.6.0 and was the one real series never offered. The headline figure reads
   the same field its line draws (`cogs`, not `total cost`), so the number and
   the chart cannot disagree.

**Not built here:** no toolchain on the machine this was written on. 2.13.2's
first build failed on point 4 above and nothing was ever published under that
number, so it is reused rather than bumped - same reasoning as the cancelled
2.4.0 direction.

## 2.13.1 - One summary style, everywhere

marko ran 2.13.0, pointed at the summary strip `Sales.tsx` has had since
1.9.0 - one border around the whole row, grey label, value beside it - and
asked for exactly that shape on every screen. **No backend change, no schema
change, no migration (next new one is still 027), no dependency change.**

1. **`SummaryStat` moved from `Sales.tsx` into `ui.tsx`** and is now the one
   implementation of a summary figure in the app. `StatCard` is a thin
   wrapper over it (trend and `sub` append inline), so its ~50 call sites
   follow automatically and Sales stops having its own private copy.
2. **`.stat-chip` became `.summary-bar`.** 2.13.0's chips were lighter than
   the old cards but still N bordered objects above a table; this is one
   border around the row. The 13 wrappers converted with it - and they no
   longer carry their own `mb-*`, since `.summary-bar` brings its own.
3. **Ticket Center's four filter cards are segments of that bar.** They were
   the largest thing on a screen whose point is the table underneath. Still
   buttons, still the filter, same counts. Their subtext moved to a `title`
   tooltip - a strip has no second line, and that text explained the count
   rather than naming it.
4. **Calendar's Today / Tomorrow / Next 7 days / Overdue** got the same
   treatment. Tiles with nothing to open now render as plain text instead of
   dead buttons.
5. **Dashboard's profit headline dropped 34px → 26px.** Still the loudest
   number on the page; no longer shouting.
6. **Sync is one button.** It works out the direction itself: if the Drive
   copy has not moved since this machine last synced, it pushes. If it has,
   both sides may hold work and nothing in the app knows whether the local
   side is also dirty - so it stops and asks (*Take theirs* / *Keep mine*)
   rather than guessing. The explicit Sync up / Sync down pair is still there
   under *Choose direction myself*.
7. **The app notices by itself.** `Layout` calls `cloud_sync_status` once on
   open and shows a dismissible bar when the other computer has newer data.
   It **checks and tells - it never syncs**; nothing moves without a click,
   so local-first still holds. This amends 2.12.0's "no startup sync" note in
   `PROTECTED_AREAS.md`, at marko's explicit request.

**Not built here:** no Node or Rust toolchain on the machine this was written
on, so nothing was compiled or tested - the first build is the first check.

## 2.13.0 - Visual redesign across seven screens, plus one balance fix

marko reviewed named alternatives one screen at a time and picked each of
these himself. **No schema change, no migration (next new one is still 027),
no dependency change, no change to refund/resell, `batch_id` or money
handling.** One backend change, unrelated to the redesign, is listed last.

1. **`StatCard` is a chip, not a bordered box** (`ui.tsx`, `index.css`). The
   summary row sat above ten screens at the same visual weight as the table
   underneath it and read as furniture. Changed on the **component**, so all
   ~50 call sites move together and the app keeps ONE summary style - which
   also means `EventDetail`, a screen this round never reviewed, changed with
   it. The callers' `grid` wrappers became `flex flex-wrap gap-2` (12 sites):
   a chip inside a grid cell stretches to the column and stops reading as a
   chip. Trend and `sub` are kept inline rather than dropped.
2. **Sidebar** (`Layout.tsx`): tighter rows, two quiet section headings, and
   the Tickets group given its own surface (the 2.6.0 hairline guide rail and
   the deep indent both go - the card is the grouping cue now). **No counts**,
   deliberately: `Layout` fetches nothing today and marko chose to keep the
   navigation pure frontend rather than add a command to feed numbers.
3. **Ticket Center** (`TicketCenter.tsx`): the `Completed` column mixed a
   stock fact ("Not Sold"), a payment fact ("Not Paid") and a count ("2
   Pending") under one heading, so it had no single meaning and a row never
   said why it was listed. Replaced by a **Needs** column built from the same
   `matchesCategory` predicate the four filter tiles already count with, so
   tiles and rows cannot disagree. Rows now sort **soonest event first**;
   orders with no event date sort last.
4. **Settings** (`Settings.tsx`): section **tabs** instead of six door-cards.
   The `settings/:section` route already existed, so every deep link still
   lands where it did; `/settings` with no section now opens the first tab.
5. **Dashboard** (`Dashboard.tsx`): profit becomes the headline figure with
   margin and ROI as its own sub-line; the other five stay as chips. Six
   identically-sized boxes claimed all six mattered equally.
6. **Finance** (`finance/Overview.tsx`): balance and what you are owed split
   into their own band **above** income/expenses/net. A stock and a flow are
   not the same kind of number, and the period filter only moves one of them.
7. **Calendar** (`Calendar.tsx`): the day detail is a **column beside the
   month grid** instead of a modal, so stepping through days no longer hides
   the grid. Empty days became selectable ("nothing on this day" is an
   answer). Week/Day/Agenda still use the modal - they have no grid to sit
   beside.
8. **Price Checker** (`PriceChecker.tsx`): the event overview is a **light
   table** instead of a card each - link state, listing count, scan age only.
   The per-marketplace breakdown moved behind the click that already opened
   the event (`onOpen={setEventId}` was there before this).
9. **Fixed (backend, separate from the redesign)**: `ACCOUNT_SELECT` had **no
   date filter at all**, so an entry dated in the future was already
   subtracted from a figure labelled *current balance*. It now cuts off at
   `date('now','localtime')`, matching `finance_forecast::eur_balance_as_of`,
   whose own test states the rule this query was breaking: *a future-dated
   entry must not already be folded into 'current' balance*. This is what made
   Finance Overview print two different "current balance" figures a few
   hundred pixels apart. **4 new tests** pin it, all using far-future/far-past
   dates so they never go stale. The cut-off is SQL's own `date()` rather than
   a `today` parameter - see the reasoning on `ACCOUNT_SELECT` before changing
   it.

**Not built in this session:** nothing was compiled or tested here (no Node,
no Rust toolchain on that machine) - the first CI run is the first real check.

## 2.12.1 - Three loose ends from 2.12.0

Cloud Sync is confirmed working end to end on both machines. These are the
follow-ups that were owed, not new features. **No schema change, no migration
(next new one is still 027), no business logic change.**

1. **Fixed**: the `tauri_plugin_deep_link::DeepLinkExt` import now carries the
   same `cfg(all(desktop, debug_assertions))` as the only call that uses it,
   so release builds stop warning. **Deleting it** - the obvious "fix" for an
   unused import - **would have broken `cargo tauri dev`**, where that
   dev-only deep-link self-registration is the whole reason the trait is in
   scope.
2. **Fixed**: the Drive 403 message. That one status covers two very different
   causes and the old text only named the less likely one ("sign in again").
   It now leads with the common case - the Google Drive API not being switched
   on for the OAuth client's Cloud project, which is a one-time, project-wide
   step that applies to every user of the build, not per person.
3. **Added**: Google's own 403 body carries the exact Cloud Console URL for
   that step, so the sync card now extracts it and offers an **Open Google
   settings** button instead of leaving it to be copied out of a toast that
   has already disappeared.
4. **Fixed**: `RELEASE.md`'s macOS instructions said right-click → Open, which
   Apple removed in macOS Sequoia (15). The working path is System Settings →
   Privacy & Security → Open Anyway.

## 2.12.0 - Cloud Sync: one database, two computers

marko works on a Windows PC and a Mac and wants what he writes on one to show
up on the other. **No schema change, no migration (next new one is still 027),
no business logic change, no new dependency, no server.**

1. **Added**: `commands/cloud_sync.rs` - keeps one database snapshot in
   marko's **own Google Drive** and syncs it whole-file, one direction at a
   time. "Sync up" uploads this machine; "Sync down" downloads and restores
   the other.
2. **Deliberately not built**: row-level merging. Every primary key here is a
   per-machine `INTEGER AUTOINCREMENT`, so two machines both mint id 5, and
   invariants like `insert_order_with_tickets`' exact-cent cost split and
   `refund_sale_impl`'s one-way transition have no correct automatic merge. A
   real merge engine is a months-long rebuild, not this release.
3. **Added**: a lost-update guard. Every upload records the Drive file version
   it wrote; the next upload re-checks it. If the other machine pushed since,
   the upload is **refused** and the UI offers an explicit "Overwrite anyway"
   decision instead of silently winning.
4. **Reused, not rebuilt**: downloads go through
   `backup::restore_database_impl`, inheriting its validation, automatic
   safety backup and automatic rollback - the safety backup path is returned
   and shown. Uploads use the SQLite Online Backup API via the new
   `backup::snapshot_db_to`, extracted from `create_safety_backup` so both
   callers share one implementation.
5. **Changed**: `OAUTH_SCOPE` gains `drive.file` - the narrowest scope that
   works, granting access only to files this app created and never to the rest
   of the Drive. **Everyone has to sign in with Google once more**: an
   existing refresh token was issued against the old scope set.
6. **Added**: a "Sync between your computers" card in Settings → Data, above
   Backup/Restore, since it is the same concern made automatic.
7. **Not changed**: nothing runs on a timer or at startup. Sync is off until
   switched on and every sync is a click. With sync off, or offline, the app
   is exactly as local-first as before.

5 new Rust unit tests.

**Build fix applied before this ever shipped**: the first attempt failed to
compile on both platforms with `no method named 'query' found for
reqwest::blocking::RequestBuilder`. `RequestBuilder::query` needs a
`serde_urlencoded` path that this crate's reqwest 0.13 feature set does not
enable, and no other module in this app had ever used it. Every URL is now
built the way `google_sheets.rs` has always built them - `format!` plus
`utf8_percent_encode` for anything dynamic. The download also streams
straight to disk via `std::io::copy` instead of buffering the whole database
in memory.

**Not verified by a build, and no Drive call has ever run.** No Node.js or
Rust toolchain on the machine this was implemented on. The Drive request
shapes are written from the Drive v3 API docs - they are the first thing to
check if sync misbehaves.

## 2.11.1 - Fix the macOS build broken by 2.11.0

2.11.0 published a working **Windows** release, but its macOS leg failed.

1. **Fixed**: the six `APPLE_*` signing variables are removed from the release
   build step. 2.11.0 wired them in "ready for the day credentials exist" -
   but **a missing GitHub secret still defines the environment variable as an
   empty string**, and the Tauri bundler treats `APPLE_CERTIFICATE` being *set*
   as a request to sign. It ran `security import` with an empty certificate and
   aborted with "failed to import keychain certificate". They are now a
   commented block carrying an explicit warning that pasting them back without
   the matching secrets breaks the build again.
2. **Fixed**: `prepare-release` now checks whether a release exists before
   deleting it, instead of letting `gh release delete` exit non-zero on the
   normal first-attempt case and paint a red "failed" annotation on an
   otherwise healthy run.

**What the failed run actually proved**: the Rust `universal-apple-darwin`
build succeeded in 3m36s and the `.app` was already bundling. The universal
target works on CI - signing was the only thing that ever failed.

Nothing else changed: no application code, no business logic, no schema, no
migration (next new one is still 027), no dependency.

## 2.11.0 - macOS builds, verified updater manifest, in-app update centre

marko asked for a professional installer + auto-updater + release pipeline.
Most of it already existed and was kept; the real gap was macOS. **No business
logic change, no schema change, no migration (next new one is still 027), no
new dependency, no new cloud service, no update server.**

1. **Added**: macOS builds. `.github/workflows/build-windows.yml` is renamed
   to `release.yml` and now builds **both** platforms - the Windows NSIS
   `.exe` and a **universal** macOS `.dmg` (Apple Silicon + Intel from one
   download). `release.ps1`'s own "did the workflow survive the mirror" guard
   was renamed in the same change - those two must always agree.
2. **Fixed**: the "delete stale GitHub release" step moved into its own job
   that runs **before** the build matrix. With a matrix, the second runner
   would otherwise delete the release the first one had just published.
3. **Changed**: the release matrix runs one platform at a time
   (`max-parallel: 1`) - both legs publish to the same release and both
   rewrite `latest.json`, so serialising them removes the race.
4. **Added**: a `verify-release` job asserting the finished release really has
   an `.exe`, a `.dmg` and a `latest.json`, that the manifest covers both
   platform families, and that every entry has a non-empty signature and url.
   A silently-wrong updater manifest is the failure nobody notices until users
   stop getting updates.
5. **Added**: a very small update status on the Dashboard. It reads the
   launch-time check result and links to Settings → Software; it does not run
   its own check and does not install anything, so there is still exactly one
   updater UI and one progress state.
6. **Added**: Settings → Software now shows Current version / Latest version /
   Last checked alongside the existing check-and-install flow and release
   notes.
7. **Changed**: the launch-time update check now also repeats every 6 hours
   for sessions that stay open - deliberately slow, no polling. Manual "Check
   for updates" is unaffected.
8. **Added**: `RELEASE.md` - the release process, every repository secret and
   what it does, the artifact list, and both the developer and user
   checklists. No secret values in it.

**Signing, stated honestly:** updater signing is configured and is what makes
in-app updates secure. **Windows code signing and macOS code signing/
notarization are NOT configured** - both need paid external credentials. The
workflow passes all six Apple variables through so adding the secrets is the
only remaining step, and nothing here fakes a certificate. Until then the
`.exe` shows a SmartScreen warning and the `.dmg` needs right-click → Open on
first launch.

**Not verified by a build.** No Node.js and no Rust toolchain on the machine
this was implemented on, and no macOS build has ever run - the first tagged
run of this workflow is itself the test.

## 2.10.0 - Price Checker event overview replaces the event dropdown

marko's own request: see every relevant event at once instead of picking one
from a dropdown. **UX only** - the scanner, parser, readers, market analysis
and history are untouched. **No schema change, no migration (next new one is
still 027), no new dependency.**

1. **Changed**: opening Price Checker now lists every **upcoming** event as a
   dense card - name, date, venue/city, a per-marketplace row (Linked / No
   link, plus that marketplace's last check and listing count), and an
   event-level line with the newest listing count or "Not scanned yet".
2. **Added**: multi-select with checkboxes, "Select all" (scoped to what is
   currently visible, so a filtered list can't silently select what you can't
   see), "Clear selection", and a "Selected: N events" bar.
3. **Added**: local search over event name, venue and city, and four quick
   filters - All / Needs link / Not scanned / Scanned.
4. **Added**: one read-only backend command, `list_price_checker_overview`,
   answering the whole list in four flat queries instead of calling
   `get_price_checker_summary` once per event. It writes nothing, triggers no
   scan and makes no marketplace request.
5. **Deliberately not added**: a "Scan failed" state. A failed scan is never
   persisted anywhere in this app - `price_checks` has no status column, and a
   row only reaches it through the explicit review-then-save step, so by
   construction every stored check succeeded. The scanner's own error/blocked
   states live in memory for the life of one scanner window. A "Scan failed"
   badge would be inventing a state the database does not have.
6. **Check selected**: opens the first selected event's own flow and keeps the
   selection, so the rest are one click each. The scanner opens a real visible
   browser window marko drives himself, so no parallel sessions and no queue
   automation were invented.
7. **Added**: an "All events" back link, since the dropdown that used to be
   the way back is gone.
8. **Not changed**: parser, readers, DOM scanning, market calculations, tier
   grouping, section/row metadata, price history, Your Tickets. Tier/Level
   stays a market grouping; section/row stay metadata. No background
   monitoring, no polling, no scheduled scanning, no repricing.

8 new Rust unit tests (42 in `price_checker.rs` total).

**Not verified by a build.** No Node.js and no Rust toolchain on the machine
this was implemented on, so `cargo test --lib`, `cargo check --lib`,
`npx tsc -b` and `npm run build` could not be run.

## 2.9.0 - Price Checker accuracy fix, scan report, filters, CSV export

marko reported the scanner reading prices wrong. Diagnosis first, then the
smallest fix per cause. **No schema change, no migration (the next new one is
still 027), no new dependency, no new marketplace.**

1. **Fixed (root cause of the wrong prices)**: `price_checker_scan.js`'s
   generic text-node walker found money in one text node and then called
   `candidateFrom(parentElement)`, which re-parsed the parent's *entire* text
   and kept the **first** money match in it. On "Was $200 / Now $120" markup
   that stored the crossed-out old price; on a wrapper whose `aria-label`
   reads "Total incl. fees" it stored the fee-inclusive total. The matched
   money, and the text it was matched in, are now passed into
   `candidateFrom`.
2. **Added**: three price rejection rules that did not exist at all before -
   struck-through prices (`<s>/<del>/<strike>`, line-through class or computed
   style), money labelled total/subtotal/fee/service charge/delivery/tax/was/
   original/RRP, and money inside header/footer/nav/aside/cart/checkout/modal
   page chrome.
3. **Fixed (metadata leak)**: `findListingContainer` fell back to "three
   ancestors up" and `nearbyListingContext` then regexed that whole subtree,
   so section/row/quantity/tier could be read off a *neighbouring* listing or
   off page chrome. The container now reports whether it is confident;
   metadata is read from a tight scope when it is not, and the listing is
   flagged `incomplete` instead of being presented as a clean read.
4. **Fixed (dedup, both directions)**: a real marketplace listing id is now
   the whole cross-scan identity - it used to be one of seven fields
   *including the price*, so the same listing re-read after a scroll counted
   twice as soon as the price read differently. And `tier` joined the fallback
   key, whose absence merged two listings that differed only by tier and
   deleted a real one. Without an id the price stays in the key on purpose.
5. **Fixed (currency blending)**: `compute_scan_stats` averaged across every
   listing regardless of currency and labelled the blended result with
   whichever currency came first. It now computes inside the largest
   single-currency group only and reports how many listings were excluded -
   which also makes it agree with `price_checker_analysis`, which has always
   partitioned by currency correctly.
6. **Added**: a Found / Accepted / Skipped / Duplicates scan summary with
   per-reason skip counts.
7. **Added**: six plain result filters (marketplace, tier, currency, min/max
   price, complete vs incomplete) - not a filter builder.
8. **Added**: Export CSV, reusing the same `plugin-dialog` `save()` + Rust
   writer every other export in this app already uses.
9. **Not changed**: the manual Visible Scanner workflow, Tier/Level as a
   grouping (never a pricing input - no section/row/seat pricing, no price
   suggestions), Your Tickets comparison, history, refund/resell, `batch_id`,
   money/integer cents, Orders/Tickets/Sales/Listings/Finance/Fulfillment/
   Attention/Calendar/Google Sheets. No background monitor, no scheduled scan,
   no polling, no CAPTCHA bypass.

15 new Rust unit tests (43 in `price_checker_scanner.rs` total).

**Not verified by a build, and the DOM fixes are not verified against a live
page.** No Node.js and no Rust toolchain on the machine this was implemented
on, so `cargo test --lib`, `cargo check --lib`, `npx tsc -b` and `npm run
build` could not be run, and the injected browser script cannot be executed
here at all. The live marketplaces have never been reachable from this
sandbox either (the same limitation the 2.1.9 script has always carried), so
every DOM-level fix is reasoned from the code and must be confirmed against a
real listings page.

## 2.8.0 - Calendar redesign + advanced calendar workflow

marko asked to make the TIQR Operations Calendar one of the app's main work
screens. **No schema change, no migration (the next new one is still 027), no
new index, no new dependency.** `commands/calendar.rs` stays what it has
always been: a read-only aggregator with no write path.

1. **Added**: four views instead of two - **Month, Week, Day, Agenda**. All
   four share one data path (`get_calendar` over a date range), one filter
   state and one search box; only the range and the layout differ, so
   switching period still fetches exactly one window.
2. **Added**: two genuinely new date sources, **`finance`**
   (`finance_entries.entry_date`) and **`recurring`**
   (`recurring_expenses.next_date`). Both were re-derived from the live
   schema and command code rather than trusting the 2.5.0 note, exactly as
   `PROTECTED_AREAS.md` instructs.
3. **Still not added**: payouts, payments, fulfillment. Re-checked this
   release - there is no payout entity or date anywhere, migration 007's
   `payments` table still has zero live SQL in any command module, and
   `tickets.delivery_status` still has no date column. What the 2.5.0 pass
   never checked was Finance, which is where this app's real dated money
   lives - so it is added under its own honest name rather than rebranded as
   a "payout". There is a standing test that fails if an invented category
   ever reaches the calendar.
4. **Added**: a real **Overdue** count. `recurring_expenses.next_date` is the
   only genuine due date in the app; the rule is the same one Finance's own
   Accounts tab applies (active template, `next_date` before today), and
   paused templates are excluded because their `next_date` is frozen and not
   actionable.
5. **Added**: a Today / Tomorrow / Next 7 days / Overdue summary strip, a Day
   Detail with a per-kind summary panel, per-day workload bars (three muted
   segments, deliberately not a color scale), event countdowns derived only
   from the event's own date, calendar-local search (dims non-matches in the
   grid, filters the lists, can jump to the first match), and filter chips
   that only appear for kinds actually present in the loaded range.
6. **Deliberately not built**: a time-of-day axis. Every date this app stores
   is date-only, so Week is seven day columns rather than a faked 24-hour
   timetable, and Day groups by kind rather than by hour.
7. **Deliberately not built**: quick-add of tasks/reminders/notes. There is
   no task, note or reminder table anywhere in this schema, and marko was
   explicit that this release must not stand up a new task database.
8. **Changed**: the grid and the lists scroll inside themselves rather than
   growing the page, so the header, filters and summary strip stay put and
   there is only ever one scrollbar.
9. **Not changed**: refund/resell, `batch_id`, money/integer cents,
   Orders/Tickets/Sales/Listings/Finance/Fulfillment/Attention business
   logic, Price Checker, Google Sheets.

9 new Rust unit tests (23 in `calendar.rs` total).

**Not verified by a build.** Implemented on a machine with no Node.js and no
Rust toolchain, so `cargo test --lib`, `cargo check --lib`, `npx tsc -b` and
`npm run build` could NOT be run - marko chose to proceed on static review.
Run all four before publishing the tag, and regenerate
`Cargo.lock`/`package-lock.json`.

## 2.7.0 - AI Import Assistant (screenshot -> pre-filled form, never straight to the database)

marko's own request: drop, paste (Ctrl+V) or upload a screenshot into the New
Event, New Order or New Sale form and have Claude read the structured data off
it. **No schema change, no migration (the next new one is still 027), no new
dependency.**

The rule the whole feature is built around, in his words: "AI NIKDY nesmie
priamo vytvoriť alebo meniť databázový záznam." The flow is image -> Claude ->
structured result -> review -> user confirms -> the EXISTING create form -> the
EXISTING create command -> DB.

1. **Added**: `src-tauri/src/commands/ai_import.rs` - one command,
   `analyze_import_image`. It takes no `AppState`, opens no `Connection` and
   has no write path of any kind, so it cannot create or change a record even
   by mistake. Reuses `ai_categorize.rs`'s build-time embedded
   `ANTHROPIC_API_KEY` (the key never reaches the frontend) and its
   retry-once-on-a-transient-status policy.
2. **Added**: a strict per-kind JSON schema (`output_config.format`). Every
   extracted value is a plain string or `null`, each carries a
   `high`/`medium`/`low` confidence, and a field that isn't on the image comes
   back `null` rather than being filled in. `sanitize_result` then drops any
   field name the form has no slot for, keeps only the first of a duplicate,
   and collapses blank values to `null`.
3. **Added**: multiple ticket groups are returned separately and never merged.
   Because `OrderInput` carries one section/row/tier/price per order, the panel
   fills one group at a time and says so, instead of inventing a multi-group
   order shape the backend has never had.
4. **Added**: `src/components/AiImportPanel.tsx` - one compact shared panel,
   embedded in all three forms. Not a new page, route, tab or sidebar.
5. **Added**: `src/lib/aiImport.ts` - image validation, downscale to 1568px
   (only when needed, so a normal screenshot is sent untouched), an FNV-1a
   fingerprint, clipboard/drop extraction, an ISO-date guard and a lookup
   matcher.
6. **Changed**: `dragDropEnabled: false` on the main window
   (`src-tauri/tauri.conf.json`) so HTML drop events reach the webview -
   Tauri's default of `true` swallows them. Nothing in the app used Tauri's own
   drag-drop event.
7. **Cost control**: one analysis per explicit user action; a per-session image
   fingerprint cache so the same screenshot is never analyzed twice; editing an
   extracted field never re-calls; retry only on a click; no background or
   timed request anywhere.
8. **Not changed**: refund/resell, `batch_id`, money/integer cents,
   Orders/Tickets/Sales/Listings/Finance/Fulfillment logic, Price Checker,
   every existing create command and every existing validation rule. AI
   extraction cannot bypass validation - the same commands still run at save
   time on values the user has seen and confirmed.

9. **Fixed (same version, after the first release run failed)**:
   `release.ps1`'s `$CommitMsg` had literal double quotes in it, which Windows
   PowerShell 5.1 cannot pass to `git.exe` intact - `git commit` exited
   non-zero and the script stopped with "git commit failed". The quotes are
   gone and there is now a guard right before the commit that catches this
   with a real explanation instead of a git error. See `PROTECTED_AREAS.md`'s
   "2.7.0" entry.

**Needs the `ANTHROPIC_API_KEY` GitHub Actions secret** (already wired into
both build paths). Without it the panel reports "AI import isn't available in
this build" and nothing else changes.

**Not verified by a build.** Implemented on a machine with no Node.js and no
Rust toolchain, so `cargo test --lib`, `cargo check --lib`, `npx tsc -b` and
`npm run build` could NOT be run - marko chose to proceed on static review.
The 28 new Rust unit tests are written but have never been executed. Run all
four before publishing the tag, and regenerate `Cargo.lock`/`package-lock.json`.

## 2.6.0 - Complete visual redesign (UI/UX only)

marko's own task: make TIQR Manager look and feel like a modern, premium
desktop app, explicitly with **no new features, no new workflow, no new
database systems**. Nothing in `src-tauri/` changed - the backend is
byte-for-byte identical to 2.5.2. No migration, no new dependency.

1. **Changed**: one shared design layer now defines the whole app's look -
   `tailwind.config.js` (retuned `slate` ramp so light and dark each get a
   real background -> surface -> line hierarchy instead of one being an
   inversion of the other; a 3-step `shadow-card`/`raised`/`overlay` scale;
   a tighter radius rhythm; a 120-180ms motion budget). The `brand` blue
   ramp is deliberately untouched.
2. **Changed**: `src/index.css` - base typography (tabular figures on every
   number in the app), restyled `.input`/`.label`/`.th`/`.td`/`.card`, new
   `.section-title`, `.field-invalid`, `.skeleton`, `.card-interactive`, and
   the new `.table-shell`/`.table-flush`/`.row-selected` table system.
   `.th-c-narrow`/`.td-c-narrow`'s measured 2.0.37 metrics are unchanged.
3. **Changed**: `src/components/ui.tsx` - every shared control restyled.
   `Button` gained an optional `size`; `Field` now turns its own control red
   on error; `Badge`/`InlineStatusSelect` share one status pattern with a
   leading dot; `StatCard` is shorter and quieter; `Modal`/`ConfirmDialog`/
   `EmptyState`/`ModalFooter`/`TabSwitcher` redesigned. The status
   vocabulary (which tones exist, which value maps to which) is unchanged.
4. **Added**: `Skeleton` and `TableSkeleton` in `ui.tsx`, plus
   `SEGMENTED_TRACK`/`segmentedItemClass` - the app's one tab/segmented
   pattern, now genuinely shared instead of hand-rolled per page.
5. **Changed**: `src/components/Layout.tsx` - sidebar redesigned (accent-bar
   active state, hairline section separation, restyled theme toggle and
   profile area). Same items, same order, same routes, same `w-48` width,
   same behaviour.
6. **Changed**: all 22 tables in the app moved onto the shared table shell -
   sticky opaque headers, internal scrolling instead of page scrolling, one
   hover treatment, one selected-row treatment. Column widths, `colgroup`
   percentages and the narrow-window breakpoint are untouched.
7. **Changed**: six list pages (Orders, Sales, Tickets, Events, Pulls,
   Ticket Center) now show a table skeleton while loading instead of a
   centred spinner.
8. **Changed**: `prefers-reduced-motion` now disables every transition and
   animation across the app.
9. **Not changed**: any business logic. The whole diff contains no `api.`
   call, no state/handler/effect, and no routing change - see
   `PROTECTED_AREAS.md`'s "2.6.0" entry.

**Not verified by a build.** This round was implemented on a machine with no
Node.js and no Rust toolchain, so `npx tsc -b`, `npm run build` and
`cargo check --lib` could NOT be run - marko chose to proceed on static
review rather than install a toolchain. Run all three before publishing the
tag, and regenerate `Cargo.lock`/`package-lock.json` (their version entries
were bumped by hand for the same reason).

## 2.5.2 - "Forgot password?" via a deep link into the app; Discord sign-in deferred

marko's own follow-up request after 2.5.1. No schema changes. Discord
sign-in was asked for too but is NOT built - see `PROTECTED_AREAS.md`'s
"2.5.2" entry for why (needs a Cloud Function + Firebase's paid Blaze plan,
which marko chose against for now).

1. **Added**: a real "Forgot password?" flow from Welcome.tsx's login form,
   using Firebase's own `sendPasswordResetEmail`/`verifyPasswordResetCode`/
   `confirmPasswordReset` - no new backend, no Cloud Functions, no Blaze
   plan.
2. **Added**: the emailed reset link now opens TIQR Manager directly
   (`handleCodeInApp: true` + a new `tiqrmanager://` custom URL scheme via
   `tauri-plugin-deep-link`) instead of a browser tab - marko's first choice
   (a typed short code) would have needed the same paid infrastructure as
   Discord above, so this was worked out with him directly as the
   alternative.
3. **Added**: `docs/reset-redirect.html`, a static hand-off page on the same
   GitHub Pages site `docs/privacy.html` already uses, forwarding Firebase's
   link into the app.
4. **Added**: `src/pages/ResetPassword.tsx` - verifies the incoming reset
   code and sets the new password.
5. **Needs one manual step**: add `biznismarko9-source.github.io` to Firebase
   Console -> Authentication -> Settings -> Authorized domains before this
   works end to end.

## 2.5.1 - Ticket Center rebuilt around orders, sidebar reorder, Calendar visual refresh

marko's own direct follow-up on the 2.5.0 release below, delivered as its
own round. No backend/schema changes. Packaging
(`REDESIGN-2.5.1-REPORT.md` + zip) was deferred past this round - marko
signaled more feedback was coming right after - and built once he
separately asked ("zabal to"; see `PROTECTED_AREAS.md`'s "2.5.1" entry).

1. **Changed**: Ticket Center moved out of Finance (was a subtab there for
   one version, 2.4.4) back to its own top-level page/route
   (`/ticket-center`).
2. **Rebuilt**: Ticket Center now lists ORDERS (via the same `api.listOrders`
   Orders.tsx already calls), not individual tickets/sale-batches - click an
   order to see and edit what's outstanding on each of its tickets on the
   existing Order Detail page. `TicketControlCenter.tsx` (2.4.3),
   `FulfillmentCenter.tsx` (2.2.12), and the `finance/TicketCenter.tsx`
   subtab shell are all deleted; 4 new quick-filter tiles (Needs
   attention/listing/payment/delivery) replace both pages' old filter/
   category systems.
3. **Changed**: sidebar top-level order now matches marko's exact list -
   Dashboard, Tickets, Price Checker, Pulls, Finance, Ticket Center,
   Calendar (Calendar moved from right after Dashboard to last).
4. **Changed**: Order Detail's "arrived from" Back link now also recognizes
   Ticket Center as an origin (`/ticket-center` -> "Back to ticket center").
5. **Visual refresh only**: the Calendar page (2.5.0) - a consistent accent
   color per entry kind across the grid/filters/modal/summary, severity
   shown as a ring/text-color layered on top instead of the only signal,
   weekend/today cell shading, and a weekday name in the Day Detail title.
   No data, hook, or navigation change.

## 2.5.0 - TIQR Operations Calendar

marko's own spec for a new cross-domain Month/Week calendar page, delivered
together with the 2.4.4 round below in this same release.

1. **New**: `/calendar` page - a Month/Week calendar aggregating every part
   of the app with a real date: events, orders, sales (grouped by batch,
   never one row per ticket), pulls, and Attention Center items. Today/
   Previous/Next navigation, a Day Detail view (click any day or its
   "+X more"), a client-side Filters row, and a "Today & next 7 days"
   summary card.
2. **New backend**: `commands/calendar.rs` (`get_calendar`, one command) +
   `CalendarFilters`/`CalendarEntry` models. No new migration, no new
   table - reuses `attention_center::get_attention_center_impl` and
   `sales::GROUP_KEY_EXPR` directly rather than re-deriving either.
3. **Deliberately NOT implemented**: payout, payment, and fulfillment
   calendar entries - none of the 3 has a real, reliably-existing date
   anywhere in this app today (see `PROTECTED_AREAS.md`'s new "2.5.0"
   entry for the full research). Nothing was invented to fill these in.
4. **New sidebar entry**: "Calendar", directly below Dashboard.
5. 14 new Rust tests (`commands/calendar.rs`), full suite green (1052
   tests); `npx tsc -b` and `npm run build` both clean.

## 2.4.4 - Ticket Center consolidation, sidebar regroup, theme toggle

marko's own request, a pure frontend/UX round (no backend, schema, or
migration changes at all) delivered before his separate 2.5.0 Calendar
spec.

1. **Merged**: Ticket Control Center (2.4.3) + Fulfillment Center (2.2.12),
   previously two standalone top-level sidebar pages, now live under
   Finance as one "Ticket Center" tab with two subtabs (Control Center,
   Fulfillment) - new `finance/TicketCenter.tsx`. `/control-center` and
   `/fulfillment` routes removed; both components reused unchanged
   internally aside from Control Center's own fixes below.
2. **Sidebar regrouped**: Events/Orders/Tickets/Sales/Inventory now sit
   under one collapsible "Tickets" entry instead of 5 flat rows - all 5
   routes themselves unchanged.
3. **New**: one-click light/dark toggle above the sidebar's profile widget,
   reusing the existing `useTheme()` hook. Settings -> Appearance (the old
   3-way Light/System/Dark picker) removed - moved, not duplicated.
4. **Fixed**: Ticket Control Center's sticky header had a translucent
   dark-mode background (`dark:bg-slate-800/60`) letting scrolled row text
   bleed through while scrolling - now fully opaque.
5. **Changed** (Ticket Control Center): "Ticket / Seats" column renamed to
   "Seats", now shows only the seat location (ticket code moved to a hover
   tooltip); Order cell now independently opens Order Detail on click.
6. **Changed** (Dashboard): "Sales by platform"'s internal scrollbar
   removed, replaced with the same slice + "Show N more" pattern the
   Activity tab's Recent cards already use.

`npx tsc -b` and `npm run build` both clean; no Rust changed, so
`cargo test --lib` is unaffected. Version bumped **2.4.3 -> 2.4.4**.

## 2.4.3 - Ticket Control Center

marko's own focused-task request: one central work screen to manage and
check tickets across every event at once, built entirely on top of the
existing tickets/listings/sales data - explicitly not a new parallel ticket
system.

1. **New**: `/control-center` page - sticky filters (Event, Date range,
   Tier/Level, Section, Row, Ticket status, Listing status, Sale status,
   Payment status, Delivery status, Marketplace), 8 quick filters (All/
   Unsold/Unlisted/Listed/Sold/Pending payment/Pending delivery/Refunded),
   search across ticket/order/event/section/row/marketplace/listing id-url,
   and a dense table (first in this app to own its own scroll instead of
   growing the whole page) over one new backend query,
   `list_control_center_tickets` (`commands/ticket_control_center.rs`). Row
   click opens the existing Sale Detail or Order Detail, whichever applies.
2. **Bulk actions - all existing mechanisms**: Section/Row/Tier/Seat/Listing
   price via the shared `BulkTicketEditBar` (now with a new Tier option,
   also benefiting Sale Detail/Order Detail); listing status via the
   existing `bulkUpdateTicketListingsStatus`; CSV export of the selection
   via the existing `exportTicketsCsvSelected`. No refund/resell bulk
   actions, per marko's explicit instruction.
3. **New, purely additive read signal**: `isRefunded` (an `EXISTS` check
   against `sales`) - lets the "Refunded" quick filter surface a
   refunded-and-not-yet-resold ticket, which the existing active-sale-only
   join can't otherwise distinguish from a never-sold one. Reads only; no
   refund/resell/money logic touched.
4. **Untouched**: refund/resell logic, `batch_id`, every money/cents column,
   Listings/Sales/Finance/Orders core - per marko's explicit "DÔLEŽITÉ" list.
   No new migration.

Full backend suite green: `cargo test --lib` (1038 passed, +12 over 2.4.2,
0 failed, 3 ignored), `cargo clippy --lib` clean of new warnings, `npx tsc
-b`, `npm run build` all clean. Version bumped **2.4.2 -> 2.4.3**.

## 2.4.2 - Live Market Monitor removed; Price Checker back to a manual tool

marko decided he does not want the 2.4.1 "Live Market Monitor" feature in
the app at all (*"TÚTO FUNKCIU NECHCEM V APLIKÁCII VÔBEC"*) and asked for
it removed entirely - no background/scheduled scanning, no automatic
monitoring, ever - with Price Checker returned to a purely manual tool.
Full reasoning in `PROTECTED_AREAS.md`'s new "2.4.2" entry.

1. **Removed entirely**: backend module `price_checker_monitor.rs` and its
   2 commands (`get_market_monitor_summary`, `list_market_snapshots`); both
   scan-result hooks into it from `price_checker_scanner.rs`; the
   `market_alert` Attention Center category (`attention_center.rs`'s
   `push_item` reverted to its original 2-shape key, Dashboard's 6th box
   and grid column removed); Auto Monitor (ON/OFF + 15m/30m/1h/3h/6h
   interval) and "Scan All" from `PriceChecker.tsx`; the Live Market
   Monitor panel and the Market History view/modal; every related Rust
   struct (`models.rs`) and TypeScript type (`types.ts`/`api.ts`).
2. **Price Checker unchanged**: event selection, marketplace URLs/source
   handling, the manual Visible Scanner, Market Analysis, tier/section
   grouping, price history, Your Tickets comparison - all pre-existing
   functionality, untouched. No redesign.
3. **Database**: migration `026_price_checker_market_monitor.sql` and its 4
   tables (`market_snapshots`, `market_snapshot_tiers`, `market_source_
   status`, `market_alerts`) were **kept in the schema, not deleted** - 2.4.1
   already shipped and marko's own local DB has already run this migration,
   so this codebase's forward-only migration rule means it can never be
   safely deleted or renumbered. Only the application code that read/wrote
   these tables was removed; no user data was touched. The next new
   migration is **027**, not a reused "026".
4. Two small judgment calls, flagged per this codebase's own "smallest
   consistent solution, flag on ambiguity" convention: "Scan All" was
   removed as in-scope (it existed purely to complement Auto Monitor, no
   standalone purpose without it); `price_checker_analysis.rs`'s
   `group_by_tier` was left `pub(crate)` rather than reverted to private
   (bumped in 2.4.1 for the now-deleted module to reuse - harmless residual,
   not worth an extra touch to that protected file for zero functional
   gain).

Full backend suite green: `cargo test --lib` (1026 passed, -32 removed with
the feature, 0 failed, 3 ignored), `npx tsc -b`, `npm run build` all clean.
Version bumped **2.4.1 -> 2.4.2** - removing a shipped feature still bumps
the version forward, never back to a number already used (same precedent as
2.3.0's revert shipping as 2.3.1) - see `PROJECT_STATE/CURRENT_STATE.md`'s
"## Version" section. See `PROTECTED_AREAS.md`'s new "2.4.2" entry before
ever touching migration 026, `group_by_tier`, or migration numbering again.

## 2.4.1 - Price Checker Live Market Monitor

Marko cancelled the "Live Event Intelligence" direction below outright
("Predchádzajúci nápad 'Live Event Intelligence' RUŠÍME ÚPLNE") and asked
for all online/live-market functionality to live directly inside Price
Checker instead: EVENT -> MARKETPLACE SOURCES -> SCAN -> SNAPSHOT -> HISTORY
-> CHANGE DETECTION -> MARKET ALERTS, built entirely on the already-shipped
Visible Scanner (2.1.9) and Market Analysis (2.2.0) - no CAPTCHA bypass, no
proxy rotation, no anti-bot workaround, and no automation beyond reading
whatever a human already has open in a real, visible window. Full reasoning
in `PROTECTED_AREAS.md`'s new "2.4.1 - Price Checker Live Market Monitor"
entry.

1. **New backend module `price_checker_monitor.rs`** - records a permanent,
   never-overwritten snapshot after every successful/partial scan
   (`market_snapshots`/`market_snapshot_tiers`, migration `026_price_
   checker_market_monitor.sql`), tracks each marketplace's connection status
   (`market_source_status`: not_connected/connected/success/failed), and
   diffs each new snapshot against the previous one - overall and per tier
   (never per section/row/seat) - to raise MARKET DROP / MARKET RISE / NEW
   SUPPLY / SUPPLY DROP alerts (`market_alerts`) at transparent, reused
   thresholds (5% price, 20% supply - the same constants Price Checker's own
   recommended-price and Inventory Intelligence's own outside-market logic
   already use elsewhere). A SOURCE FAILURE alert fires only on a genuine
   success-to-failure transition, never on the first-ever failure or on
   repeated consecutive ones - keeps this quiet instead of noisy.
2. **Auto Monitor** - an ON/OFF toggle with a 15m/30m/1h/3h/6h interval,
   scoped to one already-open Visible Scanner window per marketplace card;
   it is the identical "Scan Visible Prices" call the button already makes,
   fired on a schedule, never opening a window or reading a page on its own,
   and it turns itself off the moment that window closes. "Scan All" fires
   the same call once for every marketplace on the current event that
   already has a window open.
3. **Price Checker UI**: each marketplace card gained a Live Market Monitor
   panel - connection status, last successful scan (never cleared by a
   later failure - the app stays useful on cached data even fully offline),
   the latest snapshot's stats, Auto Monitor controls, and its recent Market
   Alerts; plus a "Market History" view of every saved snapshot.
4. **Dashboard Attention Center gained a 6th box, "LIVE MARKET ALERTS"**
   (`market_alert` category, the single most recent alert per event/
   marketplace) - named differently from marko's own literal spec wording
   ("MARKET ATTENTION") because that title was already taken by the
   existing `outside_market_price` box (2.2.11, an unrelated feature: your
   OWN listing prices vs. the market). Clicking a row jumps straight to
   Price Checker at that event and marketplace (scrolled into view and
   briefly highlighted) - no new separate dashboard.

32 new backend unit tests (27 in `price_checker_monitor.rs`, 5 in
`attention_center.rs`). Full suite green: `cargo test --lib` (1058 passed, 0
failed, 3 ignored), `npx tsc -b`, `npm run build`. See `PROTECTED_AREAS.md`'s
new "2.4.1 - Price Checker Live Market Monitor" entry before touching any of
this again, and the entry directly below for why reusing the version number
"2.4.0" was verified safe before this release ultimately moved one step
further to **2.4.1** instead (a plain filename-collision reason, not an
auto-updater one - see `PROJECT_STATE/CURRENT_STATE.md`'s "## Version"
section for the full story).

## 2.4.0 (pre-release direction, never shipped) - Live Event Intelligence Foundation - REVERTED, see entry above

marko's next spec after 2.3.5: an Event can now optionally carry a
CONFIRMED online identity on exactly 3 marketplaces - Viagogo, Vivid Seats,
Ticombo. Foundation work only - no pricing logic, no changes to the
existing Price Checker or its scanner. Full reasoning in
`PROTECTED_AREAS.md`'s new "2.4.0 (pre-release direction)" entry.

**Kept as history only (this file is append-only) - marko reviewed this
build and cancelled the whole direction outright** ("Predchádzajúci nápad
'Live Event Intelligence' RUŠÍME ÚPLNE") in favor of putting all online/
live-market functionality directly inside Price Checker instead - see the
real "2.4.1 - Price Checker Live Market Monitor" entry above. Unlike 2.3.0
(reverted as **2.3.1**, a version bump forward, because that build had
already been offered as a real release), this direction was never released -
only handed over as a review package - so no install anywhere ever recorded
it, and the version number "2.4.0" was safe to reuse for the real feature
that replaced it - though that real feature's own version ultimately moved
one more step forward, to **2.4.1**, for the separate and unrelated
practical reason explained in the entry above and in
`PROJECT_STATE/CURRENT_STATE.md`'s "## Version" section. See that section
and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.4.0 (pre-release direction)"
entry for the full reasoning, and re-verify that same fact (was anything
with this version/migration number ever actually installed anywhere?)
before assuming a THIRD reverted direction can reuse its number too - it
depends entirely on that, not on precedent alone.

1. **New table `event_online_sources`** (migration
   `026_live_event_intelligence.sql`) - a standalone table, not a new
   column on `events` and not a foreign key onto the general, marko-managed
   `marketplaces` lookup. `UNIQUE(event_id, source)` enforces "at most once
   per marketplace per event"; `verified`/`active` are two independent
   flags (confirmed-by-a-human vs. still-connected).
2. **Discovery, always human-confirmed.** "Find Online Event" opens a real,
   visible browser window (reusing the Visible Scanner's technique, never
   its code/state) on a best-effort search URL; marko searches himself;
   "Capture this page" reads only the current page's title+URL as one
   candidate; "Use this one" is the only action that ever saves a source as
   verified. "Refresh" is the identical flow against an already-saved URL -
   also how a manually-connected source becomes verified. "Connect
   manually" skips the window for when marko already has the URL.
3. **New compact "Live Event Intelligence" block** on EventDetail's
   Overview tab (above Inventory Intelligence) - always exactly 3 rows,
   Find Online Event / Connect manually / Refresh / Open source /
   Disconnect-Reconnect.
4. **No new networking primitive at all** - the only network access this
   feature ever performs is opening a real, visible window a human drives;
   no backend HTTP calls to any of the 3 marketplaces, ever.

19 new backend unit tests + 3 new migration-upgrade tests (existing events
untouched, CHECK constraint enforced on an upgraded database, cascade
delete verified). `cargo test --lib` (1042 passed), `npx tsc -b`, `npm run
build` all green.

## 2.3.5 - Sync/push redesign: self-healing push, real sync diff, no more UI freeze

Marko came back after 2.3.4 with one detailed message re-explaining the
whole intended sync/push design from scratch, using Pulls as the reference -
the narrow bug fixes so far hadn't matched his actual mental model. Three
fixes, all covered in depth in `PROTECTED_AREAS.md`'s new "2.3.5" entry:

1. **UI freeze fixed.** Every sync/push button froze the whole app until its
   network call finished (marko: "ked zapnem alebo kliknem na cokolvvek...
   apka zamrzne"). All 11 sheet-sync commands were plain synchronous `fn`,
   which Tauri runs on its single main/IPC thread - converted to `async fn`
   + `spawn_blocking`, same pattern already proven for Google sign-in
   (2.0.12->2.0.13). Zero changes to the underlying sync/push logic.
2. **Order/Sales sync now updates an already-linked row when the sheet
   changed it**, matching Pulls sync - previously every marked row was
   skipped unconditionally, no comparison at all. Tracks platform/date/
   currency/email/Order ID; deliberately never quantity/price (tickets
   already have exact-cent costs allocated against those - same "ask before
   touching" boundary as the 2.0.53 currency-push feature).
3. **Push Orders is now self-healing** - marko, twice: if he deletes an
   order's row from the sheet by hand and pushes again, it must notice and
   add it back, using the same code. Push Sales needed no changes at all for
   this: it never creates rows, so once Push Orders restores the row, Push
   Sales's existing "fill in blank cells" behavior already re-populates the
   sales columns on the next run - proved with a dedicated test chaining
   both functions. This also resolves the row-426 order that went
   permanently invisible after 2.3.4 (see that entry below).

9 new/updated sync-diff tests, 3 push self-healing tests, 1 cross-function
integration test. Full suite 1020/1020 passed, 0 failed; `tsc -b`/
`npm run build` clean. No frontend changes needed - the sync/push buttons'
busy-state/spinner UI already existed, it was just neutered by the backend
freeze.

## 2.3.4 - Sheets push: row placement fixed properly this time

2.3.3's fix wasn't enough - marko sent a screenshot proving it. Revenue (P)
and Profit (Q) in his real sheet had live formulas filled all the way to
row 425, even though only ~16 rows have real order data. Somewhere in this
sheet's history, `plan_sheet_structure_updates` had written formulas that
far down, and a formula is non-empty content too - so the raw `"A1:AZ"`
row count 2.3.3 anchored on was never actually small in his sheet, it
already agreed with Google's own confused auto-detection. Same bug,
different disguise.

Fixed properly: `next_append_row`/`next_append_range`
(`orders_sheet_sync.rs`) now scan for the LAST row whose **marker cell**
(TIQR ID) is non-empty - the one column only this app ever writes, and
only for a row holding a real order - and target the row right after it,
ignoring any stray formula residue further down. 5 unit tests added,
including the literal shape of marko's real sheet (16 real rows + 408
stray-formula rows still targets row 18) and a deliberate "never reuse a
gap in the middle" case. Full suite 1011/1011 passed, 0 failed; `tsc -b`/
`npm run build` clean.

Also found and documented (not fixed, not asked for): marko deleted the
row-426 order's content directly in the sheet while testing between
versions, which this app's own bookkeeping now can't see - that order
won't automatically come back. See `PROTECTED_AREAS.md`'s "2.3.2-2.3.4"
entry before doing anything about it.

Revenue/Profit formulas being missing on older rows is still believed to
be the same root cause, not independently fixed - marko needs to confirm
on his real sheet after this update. Not marked resolved yet.

## 2.3.3 - Sheets push: row placement fixed at the source

Follow-up to 2.3.2's investigation (see `PROTECTED_AREAS.md`'s "2.3.2/2.3.3"
entry for the full trail). Marko confirmed row 18 and rows 19-425 in his
real sheet are genuinely empty, and that retrying the push already once did
NOT bring back the missing Revenue/Profit formulas - which pointed at one
shared root cause rather than two separate bugs.

Fixed: `push_orders_impl` no longer hands row placement to Google's own
`append_values` table auto-detection (a bare `"A1"` anchor, which was
landing new rows at 426 instead of 18). It now computes the exact target
row itself, via a new pure, unit-tested `next_append_range` function, from
the same `"A1:AZ"` read this function already trusts for its header/
marker-column lookup, and writes with `update_values` instead. 3 new tests
added (`next_append_range_*`), all passing; full suite 1009/1009 passed, 0
failed. `tsc -b`/`npm run build` also clean (frontend untouched this
release).

Not independently touched, believed fixed as a side effect: the missing
Revenue/Profit formulas. `plan_sheet_structure_updates` already recomputes
formulas for the sheet's entire current extent on every push, so once new
rows land in the right place, the very next push should backfill formulas
correctly again. **Marko needs to click Push Orders/Push Sales once more
after this update and confirm** - not marked resolved until he does; see
`PROTECTED_AREAS.md` for exactly what to report back if it isn't.

Known, deliberate limitation: this does not move the order a past, buggy
push already stranded at row 426 in marko's real sheet - that needs a
manual fix in the sheet itself if he wants it back in the contiguous block,
since this app can't safely edit that live row unattended.

## 2.3.2 - Dashboard: all-time Total cost

Marko's request: a place on the Dashboard to see total cost across every
ticket he owns. Added a "Total cost" StatCard to the Financials tab's
existing "Current inventory (all time)" section, next to
Available/Listed/Sold (total)/Purchased (total) - zero backend change,
`data.inventory.totalCostCents`/`.currency` (a `FinanceSummary`) were
already computed and sent to the frontend every load, just never rendered
anywhere. Verified with `tsc -b`/`npm run build`/`cargo test --lib` (1006
passed, 0 failed, unaffected since no `.rs` file changed).

Also investigated (not yet fixed - see `PROTECTED_AREAS.md`'s "2.3.2"
entry) two Google Sheets sync complaints from the same message: Push
Orders/Sales landing a new row at 426 instead of at row 18 (the sheet's
real next empty row), and Revenue/Profit formulas missing on many rows.
Root-caused enough to have a credible fix shape for the first, but stopped
short of writing it - this touches marko's live, real-money Google Sheet,
and the fix's correctness depends on what's actually sitting in rows
19-425 of his real sheet, which cannot be verified from here. Asked marko
directly rather than guess. No Sheets-sync code was changed this release.

## 2.3.1 - Event Lifecycle removed (revert of 2.3.0)

Marko reviewed the 2.3.0 build below (delivered as a zip + Slovak report,
never actually published via `1-CLICK-UPDATE.bat`) and asked to remove it
entirely and go back to the previous version - no specific complaint beyond
not liking it in place ("mi tam nieje sympaticky"). `Events.tsx`/
`EventDetail.tsx` were reverted edit-for-edit to their exact pre-2.3.0
content (verified with `tsc -b`/`npm run build`/`cargo test --lib`, all
clean). The version was NOT rolled back to 2.2.12, even though the code
was - marko himself caught this right after asking for the revert: reusing
an old version number breaks the auto-updater for anyone already offered a
newer one ("ked dam stary tak to nefunguje potom dobre"). So this reverted
build ships as **2.3.1** instead (all 9 locations) - a version bump
forward that happens to contain strictly less than the 2.3.0 it follows.
**Lesson for future sessions:** a revert-to-previous-behavior task still
needs a version bump forward, never a rollback to a number already used
before - the updater compares versions directly. The entry right below is
kept as real history (this file is append-only), not deleted - see
`PROJECT_STATE/CURRENT_STATE.md`'s matching note for what to do if a similar
feature is requested again.

## 2.3.0 - Event Lifecycle / Event Operations (reverted - see entry above)

Marko's next task after 2.2.11/2.2.12: one consistent, derived "what stage
is this event at" lifecycle phase - no new manually-set status, no
migration, no backend change at all (`cargo test --lib` byte-for-byte
unchanged from 2.2.12). See `REDESIGN-2.3.0-REPORT.md` (Slovak) and
`PROJECT_STATE/PROTECTED_AREAS.md`'s "2.3.0" entry.

- **6 phases** - UPCOMING -> INVENTORY -> LISTED -> SELLING -> EVENT DAY ->
  COMPLETED (`computeEventLifecyclePhase`, `Events.tsx`) - a pure function
  of the already-returned `EventWithStats`, zero extra IPC calls. His
  proposed POST EVENT is folded into COMPLETED (his own literal COMPLETED
  rule leaves no gap to place it in); CANCELLED stays inside COMPLETED too
  (it already has its own Status badge). Both judgment calls explained in
  `PROTECTED_AREAS.md`.
- **Events overview**: lifecycle phase shown as a small pill stacked under
  the existing Status badge (no new column/colgroup change), plus a new
  "Lifecycle phase" filter dropdown, ANDed with the existing Upcoming/
  Completed tab.
- **Event Workspace (Overview tab)**: new `EventLifecycleBlock` at the top -
  current phase, a simple progress strip, an operational summary line
  (tickets/listed/sold/pending fulfillment), and a "Next Actions" list -
  sourced entirely from already-existing `list_sale_groups` (per-event
  `isSaleGroupDone`) and `get_attention_center` (global, filtered to this
  event) data, no new business logic.
- **Tests**: 25/25 on a disposable esbuild+Node script exercising marko's
  full scenario list (upcoming with/without inventory, listings, sales,
  event day, date passed, completed/cancelled, phase precedence,
  filter-by-phase, pending fulfillment, Next Actions aggregation) against
  the real exported functions. `cargo test --lib`: 1006 passed, 0 failed, 3
  ignored - unchanged, no `.rs` file touched. `tsc -b`/`npm run build`: 0
  errors.

## 2.2.12 - Fulfillment Center

Marko's ČASŤ C, shipped as its own release right after 2.2.11 (same
message, explicitly split into two releases). See
`REDESIGN-2.2.12-REPORT.md` (Slovak) and
`PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.12" entry. Zero backend/migration
changes - frontend only.

- **New page: Fulfillment Center** (`src/pages/FulfillmentCenter.tsx`, new
  `/fulfillment` sidebar entry right after Sales) - one place to see every
  sold ticket not yet fully paid, delivered, and completed. Fetches the
  same `SaleGroup[]` Sales.tsx already fetches and reuses its exact
  `isSaleGroupDone` rule (now exported) - no parallel status system, no new
  backend command.
- **4 clickable tiles double as KPIs and category filters**: Pending Sales
  (all), Awaiting Payment, Awaiting Delivery, Ready to Complete - the last
  one a new, pure display derivation (paid + delivered in full; the only
  way such a group is still Pending is a partial refund).
- **Table**: Event / Ticket+Seats / Sale price / Payment status / Delivery
  status (new group-level badge, existing tone colors) / Overall status /
  Action - row click or the Action button both open the existing Sale
  Detail page (`/sales/:id`), no new navigation mechanism.
- Verified with a disposable, esbuild-bundled Node script (built and run
  once, then deleted) asserting all of marko's listed test scenarios
  against the real exported functions - 21/21 passed - since this codebase
  has no frontend test framework.

## 2.2.11 - Attention Center UX rework + Dashboard cleanup

Marko's own next request, split into two explicit parts, both frontend-only
(zero backend/migration changes). See `REDESIGN-2.2.11-REPORT.md` (Slovak)
and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.11" entry for the judgment
calls behind each.

- **Attention Center reworked from one mixed feed into 5 named, always-
  visible boxes** (`Dashboard.tsx`): NO LISTING PRICE YET / NO ACTIVE
  LISTING / NOT DELIVERED YET / EVENT COMING SOON / MARKET ATTENTION -
  grouped by the item's existing `category` field instead of `priority`.
  Clicking a box reveals only that category's own rows (reusing
  `AttentionCenterRow` unchanged); the old mixed feed is gone as default
  content. A box with 0 items is disabled, not hidden. Zero backend
  changes - `attention_center.rs` already satisfied every MARKET ATTENTION
  constraint (Price-Checker-gated, no automatic pricing, section/row/tier
  never a pricing factor), confirmed by reading its own doc comment and
  tests rather than assumed. `AttentionSection`/the alert bell (older,
  separate feature) are untouched.
- **Dashboard Overview: unbounded "Sales by platform" list capped** with
  its own `max-h-72 overflow-y-auto`, so a long platform list scrolls
  internally instead of growing the whole page - paired with a small,
  one-step trim of two existing spacing values on the same tab (not a
  redesign). `Layout.tsx`'s scroll container was checked and needed no
  change.

## 2.2.10 - Eight follow-ups from marko's 2.2.9 review

Marko reviewed 2.2.9 and sent two rapid-fire messages (7 screenshots) with
eight mostly-unrelated requests. See `REDESIGN-2.2.10-REPORT.md` (Slovak)
and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.10" entry for the judgment
calls behind each. No migration this release.

- **Seats format: dropped the "Sec"/"Row"/"Seat" labels 2.2.9 had just
  added**, back to bare " · "-joined values (`formatSeatLocation`/
  `formatSeatsSummary`, `lib/format.ts`) - a real section value ("Sec 408",
  "Category D, Standing") sometimes already read as a full label, so the
  prefix produced "Sec Sec 408"-style duplication. Reaches every "Seats"
  column across the app via the same two shared helpers.
- **Orders tabs reworked: "Active"/"Paid" -> "Active"/"Completed"**, with a
  real bucketing change, not just a relabel - an order is now Completed
  once its event's date has passed (or its status is completed/cancelled)
  OR the order itself is fully sold+delivered+paid, whichever comes first.
  The New Order event picker now excludes those same "done" events too
  (previously unfiltered).
- **Attention Center "mixed" ordering fixed** - the real cause was the
  sort's own tie-break (grouping by category name before order), not the
  2.2.9 grouping-by-order logic itself. Also now excludes done events from
  3 of its 5 categories (missing listing price/no active listing/outside
  market price) - `sold_undelivered` and `event_soon` are deliberately
  exempt.
- **Sales Pending/Completed now requires sold+delivered+paid together**
  (or fully refunded) - a sale missing only its delivery status no longer
  incorrectly showed as Completed.
- **Two confirmed Google Sheets push bugs fixed** (Orders and Pulls push):
  local "already synced" bookkeeping was being written before the actual
  network write was even attempted, so a failed push still silently looked
  successful afterward. Both now record success only after the sheet write
  is confirmed.
- **Google's `invalid_grant` sign-in error now shows a short "sign in
  again" message** instead of a long raw JSON dump - best-effort fix for a
  reported long error after Google sign-in; not independently reproducible
  in this environment.
- **Native right-click context menu disabled everywhere** (no config flag
  exists for this in Tauri/WRY - the standard JS-side `preventDefault` fix).

Verified: `cargo test --lib` (1006 passed, +7 net new tests, 0 failed),
`tsc -b` and `vite build` both clean.

## 2.2.9 - Six follow-ups from marko's 2.2.8 review

Marko reviewed the just-shipped 2.2.8 Attention Center and sent six mostly-
unrelated small requests in one message. See `REDESIGN-2.2.9-REPORT.md`
(Slovak) and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.9" entry for the
judgment calls behind each.

- **Seatriks retired from Price Checker only** (`marketplaces.active = 0`,
  `migrations/025_deactivate_seatriks_price_checker.sql`) - same mechanism
  already used for StubHub. Stays fully available in Listings' "Add
  listing" picker.
- **Settings -> Integrations' Anthropic API key card relabeled** from
  "AI-assisted price reading" to the general "AI features" - same key,
  same storage, just a forward-looking name since it's meant to power more
  than one AI feature over time.
- **No live "balance" number was built** - Anthropic's API has no endpoint
  that returns a remaining credit balance for any key type (confirmed
  against Anthropic's own docs). A "Check usage & balance" link to the
  Anthropic Console was added on the same card instead of fabricating one.
- **Finance -> Overview gained "New entry"/"New account" buttons**, opening
  the exact same modals already used on the Transactions/Accounts tabs.
- **The per-event "Attention" list was removed from Event Workspace**
  (Inventory Intelligence's own 2.2.6 block) - fully superseded by the
  Dashboard's global Attention Center. The backend it was built on is
  untouched, since the Attention Center itself still depends on it.
- **Attention Center (2.2.8) reworked to group by order.** The four
  ticket-level categories now emit one row per order instead of one row
  per ticket - marko's own example was a 49-ticket order shown as 49 rows.
  Clicking a grouped row opens that order's own page, which already lists
  every affected ticket with its own status/price/delivery indicators.
  `event_soon` is unchanged (still one row per event).
- **Seats display reformatted: the "/" is gone.** `formatSeatsSummary`
  (Orders/Tickets/Inventory/Sales/Pulls' "Seats" columns) now shows
  clearly labeled, separated Section/Row/Seat text (e.g.
  "Sec 402 · Row 56 · Seat 27") instead of a bare slash-joined pair, and
  eight duplicate ad-hoc "/" joins across Sales.tsx and EventDetail.tsx
  were consolidated into the same shared formatter.

Verified: `cargo test --lib` (999 passed, +4 net new tests, 0 failed),
`tsc -b` and `vite build` both clean.

## 2.2.8 - Dashboard global "Attention Center"

Focused task on top of 2.2.6/2.2.7: a new compact Dashboard block (Activity
tab) listing individual things across EVERY event that currently need a
look, grouped by priority (Critical/Attention/Info) and sorted by priority
then soonest event. See `REDESIGN-2.2.8-REPORT.md` for the full report
(Slovak) and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.8" entry for the
judgment calls behind it.

- **New backend command**: `get_attention_center`
  (`commands/attention_center.rs`, new file) - no migration, no new
  dependency.
- **Four of five categories reuse 2.2.6's exact per-event Inventory
  Intelligence "Attention" rules** (event within 2 days with unsold
  tickets, unsold ticket with no listing price, unsold ticket with no
  active listing, unsold ticket priced 20%+ off market average - only with
  real Price Checker data), flattened into individual clickable rows.
- **New fifth category**: sold ticket whose `delivery_status` isn't
  literally `"Delivered"` yet - reuses the exact convention 2.0.66's
  "Completed" indicator already established; a refund excludes itself
  automatically (ticket reverts to `available`).
- **Navigation**: reuses `Tickets.tsx`'s existing `?code=` deep link for
  ticket-level rows, and `/events/:id` for the one event-level category
  (`event_soon`) - no new route/navigation mechanism.
- **Display**: reuses the Activity tab's existing `ShowMoreToggle`/
  `RECENT_LIST_PREVIEW_COUNT` pattern per priority group - the backend
  never truncates.
- Deliberately does NOT touch the existing Dashboard alert bell/"Attention"
  cards (pulls/pending sales/missing listing price by order/upcoming
  events) - a separate, additional block, not a replacement.
- No automatic pricing/repricing anywhere; `tier`/`section`/`row` are never
  used as a pricing factor.
- **+10 new Rust unit tests** (event-soon in/out of window, unsold ticket
  without active listing, unsold ticket without listing price, market
  alert only with real Price Checker data, sold-undelivered fires/excludes
  delivered/excludes refunded, sold-undelivered priority window, sold-out
  event still flags undelivered tickets, same ticket under 2 categories
  never twice under 1, priority+date sort order). Full suite: 995 passed /
  0 failed / 3 ignored.

## 2.2.7 - Ticket metadata: Tier / Level

Focused task on top of 2.2.6: every ticket can now optionally carry a
tier/level value (e.g. "VIP", "Lower Bowl", "Level 200"), kept strictly
separate from `ticket_type` (a delivery method, not a price tier - the
same mix-up flagged twice before, now resolved for good). See
`REDESIGN-2.2.7-REPORT.md` for the full report (Slovak) and
`PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.7" entry for the judgment calls
behind it.

- **New column**: `tickets.tier TEXT`, nullable (`migrations/
  024_ticket_tier.sql`, forward-only). Every existing ticket got NULL - no
  guessed/inferred values, per marko's own explicit instruction.
- **Entry points**: set at order creation (`OrderFormModal`, copied onto
  every generated ticket, same as section/row) and editable per-ticket
  afterward (`TicketEditModal`) - both small, plain text fields, no
  redesign.
- **CSV**: import accepts `tier` (or `level` as a synonym); fully backward
  compatible with CSVs that predate this column. Export (tickets, sales,
  and the downloadable order-import template) all include `tier`, right
  after `row`.
- **Inventory Intelligence** (2.2.6) gained a real "By tier" breakdown -
  blank/null shows as "Unknown"; clicking a tier group filters the Tickets
  table exactly like the section/marketplace breakdowns already do.
- **Deliberately not wired up this round** (prepare-the-data, not
  wire-it-in-yet): Market Analysis / Repricing's `YourTicketGroup.tier`
  still always reports `None`; no bulk-tier-edit action; no tier column
  added to any list/table view; Google Sheets Order sync not wired to
  `tier`. Zero changes to refund/resell, `batch_id`, money/cents logic,
  Orders/Sales/Finance core logic, Listings, or Price Checker scraping.
- Tests: +13 new Rust unit tests (migration upgrade/fresh-db, ticket
  create/update with tier, CSV import old/new format + the `level`
  synonym, CSV export tier presence for tickets/sales/template, Inventory
  Intelligence tier grouping). Full suite: 985 passed / 0 failed / 3
  ignored. `tsc -b` and `npm run build` both clean.

## 2.2.6 - Inventory Intelligence for Event Workspace

Focused task on top of 2.2.5: a compact "Inventory Intelligence" block
added to the Event Workspace's Overview tab, above the existing Orders/
Tickets tables. See `REDESIGN-2.2.6-REPORT.md` for the full report
(Slovak) and `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.6" entry for the
judgment calls behind it.

- **KPIs**: Total tickets, Total invested, Current listed value (active
  `ticket_listings` only), Potential profit (legacy `listing_price_cents`
  field, matching Sales' existing card), Sell-through %, Average ticket
  cost - all reusing existing money definitions, no new duplicate
  computations.
- **Aging**: 0-7 / 8-30 / 31-60 / 61+ days since purchase, unsold tickets
  only.
- **Attention**: event within 2 days with unsold stock, unsold ticket with
  no listing price, unsold ticket with no active listing, unsold ticket
  priced 20%+ off the market average (reuses Price Checker's own summary;
  shown as "not available yet" rather than a fake zero when this event has
  no Price Checker data).
- **Breakdown** by section and by marketplace. No "by tier" breakdown -
  `tickets` has no tier/level column anywhere in this schema; the UI says
  so in plain text instead of inventing fallback data, per marko's own
  explicit instruction.
- **Every row is clickable** - filters Overview's own Tickets table to the
  relevant tickets (or switches to the Listings tab, for "Current listed
  value"). No changes to Tickets.tsx, Orders.tsx, or any core Orders/
  Tickets/Sales/refund/resell logic; Finance page untouched.
- New backend: `commands/inventory_intelligence.rs` (1 new command,
  `get_inventory_intelligence`), no migration, no new dependency.
- Tests: +13 new Rust unit tests (KPI scope/formula parity with existing
  screens, aging bucket boundaries, attention-item independence and
  availability, currency-mixed handling, section/marketplace grouping).
  Full suite: 972 passed / 0 failed / 3 ignored. `tsc -b` and
  `npm run build` both clean.

## 2.2.5 - Event Workspace down to 3 tabs; Listings gets filters, search and bulk actions

Fourth pass on the Event Workspace, plus a Price Checker lookup addition.
Final tab order: `Overview | Listings | Sales`.

- **Sales absorbed Finance** - "sales a finance daj dokopy" (unambiguous
  this round). Finance's entries table now renders below Sales' own table
  (and below the Market section 2.2.4 already put there). See
  `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.5" entry for the judgment call
  behind Sales (not Finance) surviving as the name.
- **Listings: filters, search, multi-select, bulk actions.** Status filter
  (All/Active/Sold/Removed), marketplace filter, search box, always-visible
  row checkboxes with select-all/deselect-all (scoped to the currently
  filtered/searched rows), and a bulk action bar (shown only while
  something is selected) for Edit status / Edit price / Delete - each
  backed by a new **all-or-nothing** transactional Rust command
  (`bulk_update_ticket_listings_status`, `bulk_update_ticket_listings_price`,
  `bulk_delete_ticket_listings`, all in `ticket_listings.rs`). Bulk price
  edit is refused, on both the frontend and the backend, when the selection
  spans more than one currency.
- **"Add listing" ticket picker rebuilt** as an order-browse flow (search
  this event's own orders, open one, pick tickets from it) mirroring
  Sales.tsx's own New Sale flow, replacing the old flat "every ticket in
  the event in one dropdown" picker. Several tickets can be picked at once,
  creating one listing per ticket on the chosen marketplace (per-ticket
  price, with a quick-fill/apply-to-all helper); Listing ID/URL are offered
  only when exactly one ticket is selected. This create flow is NOT
  all-or-nothing (unlike the 3 bulk actions above) - a partial failure
  keeps whatever succeeded and reports the rest for retry.
- **Marketplaces: added Seatriks** - new pure-data migration
  `023_add_seatriks_marketplace.sql`, no schema change.
- Existing tickets/inventory/sales/refund logic untouched; no automatic
  listing creation, marketplace API, or repricing added.
- Tests: +11 new Rust unit tests for the 3 bulk commands (selection
  scoping, invalid input, mixed currency, dedup, all-or-nothing transaction
  safety), plus 1 existing Price Checker test updated for the new 4th
  active marketplace.

## 2.2.4 - Event Workspace down to 4 tabs; Listings is now a real multi-marketplace system

Third pass on the Event Workspace. Final tab order: `Overview | Listings |
Sales | Finance`.

- **Overview absorbed Inventory** - the Orders/Tickets tables now render
  below Overview's own stat cards instead of having their own tab.
- **Sales absorbed Market** - "Market vs. mine" and "Potential Profit" (the
  former Market tab's content) now render below the Sales table. See
  `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.4" entry for the judgment call
  behind Market landing in Sales rather than Finance.
- **Finance is unchanged**, still its own tab.
- **Listings rebuilt into a real system.** New `ticket_listings` table
  (`migrations/022_ticket_listings.sql`) - one ticket can now have several
  listings at once, one per marketplace (reuses the existing `marketplaces`
  lookup table), each with its own price/currency/status/listing id/URL/
  last-updated timestamp. Full add/edit/delete UI in the tab; summary cards
  count active listings only, the table shows every listing regardless of
  status. Manual entry only - no marketplace API, no automatic listing
  creation, no repricing. Never touches `tickets.status`/
  `tickets.listingPriceCents`.
- New backend: `commands/ticket_listings.rs` (4 commands: list-for-event,
  create, update, delete) + `commands::price_checker::
  delete_marketplace_impl`'s existing guard extended to also count
  `ticket_listings` (so deleting a marketplace with real listings against
  it is refused, same as it already was for saved links/price-check
  history).

See `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.4" entry before extending any
of these tabs or the new table further.

`cargo test --lib` (948 passed, up from 934 - 14 new tests: 13 for
`ticket_listings`, 1 for the `delete_marketplace_impl` guard extension),
`tsc -b`/`vite build` clean. One new migration (022); no changes to
existing tickets/orders/sales/refund logic. Full detail in
`REDESIGN-2.2.4-REPORT.md`.

## 2.2.3 - Event Workspace: Listings tab, Tasks removed, tables full-width

Second pass on the Event Workspace, all frontend-only. Final tab order:
`Overview | Inventory | Listings | Sales | Market | Finance`.

- **Tasks tab removed entirely** - marko decided against it before it had
  a spec; it was only ever an empty placeholder, so there was nothing to
  migrate.
- **New Listings tab** - a read-only view of this event's tickets already
  filtered to `status === "listed"`: ticket, listing price, currency,
  status, plus an Active listings/Listed value/Lowest/Highest summary.
  Deliberately does NOT show marketplace, listing URL, or last checked -
  none of the three exist anywhere in the `tickets` schema (checked all 21
  migrations) or in Price Checker's own listing data, and marko explicitly
  asked not to invent data that isn't real. The tab says so plainly.
  Reuses the same `tickets` array Inventory already loads - no new API.
- **All 4 Event Workspace tables (Orders, Tickets, Sales, Finance) now
  fill the window width** - removed the `max-w-[1400px]` cap that was
  stopping them short of the right edge, the same fix `Layout.tsx` itself
  got in 2.0.31.

See `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.3" entry before extending
any of these tabs further.

Frontend-only - no migration, no backend command changes. `tsc -b`/
`vite build` clean (cargo test suite unaffected - no `.rs` files touched
this release). Full detail in `REDESIGN-2.2.3-REPORT.md`.

## 2.2.2 - Event Workspace, plus 3 small fixes

`EventDetail.tsx` is now a tabbed "Event Workspace"
(`Overview | Inventory | Sales | Market | Finance | Tasks`, via the same
`TabSwitcher` Tickets.tsx/Events.tsx already use):

- **Overview** - exactly marko's own list (tickets, sold, available,
  total cost, revenue, profit, margin, ROI), no backend change.
- **Inventory** - the existing Orders + Tickets tables, unchanged, just
  relocated under their own tab.
- **Sales** - `list_sale_groups({ eventId })` (Sales.tsx's own Event
  filter, reused), compact table, "Open in Sales" for more.
- **Market** - `get_price_checker_summary(eventId)` (PriceChecker.tsx's
  summary command, reused) plus the existing "Potential Profit" block,
  now together in one tab.
- **Finance** - `list_finance_entries_for_order` (2.2.1) called once per
  this event's own orders, merged client-side - no new backend command.
- **Tasks** - honest placeholder (`EmptyState`), no spec given yet.

See `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.2" entry before extending
any of these tabs.

Plus three unrelated small fixes: Settings -> Lookups' 3 category lists
no longer cap their scroll area at a fixed 224px (`max-h-[60vh]` now);
Event Detail's last table (Tickets) was missing the `mb-8` its Orders
neighbor had, so the Potential Profit box after it read as crammed
against it; Price Checker's event picker now only lists
`status === "upcoming"` events, so a completed/cancelled event quietly
stops showing up there.

Frontend-only - no migration, no backend command changes. `tsc -b`/
`vite build` clean (cargo test suite unaffected - no `.rs` files touched
this release). Full detail in `REDESIGN-2.2.2-REPORT.md`.

## 2.2.1 - Finance Accounts redesign, Settings Lookups redesign, Price Checker jump links, Finance-Orders linking

Four independent, marko-requested pieces in one release:

- **Finance Accounts** (`src/pages/finance/Accounts.tsx`): the old
  `sm:grid-cols-2 lg:grid-cols-3` grid of large `AccountCard`s replaced
  with one compact divide-y list (`AccountRow`) - same dense-row visual
  language as PlatformList/EventCategoryList. Balance stays the most
  prominent number per row; opening balance moved to a hover tooltip.
- **Settings -> Lookups** (`Settings.tsx`): was one long always-expanded
  Card (Platforms/Event categories/Finance categories); now exactly 3
  clickable summary rows (same row/chevron style as Settings Home's own
  section list), each opening its list(s) in a Modal. The add/delete
  functionality itself is unchanged - only the container is new.
- **Price Checker jump links**: "Check prices" button added to
  `OrderDetail.tsx` and `SaleDetail.tsx` (hidden on a sale group spanning
  mixed events), reusing the exact `navigate("/price-checker", { state: {
  presetEventId } })` pattern `EventDetail.tsx` already used since 2.0.81;
  `PriceChecker.tsx` needed no changes.
- **Finance <-> Orders linking**: `finance_entries.order_id` (new
  `migrations/021_finance_entry_order_link.sql`, `ON DELETE SET NULL`) -
  a deliberate, marko-confirmed (via question) reversal of ONE part of
  `015_finance.sql`'s original "fully independent ledger" design. New
  "Record in Finance" button/modal on `OrderDetail.tsx` pre-fills a new
  expense entry from the order's own amount/currency/date, with
  amount/currency locked read-only so the two numbers can never drift
  apart. New `list_finance_entries_for_order` command. See
  `PROJECT_STATE/PROTECTED_AREAS.md`'s "2.2.1" entry before touching
  `finance_entries.rs` again - in particular the "must round-trip
  `orderId` unchanged on edit" trap, already fixed proactively in
  `Transactions.tsx` and `Overview.tsx`.

6 new Rust tests. 934 passed / 0 failed / 3 ignored (up from 928), clippy
clean, `tsc -b`/`vite build` clean. Full detail, including the
AskUserQuestion decision on the Finance-Orders link design, in
`REDESIGN-2.2.1-REPORT.md`.

## 2.2.0 - Price Checker Market Analysis

New `commands/price_checker_analysis.rs` (2 commands) derives tier/section
price breakdowns, comparable-ticket ranking, and Your Tickets price
recommendations from a Visible Scanner session's already-accumulated
listings - never touches the scanner's own session/lifecycle code.
`migrations/019_price_checker_market_analysis.sql` adds `price_check_tiers`
so saved checks remember a per-tier breakdown going forward. 40 new Rust
unit/integration tests (incl. 2 added during this release's own adversarial
review pass, after finding tier/section grouping was case-sensitive while
comparable-matching already wasn't - see `PROJECT_STATE/PROTECTED_AREAS.md`'s
"2.2.0" entry). Full detail, including every flagged design decision, in
`PRICE-CHECKER-MARKET-ANALYSIS-2.2-REPORT.md`.

## 2.2.0 - StubHub fully removed, including history

`migrations/020_remove_stubhub.sql` deletes the StubHub marketplace row and
every `price_checks`/`price_check_tiers`/`event_marketplace_links` row that
ever referenced it - marko's own explicit, confirmed decision to go further
than 2.1.6's "keep history, stop offering it for new checks." Irreversible
by design.

## 2.1.9 - PROJECT_STATE protocol adopted

Set up `PROJECT_STATE/CURRENT_STATE.md`, `PROJECT_STATE/KNOWN_BUGS.md`,
and `PROJECT_STATE/PROTECTED_AREAS.md` (moved verbatim from the old root
`PROTECTED-AREAS-NOTES.md`, which is now a pointer stub) per marko's
development protocol. No code changes. `KNOWN_BUGS.md` starts empty by
design - see its own header for why.

## 2.1.9 - Price Checker Visible Scanner

Replaced the hidden auto-check WebView with a visible one the user scans
himself. Full detail in `PRICE-CHECKER-VISIBLE-SCANNER-REPORT.md` and
`PROJECT_STATE/PROTECTED_AREAS.md`'s "2.1.9" entry.

## 2.0.80 - Google Sheets Summary block: Paid-gated Revenue/Profit + refund staleness fix

- `plan_orders_summary_updates` (`orders_sheet_sync.rs`): "Total
  Revenue"/"Total Profit" now use the same Paid-gated `SUMPRODUCT` as
  "Total Paid", instead of summing every sold row regardless of payment
  status. Confirmed with marko via question before implementing.
- New `order_fully_refunded` check + clearing branch in
  `apply_sales_push_internal`: once every ticket on an order has been
  refunded, "Push sales"/"Fix sync" now blank that row's 7 Sales-sync
  columns instead of leaving stale pre-refund data forever (previously not
  even "Fix sync" could correct it - `uniform_sale_for_order` returns
  `None` for a refunded order, so nothing detected the drift).
- 9 new Rust tests. 747 passed / 0 failed / 3 ignored.

## 2.0.79 - Dashboard cleanup + CSV export staleness fixes

- Removed the Dashboard Overview tab's Quick Actions button row
  (New Event/Order/Sale, Import/Export CSV) - redundant with each page's
  own button and Settings -> Data.
- Orders/Tickets/Inventory/Sales CSV exports had drifted behind the data
  model over many versions; added the missing columns (event category;
  resale/delivery status; order code, seat location, margin, ROI,
  resale/delivery status, refund details).
- Dashboard Activity's "Unpaid payments" tile replaced with "Pulls near
  deadline" (pulls not yet transferred, event date approaching/past) -
  reuses Pulls.tsx's own existing warning window rather than a new rule.
  `unpaid_orders_count` itself is untouched (still used by notifications).
- 738 passed / 0 failed / 3 ignored.

## 2.0.78 - Pushover -> ntfy

- Swapped the Pushover notification channel for ntfy (no built-in app
  token needed).

## 2.0.77 - Notification simplification

- Removed the email notification channel entirely (SMTP config, `lettre`
  dependency) at marko's request.
- Pushover simplified to user-key-only; app token is now built into the
  binary via a GitHub Actions secret, same pattern as other embedded keys.
- 732 passed / 0 failed / 3 ignored (29 in the notifications module).

## 2.0.76 - Outbound notifications: desktop, email, Pushover

- New `commands/notifications.rs`: desktop (tauri-plugin-notification),
  email (SMTP via `lettre`), Pushover channels. Settings -> Notifications
  with a "Send test" button per channel.
- Background check every 30 min (+ once on launch) against the same 4
  Dashboard "Attention" categories the alert bell (2.0.75) already shows;
  max once per category per calendar day; upcoming-events only pushes
  within a 3-day window (vs. the bell's 14-day display window).
- Secrets stored plain-text in `app_settings`, same existing trust
  boundary as the rest of the app; never echoed back to the UI.
- Only fires while the app process is running - no tray/background
  service (documented limitation, not a bug).
- 729 passed / 0 failed / 3 ignored (26 new).

## 2.0.75 - Dashboard alert bell

- New `AlertBell` on the Dashboard (top-right, next to the tab switcher):
  badge counts how many of the same 4 "Attention" categories are non-zero;
  amber, red only when the soonest upcoming event is due today/overdue.
  Reuses the exact same numbers `DashboardAlerts` already computes - no
  new backend logic. Frontend-only change.
- 703 passed / 0 failed / 3 ignored (unchanged - no Rust touched).

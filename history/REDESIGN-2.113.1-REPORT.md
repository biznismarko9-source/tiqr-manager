# TIQR Manager 2.113.1 — opravy po 2.113.0 a podstatné z mailu

Štyri veci nahlásené hneď po 2.113.0:

- *„hneď prvá chyba, finance sú rozbité"*
- *„pulls daj pod sales"*
- *„keď prídu webhooks, tak tam nie je tiqr logo"*
- *„v inventory je random nejaká biela čiara"*

A dve nové:

- *„pri mailoch ... napr. please send your tickets a tam je viac info ako
  napríklad poslať tam, meno a mail a to daj ako hidden a keď na to klikneš,
  tak sa to zobrazí, a aj cena, čiže nie len prvý text, ale aj reálne obsah,
  a vybrať z toho najpodstatnejšie"*
- *„keď je niečo také v maili, že download receipt, tak reálne keď na to
  kliknem, tak sa mi stiahne"*, *„aj takýto link"*, *„alebo také linky, kde
  sa dá kliknúť"*

---

## 1. Finance → Transactions

**Čo bolo vidieť:** stĺpce Category a Amount boli zúžené na jedno písmeno na
riadok („B iz ni s n á kl a d y", „- € 2 5 7 . 1 0"), hlavička „Cate gory" na
dva riadky, a tabuľka sa dala posúvať do strany.

**Prečo:** od 2.102.0 platí pre každú bunku tabuľky `overflow-wrap:
anywhere` - text smie zalomiť kdekoľvek, aby dlhý kód nikdy nepretiekol cez
susedný stĺpec. Komentár pri tom pravidle predpokladal, že *každá* tabuľka
má pevné šírky stĺpcov (`table-fixed`). Tabuľky vo Finance ich nemali - šírky
si rátali z obsahu - a `anywhere` prehliadaču povie, že slovo môže byť široké
jedno písmeno. Stĺpce, ktoré sa zalamovať nesmú (dátum, miesto), si zobrali
celé miesto a ostatné sa zúžili na písmeno. Tlačidlo Send to Discord z 2.113.0
pridalo do každého riadku ďalšiu ikonu, a tým to ešte zhoršilo.

**Oprava:**

- **Transactions** a **opakované výdavky v Accounts** majú pevné šírky
  stĺpcov ako každý iný zoznam v appke. Miesto/kto (pri opakovaných výdavkoch
  názov) dostane, čo ostane, a dlhý text skráti „…" - celý je vidieť po
  prejdení myšou. Typ a jeho „Business/Personal" sú pod sebou, aby sa zmestili.
- **Pravidlo pre bunky** (`index.css`): `anywhere` už len v tabuľkách s
  pevnými šírkami. V tabuľkách, ktoré si šírky rátajú z obsahu (Dashboard,
  Event detail, formuláre na pridávanie riadkov, Recap, jedna v Settings), sa
  slovo zalomí len vtedy, keď by naozaj pretieklo (`break-word`), a stĺpec sa
  nikdy nezúži pod svoje najdlhšie slovo.

Overené v prehliadači pri šírke obsahu 1000 a 1440 bodov, tmavý aj svetlý
režim: žiadny text písmeno po písmene, žiadne posúvanie do strany. Formulár
na nové objednávky má so starým aj novým pravidlom tie isté šírky stĺpcov.

## 2. Pulls pod Sales

Bočný panel: Dashboard, Events, Inventory, Sales, **Pulls** — Finance — Mail,
Notes. Pulls už nie je úplne dole, kde bol od 2.98.0.

## 3. Logo TIQR v Discorde

**Prečo chýbalo:** správy brali logo z GitHub Pages
(`biznismarko9-source.github.io/tiqr-manager/tiqr-logo.png`). GitHub Pages
však pre tento repozitár podľa všetkého **nie sú zapnuté** - v repozitári nie
je ich workflow (`pages-build-deployment`), ktorý GitHub pri zapnutých Pages
pridá, a Discord obrázok z tej adresy nenačítal.
Každá adresa `github.io` preto vracia 404 a Discord ukázal rozbitý obrázok.
Náhľad v Settings vyzeral dobre, lebo berie logo zabalené priamo v appke.

**Oprava:** logo sa berie priamo zo súboru v repozitári,
`raw.githubusercontent.com/.../main/docs/tiqr-logo.png`. Overené: vráti PNG
256 × 256. Potrebuje len ten súbor na `main` (je tam od 2.113.0) a verejný
repozitár, na ktorom aj tak stojí aktualizácia appky.

Staré správy v Discorde ostanú s rozbitým obrázkom (Discord ich znova
nenačíta); nové budú mať logo ako avatar aj v správe.

**Súvisí, nie je opravené:** na tých istých GitHub Pages stoja ešte dve
stránky - zásady súkromia (`privacy.html`, odkaz na prihlasovacej obrazovke
Googlu) a presmerovanie pri zabudnutom hesle (`reset-redirect.html`). Kým
Pages nie sú zapnuté, obe vracajú 404, takže obnovenie hesla cez mail sa
zasekne na tej stránke. Zapína sa to v GitHube jedným nastavením:
**Settings → Pages → Build and deployment → Deploy from a branch → `main` /
`/docs` → Save.** Odvtedy budú fungovať obe stránky (a aj stará adresa loga).

## 4. Biela čiara v Inventory

**Prečo:** medzi dvoma riadkami boli **dve** čiary - okraj z Tailwindu
(`divide-y`) a vlastná čiara riadka (`box-shadow`). Riadky stáli na
polovičných pixeloch (text 11 px s výškou riadka 16,5 px), takže sa tie dve
čiary na niektorých miestach stretli na tom istom pixeli a inde boli vedľa
seba - jedna čiara v zozname tak vyšla oveľa svetlejšia ako ostatné.
Presne sa to v prehliadači zopakovať nedalo (závisí od toho, ako konkrétna
obrazovka zaokrúhľuje), príčina sa však dala zmerať.

**Oprava** (`index.css`, platí pre všetky zoznamy): medzi riadkami je už len
jedna čiara (riadka), a bunky s textom 11 px majú celú výšku riadka (16 px),
takže každý riadok má celé pixely (zmerané: 32,00 px namiesto 32,5/33).

## 5. Kľúčové údaje z mailu, osobné údaje skryté

**Čo to robí:** z celého textu mailu (nie z 200-znakového náhľadu Gmailu)
vyberie najviac osem riadkov, ktoré sú podstatné, v tomto poradí:

1. kam poslať lístky;
2. meno, e-mail, telefón, adresa;
3. dokedy;
4. suma (cena, výplata, spolu);
5. event, dátum, miesto;
6. počet lístkov, sedadlá;
7. číslo objednávky.

Každý riadok má menovku, ako ju napísal mail.

**Ako, bez AI a bez hádania** (`src-tauri/src/commands/mail_facts.rs`):

- Hodnota sa berie len vedľa menovky, ktorú mail sám napísal:
  - „Buyer's name: Anna", aj s hodnotou na ďalšom riadku;
  - riadok tabuľky bez dvojbodky („Total Proceeds €162.50"), ak sú prvé
    slová známa menovka;
  - pri „kam poslať" aj veta, ktorá priamo žiada poslať/preniesť lístky na
    adresu.
- Menovky po anglicky, slovensky, česky, nemecky a španielsky; s diakritikou
  aj bez.
- Čo sa nikdy neberie:
  - citovaný starší mail (`>`, „On … wrote:");
  - čas („19:00") a webová adresa ako menovka;
  - nulová suma.
- Adresy, ktoré nikdy nie sú „kam poslať":
  - odosielateľ;
  - tvoja schránka;
  - adresy obchodu typu `help@`, `support@`, `noreply@`.
- Pri Spame a Koši sa nič nevyberá.

**V TIQR:** karta *Key details* hore v otvorenom maili:

- Osobné údaje (meno, e-mail, telefón, adresa, komu poslať) sú bodky s
  *Show*.
- Klik ich ukáže, druhý klik skryje, *Show personal details* ukáže všetky.
- Každý riadok sa dá skopírovať, aj skrytý (bez odkrytia).

**V Discorde** (`discord.rs`):

- Pole *📌 Key details*; osobné údaje sú v ňom ako spoiler `||…||`.
- Rovnaké údaje sú spoilerom aj v náhľade textu a v nadpise sú nahradené
  „•••".
- Spoiler sa nikdy nepreseká v polovici: pole sa skladá len z celých
  riadkov. Náhľad, ktorý by prerástol limit Discordu, ukáže namiesto
  spoilerov bodky.
- Čo už hovoria polia Amount/Tickets/Reference, sa neopakuje.
- Jednorazový kód je skrytý aj tu.

**Kedy má Discord celý obsah:**

- Pri príchode sa telo mailu načítalo len pri nákupe, predaji, výplate a
  refunde. Odteraz aj pri aktualizácii objednávky, zrušení a zmene eventu,
  teda pri všetkých mailoch, o ktorých Discord píše.
- Stále len z obchodov, ktoré TIQR pozná, a z posledných 14 dní.
- Ostatné majú celý obsah, keď ich otvoríš (potom aj *Send to Discord*).

**Hranica:** v samotnom maili nižšie sú údaje, ako ich mail napísal; skrytá je
karta a Discord. Spoiler v Discorde vidí každý v kanáli, kto naň klikne.

## 6. Odkazy, ktoré sú súbor, sa stiahnu

**Čo to robí:**

- Klik na odkaz, ktorý je súbor, stiahne súbor priamo do TIQR. Taký odkaz
  je napríklad:
  - „Download receipt", „Download invoice";
  - „Stiahnuť faktúru", „Rechnung herunterladen";
  - „Receipt";
  - adresa končiaca na `.pdf`, `/pdf`, `/download`.
- Súbor sa uloží k mailu medzi prílohy s poznámkou *from a link*.
- Obrázok, PDF alebo text sa hneď otvorí v náhľade; *Open* a *Save* fungujú
  ako pri prílohe.
- Pri prejdení myšou má taký odkaz *Download* (do TIQR) aj *Open* (do
  prehliadača).

**Bezpečnosť** (`mail_view.rs::download_mail_link`), rovnaké pravidlá ako pri
obrázkoch z webu:

- Sťahuje Rust, nikdy obrazovka.
- Len verejný internet: nikdy tento počítač ani domáca sieť, každé
  presmerovanie skontrolované (najviac 8), porty 80/443.
- Spojenie ide na adresu, ktorá bola skontrolovaná.
- Najviac 25 MB, 30 sekúnd.
- Druh súboru sa určuje podľa obsahu, nie podľa mena (`download.php`, ktorý
  je PDF, sa uloží ako `.pdf`).
- Program TIQR neotvorí a pri otvorení súbor dostane značku „z internetu".
- Mail v Spame alebo Koši nesťahuje nič; také odkazy sa po otázke otvoria v
  prehliadači.
- Adresa odkazu sa nikde nezapisuje do logov.

**Keď to nie je súbor:** ak adresa vráti webovú stránku (prihlásenie, stránka
faktúry), odkaz sa otvorí v prehliadači. Rovnako sa otvorí, keď sa súbor
stiahnuť nedá. Klik teda vždy niekam vedie a omylom označený odkaz nič
nepokazí.

**Ostatné odkazy:** #1160703 v Zendesku, Concerts/Festivals/Sports/Theatre a
help centre vo viagogo sa otvárajú v prehliadači, ako doteraz. V prehliadači
to bolo overené kliknutím priamo v správe.

## 7. Drobnosti

- Text z HTML mailov číta písmená zapísané ako `&Ntilde;`, `&uuml;`,
  `&Scaron;`, `&szlig;` (predtým ostali ako kód). Podľa toho textu sa čítajú
  kľúčové údaje aj čísla pre Automatic.
- Po kliknutí na odkaz v maili ostávala klávesnica v rámčeku správy, takže
  Escape nezatvoril náhľad. Opravené.

---

## Testy

- `cargo test --locked`: TESTCOUNT
- `npm test`: 34 prešlo.
- `npm run build` prešiel.
- Nové testy:
  - kľúčové údaje: mail „send your tickets", hodnota na ďalšom riadku,
    tabuľka bez dvojbodky, HTML mail, veta „transfer the tickets to …",
    vlastná schránka a odosielateľ, SK/DE/ES, citácie, limit 8;
  - Discord: kľúčové údaje so spoilermi, nič osobné mimo spoilera, spoiler
    nikdy rozseknutý;
  - stiahnutie z odkazu cez skutočný malý server: PDF s menom od servera,
    presmerovanie, stránka, maskovaná stránka, súbor nad 25 MB, chyba 410;
  - zákaz lokálnych adries, cudzích portov a ne-webových adries, mená
    súborov, rozpoznanie odkazu-súboru v 13 prípadoch, písmená s diakritikou.
- Obrazovky v prehliadači s napodobeným backendom:
  - Transactions pri 1000 a 1440 bodoch (tmavý aj svetlý režim);
  - Inventory pri Retina mierke;
  - formulár New order;
  - čítačka mailov: kľúčové údaje skryté, jeden odkrytý, všetky odkryté;
    odkaz s *Download*; sťahovanie; náhľad stiahnutého súboru; príloha
    *from a link*. V prehliadači overené aj to, že faktúra-stránka sa
    otvorí v prehliadači a #1160703 aj Concerts sa otvoria priamo.

## Súbory

- `src/index.css`
- `src/pages/finance/Transactions.tsx`, `src/pages/finance/Accounts.tsx`
- `src/components/Layout.tsx`
- `src/components/MailReader.tsx`, `src/components/icons.tsx`
- `src/lib/mailText.ts` (+ test), `src/lib/api.ts`, `src/lib/types.ts`
- `src-tauri/src/commands/mail_facts.rs` (nový), `mail_view.rs`,
  `discord.rs`, `mail_sync.rs`, `mail.rs`, `mod.rs`, `lib.rs`
- `CHANGELOG.md`, `PROJECT_STATE/*`, verzia v piatich súboroch.

Žiadna migrácia (ďalšia je 047), žiadna zmena dát.

# TIQR Manager 2.109.0 — Mail: vlastné pravidlá, nastavenia v maily, starred naozaj vždy

Štyri veci, ktoré si pýtal, plus jedna skutočná chyba, ktorú som pri tom našiel.

---

## 1. Nastavenia sú teraz v Maily, dole pod zložkami

> *„tie nastavenia ktore si dal do integrations ku mailu tak odstran a presun ich
> do tych settings v maily samotnom v zlozke"*
> *„tu dole dat nastavenia a nejake mudre veci tam dat"*

V **Mail** máš úplne dole v zozname zložiek riadok **Settings**. Drží sa dole aj
keď máš veľa záložiek — neodroluje preč.

Je tam **všetko**, čo bolo predtým v Settings → Integrations → Email, plus to nové:

| Sekcia | Čo tam je |
|---|---|
| **Mailbox** | pripojiť / odpojiť, Test connection, pauza, koľko ešte ostáva stiahnuť |
| **Rules** | tvoje pravidlá (nižšie) |
| **Senders TIQR watches** | tých 17 domén — teraz upraviteľných — a výber celá schránka / len marketplace |
| **Your own Gmail filter** | vlastný Gmail filter |
| **What TIQR may create by itself** | orders / sales / finance: off, manuálne, automaticky |
| **Fetching and keeping** | ako často kontrolovať, ako dlho držať maily |

V **Settings → Integrations → Email** ostal jediný riadok s odkazom sem.
Zámerne — keby tam nebolo nič, vyzeralo by to, že funkcia zmizla, a hľadal by si
ju práve tam.

---

## 2. Pravidlá — „čo a kedy"

> *„ze napriklad ktoru hlavicku rovno ma riesit alebo ktoru rovno dat do
> presunutych alebo co a kedy do spamu atd, pohraj sa s tym"*

Jedno pravidlo = **jedno pole**, **jeden text**, **jedna akcia**.

```
Odosielateľ   obsahuje   "no-reply"        →  Do koša
Predmet       obsahuje   "order confirmation" →  Keep — zastaviť
Náhľad        začína na  "Your payout"     →  Needs review
```

**Štyri akcie, a každá povie pravdu o tom, čo robí:**

| Akcia | Čo sa naozaj stane |
|---|---|
| **Move to trash** | Do koša **TIQR-u**. Dá sa vrátiť. V Gmaily sa nestane nič. |
| **Mark as read** | Ostane kde je, len prestane počítať ako neprečítané. |
| **Send to Needs review** | Vyskočí v Needs review, aby si to videl prvé. |
| **Keep — stop here** | Nezapíše nič. Zastaví zoznam, takže **žiadne pravidlo pod ním už nesmie zasiahnuť**. |

### Prečo je `Keep` najdôležitejšie

Pravidlá idú **zhora dole a prvá zhoda vyhráva**. To znamená, že si môžeš
dovoliť napísať široké pravidlo („všetko z no-reply do koša"), lebo nad neho
dáš úzku výnimku („order confirmation — Keep"). Bez toho by si musel dopredu
vymyslieť každú výnimku a stavil by si na to celú schránku.

Poradie meníš šípkami ▲▼ pri pravidle. **Poradie je význam pravidla, nie vzhľad.**

### Ďalšie veci okolo

- Pri každom pravidle vidíš, **koľkokrát už zabralo**. Pravidlo s nulou po mesiaci
  je buď zlé alebo zbytočné — a oboje je dobré vidieť.
- **„Run rules on mail I already have"** ich pustí na to, čo už máš uložené,
  a povie ti číslo. Inak by platili len na budúce maily a nemal by si ako zistiť,
  či robia to, čo si myslel.
- Pravidlá bežia **len na nové maily** pri syncu. Keď niečo vrátiš z koša ručne,
  ďalší sync ti to tam nehodí späť.
- **Štyri hotové predlohy** pod formulárom (Protect order confirmations, Payouts
  → Needs review, Silence newsletters, Trash no-reply). Kliknutím sa predvyplní
  formulár a ďalej to upravuješ normálne.

### Čo pravidlá NEMÔŽU

Appka má od Googlu **len na čítanie**. Pravidlo **nesiahne do Gmailu** — nič
tam nezmaže, neoznačí ani nepresunie. A **nič nikdy nemaže natrvalo**, ani
lokálne: najhoršie, čo zlé pravidlo urobí, je presun do koša TIQR-u, odkiaľ sa
to vráti. Tvoje pravidlo *„vzdy najskor presunutie do kosa, az potom zmazanie"*
platí aj tu.

---

## 3. Odosielatelia sa dajú upravovať

> *„toto s moznostou upravit, odstranit pridat"*

Tých 17 domén bolo doteraz **zadrôtovaných v binárke** — videl si ich, ale nemohol
s nimi nič urobiť. Jediná cesta, ako s niektorou nesúhlasiť, bolo pridať ďalšie.

Teraz:
- **klikneš na doménu** → prepíšeš ju,
- **×** → zmažeš ju (aj tú, ktorá prišla s appkou — „builtin" nič neznamená),
- pole dole → pridáš novú.

Je to **jeden zoznam namiesto dvoch**. Tvoje staré „Also watch these senders"
sa pri prvom spustení automaticky prenesú dnu, takže o nič neprídeš.

**Keď zmažeš všetky:** appka ťahá celú schránku. Nevráti si potichu svojich 17 —
to by bolo, že ťa appka prehlasuje.

Čo sa **nezmenilo zámerne**: rozpoznávanie značky pri klasifikácii (že mail
z `paypal.com` je PayPal) ostáva na vstavanom zozname. To nie je sledovací
zoznam, to je slovník názvov, a nechcel som pri tejto úlohe hýbať klasifikáciou.

---

## 4. Starred — a tu bola naozajstná chyba

> *„je to jedno ake stare su tie maily ktore mam oznacene tie musia byt vzdy"*

**Čo bolo zle.** Označené aj bežné maily sa listovali do **jedného spoločného
stropu 900**. Gmail vracia od najnovších. Takže keď máš viac ako 900 označených,
vrátilo sa tých istých najnovších 900, appka zistila, že ich už má, strop bol
plný — a **staršie označené sa nemali ako dostať dnu. Nikdy.**

Prepočítal som si to na číslach, až potom opravoval.

**Čo je teraz:**
- Starred má **vlastné listovanie** až do 30 000 správ, nie podiel z 900.
- Ide **prvý v poradí** — nie za mesiacom newsletterov.
- `starred_query()` **zámerne neberie žiadne argumenty**: neplatí naň dátumové
  okno, ani výber „len marketplace", ani tvoj vlastný filter. Označenie je
  najjasnejší signál, aký vieš dať, že mail je dôležitý — tak nič v appke
  nesmie mať právo ho skryť.

---

## 5. Maily pribúdajú priebežne

> *„ano toto chcem aby sa vzdy po kazdom urobenom zobrazili"*

**Prečo sa to dalo urobiť:** u Googlu stojí vypýtať si *zoznam* 500 ID rovnako
ako stiahnuť *jednu* správu. Listovanie je skoro zadarmo, sťahovanie je celý účet.

Takže sync teraz **zoznam urobí raz**, odloží si ho **na disk**
(`mail_fetch_queue`) a sťahuje po **40**. Ďalší beh už nelistuje, len berie ďalších
40. Vidíš *„ostáva ešte 4 812"* namiesto ticha.

Na disku, nie v pamäti — lebo prvý sync veľkej schránky trvá minúty a ty appku
medzitým zavrieš. V pamäti by to začínalo odznova pri každom spustení. Takto
pokračuje tam, kde skončilo.

**Jedna vec, ktorú treba vysvetliť:** kým sa dobieha pozadu, **dátumové okno sa
neposúva**. Keby sa posúvalo, maily, ktoré sa ešte nestihli stiahnuť, by sa ocitli
mimo dosahu každého ďalšieho dotazu — stiahli by sa nikdy a nič by neukázalo,
že chýbajú.

---

## 6. Vlastný Gmail filter + všetky záložky

> *„moznost nastavit si aj filtre v tiqr"* / *„nech to cita vsetky zalozky"*

**Filter** (Mail → Settings): píšeš priamo v Gmail syntaxi —
`-category:promotions`, `has:attachment`, `label:PULLS`, `-from:noreply@x.com`.
Je to lepší jazyk, než aký by som vymyslel, tak sa použije ako je.

**Pridáva sa k oknu, nikdy ho nenahrádza** — filter vie zúžiť, čo príde, ale nevie
sync potichu zmeniť na niečo neohraničené. Nevyvážené zátvorky alebo úvodzovky
sa **odmietnu celé**, nie polovične — polovičný filter by zjedol aj dátumové okno.

**Záložky:** teraz vidíš **všetky svoje**, aj prázdne. Doteraz sa skrývali tie,
z ktorých sa ešte nič nestiahlo — čo je pri prvom syncu skoro všetko, a nedalo
sa odlíšiť „zatiaľ prázdna" od „chýba".

---

## 7. Rust testy

> *„a tie rust testy opravit"*

**Tri testy boli červené od 2.105.0** a bolo to takto: kontrolovali, že dotaz
obsahuje `-in:spam` a **žiadne** `in:anywhere`. Lenže v 2.105.0 si pýtal vidieť
spam a kôš (*„vediet rozlisit tiez promotions, spam kos atd"*) a dotaz sa zámerne
zmenil presne naopak. Testy nikto neupravil, takže mesiace popisovali pravidlo,
ktoré appka vedome prestala dodržiavať.

Opravil som ich proti tomu, **čo appka naozaj robí**. Bezpečnosť nedrží dotaz —
drží ju `is_untrusted`: klasifikácia na spame a koši **odmietne bežať**, takže
podvodný mail *„your payout of 9 999 EUR"* si vieš pozrieť, ale nikdy sa
nepremení na finančný záznam. Na to sú vlastné testy.

**Pribudlo 25 nových testov**: pravidlá (poradie, Keep, prázdny text, neznáme
pole), domény (čo je a čo nie je doména), filter (vyvážené zátvorky).

---

## 8. Chyba, ktorú som našiel pred odoslaním

Nový SQL dotaz čítal stĺpec `preview`. Ten stĺpec sa v databáze volá **`snippet`**.
Rust to neodhalí — pre neho je to len text. **Padlo by to až u teba za behu.**

Našiel to kontrolór, ktorý som si na to napísal: prejde každý SQL dotaz v kóde
a overí každý názov stĺpca proti migráciám.

A rovno aj to, prečo mu teraz verím: **prvá verzia toho kontrolóra potichu
preskakovala polovicu súboru** — hľadala reťazce regexom a jedna úvodzovka
v komentári jej rozhodila zvyšok súboru. Napriek tomu hlásila „OK". Takže:
číta sa to teraz skutočným lexerom a **vypisuje počet skontrolovaných dotazov**,
lebo „OK" po skontrolovaní ničoho nie je výsledok.

Overený na **troch zámerne pokazených kópiách** (každú musel nájsť) a čistý na
**1 068 dotazoch** v celom backende.

---

## Čo je overené a čo nie — bez prikrášľovania

**Overené:**
- 45 migrácií, 45 súborov, kontrolka v `database.rs` na 45 — sedí.
- Zátvorky a bloky vo všetkých zmenených Rust aj TS súboroch.
- Každý `api.*` volaný z nového panela existuje; každý nový príkaz je
  zaregistrovaný v `lib.rs`.
- Každý názov SQL stĺpca v `mail.rs` existuje v migráciách (98 dotazov).
- **Všetkých ~60 nových testovacích tvrdení som prepísal do Pythonu a nechal
  bežať** — logiku dotazu, filtra, domén aj pravidiel. Všetky prechádzajú.

**Neoverené, a treba to povedať:**
- **Nič z toho som nespustil.** Na tomto Macu nie je node ani cargo —
  **prvý a jediný kompilátor je GitHub CI.** Moje kontroly pozerajú na tvary,
  nie na typy a mená, takže zelené kontroly **nehovoria, že sa to skompiluje**.
- Vzhľad panela som nevidel bežať.
- Chovanie fronty proti skutočnému Gmailu s tisíckami označených mailov
  je odvodené z dokumentácie kvót, nie odmerané.

---

## Súbory

**Nové:** `src-tauri/migrations/044_mail_fetch_queue.sql`,
`045_mail_rules_and_senders.sql`, `src/components/MailSettingsPane.tsx`

**Zmenené:** `src-tauri/src/commands/mail.rs`, `models.rs`, `lib.rs`, `db.rs`,
`commands/database.rs`, `src/pages/MailIntelligence.tsx`, `src/pages/Settings.tsx`
(−366 riadkov), `src/lib/api.ts`, `src/lib/types.ts`, `GMAIL-SETUP.md`

**Ďalšia nová migrácia je 046.**

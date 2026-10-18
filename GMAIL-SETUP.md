# Mail Intelligence — ako pripojiť Gmail, krok po kroku

Toto je jednorazové nastavenie na strane Googlu plus pripojenie v appke.
Platí od verzie **2.100.0**. Nastavenia sa v **2.109.0** presunuli
z Settings priamo do Mailu — pozri krok 5.

**Dôležité rozdelenie, aby si nerobil zbytočnú prácu dvakrát:**

| | kde sa robí | ako často |
|---|---|---|
| Kroky **1–4** | Google Cloud Console | **raz za projekt** (nie za počítač) |
| Kroky **5–7** | v appke TIQR | **na každom počítači zvlášť** |

Máš dva počítače, ktoré si synchronizuješ. Google konfigurácia je spoločná,
ale pripojenie schránky je viazané na konkrétny stroj — pozri časť
„Prečo sa schránka nesynchronizuje medzi počítačmi" na konci.

---

## KROK 1 — Otvor SPRÁVNY projekt

Toto je krok, na ktorom sa to najčastejšie láme: ak máš v Google Cloud viac
projektov, všetko ostatné spravíš v nesprávnom a nič nebude fungovať.

1. Choď na <https://console.cloud.google.com/>
2. Hore vľavo klikni na **prepínač projektov** (vedľa loga Google Cloud).
3. Vyber projekt, v ktorom je OAuth klient pre TIQR.

**Ako spoznáš ten správny:** otvor
<https://console.cloud.google.com/auth/branding> — v poli **App name** musí
byť **TIQR Manager**. To je presne to meno, ktoré ti Google ukázal na
chybovej obrazovke.

Druhá kontrola: <https://console.cloud.google.com/auth/clients> — má tam byť
OAuth klient typu **Desktop app**. To je ten, ktorého ID je v GitHub secrets
ako `GOOGLE_OAUTH_CLIENT_ID`.

> Ak tam žiadny takýto projekt nie je, Gmail sa pripojiť nedá — appka
> používa ten istý OAuth klient ako prihlásenie do Sheets.

---

## KROK 2 — Zapni Gmail API

Bez tohto appka dostane od Googlu **403** aj s úplne správnym povolením.

1. Choď na <https://console.cloud.google.com/apis/library/gmail.googleapis.com>
2. Skontroluj, že hore je vybraný **ten istý projekt** z kroku 1.
3. Klikni **Enable**.

Ak tam už svieti „API enabled", je to hotové.

> Po zapnutí daj tomu **minútu či dve**, než to Google rozbehne. Keď to
> vyskúšaš hneď, môžeš ešte dostať 403.

---

## KROK 3 — Pridaj oprávnenie na čítanie mailu

1. Choď na <https://console.cloud.google.com/auth/scopes>
   (v konzole: **Google Auth Platform → Data access**)
2. Klikni **Add or remove scopes**.
3. Do filtra napíš `gmail.readonly`.
4. Zaškrtni `https://www.googleapis.com/auth/gmail.readonly`
   — popis znie *„Read all resource and their metadata—no write operations."*
5. **Update** → potom dole **Save**.

**Pridaj LEN `gmail.readonly`.** Nie `gmail.modify`, nie `mail.google.com`.
TIQR z príncípu nič v schránke nemaže, neposiela ani neoznačuje, a tento
scope je to, čo mu v tom bráni na strane Googlu — nie to, že je kód opatrný.

---

## KROK 4 — Pridaj seba medzi testerov

1. Choď na <https://console.cloud.google.com/auth/audience>
   (v konzole: **Google Auth Platform → Audience**)
2. Nájdi sekciu **Test users**.
3. **Add users** → napíš `biznismarko9@gmail.com` → **Save**.

Bez tohto dostaneš *„Access blocked: TIQR Manager has not completed the
Google verification process"* a Google ťa k súhlasnej obrazovke vôbec
nepustí.

> **Publishing status nechaj na `Testing`.** Dôvod je v poslednej časti
> tohto dokumentu — publikovanie by vyžadovalo platený bezpečnostný audit.

---

## KROK 5 — Pripoj schránku v appke

1. V TIQR otvor **Mail** (ľavé menu) → dole v zozname zložiek **Settings**.

   > Od verzie **2.109.0** sú tu, nie v Settings → Integrations. Tam ostal
   > len riadok s odkazom sem.
2. Ak už tam svieti „Connected", klikni najprv **Disconnect.**

   **Toto nepreskakuj**, ak si sa už raz pokúšal pripojiť pred krokom 3.
   Uložené povolenie nesie starú sadu oprávnení — bez Gmailu — a samotné
   obnovenie tokenu ju nezmení. Nové oprávnenie dostaneš len novým
   pripojením.

3. Klikni **Connect Gmail**. Otvorí sa prehliadač.

---

## KROK 6 — Na Google obrazovke nechaj mail zaškrtnutý

1. Vyber účet `biznismarko9@gmail.com`.
2. Google ukáže varovanie, že appka nie je overená — klikni **Advanced** →
   **Go to TIQR Manager (unsafe)**. Je to tvoja vlastná appka; toto
   varovanie dostane každá neoverená appka v stave Testing.
3. **Nechaj zaškrtnuté políčko pre čítanie mailu.**

   Google pri citlivých oprávneniach dáva zaškrtávacie políčka a **dá sa
   prejsť ďalej aj s odškrtnutým**. Vtedy budeš „pripojený", ale bez
   prístupu k mailu — a appka ti to povie červeným pruhom
   *„Google did not grant permission to read mail"*.

4. **Continue.** Prehliadač ukáže, že sa môžeš vrátiť do appky.

---

## KROK 7 — Over a spusti

1. V **Mail → Settings** klikni **Test connection**.
   Má sa objaviť zelené **Reachable** a tvoja adresa. Toto volanie
   **nečíta ani jeden mail** — len sa pýta Googlu, či token funguje.
2. **Nemusíš nič stláčať.** Od verzie **2.110.0** appka začne čítať sama,
   pár sekúnd po pripojení, a číta ďalej sama.
3. Číta po priečinkoch, v každom **od najnovšieho po najstarší**, bez
   dátumového limitu, v tomto poradí: **Important** (odosielatelia, ktorých
   sleduješ), **Starred**, **tvoje labels**, **Finances** — a až potom
   všetko ostatné (Inbox, Promotions, Spam, Trash).

V hlavičke Mailu vidíš, ktorý priečinok práve číta a koľko ešte čaká. Kde
skončil, je uložené na disku, takže pokračuje aj po zavretí a otvorení
appky. Keď je všetko prečítané, kontroluje nové maily v intervale z
**Mail → Settings → Reading and keeping**. Appka nemá službu na pozadí —
keď je zavretá, nečíta nič.

---

## Keď to nejde: ako zistiť, ktorá z dvoch príčin to je

Od **2.110.0** appka číta z odpovede Googlu, *prečo* odmietol, a povie to
vlastnými slovami:

- **„Google asked TIQR to slow down"** — nie je to chyba. Google obmedzuje,
  koľko sa dá čítať za sekundu. Nič sa nestratí, appka chvíľu počká a
  pokračuje sama. (Do 2.109.1 sa toto mylne hlásilo ako „Google refused
  access" a maily, ktoré sa práve nestiahli, vypadli z fronty.)
- **„Could not reach Gmail"** — počítač nie je na internete. Appka to skúsi
  znova každých 30 sekúnd a z fronty nevyhodí nič.

Skutočné odmietnutie má dve príčiny:

- **„…connected without permission to read mail"** — chýba povolenie, pozri
  nižšie.
- **„The Gmail API is switched off…"** — krok 2.

Pre obe platí toto:

Pozri sa do **Mail → Settings → Mailbox**:

**Je tam červený pruh o povolení na mail?**
→ Token nemá Gmail. Sprav krok 3, potom **Disconnect** a krok 5–6 znova,
a tentoraz nechaj to políčko zaškrtnuté.

**Červený pruh tam nie je?**
→ Scope je v poriadku, ale **nie je zapnuté Gmail API** alebo je zapnuté
v inom projekte. Sprav krok 2 a over krok 1.

**Stále nič?** Zmaž staré povolenie na
<https://myaccount.google.com/connections> → nájdi **TIQR Manager** →
**Remove access** → a pripoj znova od kroku 5.

---

## Čo ťa čaká o týždeň: 7-dňová platnosť

Appka v stave **Testing** dostáva od Googlu prístup s platnosťou **7 dní**.
Po týždni ti sync prestane chodiť a appka napíše:

> *Google would not accept the saved authorisation. Reconnect the mailbox
> in Settings.*

**Nie je to chyba.** Je to pravidlo Googlu pre neoverené appky. Oprava je
**Disconnect → Connect Gmail** (krok 5–6), trvá to pár sekúnd.

**Prečo to neriešim publikovaním appky:** publikovanie ten limit ruší, lenže
`gmail.readonly` je u Googlu *restricted scope* — na publikovanie s ním
treba prejsť overením vrátane bezpečnostného auditu (CASA), čo je platená,
každoročná záležitosť. Pre appku, ktorú používaš len ty, sa to neoplatí.

**Jediná čistá cesta von**, ak ju raz budeš chcieť: Google Workspace účet na
vlastnej doméne. Tam sa projekt prepne na **Internal** a 7-dňový limit
zmizne bez akéhokoľvek overovania.

---

## Dva počítače (od 2.110.0)

**Pripojenie, pravidlá, odosielatelia, filter aj webhooky sú na oboch
počítačoch rovnaké** — prejdú cez cloud sync. Pripojíš schránku na jednom
a druhý ju dostane pri ďalšom syncu; odpojíš na jednom, odpojí sa aj na
druhom. Ak niečo zmeníš na oboch, platí novšia zmena.

**Samotné maily necestujú.** Každý počítač si ich číta z Gmailu sám, do
vlastného súboru `tiqr-manager.mail.sqlite3` vedľa hlavnej databázy. Tak
ostáva malý aj súbor, ktorý ide na Drive, a čo počítač prečítal, nemôže
prepísať ten druhý. To je aj to, čo bolo dôvodom pôvodného „každý počítač
zvlášť" v `migrations/040_mail_intelligence.sql`: spoločný bod, odkiaľ sa
číta, by jednému z nich potichu zastavil mail. Ten bod ostáva na každom
počítači vlastný.

**Na každom počítači zvlášť ostáva:** pauza, a čo smie TIQR vytvoriť samo
(orders / sales / finance na „Automatic"). Dva počítače na Automatic by
z toho istého mailu vytvorili ten istý záznam dvakrát.

Záznamy, ktoré z mailu raz vzniknú (predaje, financie), sa medzi počítačmi
synchronizujú normálne ako všetko ostatné.

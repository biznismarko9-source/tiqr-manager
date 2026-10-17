# TIQR Manager 2.100.0 — Mail Intelligence, fáza 1 zo 4

Žiadal si, aby TIQR čítal schránku, rozpoznal dôležité maily a automatizoval
z nich predaje, sklad a financie.

**Dodávam fázu 1 celú a funkčnú.** Tvoj vlastný prompt to pripúšťa:
*„If the full feature cannot be implemented safely in one pass, deliver the
first phase as a complete, working feature and clearly document what
remains."* Toto je presne to — nie štyri fázy spravené na polovicu.

Najskôr tri veci, ktoré musím povedať nahlas.

---

## 1. Hranica, ktorú treba vedieť ako prvú

**Nič v tomto vydaní nezapíše predaj, objednávku, lístok ani finančný
záznam.** Ani jeden `INSERT`, ani jeden `UPDATE` do týchto tabuliek —
v celom module.

Nie je to opatrnosť pre opatrnosť. Všetko menované je chránená oblasť, a
parser mailov, ktorý zapisuje peniaze skôr, než si jeho výsledky videl na
svojom skutočnom maile, je presne to, ako sa zo schránky stane pokazená
účtovná kniha.

Je na to **test**, ktorý spočíta riadky v `sales`, `orders`, `tickets`,
`finance_entries` a `events` pred spracovaním mailu a po ňom a overí, že sa
nepohli. Nespolieham sa na to, že som to tak napísal — tvrdím to a meriam.

Fáza 1 si právo zapisovať zaslúži tým, že ti najprv na reálnom maile ukáže,
čo by bola zapísala.

---

## 2. Tri rozdiely oproti promptu — nedali sa splniť doslova

**IMAP tam nie je, a je to rozhodnutie o závislostiach.** V tomto projekte
nie je IMAP klient a z toho, čo tu je, sa napísať nedá — potreboval by nový
crate, a tvoje stále pravidlo znie **žiadne nové závislosti**. Nechcel som
ho porušiť potichu. Rozhranie (`MailProviderKind`, `FetchedMessage`) je
postavené tak, že IMAP pribudne **vedľa** Gmailu, nie cezeň, v deň keď
povieš, že ten crate môže ísť dnu.

**Gmail naopak nepotreboval ani jednu novú závislosť** — a to bol nález
auditu, nie plán. Appka už má kompletný Google OAuth 2.0 desktop tok (PKCE,
loopback listener, výmena kódu, refresh token) postavený pre Sheets, a
`run_sign_in` berie **scope ako parameter**. Už existoval aj precedens
druhého samostatného scope (`FIREBASE_SIGN_IN_SCOPE`). Takže schránka je
**tretí samostatný grant** cez tú istú mašinériu. Gmail API je obyčajné
HTTPS + JSON, čo `reqwest` a `serde_json` už robia, a `base64` (tiež už
závislosť) dekóduje telo správy.

**Kurzor je dátové okno, nie Gmail `historyId`.** Toto je zámerná odchýlka
od zjavného návrhu a stojí za vysvetlenie.

Gmail drží históriu asi týždeň a na starší `startHistoryId` odpovie `404`.
Táto appka **nemá žiadnu službu na pozadí a beží len vtedy, keď ju máš
otvorenú** (je to zapísané v `KNOWN_BUGS.md` ako dokumentované obmedzenie).
Notebook zavretý cez sviatky je teda **bežný stav, nie výnimka** — takže
cesta cez históriu by bola expirovaná častejšie než nie, a „fallback" by
robil skutočnú prácu a zbieral chyby.

Takže sync listuje podľa dátumu, s prekryvom jedného dňa oproti minulému
behu, a **prekryv je zadarmo**, lebo ukladanie je idempotentné:
`UNIQUE (provider, account_email, provider_message_id)`. Znovuvidenie
správy stojí jeden zápis, ktorý nič nespraví, a nikdy druhý riadok.
`historyId` sa aj tak ukladá — budúca push cesta potrebuje odkiaľ začať.

---

## 3. Čo musíš spraviť ty, aby to vôbec fungovalo

Toto je **jediná vec, ktorá ti to zablokuje**, a nie je v kóde.

OAuth klient sa do buildu vkladá z GitHub secrets (`GOOGLE_OAUTH_CLIENT_ID`,
`GOOGLE_OAUTH_CLIENT_SECRET`) — to už funguje pre Sheets. Aby fungoval aj
mail, v **tom istom** Google Cloud projekte treba:

1. **Zapnúť Gmail API** (APIs & Services → Library → Gmail API → Enable).
2. Na **OAuth consent screen** pridať scope
   `https://www.googleapis.com/auth/gmail.readonly`.
3. Kým je appka v režime *Testing*, pridať svoju adresu medzi **Test users**.

Bod 3 stojí za vetu navyše: `gmail.readonly` je u Googlu **citlivý scope**.
Pre appku v *Testing* stačí byť test user. Pre *Published* appku Google
vyžaduje overenie (security assessment) — ak by si to niekedy rozdával
ďalším ľuďom, toto je ten krok, o ktorý sa to zasekne. Pre teba samého to
problém nie je.

**Žiadna nová premenná prostredia nepribudla.** Použijú sa tie isté dve,
ktoré už máš.

Kým to nespravíš, appka sa správa čestne: Google vráti `403` a Settings
ukáže *„Gmail API nie je pre túto appku zapnuté, alebo schránka nedala
povolenie čítať mail."* — nie tiché zlyhanie.

---

## 4. Čo je hotové

**Settings → Integrations → Email** — jediné miesto, kde sa schránka
nastavuje. Connect (otvorí skutočný Google súhlas), Test connection,
Pause/Resume, Disconnect, retention, posledný sync, posledná chyba.
**Nie je tam žiadne pole na token** — pripojenie je OAuth grant, nie heslo,
ktoré by si niekam lepil.

**Mail → tri záložky:**

- *Overview* — päť skutočných počtov (žiadne vymyslené čísla), posledná
  aktivita, rozpad podľa kategórie.
- *Inbox Activity* — tabuľka s filtrami (hľadanie, kategória, stav, dátum
  od/do). Klik na riadok rozbalí **čo TIQR rozpoznal a prečo**, plus celú
  históriu spracovania.
- *Review Queue* — to isté, pripnuté na „čaká na teba".

**Klasifikácia** do deviatich kategórií (predaj, nákup, výplata, refund,
zrušenie, zmena eventu, stav objednávky, účet, ostatné), deterministicky:
doména odosielateľa + kľúčové slová. Rozpoznávam Viagogo, StubHub, Ticombo,
Vivid Seats, Ticketmaster, Live Nation, SeatGeek, Eventim, Ticketportal,
Predpredaj, Gigsberg, PayPal, Stripe, Revolut, Wise.

**Sync** — manuálne tlačidlom aj automaticky každých 15 minút, kým je appka
otvorená. Beh sa nikdy neprekrýva s iným (zámok v databáze, nie v UI), pauza
sa kontroluje v backende, a zámok sa uvoľní aj keď beh spadne.

---

## 5. Bezpečnosť — bod po bode, ako si ich napísal

**„Tokeny bezpečne, nikdy v logoch, odpovediach, chybách."**
- Refresh token leží v `app_settings` presne tam a tak, ako už leží ten pre
  Sheets. Jeden sklad tajomstiev, jedna konvencia.
- `MailConnectionStatus`, čo číta frontend, nesie **booleany a adresu
  schránky** — nikdy token. Je na to test, ktorý uloží falošný token,
  zoserializuje stav a overí, že v ňom nie je.
- `safe_detail` rediguje všetko, čo vyzerá ako credential, **než** sa to
  dostane do log riadku, chyby alebo na frontend — vrátane „dlhý súvislý
  reťazec token-ovitých znakov", nech sa to volá akokoľvek.

**„Nevracaj surové chybové telá."** Telo Google odpovede sa **nikdy
nečíta**. Berie sa `.status()` a hlavička `retry-after`, nič viac.

**„Najmenšie potrebné oprávnenia."** `gmail.readonly`. TIQR **nevie**
poslať, zmazať ani označiť nič vo tvojej schránke — a vynucuje to Google,
nie to, že som bol opatrný. Zámerne nie `gmail.modify`.

**„Minimálne uchovávanie tiel správ."** Telá sa neukladajú. Ukladá sa
orezaný (600 znakov), od HTML očistený a redigovaný výňatok. Retention ho po
nastavenom počte dní **vymaže, ale riadok nechá** — riadok je audit.

**„Bezpečné renderovanie HTML, žiadne skripty."** HTML sa nerenderuje
vôbec — striptuje sa v Ruste na čistý text, a obsah `<script>` a `<style>`
sa zahodí **celý**, nie len jeho tagy (inak by zdroják skriptu prežil ako
„text"). V celej appke neexistuje cesta, ktorá by výňatok podala DOM-u ako
markup.

**„Prílohy nespracúvať automaticky."** Prílohy sa nesťahujú **vôbec** —
nie schované za prepínačom, jednoducho to nie je implementované. To je
jediná záruka, ktorá za niečo stojí.

**„Obsah mailu nesmie meniť pravidlá."** Klasifikácia je deterministická,
žiadny model sa nepýta, čo s mailom spraviť. Text v tele sa matchuje na
kľúčové slová presne ako každý iný text a **nevie sa dostať na žiadnu
zapisovaciu cestu, lebo žiadna neexistuje.** Je na to test s reálnym prompt
injection pokusom. **Ten test treba udržať zelený, keď fáza 2 pridá AI** —
vtedy prestane byť zadarmo.

**Spoofing odosielateľa.** Zobrazované meno v hlavičke `From` je text, ktorý
ovláda útočník — `"noreply@viagogo.com" <podvodnik@evil.test>` je legálna
hlavička. Preto sa pravidlá matchujú **výhradne na adresu**, nie na meno, a
doména sa porovnáva na hranici bodky, takže `viagogo.com.evil.test`
Viagogom nie je. Oboje má test.

**SQL.** Každá hodnota vo filtroch je **bindovaná**, nikdy formátovaná do
SQL. Je na to test, ktorý do hľadania pošle `'; DROP TABLE mail_messages; --`
a overí, že tabuľka aj jej riadky stále existujú.

---

## 6. Overenie — a čo som NEoveril

**Toto musím povedať rovno: testy som nespustil.** Na tomto Macu nie je
cargo ani node — CI je prvý kompilátor, a z toho istého dôvodu sa nedá
spustiť ani dev server. Napísal som **61 testov** v `mail.rs`; či prejdú,
povie CI, nie ja.

Rovnako **nie je overené**: skutočné pripojenie ku Gmailu. Tento sandbox sa
k Google nedostane, nemám tvoj OAuth klient a nemám schránku. Cesta cez sieť
je napísaná podľa Gmail API dokumentácie a preverená proti tomu, ako tá istá
appka už volá Google Sheets — ale prvé skutočné pripojenie spravíš ty.

Čo som overil, a ako:

| Kontrola | Výsledok |
|---|---|
| Zátvorky, všetkých 57 Rust súborov | BALANCED |
| Zátvorky, všetkých 66 TS/TSX súborov | 66/66 OK |
| Komentárové pasce (A0/A2/A3) | 123 súborov — čisté |
| **Rust struct literály (E0063)** | **559 overených — OK** (5 preskočených, vypísaných menovite a 2 z nich moje, dokontrolované ručne) |
| **Fantómové importy, celý frontend** | **1153 pomenovaných importov — všetky existujú** |
| Registrácia príkazov | 256/256 |
| Typy z `models.rs` použité v `mail.rs` | 9/9 existujú |
| Cross-modul funkcie (OAuth, settings) | 7/7 existujú |
| CSS triedy použité novou stránkou | všetky existujú |
| Render stránky s realistickými dátami | pozreté |

Dve veci, ktoré našlo overovanie a čítanie by nenašlo:

**`toast.info?.(x) ?? toast.success(x)`** — `info()` vracia `void`, takže
`??` by **vždy spustil aj `success`**. Dva toasty na jednu udalosť.

**`<>` fragment s `key`** — skrátený fragment kľúč niesť nevie, a práve on
bol prvkom poľa. Prepísané na `<Fragment key=...>`.

A jedna, ktorú našiel až **render**: figúrka **„Failed"** na Overview mohla
ukazovať **len nulu** — vo fáze 1 nič ten stav nenastaví. Číslo, ktoré je
štrukturálne vždy nula, sa číta ako pokazené, nie ako pravdivé. Je preč;
`failedCount` ostáva v modeli pre fázu, ktorá ho vie naplniť.

---

## 7. Zmeny v databáze

**Migrácia 040**, tri nové tabuľky: `mail_messages`, `mail_processing_log`,
`mail_sync_state`. **Žiadna existujúca tabuľka sa nemení** — migrácia teda
nevie dotknúť ani jednu tvoju objednávku, predaj alebo lístok.

**Nie sú v `MERGE_TABLES`**, a to je rozhodnutie: pripojenie schránky patrí
jednému počítaču, a dva stroje zdieľajúce jeden sync kurzor je presne to,
ako mail na jednom z nich potichu prestane chodiť. Záznamy, ktoré z toho
raz vzniknú (predaje, financie), v `MERGE_TABLES` už sú a cestujú normálne.

---

## 8. Zmenené súbory

**Nové:** `migrations/040_mail_intelligence.sql` (210),
`commands/mail.rs` (2203), `pages/MailIntelligence.tsx` (666).

**Upravené:** `db.rs` (migrácia + `AppState` slot), `models.rs`,
`lib.rs` (13 príkazov), `commands/mod.rs`, `commands/database.rs`,
`lib/types.ts`, `lib/api.ts`, `pages/Settings.tsx`, `components/Layout.tsx`
(nav + 15-min sync), `components/icons.tsx`, `App.tsx`.

---

## 9. Čo NIE JE hotové — menovite

- **Fáza 2:** AI extrakcia polí (suma, mena, počet lístkov, referencia).
  Dnes je rozpoznaná **kategória**, nie čísla v maile.
- **Fáza 3:** párovanie na tvoje eventy/objednávky/predaje a vytváranie
  záznamov. **Celá.** Preto v review queue nie je tlačidlo „schváliť a
  vytvoriť" — nebolo by za ním nič.
- **Fáza 4:** pravidlá automatizácie, auto-processing, Discord notifikácie
  z mailu.
- **IMAP** — viď bod 2.
- **Sales / Finance / Inventory ako cieľové Discord kanály pre mail** —
  kanály z 2.99.0 existujú, mail do nich zatiaľ nič neposiela.
- **Pulls detail route + pohľad po osobách** (body 6 a 7 z 2.98.0) — stále
  odložené.

---

## 10. Verzia

2.99.0 → **2.100.0**, 9 výskytov v 7 súboroch plus `PROJECT_STATE/
CURRENT_STATE.md` a `CHANGELOG.md`.

# TIQR Manager 2.114.0 — jednoduché webhooky, hromadné mazanie, AI upratanie, nastavenia mailu

Čo si napísal po 2.113.1:

- *„takto je v maili meno a mail a takto dal webhook ako meno, treba
  opraviť"* (Ticket Holder: Details namiesto Luis Somoza)
- *„nie vždy a všade sú tieto personal details, tak to nemusí byť pri
  každom maili"*
- *„v bulku vedieť označiť maily a vymazať ich"*
- *„keď veľmi veľa spamujú promotions, tak 80 % z toho sú shits, ktoré
  vôbec netreba, AI scan dať úplne čo najlacnejší, ako môže byť, s tým, že
  bude fungovať rovnako dobre"*
- *„tie webhooks viac zjednodušiť, lebo tam je veľa info a je to potom
  neprehľadné, fakt len to najdôležitejšie"*
- *„viac vecí pridať do settings, aby si človek presne vedel nastaviť, ako
  ten mail má fungovať, čo a ako prijať, čo posielať ďalej, ako posielať,
  rules lepšie, mute atď."*

---

## 1. Meno kupujúceho vo webhooku

**Prečo bolo zle:**

- Viagogo mail má nad údajmi nadpis „Ticket Holder Details". TIQR pozná
  „Ticket Holder" ako menovku mena, a tak riadok prečítal ako meno
  „Details".
- Meno má len jedno miesto, takže skutočné „Full Name: Luis Somoza" sa už
  nezmestilo.

**Oprava** (`mail_facts.rs`):

- Najprv sa čítajú riadky s dvojbodkou („Full Name: Luis Somoza") - to, čo
  mail sám napísal ako menovku a hodnotu. Riadky tabuľky bez dvojbodky
  prídu na rad až potom, len ak zostalo miesto.
- Slová nadpisov (Details, Information, Summary, Overview, Below a
  podobne, aj slovensky, nemecky, španielsky) nie sú nikdy hodnota.
- Výsledok pri tvojom maili: **Full Name: Luis Somoza**, **Email Address:
  d1jedi@aol.com** - presne ako v maili.

## 2. Osobné údaje len tam, kde dávajú zmysel

Meno, e-mail, telefón a adresa sa medzi kľúčové údaje dostanú, len keď
platí jedno z dvoch:

- mail žiada **poslať, preniesť alebo doručiť lístky** (send, transfer,
  deliver, poslať, senden, enviar... spolu s ticket/lístky/Karten...);
- menovka priamo **menuje kupujúceho**: Ticket holder, Buyer, Recipient,
  Attendee, Kupujúci a podobne.

Potvrdenie objednávky, ktoré ťa oslovuje vlastným menom, ich už nemá.

## 3. Jednoduché webhooky

Každá správa v Discorde je **nadpis a dva až štyri krátke riadky**, bez
mriežky polí (predtým až desať polí pri nákupe).

| Druh | Riadky |
|---|---|
| Nákup | **2 tickets** bought on **Ticketmaster** for **€225.00** · 📅 dátum · miesto · 💺 sedadlá |
| Predaj | **2 tickets** sold for **€600.00** (· pending) · Profit **€320.00** · ROI 145 % · 📅 dátum · 💺 sedadlá |
| Event | 📅 dátum · miesto · 🎟️ 3 tickets · 2 in stock · 1 sold · Bought €305.00 |
| Financie | **-€45.50** out · Shell · 📅 dátum · účet · Business · poznámka |
| Mail | **viagogo** · 2 tickets · **€162.50** · 📤 ‖meno‖ · ‖e-mail‖ · ⏰ Send by … · 🎫 event · 💶 iná cena · 💺 sedadlá |

- Kód záznamu a pri mailoch číslo objednávky sú dole pri logu TIQR.
- Úvodné slová mailu sa ukážu len vtedy, keď o maili nie je známe nič
  lepšie.
- Spoiler sa nikdy nepreseká: správa sa skladá len z celých riadkov.

## 4. Hromadné mazanie

- Nad zoznamom je **Select all** (načítané maily). Hneď vedľa počtu
  označených je **Select all 1,840**, ktoré označí celý priečinok, aj s
  vyhľadávaním, nie len načítanú stranu.
- **Delete** presúva do Koša (Gmail sa nemení). Pri viac ako 50 mailoch sa
  najprv opýta: „Move 1,840 messages to Trash?".
- V Koši je **Delete N forever**. Platí tvoje pravidlo „vždy najskôr do
  koša, až potom zmazanie".
- Presun, obnovenie, prečítanie aj hviezdička pre tisíce mailov sa robia v
  jednej transakcii, takže idú hneď.

## 5. AI upratanie Promotions

V priečinku Promotions je hore pruh **Clean up with AI**. Ako funguje
(`commands/mail_cleanup.rs`):

1. **Najprv zadarmo, bez AI.** Vždy sa ponechá mail s hviezdičkou, od
   predajcu lístkov, ktorého TIQR pozná, s kódom, rozpoznaný ako
   objednávka, predaj alebo aktualizácia objednávky, alebo mail, ktorý si
   sám presunul.
2. **Zvyšok čo najlacnejšie:**
   - najlacnejší model (`claude-haiku-5-5`);
   - bez premýšľania, s najnižším úsilím;
   - 60 mailov na jeden dopyt, najviac 10 dopytov na jedno kliknutie
     (600 mailov), po tri naraz;
   - posiela sa len **doména odosielateľa** (nie adresa), predmet a 140
     znakov náhľadu;
   - odpoveď je jedno písmeno na mail: K (môže byť dôležité pre predajcu
     lístkov: presale, on-sale, turné, koncerty, šport, účty, platby,
     kódy), J (junk), ? (neviem).
3. **Každý mail sa kontroluje len raz.** Odpoveď sa uloží, takže sa za ten
   istý mail nikdy neplatí dvakrát. Orientačná cena: 600 mailov je asi
   40 000 tokenov, teda menej ako cent.
4. **Nič sa nepresunie samo.** Okno ukáže:
   - „Junk" - zaškrtnuté;
   - „Not sure" - nezaškrtnuté, na tvoje rozhodnutie;
   - zhrnutie (koľko pozrel, koľko junk, koľko neisté, koľko ponechané,
     koľko dopytov a tokenov, približná cena).

   Až *Move N to Trash* ich presunie do Koša, odkiaľ sa dajú vrátiť.
   *Look at 600 more* pokračuje v ďalších.
5. Mail je pre AI len dáta: nemôže uzavrieť zoznam, v ktorom je, a AI má
   výslovne zakázané riadiť sa čímkoľvek, čo je v ňom napísané. Odpoveď,
   ktorej TIQR nerozumie, znamená „neviem", nikdy „junk".

## 6. Viac nastavení mailu

V Mail → Settings sú sekcie: Mailbox, Reading, **What is read**, **Rules**,
**Muted senders** (nová), **Discord** (nová), Automatic.

- **What is read → Folders TIQR reads**:
  - zaškrtávanie pre Promotions, Spam a Kôš;
  - vypnutý priečinok sa neprechádza a nové maily z neho sa nesťahujú
    (ani z histórie zmien);
  - mail s hviezdičkou alebo v tvojom štítku sa číta stále;
  - čo už v TIQR je, ostáva.
- **Rules**:
  - nové pole **Sender, subject or preview** (jedno pravidlo pre
    odosielateľa aj predmet);
  - nová akcia **Star it**;
  - pri písaní pravidla hneď vidíš, **koľko mailov, ktoré už máš, by
    chytilo**.
- **Muted senders**: zoznam stíšených s tým, dokedy, *Unmute* a pridanie
  adresy alebo domény na 1, 7, 30 dní alebo navždy.
- **Discord → Mail that goes on to Discord**:
  - či sa dôležité maily posielajú samy;
  - ktoré druhy: Sales, Purchases, Payouts and refunds, Cancellations and
    event changes, Order updates.
- **Discord → What a mail message carries**:
  - Key details;
  - Personal details (vždy ako spoiler; vypnuté = do Discordu sa nikdy
    nepošlú);
  - The opening words.
  - Platí aj pre *Send to Discord*.
- Všetko sa ukladá s nastaveniami mailu a prejde aj na druhý počítač.

---

## Testy

- `cargo test --locked`: 1577 prešlo, 0 zlyhalo (4 ignorované, ako doteraz).
- `npm test`: 38 prešlo.
- `npm run build` prešiel.
- Nové testy:
  - meno pod nadpisom „Ticket Holder Details" (tvoj mail cez HTML) a
    nadpisy, ktoré nie sú hodnota;
  - osobné údaje len pri posielaní lístkov;
  - kompaktné správy pre nákup, predaj, event, financie a mail;
  - nastavenia Discordu (vypnuté osobné údaje sa nepošlú, vypnutý druh
    mailu sa nepošle);
  - označenie celého priečinka (1205 mailov) a ich zmazanie po častiach;
  - upratanie (čo sa posiela, čítanie odpovede, ponechanie zadarmo, nič
    dvakrát);
  - pravidlá „Sender, subject or preview", Star it, počet zhôd;
  - vynechané priečinky.
- Obrazovky v prehliadači s napodobeným backendom: Select all → Select all
  1,840 → Delete → otázka; Clean up with AI a okno s návrhmi; sekcie What
  is read, Rules (počet zhôd), Muted senders, Discord.

## Súbory

- `src-tauri/src/commands/mail_facts.rs`, `discord.rs`, `notifications.rs`,
  `mail.rs`, `mail_sync.rs`, `mail_view.rs`, `mail_cleanup.rs` (nový),
  `shared_settings.rs`, `mod.rs`, `lib.rs`, `models.rs`
- `src/components/PromoCleanup.tsx` (nový), `MailSettingsPane.tsx`,
  `src/pages/MailIntelligence.tsx`
- `src/lib/promoCleanup.ts` (nový, s testom), `src/lib/api.ts`,
  `src/lib/types.ts`
- `CHANGELOG.md`, `PROJECT_STATE/*`, verzia v piatich súboroch.

Žiadna nová migrácia ani krok mailovej databázy (ďalšia migrácia je 047).

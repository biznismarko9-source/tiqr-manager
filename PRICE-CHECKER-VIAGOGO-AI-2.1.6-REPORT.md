# TIQR Manager 2.1.6 — Price Checker: Viagogo namiesto StubHub + AI ako posledná záchrana

Toto je presne to, čo si chcel: StubHub preč z nových price checkov, Viagogo namiesto neho (skúsi sa to
rovnako automaticky ako pri Vivid Seats/Ticombo, s ručným fallbackom keď to nejde), StubHubova história
zostáva celá vidno, a pribudla AI (Anthropic) ako posledná záchrana, keď appka sama nič nenájde. Poriadne
som si to celé aj sám preveril druhým pohľadom (presne ako pri 2.1.3) — a ten pohľad reálne našiel a ja som
opravil 5 skutočných chýb, jednu z nich dosť dôležitú. Všetko je dole.

## Čo je nové

**Viagogo je teraz plnohodnotná marketplace**, presne na rovnakej úrovni ako Vivid Seats a Ticombo — vlastná
karta na stránke Price Checker, funguje na nej "Check Prices" (ručné/paste zadanie ako doteraz) aj
"Auto-check" (appka sa sama pokúsi otvoriť link a prečítať ceny na pozadí). Skúsil som si Viagogo aj sám
reálne otestovať proti živej stránke priamo teraz z tohto prostredia — nedá sa, presne z rovnakého dôvodu, ako
sa nedalo pri StubHub/Vivid Seats/Ticombo v 2.1.3 (vlastná sieťová politika tohto sandboxu to blokuje, nie
Viagogo). Čo to znamená pre teba nižšie v časti "Čo som NEVEDEL overiť".

**StubHub zostáva, ale iba na čítanie.** Na karte StubHub (len tam, kde už máš uloženú históriu alebo link)
teraz vidíš sivý štítok "Retired" vedľa jej mena, jej link je zobrazený ako obyčajný text (nedá sa prepísať) a
tlačidlá "Check Prices"/"Auto-check" tam vôbec nie sú. Všetko, čo si už predtým uložil — ceny, grafy, celá
história — zostáva presne také, aké to bolo, a naďalej sa to dá kedykoľvek pozrieť. Iba nové price checky a
nové/zmenené linky proti StubHub appka od teraz odmieta. Event, ktorý StubHub nikdy nepoužíval, ju vôbec
neponúkne — to sa nezmenilo, funguje to ako doteraz.

**AI (Anthropic) ako posledná záchrana.** Keď appka na Auto-check vyskúša svoj bežný, rýchly a bezplatný
spôsob čítania cien a naozaj nič nenájde, teraz môže — iba ak si si v Settings uložil vlastný Anthropic API
kľúč — skúsiť ešte JEDEN pokus: pošle text stránky AI modelu (Claude Haiku 4.5, najlacnejší/najrýchlejší
model presne na túto úlohu) s otázkou "sú tu nejaké ceny lístkov, a aké". Počas tohto pokusu appka teraz
ukazuje "Asking AI..." — predtým by na tomto mieste ešte chvíľu visela na "Cleaning up..." bez vysvetlenia, čo
sa presne deje, čo si sám spomínal, že ťa znepokojuje pri platenom API. Ak AI niečo nájde, formulár sa
predvyplní ako doteraz — ale s jasnou modrou poznámkou "AI read these prices off the page... double-check the
numbers below before saving", aby si vedel, že tieto konkrétne čísla stoja za druhú kontrolu pred uložením.

Bez uloženého kľúča sa nemení vôbec nič — appka sa AI ani nepokúsi opýtať, žiadne peniaze, žiadna zmena
správania oproti 2.1.5. Kľúč pridáš/zmeníš/zmažeš v **Settings → Integrations**, kde pribudla nová karta
"Anthropic API Key" vedľa Google Sheets kariet.

## Koľko to reálne stojí

Model je Claude Haiku 4.5 — najlacnejší, ktorý Anthropic ponúka — so stropom 1024 tokenov na odpoveď. Pri
bežnej dĺžke textu z jednej stránky vychádza jeden pokus na zlomok centu podľa dnešného cenníka. AI sa navyše
skúša len raz na jeden Auto-check, iba keď zvyšných aspoň ~8 sekúnd z existujúceho 60-sekundového limitu (ten
istý limit, čo appka má už od 2.1.2/2.1.3, nič sa v ňom nemenilo) — takže ťa to nikdy nemôže vtiahnuť do
dlhšieho/drahšieho čakania, než na aké si zvyknutý. Odporúčam prvých pár AI-asistovaných checkov porovnať s
tým, čo vidíš na Anthropic účte (console.anthropic.com), aby si mal sám istotu, že sedí to, čo tu píšem.

## Prečo je StubHub "read-only" vynútené aj v appke samotnej, nielen schované tlačidlo

Toto je detail, ktorý sa oplatí vysvetliť, lebo je to zámerná poistka. Nestačilo mi len schovať tlačidlá na
stránke — pridal som appke aj vlastnú kontrolu priamo v mieste, kde sa dáta skutočne ukladajú
(`require_marketplace_active`), ktorá odmietne uložiť nový link alebo nový price check proti StubHub, nech by
prípadný budúci bug v obrazovke čokoľvek dovolil kliknúť. Presne ten istý princíp appka už dávno používa pri
mazaní marketplace (nedovolí zmazať tú, čo má históriu) — "backend nikdy nesmie spraviť to, čo si výslovne
povedal, že sa už nemá diať, aj keby raz niečo v obrazovke zlyhalo".

## Čo našiel nezávislý druhý pohľad — a čo som opravil

Presne ako pri 2.1.3 (reálne peniaze za AI + reálna migrácia databázy = dosť vysoké stávky na to, aby som sa
spoľahol len sám na seba), dal som celú túto zmenu ešte raz prejsť nezávislým pohľadom bez prístupu k mojim
vlastným úvahám. Reálne našiel 5 vecí, všetky som opravil a pridal k nim testy:

1. **Najdôležitejšie: posledný pokus o čítanie cien bol nedosiahnuteľný kód.** Auto-check mal už predtým
   sľubovať "keď sa počas čakania stránka nestihne pripraviť, aspoň naposledy skúsim prečítať, čo tam je" —
   lenže kód, čo sa mal o toto starať, mal chybu, kvôli ktorej sa čakací rozpočet vždy minul do nuly PRESNE
   predtým, než sa k tomuto poslednému pokusu vôbec dostalo. Inak povedané: tento posledný pokus sa v praxi
   NIKDY nespustil. A keďže presne tento posledný pokus je to jediné miesto, čo by malo spustiť novú AI
   zálohu (tá sa spúšťa len keď appka povie "unable_to_read") — **toto by v praxi znamenalo, že AI záloha by
   sa skoro nikdy nespustila práve v tvojom vlastnom nahlásenom prípade** ("Auto-check bežal celých 60 sekúnd
   a nič nenašiel"), presne tam, kde by mala najviac pomôcť. Opravil som výpočet rozpočtu tak, aby si appka
   vždy vyhradila posledné 3 sekundy presne na tento posledný pokus, a pridal 3 nové testy, čo to strojovo
   dokazujú.
2. **Cancel počas AI volania ticho nerobil nič.** Appka si pri spustení Auto-checku vyčistila svoje interné
   "dá sa toto zrušiť" miesto príliš skoro — ešte pred tým, než AI volanie vôbec začalo. Keby si počas tých
   pár sekúnd, čo appka čaká na AI odpoveď, klikol Cancel, appka by to tíško ignorovala a nechala volanie
   dobehnúť. Preusporiadal som to tak, aby Cancel fungoval počas celého behu, AI volanie nevynímajúc.
3. **Nesprávne uložená mena pri AI-nájdenom výsledku.** Keď appka predvypĺňala formulár z Auto-check
   výsledku, mena sa dala rozoznať len z troch znakov ($/€/£) — čokoľvek iné (a AI vie vrátiť ľubovoľný ISO
   kód meny) sa ticho premenilo na "USD", aj keby to v skutočnosti bolo napríklad CZK. Opravil som to tak, že
   appka teraz použije presne tú menu, akú AI/appka skutočne rozoznala, nie odhad zo symbolu.
4. **Migrácia mohla appku natrvalo zablokovať pri štarte.** Táto konkrétna appka pri spúšťaní migrácií
   nepoužíva transakciu okolo jednotlivých krokov — takže keby (čisto teoreticky, cez existujúci ale v
   appke nepoužívaný príkaz na pridanie marketplace) už predtým existoval riadok "Viagogo", migrácia by
   zlyhala v polovici, no časť pred zlyhaním by ostala natrvalo uložená a neoznačená ako hotová — appka by sa
   pri každom ďalšom spustení znova a znova pokúšala tú istú migráciu a znova zlyhávala, navždy. Opravil som
   to obalením celej migrácie do jednej transakcie (buď sa uloží všetko, alebo nič) a pridal 2 testy presne na
   tento scenár.
5. **Dva testy dokazovali menej, než sa tvárili.** Dva z pôvodných testov na StubHub read-only správanie by
   prešli aj keby som celú novú poistku (bod vyššie o `require_marketplace_active`) úplne zmazal — omylom
   overovali len "nastala nejaká chyba", nie "nastala TÁTO KONKRÉTNA chyba". Sprísnil som ich, aby naozaj
   dokazovali to, čo mali.

## Čo som overil

```
cargo test --lib   -> 913 testov, 0 zlyhaní, 3 ignorované
cargo check --lib  -> čisté, bez chýb
cargo clippy --lib -> žiadne nové upozornenie z tejto práce (tých pár, čo clippy hlási aj v mnou upravených
                       súboroch, je ten istý kozmetický štýl komentárov, aký appka používa všade inde už
                       dávno predtým — nič, čo by som teraz spôsobil)
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.1.6 build" v hlavičke)
```

Naviac som spravil kompletnú **clean-room verifikáciu**: zabalený zip som rozbalil do úplne čistého priečinka,
porovnal zoznam aj obsah (SHA-256 kontrolný súčet) KAŽDÉHO súboru medzi tvojou kópiou a tým, čo som testoval —
100% zhoda, žiadny súbor sa pri balení nestratil ani nezmenil. Potom som v tom čistom priečinku ešte raz od
nuly spustil `npm ci` + build aj `cargo check --lib` — oboje prešlo rovnako čisto ako v mojej pracovnej kópii,
vrátane identického výstupu buildu (rovnaké názvy/hashe vygenerovaných súborov).

Aj `release.ps1` a `1-CLICK-UPDATE.bat` som tentokrát skontroloval riadok po riadku (nielen tie 3 zjavné
súbory s číslom verzie) — presne to, čo minule chýbalo a vyhodilo ti STOPPED pri 2.1.1. `$Version`,
`$CommitMsg` aj text v `.bat` súbore teraz všetky správne hovoria "2.1.6", `Cargo.lock`/`package-lock.json` sú
prečerstva pregenerované. Zapísal som si to aj do `PROTECTED-AREAS-NOTES.md`, aby sa na to nezabudlo nabudúce.

## Čo som NEVEDEL overiť odtiaľto

Toto je pri tejto verzii dôležitejšie ako zvyčajne, lebo obe hlavné novinky (Viagogo aj AI) stoja a padajú na
niečom, čo sa v tomto prostredí jednoducho nedá vyskúšať:

- **Reálne čítanie Viagogo stránky.** Skúsil som sa práve teraz priamo pripojiť na viagogo.com z tohto
  prostredia — odmietnuté vlastnou sieťovou politikou sandboxu (rovnaké odmietnutie som pre porovnanie práve
  teraz dostal aj na stubhub.com). Čiže neviem povedať, či appka na Viagogo skutočne nájde ceny automaticky,
  ani ako presne ich tabuľka na stránke vyzerá — len to, že kód je napísaný a otestovaný rovnako dôkladne, ako
  bol pre Vivid Seats/Ticombo, keď sa neskôr ukázalo, že tie reálne fungujú.
- **Reálny úspešný AI extrakt.** Vedel som si overiť, že appka sa vie pripojiť na `api.anthropic.com` (skúsil
  som to priamo teraz, spojenie prešlo) — ale nemám tvoj kľúč ani reálnu Viagogo stránku, takže som nevidel
  AI naozaj prečítať skutočné ceny zo skutočnej stránky. Otestované je len: appka správne poskladá požiadavku,
  správne spracuje odpoveď (aj keď je prázdna/nezmyselná/appka dostane chybu), a správne to všetko ukáže v
  UI — nie samotná kvalita toho, čo AI odpovie na reálnom Viagogo texte.
- **Skutočná Windows appka.** Ako pri každej doterajšej verzii — žiadna obrazovka, žiadny Windows, žiadny
  WebView2 v tomto prostredí. Kompilácia/testy/typová kontrola sú čisté, ale to isté platilo aj predtým, keď
  sa napriek tomu ukázali reálne chyby až na tvojom počítači.

Presne kvôli tomuto dole v STOP časti trvám na tom, aby si si Viagogo aj AI vyskúšal sám, skôr než na to
začneš spoliehať.

## Drobnosti, čo som si všimol a nechal tak

- **Anthropic kľúč sa ukladá do databázy ako obyčajný text** (vlastná tabuľka, nikdy sa neposiela späť do
  appky ako hodnota — appka vie len povedať "kľúč je uložený", nie aký je), presne tak, ako to appka odjakživa
  robí aj s tvojím Google Sheets prepojením. Znamená to, že kľúč skončí aj v zálohách databázy (Backup),
  presne ako všetko ostatné v nej. Nie je to niečo, čo by som teraz menil (nič v appke sa dnes neukladá inak),
  ale patrí sa, aby si o tom vedel.
- **Appka má teraz dva rôzne "AI cez Anthropic" mechanizmy** — tento nový (Price Checker, TVOJ vlastný kľúč
  zo Settings) a starší, čo appka používa pri kategorizácii eventov zo Sheets synchronizácie (ten používa
  kľúč zabudovaný priamo v builde appky, nie tvoj). Zámerne som to takto nechal — táto nová AI záloha sa
  môže reálne spúšťať oveľa častejšie (pri každom neúspešnom Auto-checku), tak dáva zmysel, že náklady idú na
  tvoj vlastný účet, o ktorom máš ty sám prehľad, nie skryté v appke.

## Zmenené súbory

**Backend (Rust):**
- `src-tauri/migrations/017_price_checker_viagogo.sql` — nová migrácia (Viagogo pribúda, StubHub sa retiruje,
  nová tabuľka na Anthropic kľúč), obalená transakciou po review-oprave
- `src-tauri/src/commands/price_checker.rs` — `require_marketplace_active` guard + jeho zapojenie do
  ukladania linkov a price checkov
- `src-tauri/src/commands/price_checker_auto.rs` — AI záloha (request/response/orchestrácia), nová fáza
  "asking_ai", + 3 review-opravy (rozpočet na posledný pokus, poradie cancel-slotu, cena/rozpočet AI volania)
- `src-tauri/src/commands/settings.rs` — `get_anthropic_api_key_configured`/`set_anthropic_api_key`
- `src-tauri/src/db.rs` — 2 nové testy na bezpečnosť migrácie 017
- `src-tauri/src/lib.rs`, `src-tauri/src/models.rs` — nové polia/registrácia príkazov

**Frontend:**
- `src/pages/PriceChecker.tsx` — Retired štítok + read-only card pre StubHub, AI-asistovaná poznámka,
  oprava predvyplnenia meny, nová fáza "Asking AI..."
- `src/pages/Settings.tsx` — nová karta "Anthropic API Key" v Integrations
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy/volania pre vyššie

**Verzia (bump do 2.1.6):** `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`,
`release.ps1` (`$Version` aj `$CommitMsg`), `1-CLICK-UPDATE.bat`, `Cargo.lock`, `package-lock.json`.

Nič iné sa nemenilo — Sales/Orders/Tickets/Inventory/Finance/Backup/CSV/Google Sheets sync bez zmeny.

## STOP

2.1.6 hotové, otestované všetkým, čo sa dá otestovať odtiaľto, zabalené a clean-room overené. Keďže Viagogo aj
AI sú presne tie dve veci, čo sa odtiaľto overiť nedali, prosím prejdi si na svojom počítači aspoň toto:

1. Spusti `1-CLICK-UPDATE.bat` — tentokrát by malo prejsť čisto, bez "STOPPED".
2. **Settings → Integrations** → pridaj svoj Anthropic API kľúč (ak ešte nemáš, vytvoríš si ho na
   console.anthropic.com) → over, že sa zobrazí "A key is saved.", že "Change key"/"Remove" fungujú.
3. Na evente, čo má uložený Viagogo link (alebo si nejaký zadaj), skús **Auto-check** — sleduj, či appka
   naozaj nájde ceny, a či fázy postupne ukazujú Starting → Loading → Analyzing → (možno) "Asking AI..." →
   Cleaning up.
4. Ak sa niekedy zobrazí modrá poznámka o AI, dvakrát skontroluj tie čísla predtým, než ich uložíš.
5. Skús Auto-check aj na linku, kde vieš, že appka nič nenájde — over, že sa BEZ uloženého Anthropic kľúča
   nič neúčtuje a appka sa správa presne ako v 2.1.5 (čistý "unable_to_read", žiadne "Asking AI...").
6. Po pár AI-asistovaných checkoch pozri svoj účet na console.anthropic.com — over, že náklady sedia s tým,
   čo píšem vyššie (zlomky centu).
7. Na evente s existujúcou StubHub históriou — over "Retired" štítok, read-only link, žiadne
   Check Prices/Auto-check tlačidlá, ale celá stará história/grafy stále vidno.
8. Rýchla kontrola, že nič staré sa nepokazilo — Sales/Orders/Events/Finance/Sheets sync ako doteraz.

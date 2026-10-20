# TIQR Manager 2.1.2 — Auto-check: oprava zamŕzania appky

Prešiel som si tvoj popis aj priložené súbory (boli identické s tým, čo už bolo v zipe, takže žiadna nová
informácia navyše, ale poslúžili na potvrdenie). Toto beriem ako prioritu #1, presne ako si napísal — a
neberiem predchádzajúci report ("Auto-check funguje") ako dôkaz ničoho. Reálne správanie na tvojom počítači
je jediné, čo sa počíta.

Dobrá správa: problém sa dá presne pomenovať, má jednu jedinú príčinu, ktorá vysvetľuje všetky tri veci, čo
si opísal (zaseknuté "loading", zamrznutá appka, appka sa nedá znova spustiť bez reštartu PC) — a opravil
som ju podľa oficiálnej dokumentácie Tauri (appka, na ktorej je TIQR Manager postavený), nie odhadom.

## Prečo appka mrzla a nedala sa znova spustiť

Auto-check si pri kliknutí otvára vlastné skryté okno prehliadača (na prečítanie cien zo stránky). Kód, čo
to okno vytváral, to robil zle — volal to priamo, "na počkanie", z toho istého miesta, kde appka spracúva
tvoje kliknutie na tlačidlo. Dokumentácia Tauri o presne tejto funkcii píše (citujem doslovne, neodhadoval
som to):

> "On Windows, this function deadlocks when used in a synchronous command and event handlers... You should
> use async commands and separate threads when creating windows."

Čiže: presne tento spôsob vytvárania okna sa na Windows dokáže zaseknúť sám do seba (deadlock) — a keďže sa
to deje na tom istom mieste, kde appka spracúva VŠETKY tvoje kliknutia, zasekne sa CELÁ appka, nie len
tlačidlo Auto-check. To vysvetľuje aj tie ostatné dve veci naraz:
- **Prečo je appka úplne mŕtva, nielen Auto-check** — appka má len jedno "hlavné vlákno", čo obsluhuje
  všetky kliknutia. Keď sa toto vlákno zasekne pri vytváraní okna, nemá sa čo postarať o zvyšok appky.
- **Prečo sa appka nedá znova spustiť bez reštartu PC** — appka má poistku proti dvojitému spusteniu
  (aby dve kópie appky nezapisovali naraz do tej istej databázy). Táto poistka drží zámok, kým appka
  BEŽÍ — a zaseknutá appka stále "beží" (len nič nerobí), takže zámok drží ďalej. Druhé spustenie sa oň
  odrazí a nemá sa ako dostať dnu. Zabiť zaseknutý proces (alebo reštartovať PC) je jediný spôsob, ako ten
  zámok pustiť.

## Čo som spravil

Prepísal som celú vnútornú logiku Auto-checku (`price_checker_auto.rs`) tak, aby toto vytváranie okna
bežalo na úplne samostatnom vlákne appky — presne to, čo dokumentácia Tauri odporúča. Appka na to vlákno
len čaká, s tvrdým stropom — nikdy nečaká donekonečna.

- **Auto-check nikdy nezamrzne appku.** Vytváranie okna aj celé čítanie cien beží mimo hlavného vlákna —
  appka zostáva plne použiteľná (menu, iné stránky, všetko ostatné) po celý čas, kým Auto-check beží.
- **Tvrdý strop 15 sekúnd.** Nech sa deje čokoľvek, Auto-check sa najneskôr po 15 sekundách sám ukončí s
  jasnou správou "trvalo to príliš dlho" — nikdy nekonečné "loading".
- **Skutočné tlačidlo Cancel.** Zobrazí sa hneď vedľa priebehu (Starting/Loading/Analyzing), dá sa
  kliknúť kedykoľvek počas behu a appka to zaregistruje prakticky okamžite (appka to kontroluje každých
  ~100 milisekúnd, aj keď práve číta stránku) — tlačidlo Auto-check je hneď potom znova použiteľné.
- **Okno prehliadača sa VŽDY zavrie** — či už Auto-check skončí úspechom, chybou, timeoutom, alebo ho
  zrušíš cez Cancel. Použil som vzor, kde Rust sám zaručí zavretie okna nech sa vyskočí z tej funkcie
  akoukoľvek cestou (tzv. RAII/`Drop`) — nie ručné "nezabudni zavrieť okno" na každom mieste zvlášť, kde sa
  dá ľahko na jedno miesto zabudnúť.
- **Zavretie appky počas behu Auto-checku je bezpečné.** Aj keby si appku zavrel presne v momente, keď
  Auto-check ešte behal, nemá sa čo zaseknúť pri ďalšom spustení — samotná príčina zaseknutia (vytváranie
  okna na zlom vlákne) je preč.
- **Šesť jasných výsledkov, nikdy len "načítava sa navždy":** úspech / stránka ma zablokovala / nepodarilo
  sa prečítať / chyba / zrušené (Cancel) / vypršal čas (15s). Presne to, čo si žiadal.

### Ako to teraz vyzerá v appke

Tlačidlo "Auto-check" sa počas behu zmení na "Starting..." → "Loading page..." → "Analyzing..." (skutočný
priebeh z appky, nie len falošná animácia) a hneď vedľa je tlačidlo **Cancel**. Kým Auto-check beží, ostatné
karty marketplace-ov majú tlačidlá Auto-check/Check Prices dočasne vypnuté (aby sa dva behy nepobili o
jedno miesto v appke, ktoré appka interne používa) — zvyšok appky (iné stránky, menu, všetko ostatné) je
celý čas úplne použiteľný.

Ak Auto-check skončí inak než úspechom (zablokované, nepodarilo sa prečítať, chyba, zrušené, vypršal čas),
formulár na ručné zadanie sa otvorí presne ako doteraz — teraz navyše s dvoma odkazmi: **"Try Auto-check
again"** (skúsi to isté znova) a **"Open the page myself"** (otvorí tú stránku v tvojom bežnom prehliadači,
nech sa pozrieš sám) — a manuálne/paste polia sú samozrejme stále funkčné presne ako predtým.

## Čo som overil a čo overiť NEVIEM (a prečo)

Toto prostredie nemá obrazovku (display server) ani Windows — takže naozaj OTVORIŤ skutočné okno appky a
kliknúť na Auto-check tu nejde, na žiadnom sandboxe doteraz v tomto projekte to nešlo (rovnaké obmedzenie
ako pri "Sign in with Google"). To znamená: **jediné, čo naozaj dokáže potvrdiť, že appka sa už nezasekne,
je tvoj vlastný počítač.** Nižšie presne čo som overiť VEDEL, a čo je na teba (checklist na konci).

Overené priamo v tomto prostredí:
```
cargo check --lib          -> celý kód sa skompiloval bez jedinej chyby
cargo test --lib           -> 859 testov, 0 zlyhaní (846 pôvodných + 13 nových/upravených k tejto oprave)
cargo clippy --lib --all-targets -> žiadne nové upozornenia z tejto opravy (jedno drobné som rovno opravil)
npx tsc -b                 -> 0 chýb
npm run build              -> OK ("tiqr-manager@2.1.2")
```
`cargo tauri build` som skúsil spustiť presne ako si žiadal — v tomto prostredí nie je nainštalovaný
(`cargo: no such command: 'tauri'`), rovnako ako pri každej predchádzajúcej verzii tejto appky. Vyžaduje si
to tvoj vlastný build stroj.

Pridal som aj poriadnu sadu testov presne na to, čo si vymenoval: úspech, timeout, cancel (aj to, že appka
reaguje na Cancel takmer okamžite, nie až po plnom čase), zlyhanie pri otváraní okna (aspoň v tej časti, čo
sa dá simulovať bez skutočného okna), zavretie okna sa deje vždy (RAII vzor), opakovaný check aj check po
zrušení. Dve veci sa nedajú otestovať v ŽIADNOM sandboxe (nielen tomto) — či appka fakt nezamrzne pri
skutočnom otváraní okna na Windows, a čo sa presne stane pri zavretí appky uprostred behu. Na obe som
odpovedal architektúrou (vlákno mimo appky + RAII zavretie), nie testom, ktorý tu fyzicky nejde spustiť —
presne preto je checklist nižšie dôležitý.

## Čo sa NEZMENILO

Žiadny zásah do: Check Prices (manuálne), ručné zadanie cien, uložené marketplace linky, história cien,
čítanie StubHub/Vivid Seats/Ticombo stránok (tie isté tri spôsoby čítania cien ako predtým), ukladanie
výsledku (`save_price_check` bez zmeny), Sales/Orders/Tickets/Inventory/Finance/Backup/CSV/Google Sheets.
Celých 859 pôvodných aj nových testov prešlo naraz, takže nič z toho, čo fungovalo, som nepokazil.

## Verzia

`2.1.1 → 2.1.2`, všetkých 9 zvyčajných miest (poučenie z minula: menil som VŠETKÝCH 9, nie len tie, čo appka
sama pri behu číta).

## STOP

Opravené, otestované v rámci toho, čo tento sandbox dovolí, zabalené. Toto ale nie je appka, ktorú si mohol
vidieť naozaj bežať — potrebujem, aby si na svojom počítači skúsil presne toto:

1. **Bežný Auto-check** — klikni na marketplace s uloženým linkom, sleduj či tlačidlo prejde
   Starting → Loading → Analyzing a či sa appka správne ukončí (úspech, alebo čistá správa prečo nie).
2. **Cancel uprostred behu** — klikni Auto-check a hneď potom Cancel. Appka by mala byť ihneď použiteľná
   (aj to konkrétne tlačidlo, aj zvyšok appky) — nie zaseknutá čo i len na chvíľu.
3. **Appka ostáva použiteľná POČAS behu** — kým Auto-check beží (predtým, než klikneš Cancel), skús
   kliknúť na inú stránku appky (napr. Dashboard) — malo by to fungovať úplne normálne.
4. **Zavretie appky počas behu** — spusti Auto-check a rovno zatvor appku. Otvor ju znova — mala by
   naštartovať úplne normálne, bez reštartu PC.
5. **Timeout** — ak máš po ruke link, čo sa dlho načítava/neodpovedá, skús naň Auto-check a over, že sa
   appka po ~15 sekundách sama vzdá s jasnou správou, nie že visí ďalej.
6. **Retry / Open page** — keď Auto-check niečo nenájde (napr. na StubHub linku), skús oba nové odkazy vo
   formulári ("Try Auto-check again" a "Open the page myself").
7. **Nič staré sa nepokazilo** — skús bežné Check Prices s ručným zadaním/paste, presne ako doteraz.

Ak čokoľvek z toho vyzerá inak, než by malo, napíš mi presne čo — najviac ma zaujíma bod 2 a 4, keďže to je
presne to, čo si nahlásil ako najhoršie.

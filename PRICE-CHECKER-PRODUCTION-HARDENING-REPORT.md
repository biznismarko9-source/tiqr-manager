# TIQR Manager 2.1.3 — Price Checker Auto-check: production hardening

Prešiel som si celý tvoj 26-bodový zadanie znova, bod po bode. Toto nebola reakcia na nový bug report — appku
tu nemám ako reálne spustiť (žiadna obrazovka, žiadny Windows), takže jediný spôsob, ako nájsť niečo, čo
stojí za opravu, bol znova prejsť KAŽDÚ funkciu v `price_checker_auto.rs`, `lib.rs`, `db.rs` a
`PriceChecker.tsx` s otázkou "čo sa tu ešte môže pokaziť" — nie len dôverovať tomu, že 2.1.2 report mal
pravdu. Dole je presne to, čo som našiel, čo som opravil, čo som overil a čo overiť neviem.

**Predtým, než čokoľvek iné**: jedna vec z tohto procesu stojí za to, aby si o nej vedel. Prvá verzia opravy
nižšie (frontend request ID mechanizmus) mala SAMA v sebe skutočný bug — našiel ho až nezávislý druhý pohľad
(samostatný review, bez prístupu k mojim vlastným úvahám), potom, čo som si sám myslel, že je hotovo. Opravil
som to, poslal späť na nezávislé overenie znova, a až potom pokračoval ďalej. Prečo to hovorím: presne toto
je dôvod, prečo dole nikde nenájdeš "A) VERIFIED READY" bez toho, aby to bolo skutočne overené — aj ja sám
som sa v tomto pri prvom pokuse pomýlil.

## 1. Root causes

Toto nebola oprava jednej príčiny zamŕzania — tá bola nájdená a opravená v 2.1.2 (synchrónne vytváranie okna
na hlavnom vlákne, potvrdené priamo z Tauri dokumentácie). 2.1.3 je hĺbkový audit VŠETKÉHO ostatného okolo
tejto istej funkcie. Reálne, predtým neriešené medzery, ktoré som našiel:

1. **Race medzi dvomi prekrývajúcimi sa requestami.** `auto_check_price` predtým nastavoval cancel-flag
   bezpodmienečne — druhé volanie (napr. rýchly dvojklik skôr, než React stihne prekresliť `disabled`
   tlačidlo) by ticho prepísalo prvého requestu vlastný flag. Prvý request by potom bežal ďalej BEZ možnosti
   ho zrušiť cez Cancel, a jeho neskorý výsledok by mohol znova otvoriť modálne okno, ktoré si už dávno
   zavrel/použil na niečo iné.
2. **Zlyhanie JS evaluácie sa hlásilo ako timeout.** `eval_and_wait` predtým vracal ten istý `None` aj pri
   skutočnom zlyhaní spustenia JS, aj pri obyčajnom vypršaní času — takže skutočná chyba sa vždy zobrazila
   ako "trvalo to príliš dlho", nie ako to, čo sa naozaj stalo.
3. **Žiadna validácia extrahovaných dát.** Žiadny strop na počet nájdených cien, žiadna kontrola tvaru meny —
   rozbitá/nepriateľská stránka mohla v princípe vrátiť čokoľvek a appka by to nahlásila ako bežný úspech.
4. **Frontend nemal vlastnú poistku.** Backend mal svoj tvrdý strop, ale frontend jeho promise-u
   bezpodmienečne dôveroval — žiadna nezávislá záchranná sieť, keby sa z nejakého nepredvídaného dôvodu
   nikdy nevrátil.

## 2. Architecture changes

- **Request ID naprieč celým tokom**: `PriceChecker.tsx` vytvorí ID pre každý pokus (`requestIdRef`), pošle
  ho do `auto_check_price`, backend ho posiela späť v KAŽDOM progress evente (`ProgressPayload { requestId,
  phase }`, predtým to bol iba holý string) aj v každom log riadku. Backend s touto hodnotou nič nerozhoduje
  — je to čisté echo pre frontend vlastné rozlíšenie "starý vs. nový request".
- **Single-flight guard v `auto_check_price`**: kontrola-a-nastavenie pod JEDNÝM zamknutím zámku — druhý
  pokus počas bežiaceho prvého sa okamžite odmietne (`status: "busy"`), nikdy sa nedotkne prvého flagu,
  nikdy neotvorí druhé okno.
- **Frontend re-entrancy guard** (`PriceChecker.tsx`, `autoCheckRef`) — synchrónna kontrola priamo na
  začiatku `startAutoCheck`, nezávislá od Reactovho prekresľovania. Toto je presne to miesto, kde prvá verzia
  mala bug — pozri "Cancellation"/"Tests" nižšie pre presný popis a opravu.
- **`EvalOutcome` enum** (bol `Option<String>`): `Ready`/`Cancelled`/`TimedOut`/`Failed` — teraz naozaj
  rozlíšiteľné hodnoty.
- **`MAX_PRICES` (500) a `sanitize_currency`**: validácia dát pred tým, než sa stanú `AutoCheckResult`.
- **`OPEN_WEBVIEW_COUNT`** a **`log_lifecycle`**: interné počítadlo otvorených okien + dev logy presne na
  miestach, ktoré si vymenoval (request started/webview created/page ready/analysis started/result
  received/cleanup started/cleanup completed).
- **`lib.rs` shutdown hook**: `RunEvent::ExitRequested` — best-effort, neblokujúci.
- **Frontend Level-2 watchdog**: 20s `setTimeout`, nezávislý od backendu.

Žiadna nová "feature" — všetko vyššie je o tom, ako sa TÁ ISTÁ funkcia správa v hraničných/pretekárskych
situáciách, presne ako si žiadal.

## 3. State machine

Explicitné stavy, žiadne implicitné "loading = true":

**Frontend (`AutoCheckPhase`, cez živé eventy z backendu):** `starting` → `loading` → `analyzing` →
`cleaning_up`. "Nič nebeží" = `autoCheck === null` na úrovni stránky (toto je tvoj IDLE).

**Backend terminálne stavy (`AutoCheckResult.status`):** `ok` (= tvoj SUCCESS) / `unable_to_read` a `error`
(= tvoj FAILED, rozlíšené podľa toho, či appka vôbec niečo prečítala) / `blocked` / `cancelled` / `timeout` /
**`busy`** (nové v 2.1.3 — request zamietnutý single-flight guardom, pozri vyššie).

Každý request má vlastné ID (bod 2 vyššie). Test `every_read_outcome_variant_maps_to_a_real_terminal_status_
never_a_loading_one` (v `price_checker_auto.rs`) strojovo overuje, že ŽIADNY z týchto šiestich `ReadOutcome`
variantov sa nedá zamieňať za `starting`/`loading`/`analyzing`/`cleaning_up` — presne tvoje "NO TERMINAL STATE
MAY LEAVE REQUEST IN LOADING".

## 4. Cancellation

Beží kedykoľvek — nezmenené od 2.1.2 (`cancel_auto_check_price` volá ten istý `cancel_google_sign_in_impl`,
kontrolovaný každých ~100ms aj počas JS evaluácie). Dvojité kliknutie na Cancel je bezpečné (druhé volanie na
prázdny/už-zrušený slot je no-op, testované). Cancel po úspešnom dokončení je no-op (slot je už `None`).

Nové v 2.1.3: keďže teraz môže existovať NAJVIAC jeden bežiaci request (single-flight guard), Cancel už nemá
ani teoretickú šancu trafiť "nesprávny" request — vždy existuje najviac jeden kandidát.

Frontend `cancelAutoCheck` zámerne NEČISTÍ `autoCheck` sám — čaká, kým to potvrdí samotný bežiaci request
(jeho `.finally()`), presne ako predtým. Toto je zámerné a nezmenené.

## 5. Timeout

Teraz skutočne DVE úrovne na backende plus JEDNA nezávislá na frontende:
- **Level 1** — `OVERALL_TIMEOUT` = 15s, budget-aware (každé čakanie v `poll_then_extract` sa zmenšuje podľa
  toho, koľko z rozpočtu ešte zostáva).
- **Level 2 (backend)** — `run_with_outer_deadline`: vonkajší `recv_timeout(OVERALL_TIMEOUT + OUTER_GRACE)`
  = 17s. Ak by sa z akéhokoľvek nepredvídaného dôvodu vnútorné čakania nevrátili sami, TOTO zaručuje, že sa
  command (a teda UI) vráti v ohraničenom čase — nič sa nezabíja na silu (bezpečný Rust to nevie), len sa
  naň prestane čakať; jeho vlastný `WebviewGuard` okno zavrie, keď sa vlákno raz aj tak dokončí.
- **Level 2 (frontend, NOVÉ v 2.1.3)** — nezávislý 20s `setTimeout` v `PriceChecker.tsx` (17s backend strop +
  3s rezerva na IPC/serializáciu). Vždy vráti tlačidlo do Idle, nech backend robí čokoľvek. **Úprimné
  obmedzenie**: `invoke()` sa nedá "zrušiť" z JS strany — toto resetuje UI, ale nevie zastaviť samotné
  bežiace volanie, keby z nejakého dôvodu naozaj ešte bežalo. To je zdokumentované v "Known limitations".

Po timeoute na ktorejkoľvek úrovni: UI sa okamžite odblokuje, tlačidlo ide späť do Idle, môžeš spustiť nový
check okamžite.

## 6. WebView lifecycle

Jediný vlastník: `webview` je lokálna premenná vo `run_browser_read`, `WebviewGuard` drží iba referenciu na
ňu a zatvára ju v `Drop` — Rustove vlastníctvo garantuje, že `Drop` beží presne raz na inštanciu, takže
dvojité zatvorenie ani zatvorenie po uvoľnenom objekte nie je štrukturálne možné (nie je to niečo, čo treba
"otestovať" — vyplýva to priamo z jazyka). Unikátne meno okna (nanosekundová časová pečiatka) zabraňuje
kolízii pri opakovaných behoch.

Rozhodnutie: **skryté okno ostáva skryté.** Tvoja špecifikácia to robí podmieneným ("ak je to technicky
bezpečnejšie") — nič v tomto audite nenašlo, že by skryté okno bolo menej bezpečné než viditeľné; skutočná
pôvodná chyba nikdy nebola o viditeľnosti, bola o tom, NA KTOROM VLÁKNE sa okno vytváralo (opravené v 2.1.2).
Prepnutie na viditeľné okno by pridalo ďalší reálny rozdiel na premyslenie za nulový bezpečnostný prínos.

`OPEN_WEBVIEW_COUNT` (nové): počítadlo otvorených reader okien, prírastok pri úspešnom vytvorení, úbytok v
`WebviewGuard::drop`. Najbližšie k "tvrdému meraniu" bodu 15 tvojej špecifikácie, čo tento sandbox (bez
obrazovky) dovoľuje — na tvojom počítači to môžeš sledovať v logoch a potvrdiť, že sa vždy vráti na 0.

## 7. Thread safety

Architektúra z 2.1.2 (celá práca s oknom na samostatnom `std::thread`, hlavné/command vlákno čaká iba na
ohraničený `recv_timeout`) ostáva nezmenená — stále presne to, čo Tauri dokumentácia odporúča, a stále
funguje rovnako. Čo je nové: pridaný `single-flight` zámok (`price_checker_auto_cancel_flag`, `Mutex`) —
skontroloval som, že sa nikdy nedrží súčasne so `state.db`/`state.db_path` zámkami (je to úplne nezávislý
slot, presne ako `oauth_cancel_flag`/`firebase_oauth_cancel_flag` — pozri `db.rs`, ktorého vlastný
lock-ordering komentár sa týka LEN `db`+`db_path` páru, nie tohto slotu), takže nemôže vzniknúť nová
príčina deadlocku medzi touto funkciou a zvyškom appky. Prešiel som všetky `spawn`/`channel`/`recv`/
`recv_timeout`/`Mutex` v tomto module — žiadny nový blokujúci wait nepribudol na hlavnom/command vlákne,
zámok sa drží len na krátky, nekonečne rýchly úsek (jedno porovnanie + jeden zápis).

## 8. Shutdown safety

Nový `RunEvent::ExitRequested` hook v `lib.rs` (API overené priamo z Tauri v2 dokumentácie cez WebFetch
predtým, než som čokoľvek napísal — nie z pamäti): pri zatváraní appky sa best-effort, NEBLOKUJÚCO nastaví
cancel flag pre Price Checker (ak práve niečo beží), aby prípadné bežiace vlákno zistilo zrušenie o čosi
skôr. Nikdy nevolá `prevent_exit()` — nemôže a nesmie predĺžiť zatváranie appky ani o milisekundu. Úprimne:
tento hook je len malé vylepšenie — samotný OS aj tak uvoľní každé okno/handle, ktoré proces vlastnil, hneď
pri jeho ukončení, takže reálny rozdiel je marginálny. Pridal som ho, lebo je lacný, bezpečný a tvoja
špecifikácia ho výslovne pripúšťa ("ak treba").

Samotná príčina zaseknutia appky pri zatváraní (poistka proti dvojitému spusteniu držaná zaseknutým
procesom) bola odstránená už v 2.1.2 tým, že appka sa už vôbec nemá ako zaseknúť na hlavnom vlákne — táto
časť sa touto verziou nemenila, len sa pridala táto jedna extra poistka navyše.

## 9. Resource cleanup

RAII vzor (`WebviewGuard`) nezmenený vo svojej podstate od 2.1.2, rozšírený o logovanie a počítadlo (bod 6).
Cancel-flag slot sa vždy vyčistí bezpodmienečne na konci `auto_check_price`, nech je výsledok akýkoľvek —
otestované na 10 opakovaných cykloch (`cancel_flag_slot_survives_ten_repeated_start_finish_cycles_without_
leaking_state`). Žiadny nový stav sa nehromadí na strane servera medzi requestami — `request_id` je len
prechodná hodnota, nič si ju neukladá.

## 10. Tests

Celkovo **870 testov, 0 zlyhaní** (859 pôvodných + 11 nových/rozšírených k tomuto hardeningu). Nové testy
pokrývajú presne to, čo si vymenoval:
- single-flight guard (odmietnutie druhého requestu, prvý flag nedotknutý),
- `EvalOutcome::Failed` ako odlišná hodnota od `TimedOut`/`Cancelled`,
- `sanitize_currency` (platné aj neplatné tvary),
- `MAX_PRICES` (presne na hranici aj o jeden nad ňou),
- `OPEN_WEBVIEW_COUNT`-štýl počítadla vrátane panic-scenára,
- `ProgressPayload` sa serializuje presne v tvare, aký frontend očakáva (camelCase),
- vyčerpávajúci test "žiadny `ReadOutcome` nikdy nezostane v loading-like stave",
- 10-násobný opakovaný cyklus cancel-flag slotu.

**Čo NIE je pokryté automatizovaným testom, a prečo (úprimne):** frontend logika (`PriceChecker.tsx`) sa
nedá automatizovane otestovať — v tomto projekte neexistuje žiadny JS/TS test runner (skontrolované:
`package.json` nemá `"test"` skript, `node_modules` neobsahuje vitest/jest). Opravu frontendovej race
podmienky (bod 2 vyššie) som overil ručným prechádzaním všetkých možných poradí vykonania KROK PO KROKU,
plus nezávislým druhým pohľadom (samostatná kontrola, bez mojich vlastných poznámok) — nie automatizovaným
testom. Toto je jediný spôsob, aký tento projekt momentálne má na overenie frontend logiky, a je to slabšie
než skutočný test.

## 11. Windows runtime test

**Nedá sa spustiť v tomto prostredí** — žiadna obrazovka, žiadny Windows, presne ako pri každej
predchádzajúcej verzii tejto appky. Toto je jediná vec, ktorá tento report NEMÔŽE tvrdiť, že overila.

Presne to, čo potrebujem, aby si vyskúšal na svojom počítači (nadväzuje na 2.1.2 checklist, plus nové body):

1. **Bežný Auto-check** — Starting → Loading → Analyzing → Cleaning up (nový stav, mal by sa mihnúť tesne
   pred výsledkom) → výsledok.
2. **Cancel uprostred behu** — appka aj tlačidlo hneď použiteľné, žiadne zaseknutie.
3. **Appka použiteľná POČAS behu** — skús Dashboard/iné stránky, kým Auto-check beží.
4. **Rýchly dvojklik na Auto-check** (nové, priamo testuje bod 1 z "Root causes") — druhý klik by mal buď
   nič nespraviť, alebo ukázať krátku správu "Another auto-check is already running" — v ŽIADNOM prípade by
   nemalo dôjsť k dvom otvoreným oknám naraz ani k zaseknutiu.
5. **Zavretie appky počas behu** — spusti Auto-check, rovno zatvor appku, otvor znova, over že naštartuje
   normálne. Opakuj aspoň 5×.
6. **Timeout** — link, čo sa dlho nenačíta, over že appka sa vzdá po ~15-17s s jasnou správou.
7. **Retry / Open page** — oba odkazy vo formulári po neúspešnom pokuse.
8. **Opakovanie 10×** — Check→Finish→Check znova; Check→Cancel→Check znova; Check→Timeout→Check znova, každé
   aspoň 10×. Nesmie pribúdať pamäť/okná/spomaľovanie.
9. **Konzola/logy** (ak appku spustíš cez `npm run tauri dev` alebo z terminálu) — mal by si vidieť riadky
   `[price-checker-auto] request N: ...` postupne pre každý krok, a `open reader windows now: 0` po každom
   dokončenom checku.
10. **Nič staré sa nepokazilo** — bežné Check Prices, ručné zadanie/paste, presne ako doteraz.

## 12. Real marketplace test

Skúsil som to znova, teraz, v tejto konkrétnej relácii (nie predpoklad z minulých session-ov) — priamy
pokus o pripojenie na stubhub.com, vividseats.com aj ticombo.com skončil na všetkých troch rovnako:
zablokované vlastnou sieťovou politikou tohto sandboxu (`connect_rejected`, port 443). Čiže: **žiadny reálny
test proti živej marketplace stránke sa v tomto prostredí spraviť nedal, ani teraz.** Netvrdím žiadny úspech
bez reálne nájdených cien — presne ako si žiadal.

## 13. Known limitations

- Nič, čo potrebuje skutočné WebView okno (vytvorenie okna, `eval_with_callback` zlyhanie/úspech, skutočné
  čítanie stránky) sa nedá otestovať v ŽIADNOM sandboxe doteraz videnom v tomto projekte.
- Frontend logika nemá automatizované testy (žiadny JS test runner v projekte) — overená len ručne + druhým
  pohľadom, nie strojovo.
- Frontend watchdog vie resetovať UI, ale nevie zastaviť skutočné bežiace `invoke()` volanie (JS to
  nedovoľuje) — ak by backend naozaj visel dlhšie než 17s (čo by znamenalo, že aj Level 2 backend poistka
  zlyhala), UI by sa síce spamätalo, ale teoreticky by mohol vzniknúť krátky moment, keď frontend už myslí,
  že nič nebeží, a backend v skutočnosti ešte dobieha.
- Shutdown hook je best-effort a marginálny (OS by uvoľnil zdroje aj bez neho) — nie plnohodnotný "graceful
  shutdown" mechanizmus.
- Teoretický, nízko-závažný okrajový prípad: ak by vnútorné vlákno niekedy presiahlo 17s vonkajší strop
  (jediný spôsob: `WebviewWindowBuilder::build()` samotný by sa zasekol — presne ten scenár, kvôli ktorému
  celý tento vláknový redizajn v 2.1.2 existuje), slot sa vyčistí a NOVÝ pokus by mohol v princípe bežať
  súbežne so stále živým starým vláknom. `OPEN_WEBVIEW_COUNT` by to correctne ukázal ako 2, nie je to chyba v
  počítaní — je to len hranica toho, čo dokáže zaručiť dizajn, ktorý nič nesmie nasilu zabíjať (bezpečný Rust
  to nevie).

## 14. Remaining risks

Najväčšie zostávajúce riziko je jednoduché: **nikdy som nevidel túto appku skutočne bežať.** Každý riadok
uzatvorenia/vlákien/RAII/timeoutov je postavený na dokumentácii a na tom, čo sa dá overiť staticky (kompilácia,
testy, typová kontrola) — nie na pozorovaní skutočného správania na Windows. Menšie zostávajúce riziká: okrajový
prípad z bodu 13 (súbežné okná len pri zaseknutom `build()`); frontend logika overená len ručne, nie testom;
`RunEvent`/`Manager::try_state` API overené z dokumentácie, nie behom appky.

---

## A) VERIFIED READY / B) NOT VERIFIED — RUNTIME TESTING REQUIRED

# B) NOT VERIFIED — RUNTIME TESTING REQUIRED

Kompilácia, 870/870 testov, clippy, tsc aj build sú čisté — ale to isté platilo aj pred 2.1.2, a appka aj
tak zamŕzala na tvojom počítači presne v tej časti, čo sa v tomto sandboxe nedá simulovať. Kým neprejdeš
checklist v bode 11 na svojom vlastnom počítači (obzvlášť body 4, 5 a 8 — presne tie, čo testujú nové veci z
tejto verzie), toto nemôžem označiť za "READY" a ani to neurobím.

## STOP

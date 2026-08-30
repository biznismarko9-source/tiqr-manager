# TIQR Manager 2.1.1 — Auto-check: opravené a overené

Prešiel som si `tiqrmanager2_1_1_1.zip` aj chybu, čo ti vyskočila v termináli (`STOPPED: this clone does
not actually have 2.1.0 everywhere`). Dobrá správa: samotná nová funkcia (Auto-check) je postavená správne
a robí presne to, čo má. Problém bol v jednom detaile pri publikovaní novej verzie — opravil som ho, k
tomu poriadne preveril úplne všetko okolo Auto-checku (aj to, čo sa v predchádzajúcom kroku overiť
nedalo), a nižšie je presne čo a prečo.

## Prečo to vyhodilo STOPPED

`release.ps1` (skript, čo appku publikuje na GitHub) má vlastnú, samostatnú poistku — predtým než čokoľvek
pošle, sám si overí, že `package.json`, `tauri.conf.json` a `Cargo.toml` naozaj hovoria tú verziu, akú
očakáva. Presne kvôli tomu, aby sa nikdy nestalo, že sa omylom publikuje appka poskladaná z rôznych verzií.

Tie tri súbory mali "2.1.1" správne — presne to vidno aj v tvojej hláške. Lenže samotný `release.ps1` mal
napevno napísané "čakám 2.1.0" — samostatný riadok, nezávislý od tých troch súborov, na ktorý sa
jednoducho zabudlo. Keďže sa to nezhodovalo, poistka to zastavila. Nebola to appka rozbitá — presne
naopak, poistka urobila presne to, na čo je určená. Len bolo treba posunúť aj ju na "2.1.1", čo som teraz
spravil.

Popri tomto som našiel a opravil aj dve ďalšie miesta, čo mali ostať na "2.1.0" bez povšimnutia —
`Cargo.lock` (to sa opravilo samo, keď som spustil testy) a text v `1-CLICK-UPDATE.bat` (len titulok okna,
nič funkčné, ale patrí to k sebe).

## Čo som ešte skontroloval v samotnom Auto-checku

Kód appky sa v predchádzajúcom kroku nedal ani skompilovať, ani otestovať — prostredie, kde vznikal,
nemalo dosť novú verziu Rustu. Moje prostredie to zvládne, tak som spustil naozaj všetko:

```
cargo check    -> celý kód sa skompiloval bez jedinej chyby
cargo test     -> 846 testov, 0 zlyhaní (836 pôvodných + 10 nových)
npx tsc -b     -> 0 chýb
npm run build  -> OK ("tiqr-manager@2.1.1")
```

Prešiel som si aj riadok po riadku nový kód, nielen report, čo k tomu prišiel — a našiel dve reálne miesta
na zlepšenie, obe rovno opravil:

1. **Auto-check odmietal linky bez "https://" na začiatku.** Väčšina prehliadačov "https://" v adresnom
   riadku vôbec nezobrazuje — keby si niekedy skopíroval alebo napísal link bez neho, appka by ho
   odmietla, aj keď presne ten istý link appka bežne ukladá a používa bez problémov všade inde. Teraz si
   ho appka sama doplní, ak chýba.

2. **Test "je stránka už pripravená na čítanie cien" bol príliš voľný.** Appka predtým považovala stránku
   za pripravenú hneď, ako našla čo i len jeden dolárový/eurový/librový znak KDEKOĽVEK na nej — pokojne aj
   v reklamnom texte hore, kde ešte nie sú žiadne reálne ceny. To mohlo appku zmiasť, aby sa pozrela na
   stránku príliš skoro, ešte predtým, než sa tabuľka s cenami vôbec stihla načítať — a skončila by
   zbytočne na "nepodarilo sa prečítať", hoci o pár sekúnd neskôr by to už fungovalo. Opravil som to tak,
   aby appka čakala na skutočnú tabuľku cien, nie na hocijaký peňažný znak.

   Toto druhé je dôležité hlavne pre Vivid Seats — práve tam sa pri prieskume potvrdilo, že appka reálne
   VIE nájsť ceny (majú tabuľku rovno na stránke) — táto oprava zvyšuje šancu, že to Auto-check aj naozaj
   nájde, namiesto zbytočného predčasného vzdania sa.

## Čo som overil, že je v poriadku, a radšej nemenil

Zvažoval som ešte jednu opravu — ako appka číta ceny z tabuľky, keď je desatinná čiarka namiesto bodky
(európsky formát typu "1.234,56"). Skúsil som to opraviť, ale keď som to poriadne otestoval, moja
"chytrejšia" verzia by naopak POKAZILA bežnejší prípad — cenu ako "$1,234" (celé číslo bez desatín) by
omylom prečítala ako "1.234". Keďže jediný marketplace, kde appka reálne číta z tabuľky (Vivid Seats),
používa americký formát, pôvodný kód je presne taký, aký má byť — nechal som ho bez zmeny. Radšej dôkladne
otestujem, než aby som opravoval niečo, čo pokazené nebolo.

## Čo sa NEMENILO

Žiadna zmena `price_checker.rs` (pôvodná manuálna logika), žiadna nová migrácia, Sales/Orders/Tickets/
Inventory/Finance/Backup/CSV/Google Sheets sync bez zmeny. Celý strom súborov som porovnal so svojou
vlastnou overenou 2.1.0 kópiou — presne 9 súborov sa zmenilo kvôli Auto-checku (3 nové + 6 upravených) a 7
kvôli číslu verzie. Nič iné.

## Ako to vyzerá v appke

Vedľa "Check Prices" pribudlo tlačidlo "Auto-check". Appka otvorí uložený (alebo práve zadaný) link na
pozadí, počká pár sekúnd, prečíta ceny a predvyplní ten istý formulár, aký poznáš z "Check Prices" — nič sa
neuloží samo, vždy si to najprv skontroluješ a klikneš Save. Ak appka narazí na ochranu proti robotom
alebo nič nenájde, formulár sa otvorí prázdny — presne ako doteraz, nič sa nezasekne.

## STOP

Opravené, otestované (aj to, čo sa predtým otestovať nedalo), zabalené. Skontroluj:

1. Znova stiahni zip, rozbaľ do NOVÉHO prázdneho priečinka a spusti `release.ps1` (alebo
   `1-CLICK-UPDATE.bat`) — tentokrát by mal prejsť bez STOPPED.
2. V appke skús Auto-check na evente, kde máš uložený Vivid Seats link — over, či sa formulár naplní
   cenami.
3. Skús aj event so StubHub linkom — over, že dostaneš čistú správu "nepodarilo sa prečítať" a prázdny
   formulár, nič rozbité.
4. Skús zadať marketplace link BEZ "https://" na začiatku (napr. rovno "vividseats.com/...") a klikni
   Auto-check — malo by to fungovať rovnako, appka si to doplní sama.

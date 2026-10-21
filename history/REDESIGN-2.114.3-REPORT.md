# TIQR Manager 2.114.3 — mail na každom počítači dobehne sám

Tvoja správa:

- *„neviem, čo je s mailom, ale na Macu mám 100 000 správ uložených a na
  Windowse ukazuje len 16k, ako keby fungovali zvlášť"*

---

## Prečo to tak je

Naozaj fungujú zvlášť, a je to tak zámerne:

- **Každý počítač číta Gmail sám** a ukladá si vlastnú kópiu mailov.
- **Maily sa nikdy nekopírujú cez cloud** (Firebase ani Google Drive). Cez
  živú synchronizáciu idú len nastavenia: schránka, pravidlá, stíšení
  odosielatelia, priečinky, webhooky. Je to kvôli súkromiu, aby obsah
  mailov neskončil na cudzom serveri.
- Počítač teda dobieha **len vtedy, keď na ňom TIQR beží**, a jeden môže
  mať oveľa viac ako druhý.

Pri hľadaní, prečo je Windows tak pozadu, sa našli tri veci, ktoré druhý
počítač brzdili alebo vracali späť. Všetky tri sú opravené.

## 1. Čítanie už nezávisí od okna

**Predtým:** čítanie poháňal časovač v okne TIQR. Keď je okno
minimalizované alebo za iným oknom, webview (na Windowse WebView2) časovače
spomalí, a s nimi aj čítanie mailov.

**Teraz:** číta samotný TIQR na vlastnom vlákne v pozadí, stále rovnakou
rýchlosťou, aj keď je okno minimalizované. Okno ho len spustí, posúva mu
tlačidlo Sync a ukazuje priebeh. Pravidlá sú rovnaké ako doteraz:

- kým je čo čítať, ide beh za behom;
- keď Google povie „spomaľ", počká, koľko treba;
- keď je všetko prečítané, pozrie nové maily podľa tvojho intervalu;
- interval 0 znamená „len keď stlačím Sync";
- po odhlásení z TIQR sa nový beh nezačne, kým sa znova neprihlásiš.

## 2. Spojenie s Google Drive už nevracia čítanie na začiatok

**Predtým:** keď sa počítač spojil s kópiou z Google Drive a druhý počítač
medzitým zmenil hocijaké nastavenie mailu (stíšený odosielateľ, priečinok,
pravidlo, jazyk prekladu), začalo sa čítanie na tomto počítači odznova:

- všetky priečinky od najnovšieho mailu;
- fronta nájdených a ešte nestiahnutých mailov sa vyprázdnila.

Uložené maily ostali, ale veľa práce sa robilo znova.

**Teraz:** čítanie sa vráti na začiatok len pri **inej schránke** (iná
Gmail adresa). Rovnako to už robila živá synchronizácia. Pri inej schránke
sa zároveň zruší prihlásenie k starej, aby sa nečítala pod novým menom.

## 3. Priečinok, ktorý Gmail raz odmietol, sa skúsi znova

**Predtým:** keď Gmail raz odmietol vypísať priečinok (napríklad pri chvíľkovej
chybe), TIQR ho odložil a už ho nikdy nečítal, ako keby bol prečítaný.

**Teraz:** odložený priečinok sa po **6 hodinách** prečíta znova, od
najnovšieho mailu.

## 4. Vidno, koľko počítaču chýba

Na obrazovke Mail (hore pod názvom) aj v Mail → Settings → Mailbox je teraz:

> **16,234 on this computer · Gmail has 104,512 in all**

- Číslo za „Gmail has" hlási Gmail sám, TIQR sa naň pýta každých 10 minút.
- Gmail ráta aj odoslanú poštu, ktorú TIQR nečíta, takže sa čísla na konci
  nemusia stretnúť úplne presne.
- V nastaveniach je aj veta, že každý počítač číta Gmail sám a maily sa
  nikdy nekopírujú cez cloud.

## Čo urobiť na Windowse

1. Vydaj túto verziu (zip, 1-CLICK-UPDATE.bat) a nainštaluj aktualizáciu aj
   na Windowse.
2. Nechaj TIQR na Windowse zapnuté, pokojne minimalizované. Gmail dovolí
   zhruba 20 až 50 mailov za sekundu, takže chýbajúcich asi 84 000 by malo
   dobehnúť zhruba za hodinu až dve, podľa toho, ako rýchlo Gmail odpovedá.
3. V Mail → Settings → Mailbox sleduj, ako číslo „on this computer" rastie.
   Ak nerastie alebo je tam chyba, pošli mi screenshot tej časti.

## Testy

- `cargo test --locked`: 1584 prešlo, 0 zlyhalo (4 ignorované, ako doteraz).
- Nové Rust testy:
  - čítanie drží tempo, aké malo okno (beh za behom, pauza od Google,
    interval, interval 0, pozastavené, prvý beh po štarte);
  - Sync preruší čakanie a nestratí sa ani vtedy, keď príde počas behu;
  - spojenie s Drive s novším nastavením zachová, kam čítanie došlo, a iná
    schránka ho vráti na začiatok aj s odhlásením starej;
  - odmietnutý priečinok sa do hodiny neskúša, po 6 hodinách áno;
  - počet z Gmailu sa pýta každých 10 minút a nikdy neposunie bod, odkiaľ
    sa sledujú zmeny v Gmaili.
- `npm test`: 41 prešlo (nový test pre text „on this computer · Gmail has").
- `npm run build` prešiel.

## Čo sa tu overiť nedalo

Skutočný Gmail ani WebView2 na skutočnom Windowse tu nie sú. Engine je
overený proti napodobenému Gmail serveru v testoch. To, ako rýchlo bude
Windows dobiehať, ukáže až číslo v nastaveniach po aktualizácii.

## Súbory

- `src-tauri/src/commands/mail_sync.rs` (čitateľ na vlákne, opakovanie
  priečinka, počet z Gmailu), `mail.rs` (príkazy), `shared_settings.rs`
  (Drive spojenie), `mail_store.rs` (krok 5), `models.rs`, `lib.rs`
- `src/components/Layout.tsx`, `src/components/MailSettingsPane.tsx`,
  `src/pages/MailIntelligence.tsx`, `src/lib/mailSync.ts`,
  `src/lib/mailSyncStatus.ts` (s testom), `src/lib/api.ts`, `src/lib/types.ts`
- `CHANGELOG.md`, `PROJECT_STATE/*`, verzia v piatich súboroch.

Mailová databáza krok 5, žiadna číslovaná migrácia (ďalšia je 047).

# TIQR Manager - Report k verzii 2.5.2

Tvoja vlastná požiadavka hneď po 2.5.1: funkčné "zabudnuté heslo" a
prihlásenie cez Discord. Žiadna zmena v databáze/migráciách. Jedno z dvoch
sa nakoniec nepostavilo - nižšie presne prečo, aj čo namiesto toho vzniklo.

---

## 1. Zabudnuté heslo - ako to funguje

Na Welcome obrazovke, pod heslom pri Log in, je teraz link **"Forgot
password?"**. Zadáš email, appka pošle mail cez Firebase. V maile je odkaz -
klik naň **otvorí priamo TIQR Manager** (nie prehliadač), appka si sama
overí, že odkaz je platný, a ukáže "Set a new password for [tvoj email]".
Zadáš nové heslo, appka ho nastaví, a vrátiš sa prihlásiť.

Celé je to postavené na Firebase vlastných funkciách
(`sendPasswordResetEmail`/`verifyPasswordResetCode`/`confirmPasswordReset`)
- rovnaký princíp ako doterajšie prihlásenie emailom/heslom a Google. Žiadny
nový server, žiadny Cloud Function, žiadny platený Firebase plán.

**Ako sa mail dostane späť do appky** (namiesto do prehliadača): appka pošle
Firebase príkaz s nastavením `handleCodeInApp: true` a vlastnou adresou -
vďaka tomu Firebase do mailu vloží odkaz na jednu novú statickú stránku
(`docs/reset-redirect.html`), ktorú appka pridala na **ten istý GitHub
Pages web, kde už dnes beží tvoja privacy policy stránka**
(`docs/privacy.html`) - žiadny nový hosting, len jeden súbor navyše na
existujúcom webe. Táto stránka nič nerobí okrem toho, že hneď presmeruje na
novú vlastnú adresu appky `tiqrmanager://...` - tú appka zaregistruje vo
Windows pri inštalácii (nová závislosť `tauri-plugin-deep-link`). Keď
appka tento odkaz dostane (či už bola vypnutá, alebo bežala na pozadí),
otvorí novú obrazovku `ResetPassword.tsx`, kde sa celý zvyšok odohrá.

## 2. Prečo nie krátky kód, a prečo (zatiaľ) nie Discord

Pýtal si sa na krátky číselný kód na odpísanie do appky, aj na Discord
prihlásenie vedľa Google. Kým som sa pozeral, ako presne oboje postaviť,
zistil som, že **obe vedú do toho istého bodu**:

- Firebase vie poslať iba mail s odkazom (dlhý, neopísateľný kód) - nikdy
  nie vlastný krátky kód. Poslať vlastný mail s vlastným krátkym kódom, a
  potom niekomu bez prihlásenia zmeniť heslo, dokáže urobiť iba dôveryhodný
  server (Firebase Admin SDK) - appka sama od seba to spraviť nemôže.
- Google prihlásenie funguje jednoducho, lebo Firebase ho podporuje priamo.
  Discord medzi podporovanými spôsobmi nie je - aj tu by bolo treba ten istý
  server, ktorý by Discord prihlásenie premostil do Firebase.
- Taký server (Cloud Function) by bol pre tento projekt úplne nový kus
  infraštruktúry (doteraz žiadny nie je) a vyžadoval by prechod na platený
  Firebase plán "Blaze" - reálne použitie by ostalo v bezplatnom limite, ale
  je potrebná platobná karta na projekte.

Namiesto toho, aby som jedno z toho potichu postavil (a prekvapil ťa
nečakaným nákladom) alebo potichu vynechal (a nepovedal prečo), pýtal som sa
priamo - vybral si si odkaz-do-appky namiesto krátkeho kódu (hotovo, popísané
vyššie) a Discord zatiaľ vynechať úplne (nič som preň nepísal - ostáva
k dispozícii kedykoľvek budeš chcieť ten Blaze plán zvážiť).

## 3. Čo musíš urobiť ty, aby to naozaj fungovalo

Jeden manuálny krok, rovnaký princíp ako pri Firestore pravidlách -
nedá sa nastaviť z kódu, treba to raz kliknúť v Firebase Console:

**Firebase Console -> Authentication -> Settings -> Authorized domains ->
Add domain -> `biznismarko9-source.github.io`**

Bez tohto Firebase odmietne poslať mail s odkazom na túto adresu.

Druhá vec - skôr na overenie než na spravenie: GitHub Pages pre `docs/`
priečinok si pravdepodobne už zapol dávnejšie (bolo to treba pre Google
Sheets prihlásenie, report 2.0.5) - over si, že
`https://biznismarko9-source.github.io/tiqr-manager/reset-redirect.html`
naozaj načíta stránku. Ak nie, GitHub -> repozitár -> Settings -> Pages ->
Source: **Deploy from a branch** -> Branch: **main**, priečinok **/docs** ->
Save.

## Rozhodnutia, ktoré som urobil sám

- Názov vlastnej adresy appky - `tiqrmanager://` - jednoduché, krátke, nič
  iné to nepoužíva.
- `docs/reset-redirect.html` je zámerne "hlúpa" stránka - len prepošle, čo
  dostane, appke. Neplatí len pre reset hesla - ak niekedy pribudne napr.
  overenie emailu, môže ísť cez tú istú stránku bez zmeny.
- Nepostavil som novú Firebase Hosting infraštruktúru (aj keď by to tiež
  fungovalo) - tento projekt nemá a nikdy nemal pripojený Firebase CLI
  (rovnaká poznámka je aj pri `firestore.rules`), tak som sa držal toho istého
  princípu a použil web, čo už existuje.

## Overenie

`npx tsc -b`, `npm run build` a `cargo check --lib` čisté. `cargo test --lib`
zelené - 1052 passed, 0 failed, 3 ignored (rovnaký počet ako predtým, keďže
pribudlo len zapojenie pluginu, nie nová logika na testovanie).

## Balík

`tiqr-manager-2.5.2.zip` - verzia zjednotená vo všetkých 5 miestach
(`package.json`, `tauri.conf.json`, `Cargo.toml`, `release.ps1`,
`1-CLICK-UPDATE.bat`) + oboch lockfile-och, integrita zip súboru overená.
Starý, už doručený zip (2.5.1) som zo staging priečinka vymazal kvôli
miestu na disku.

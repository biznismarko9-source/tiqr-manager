# TIQR Manager 2.0.5 — Sign in with Google (skutočné prihlásenie za seba)

Report k verzii **2.0.5**. Nadväzuje na otázku, ktorú si položil po 2.0.4 ("ako sa pripoja iní ľudia?"), na moju odpoveď (jedna zdieľaná identita service accountu - v poriadku pre malú skupinu, ale nie je to skutočná izolácia po osobách), a na tvoje jasné rozhodnutie potom: chceš reálne Google prihlasovacie okno, kde si každý napíše svoj vlastný e-mail. Predtým som ťa upozornil, že to má reálne požiadavky od Googlu (vlastná doména, video, schvaľovací proces) - vybral si si napriek tomu **"Plné OAuth s Google schválením"**. Toto je tá verzia. 2.0.2-2.0.4 (pripojenie cez URL, Sync now, Create a new sheet for me - všetko cez service account) zostávajú presne tak, ako boli, nič sa im nemení.

## 1. Ako to funguje teraz

Settings → Integrations má odteraz navrchu novú kartu **"Sign in with Google"**, nad existujúcou kartou pre Pulls. Klikneš na "Sign in with Google" a:

1. Appke sa v tvojom bežnom prehliadači (Chrome, Edge, čokoľvek máš predvolené) otvorí skutočná Google prihlasovacia stránka - presne tá istá, akú poznáš z prihlasovania do Gmailu, YouTube a podobne. TIQR Manager toto okno nijako neobalí ani nezobrazuje sám - je to naozaj Google, na `accounts.google.com`.
2. Vyberieš si (alebo napíšeš) Google účet, prihlásiš sa, Google ti ukáže presne to, o čo appka žiada ("TIQR Manager chce mať prístup k: Google Sheets, tvoja e-mailová adresa") a ty to potvrdíš.
3. Prehliadač ťa presmeruje späť na appku (appka na pozadí na chvíľu počúva na tvojom počítači presne na jednom voľnom porte, len kým sa toto presmerovanie nestane - nič nie je vystavené von, nič nepočúva dlhšie než je treba).
4. Appka si uloží prihlásenie a v karte ukáže "Signed in as tvoj@email.com" s tlačidlom "Sign out".

Odvtedy appka pre **všetko**, čo súvisí s Google Sheets - test pripojenia, Sync now, aj "Create a new sheet for me" pri Pulls - použije **teba**, nie zdieľaný service account. Konkrétne pri "Create a new sheet for me": keď si prihlásený, appka novú tabuľku rovno založí priamo v tvojom vlastnom Google Drive - už ju netreba zvlášť zdieľať (je tvoja od prvej sekundy), takže pole na e-mail v tom formulári zmizne a text sa zmení, aby to bolo jasné.

Ak sa neprihlásiš vôbec, nič sa nemení oproti 2.0.4 - appka ticho použije zdieľaný service account presne ako doteraz. Sign-in je nadstavba vedľa, nie náhrada.

Tlačidlo "Sign out" v karte okamžite appku odhlási - zabudne uložené prihlásenie aj e-mail, a všetko sa vráti na zdieľaný service account (ak je nastavený) alebo na "not connected" (ak nie je).

## 2. Prečo "hocikto, hocijaký e-mail, funguje hneď" ešte úplne neplatí

Toto je najdôležitejšia časť tohto reportu - presne to, na čo som ťa upozorňoval predtým, než si sa rozhodol.

Google net rozdeľuje appky na dva stavy: **Testing** (appka funguje hneď, ale len pre vopred ručne pridané e-maily, max 100) a **schválená appka** (funguje pre kohokoľvek, natrvalo). Táto appka teraz **funguje a je hotová** - je v stave Testing, kým neprejde Googlovým schválením. V praxi to znamená:

- Musíš v Google Cloud Console ručne pridať e-mailovú adresu každého, kto sa má vedieť prihlásiť (vrátane seba) - do zoznamu "Test users". Max 100 adries.
- Každé takéto prihlásenie **vyprší po 7 dňoch** - po týždni sa bude treba znova prihlásiť (appka to jasne ukáže, nič sa nepokazí, len treba kliknúť "Sign in with Google" znova).
- Kým appka neprejde schválením, nikto mimo tohto zoznamu sa prihlásiť nemôže - dostane od Googlu chybu, že appka nie je overená.

Toto nie je chyba ani polovičná práca - je to presne to, čo Google od každej appky vyžaduje, kým prejde ich schvaľovacím procesom (dôvod: appka žiada prístup k obsahu Google Sheets, čo Google označuje ako "citlivé" oprávnenie). Cesta k plnému stavu - hocikto, hocijaký e-mail, natrvalo - je v sekcii 4 nižšie.

## 3. Čo musíš nastaviť (tri kroky, prvý krát)

Bez týchto krokov appka po vybuildení ukáže kartu "Sign in with Google" ako nedostupnú ("not available in this build").

**Krok 1 - Vytvoriť OAuth Client ID v Google Cloud Console** (v tom istom projekte, kde už máš service account z 2.0.2):

1. Otvor [Google Cloud Console](https://console.cloud.google.com/) a hore prepni na ten istý projekt, v ktorom si robil service account.
2. V ľavom menu: **APIs & Services → OAuth consent screen**. Ak si to tu ešte nevypĺňal: User type **External**, vyplň názov appky (napr. "TIQR Manager"), svoj e-mail ako support e-mail aj ako developer contact. Uloží sa v stave "Testing" - presne to teraz chceš.
3. V tej istej sekcii, niže, pridaj seba (a kohokoľvek ďalšieho, kto to má hneď skúšať) do **Test users**.
4. V ľavom menu: **APIs & Services → Credentials → + CREATE CREDENTIALS → OAuth client ID**.
5. Application type: **Desktop app** (dôležité - nie "Web application"; Desktop app je presne ten typ, ktorý Google podporuje pre appky ako táto, bez toho, aby som musel vopred nahlásiť presný port).
6. Meno napíš čokoľvek rozpoznateľné, napr. "TIQR Manager Desktop".
7. Klikni Create. Google ti ukáže **Client ID** a **Client Secret** - obe skopíruj a pošli mi ich (v ďalšej správe v appke, nie inde). Client Secret pri type "Desktop app" Google sám nepovažuje za tajný v tom zmysle ako heslo, ale aj tak ho posielaš len mne a ja ho dám len do GitHub secrets, nikdy do repozitára.

**Krok 2 - Dva nové GitHub secrets** (rovnaké miesto ako `GOOGLE_SERVICE_ACCOUNT_JSON`): GitHub → repozitár `tiqr-manager` → Settings → Secrets and variables → Actions → New repository secret, dvakrát:

| Meno | Hodnota |
|---|---|
| `GOOGLE_OAUTH_CLIENT_ID` | Client ID z kroku 1 |
| `GOOGLE_OAUTH_CLIENT_SECRET` | Client Secret z kroku 1 |

**Pripomienka k tomu, čo už malo byť nastavené:** naposledy si potvrdil, že `GOOGLE_SERVICE_ACCOUNT_JSON` ešte nebol pridaný. Tieto tri secrety sú od seba **nezávislé** - každý zapína inú vetvu appky:

- `GOOGLE_SERVICE_ACCOUNT_JSON` chýba → zdieľaný service account (URL pripojenie, Sync now, Create a new sheet bez prihlásenia) nefunguje, presne ako doteraz.
- `GOOGLE_OAUTH_CLIENT_ID`/`GOOGLE_OAUTH_CLIENT_SECRET` chýbajú → karta "Sign in with Google" ukáže "not available in this build".

Pre appku, ktorá vie úplne všetko, treba mať pridané všetky tri.

**Krok 3 - GitHub Pages pre stránku o ochrane súkromia** (potrebné až pre plné schválenie zo sekcie 4, appka sama o sebe funguje aj bez tohto v Testing režime): GitHub → repozitár → Settings → Pages → Source: **Deploy from a branch** → Branch: **main**, priečinok **/docs** → Save. O minútu-dve bude stránka živá na `https://biznismarko9-source.github.io/tiqr-manager/privacy.html` - text tejto stránky (`docs/privacy.html`) je súčasťou tohto zipu, hotový, nič na ňom netreba meniť, iba zapnúť Pages.

Po pridaní oboch nových secrets spusti `1-CLICK-UPDATE.bat` (teraz publikuje v2.0.5) presne ako doteraz.

## 4. Cesta k plnému stavu ("hocikto, hocijaký e-mail, natrvalo")

Toto je mimo toho, čo viem urobiť ja - časť z toho vyžaduje teba osobne a časť je čisto na Googli:

1. **Najprv over si sám** - pridaj sa do Test users (krok 1 vyššie), vybuilduj 2.0.5, klikni "Sign in with Google", over že sa naozaj prihlásiš a že Create a new sheet for me / Sync now fungujú aj takto. Toto je dôležité urobiť skôr, než pôjdeš ďalej.
2. **Nahraj krátke ukážkové video** - Google pri schvaľovaní vyžaduje neverejné (unlisted) YouTube video, kde je vidieť skutočný človek klikajúci cez skutočnú prihlasovaciu obrazovku appky od začiatku po koniec. Toto musí byť skutočné nahrávanie - nedá sa to vyrobiť tu v sandboxe (nemám tu appku s reálnym Google prihlásením, ktoré by som mohol nahrať).
3. **Over vlastníctvo domény** cez Google Search Console (bezplatné) pre `biznismarko9-source.github.io` - potvrdzuje, že stránka o ochrane súkromia z kroku 3 vyššie je naozaj tvoja.
4. **Odošli appku na schválenie** - v OAuth consent screen v Cloud Console je tlačidlo "Submit for verification", kde priložíš odkaz na video a vysvetlíš, prečo appka žiada prístup k Sheets.
5. **Čakanie na Google** - sami udávajú typicky 3-5 pracovných dní, v praxi to ale môže trvať dlhšie, hlavne ak sa opýtajú na doplňujúce otázky (bežné pri appkách žiadajúcich Sheets prístup). Toto je úplne mimo mojej aj tvojej kontroly - len treba počkať a prípadne odpovedať, ak sa Google ozve.

Až po schválení odpadne aj limit 100 e-mailov aj to 7-dňové vypršanie - dovtedy appka funguje presne podľa sekcie 2 vyššie, čo je úplne bežný a legitímny medzistav (väčšina appiek ním prechádza).

## 5. Čo sa v tomto sandboxe nedalo overiť naživo

Rovnaké obmedzenie ako v 2.0.2-2.0.4 - `googleapis.com` aj `accounts.google.com` sú v tomto sandboxe nedostupné, takže samotný beh prihlásenia (otvorenie prehliadača, výmena kódu za token, obnovenie tokenu) som nemohol reálne spustiť od začiatku do konca. Rozdelil som to rovnako ako v predošlých verziách - všetko, čo sa dá overiť bez skutočného Googlu, je plne otestované:

- Generovanie PKCE (bezpečnostný mechanizmus prihlásenia) a jeho matematické vlastnosti - 5 testov.
- Zostavenie prihlasovacej URL adresy so všetkými správnymi parametrami - 1 test.
- Spracovanie odpovede, ktorú appke pošle prehliadač po prihlásení (úspech aj zamietnutie, aj úplne nezmyselný vstup, aby appka nikdy nespadla) - 4 testy.
- Samotné "počúvanie" appky na tvojom počítači počas prihlasovania - toto som overil naozajstným sieťovým testom (appka si otvorí skutočný port, ja sa naň v teste naozaj pripojím a pošlem dáta, presne ako by to spravil prehliadač) - 2 testy, vrátane toho, že appka po 5 minútach nečinnosti čisto skončí namiesto večného čakania.
- Ukladanie/mazanie prihláseného e-mailu a tokenu, a rozhodovanie "kto má prednosť - OAuth alebo service account" vo všetkých kombináciách (nikto prihlásený / prihlásený, ale appka bez nastaveného OAuth / prihlásený a funguje) - 7 testov.

Jediné, čo naozaj overiť nejde bez skutočného Googlu, je samotná výmena kódu za token na `oauth2.googleapis.com` a načítanie e-mailu z `googleapis.com` - tie som len starostlivo skontroloval podľa oficiálnej Google dokumentácie (presne ten istý tvar požiadaviek, aký popisujú). Prvé skutočné prihlásenie preto vyskúšaj podľa kroku 1 v sekcii 4 a daj vedieť, ako to dopadlo.

## 6. Testy

```
cargo test --lib -> 271 passed; 0 failed; 3 ignored
```

271 = 251 (z 2.0.4) + 20 nových: 13 v `google_oauth::tests` (PKCE, prihlasovacia URL, spracovanie presmerovania, sieťové počúvanie) + 7 v `commands::google_auth::tests` (stav prihlásenia, odhlásenie, rozhodovanie OAuth vs. service account).

## 7. Build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 271 passed, 0 failed
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.5 build" v hlavičke)
```

## 8. Zmenené/nové súbory a verzia

**Backend nové:** `google_oauth.rs` (PKCE, prihlasovacia URL, sieťové "počúvanie" na presmerovanie, výmena/obnova tokenu), `commands/google_auth.rs` (3 nové commandy, uloženie prihláseného účtu, `resolve_google_credential` - jedno miesto, kde si každý Sheets-command pýta token a dostane buď teba, alebo service account)
**Backend upravené:** `google_sheets.rs` (jedna funkcia sprístupnená aj pre `google_oauth.rs`), `commands/sheets_sync.rs` (test pripojenia teraz cez `resolve_google_credential`), `commands/pulls_sheet_sync.rs` (Sync now aj Create a new sheet teraz cez `resolve_google_credential`, zdieľanie sa preskočí, ak si prihlásený), `commands/mod.rs`, `lib.rs` (nový modul, 3 nové commandy, nový plugin `tauri-plugin-opener`), `build.rs` (zapeká `GOOGLE_OAUTH_CLIENT_ID`/`GOOGLE_OAUTH_CLIENT_SECRET` do buildu rovnako ako doteraz service account), `Cargo.toml`/`Cargo.lock` (nové závislosti: `tauri-plugin-opener`, `rand`, `sha2`), `capabilities/default.json` (nové oprávnenie na otvorenie prehliadača)
**Backend nový súbor mimo kódu:** `docs/privacy.html` (stránka o ochrane súkromia pre GitHub Pages, sekcia 3, krok 3)
**Frontend upravené:** `lib/types.ts` (`GoogleSignInStatus`), `lib/api.ts` (3 nové volania), `pages/Settings.tsx` (nová karta "Sign in with Google", karta pre Pulls teraz vie, či si prihlásený, a mení sa podľa toho), `package.json`/`package-lock.json` (nová závislosť `@tauri-apps/plugin-opener`)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.5`, `package-lock.json` sa zosynchronizoval cez `npm install`.

## STOP

2.0.5 hotové a overené (271/271 backend testov, čisté `tsc`/`build`). "Sign in with Google" je funkčné vedľa service accountu - obe cesty vedú k pripojeniu, appka si sama vyberie, ktorú použiť, podľa toho, či si prihlásený.

**Než to skúsiš naostro, potrebuješ (sekcia 3):** (1) pridať `GOOGLE_SERVICE_ACCOUNT_JSON`, ak ešte chýba - naposledy si potvrdil, že áno, (2) vytvoriť nový OAuth Client ID (typ "Desktop app") v Cloud Console a poslať mi Client ID + Client Secret, aby som ich mohol dať do GitHub secrets, (3) zapnúť GitHub Pages pre `/docs` (jedno kliknutie, súbor už je hotový v zipe). Potom `1-CLICK-UPDATE.bat`, počkaj na zelený build, a vyskúšaj "Sign in with Google" - najprv seba pridaj do Test users v Cloud Console (sekcia 3, krok 1, bod 3), inak ťa appka prihlásiť nedá.

**Reálne očakávanie:** hneď po tomto všetkom appka funguje presne pre teba a kohokoľvek, koho ručne pridáš do Test users (max 100, prihlásenie im vyprší po 7 dňoch). Plné "hocikto, hocijaký e-mail, natrvalo" príde až po Googlovom schválení (sekcia 4) - tam ťa budem potrebovať pre video a pár klikov, ktoré musia byť od teba, a potom čakáme na Google (ich odhad 3-5 pracovných dní, môže byť aj viac). Nezačínam nič ďalšie, kým mi nedáš vedieť Client ID/Secret a kým nepotvrdíš, ako dopadlo prvé skutočné prihlásenie.

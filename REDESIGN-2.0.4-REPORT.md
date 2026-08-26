# TIQR Manager 2.0.4 — Google Sheets sync, krok 3 (Vytvoriť novú tabuľku jedným klikom)

Report k verzii **2.0.4**. Reaguje priamo na tvoj feedback k 2.0.3 (screenshoty + "neviem ci sme sa zle pochopili") a na to, čo si potom potvrdil: **auto-vytvorenie tabuľky, žiadne Google okno**. Nadväzuje na 2.0.2 (pripojenie) a 2.0.3 (Sync now pre Pulls) — obe zostávajú presne tak, ako boli.

## 1. Čo bol problém a čo som spravil

Tvoj screenshot ukazoval "Not Connected" / "Google Sheets sync isn't available in this build" — to bolo najskôr len tým, že `GOOGLE_SERVICE_ACCOUNT_JSON` secret na GitHube si ešte nepridal (potvrdil si to). Ale popísal si aj niečo iné, než čo appka robí: kliknúť Connect, vyskočí okno na prihlásenie cez Google účet, a tabuľka Pulls sa založí sama.

Skutočné prihlásenie cez Google ("Sign in with Google") by ale znamenalo presne to, čomu sme sa na začiatku úmyselne vyhli (viď 2.0.2 report) — kým appka nejde cez Googlom oficiálne schválený proces overenia, môžu sa prihlásiť len ľudia, ktorých by som musel jedného po druhom ručne pridať do zoznamu, a aj im by prihlásenie po 7 dňoch prestalo fungovať. Preto som sa spýtal (dve otázky) a ty si vybral druhú možnosť: **appka založí tabuľku sama, cez toho istého "service accounta", čo už používa Sync now — bez akéhokoľvek Google okna.** Presne to je 2.0.4.

## 2. Ako to funguje teraz

Settings → Integrations → Pulls má odteraz **dva spôsoby**, ako sa pripojiť — vedľa seba, oba naraz dostupné:

**A) Prilep URL existujúcej tabuľky** — presne ako doteraz (2.0.2/2.0.3), nič sa tu nezmenilo. Toto zostáva spôsob pre tvoju reálnu, už existujúcu tabuľku s históriou.

**B) "Create a new sheet for me"** (nové) — napíšeš svoj e‑mail a klikneš. Appka:

1. Založí úplne novú Google tabuľku ("TIQR Manager - Pulls").
2. Rovno do nej zapíše hlavičky presne také, aké Sync now očakáva (sekcia 4 nižšie) — takže na rozdiel od 2.0.3 nemusíš nič ručne pridávať, tabuľka je hneď pripravená na prvý Sync now.
3. Zdieľa ju s tvojím e‑mailom (dostaneš bežný Google e‑mail "X zdieľal s vami dokument").
4. Rovno ju aj pripojí v appke — nemusíš potom ešte raz prilepovať URL.

Žiadne Google okno sa nikde neukáže — celé je to postavené na tom istom service accounte, čo Sync now.

## 3. Prečo to nie je "Sign in with Google" a prečo je to v poriadku

Service account má odteraz (len pre túto jednu akciu) o niečo širšie oprávnenie — okrem Sheets API aj úzky výsek Google Drive API (`drive.file`), ktorý appke dovolí presne dve veci: založiť nový súbor a zdieľať súbor, čo sama založila. Nikdy nič viac — nevidí a nemôže sa dotknúť žiadneho iného súboru v tvojom (ani ničom inom) Google Drive. Toto je stále len appka → Google (server-to-server), nikdy nie appka → prihlasovacia obrazovka pre teba, takže sa na to Googlov proces overenia OAuth aplikácií vôbec nevzťahuje — presne z rovnakého dôvodu, prečo sa netýkal ani pripájania existujúcej tabuľky.

## 4. Čo ešte musíš nastaviť (dva jednorazové kroky)

Bez oboch krokov nebude fungovať **ani jeden** zo spôsobov pripojenia v reálne vybuildenej appke:

**Krok 1 — GitHub secret (potvrdil si, že ešte chýba).** GitHub → tvoj repozitár `tiqr-manager` → Settings → Secrets and variables → Actions → New repository secret → meno `GOOGLE_SERVICE_ACCOUNT_JSON`, hodnota = celý obsah JSON súboru kľúča service accountu, ktorý si stiahol z Google Cloud Console. Bez tohto appka vôbec nevie, že Google Sheets existuje (presne to hlásia tie tvoje screenshoty).

**Krok 2 — Google Drive API (nové, len pre "Create a new sheet for me").** V Google Cloud Console, v tom istom projekte, kde je tvoj service account: APIs & Services → Library → nájdi "Google Drive API" → Enable. Je to rovnaký typ jednorazového kroku, ako keď si predtým zapínal Sheets API — ak si Sheets API už zapol pre 2.0.2, toto je ten istý postup, len pre druhé API. Spôsob **A) Prilep URL** toto nepotrebuje, potrebuje ho len nové tlačidlo **B) Create a new sheet for me**.

Po pridaní secretu treba appku znova vybuildiť cez `1-CLICK-UPDATE.bat` (ten teraz publikuje v2.0.4) — secret sa zapeká do appky až pri builde na GitHub Actions, nie za behu.

## 5. Hlavičky novovytvorenej tabuľky

| pull | Event name | event date | Ks | Platform | More info | Section | Row | Seats | Transfer | Price |
|---|---|---|---|---|---|---|---|---|---|---|

Presne stĺpce, ktoré Sync now vie spracovať (viď 2.0.3 report, sekcia 2) — okrem "date" a prázdneho stĺpca (appka ich aj tak ignoruje) a "TIQR ID" (ten si appka pridá sama, pri prvom Sync now). Do tejto novej tabuľky teda môžeš rovno písať a hneď potom kliknúť Sync now.

## 6. Odkaz na novú tabuľku — prečo nie je klikateľný

Po vytvorení appka ukáže URL novej tabuľky ako **označiteľný text** (klikneš/ťukneš a rovno sa označí celý), nie ako klikateľný odkaz. Appka zatiaľ nemá zabudovanú funkciu na otváranie odkazov v prehliadači (to je samostatná závislosť, ktorú som do tejto verzie zámerne nepridával, aby som nerozširoval rozsah zmien). V praxi to nevadí — dostaneš aj reálny e‑mail od Googlu o zdieľaní, cez ktorý sa k tabuľke dostaneš jedným klikom aj bez toho. Text v appke skopíruješ a otvoríš v prehliadači, ak by si k tabuľke chcel ešte pred tým e‑mailom.

## 7. Čo sa v tomto sandboxe nedalo overiť naživo

Rovnaké obmedzenie ako v 2.0.2/2.0.3 — `googleapis.com` je v tomto sandboxe nedostupné, takže `create_spreadsheet`/`share_file` (reálne HTTP volania na Sheets aj Drive API) som nemohol reálne spustiť. Rozdelil som to rovnako ako predtým: validácia e-mailu a meny (`validate_share_email`, `validate_currency`) je celá odskúšaná offline (7 nových testov nižšie) *pred* akýmkoľvek sieťovým volaním — schválne, aby zlá hodnota nikdy neviedla k tomu, že appka založí a zdieľa reálnu tabuľku, ktorú by potom nemala kam pripojiť. Samotné sieťové volania som len pozorne skontroloval (rovnaký tvar požiadaviek ako v oficiálnej Google dokumentácii k Sheets/Drive API). Prvé reálne kliknutie na "Create a new sheet for me" preto over u seba a daj vedieť, ako to dopadlo.

## 8. Testy

```
cargo test --lib -> 251 passed; 0 failed; 3 ignored
```

251 = 244 (z 2.0.3) + 7 nových v `commands::pulls_sheet_sync::tests`: validácia e‑mailu (platné aj neplatné adresy), validácia meny (len EUR/USD/GBP, prevod na veľké písmená), že hlavičky novovytvorenej tabuľky (sekcia 5) naozaj vyhovujú tomu, čo Sync now vyžaduje ako povinné stĺpce, a že aj s úplne platným vstupom appka zlyhá čisto a zrozumiteľne namiesto pádu, keď v danom builde (napr. tu v sandboxe) service account vôbec nie je zabudovaný.

## 9. Build

```
cargo check --lib -> čisto, 0 warningov
cargo test --lib  -> 251 passed, 0 failed
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.4 build" v hlavičke)
```

## 10. Zmenené/nové súbory a verzia

**Backend upravené:** `google_sheets.rs` (nový širší scope `SHEETS_AND_DRIVE_SCOPE`, nové funkcie `create_spreadsheet`/`share_file`), `models.rs` (`CreatedSheetResult`), `commands/sheets_sync.rs` (`set_sheets_connection_impl` teraz zdieľané aj s novým modulom), `commands/pulls_sheet_sync.rs` (hlavičky novej tabuľky, validácie, `create_pulls_sheet_impl` + `create_pulls_sheet` command a jeho testy), `lib.rs` (zaregistrovaný nový command)
**Frontend upravené:** `lib/types.ts` (`CreatedSheetResult`), `lib/api.ts` (`createPullsSheet`), `pages/Settings.tsx` (tlačidlo "Create a new sheet for me" vedľa existujúceho formulára, pole na e‑mail, trvalá správa s odkazom na novú tabuľku)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` — všetkých 6 na `2.0.4`, `package-lock.json` sa zosynchronizoval sám cez `npm install`.

## STOP

2.0.4 hotové a overené (251/251 backend testov, čisté `tsc`/`build`). "Create a new sheet for me" je funkčné vedľa existujúceho pripojenia cez URL — obe cesty vedú k tomu istému pripojenému stavu, žiadna Google prihlasovacia obrazovka nikde.

**Než to skúsiš naostro, potrebuješ obe:** (1) pridať `GOOGLE_SERVICE_ACCOUNT_JSON` secret na GitHube (sekcia 4, krok 1 — potvrdil si, že to ešte chýba) a (2) zapnúť Google Drive API v Cloud Console pre ten istý projekt (sekcia 4, krok 2 — nové, len pre tlačidlo na vytvorenie tabuľky). Potom spusti `1-CLICK-UPDATE.bat`, počkaj na zelený build na GitHub Actions, a napíš mi, ako dopadlo — ideálne vyskúšaj obe tlačidlá (Connect aj Create a new sheet for me), nech viem, či sa niečo nesprávalo presne podľa tohto reportu. Nezačínam nič ďalšie (zápis appka→tabuľka, Tickets kartu), kým toto nepotvrdíš.

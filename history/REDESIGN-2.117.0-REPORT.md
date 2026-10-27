# TIQR Manager 2.117.0 — TIQR v prehliadači a v mobile

Tvoje správy:

- *„ako by sme vedeli urobit mobilnu verziu alebo web ?"*
- *„tak skor by som siel do webu"*
- *„tiqr.site — toto je moja domena mozes ju pouzit"*

Toto vydanie obsahuje aj **2.116.1** (odstránenie obrázkov udalostí,
migrácia 048), ktorá ešte nebola vydaná. Jej report je
`history/REDESIGN-2.116.1-REPORT.md`.

---

## 1. Ako to funguje

TIQR na tvojom Windows serveri je ďalší počítač ako tvoje dva: má živú
synchronizáciu, mail aj Discord. Nové je, že vie svoje obrazovky ukázať aj
prehliadaču, teda aj iPhonu:

```
iPhone  ->  https://app.tiqr.site  ->  Cloudflare (tunel)  ->  TIQR na serveri
                                                                 |
                                               živá synchronizácia
                                                                 |
                                                       tvoje dva počítače
```

- V mobile je **tá istá appka** ako na počítači, len upravená pre malú
  obrazovku. Čo v mobile urobíš, zapíše TIQR na serveri do svojej databázy
  a živá synchronizácia to pošle na počítače.
- Na serveri sa **neotvára žiadny port**. Cloudflare Tunnel vedie zo
  servera von a TIQR počúva len sám na seba (127.0.0.1).
- **Všetko je zadarmo**: Cloudflare (Free), tunel, Firebase ako doteraz.
  Platíš len doménu, ktorú už máš.
- Zapína sa to **len v TIQR na serveri** (Settings → Integrations → Web
  access). Na tvojich dvoch počítačoch sa nič nemení a nič nezapínaj.

## 2. Čo je nové v TIQR

**Web access** (Settings → Integrations): jedno zaškrtávacie políčko a port
(8787). Pod ním TIQR ukáže, či odpovedá a kam nasmerovať tunel. Nastavenie
je v súbore vedľa údajov, nie v databáze, takže ho záloha ani synchronizácia
neprenesú na iný počítač.

**Webové obrazovky** sú zabudované priamo v TIQR, nič ďalšie sa na server
neinštaluje. V Safari sa dajú pridať na plochu: TIQR potom má vlastnú ikonu
a otvára sa bez lišty prehliadača.

**Rozloženie pre mobil** (len na úzkej obrazovke, na počítači sa nemení nič):

- dole lišta **Dashboard · Events · Inventory · Sales · More**, pod „More"
  sú Pulls, Finance, Mail, Notes, Settings, tmavý režim a odhlásenie,
- filtre nad zoznamom sú jeden riadok, ktorý sa posúva do strany,
- tabuľky si nechajú šírku stĺpcov z počítača a posúvajú sa do strany vo
  svojom ráme (predtým by sa stĺpce stlačili na pár písmen),
- tlačidlá v hlavičke stránky sa zalomia na ďalší riadok,
- čísla vo Finance a v detaile udalosti sú po dvoch v riadku,
- **Mail**: priečinok vyberieš zo zoznamu nad správami, správa sa otvorí cez
  celú obrazovku a šípka hore ťa vráti do zoznamu na to isté miesto,
- **Poznámky**: najprv zoznam, ťuknutím sa poznámka otvorí, šípka späť,
- hlásenia (napr. „Saved") sú nad dolnou lištou.

## 3. Bezpečnosť

- **Každá požiadavka z prehliadača nesie prihlásenie do TIQR** (Firebase ID
  token). TIQR na serveri ho overí podpisom Google a pustí len **účet, ktorý
  má sám otvorený**. Keď na serveri nie je nikto prihlásený, nepustí nikoho.
- Príkaz z mobilu ide do toho istého spracovania ako klik v okne, so
  všetkými kontrolami. Mobil nemá žiadnu vlastnú cestu k údajom.
- **Prihlásenie cez Google ostáva na app.tiqr.site.** TIQR na serveri podáva
  prihlasovacie stránky Firebase ďalej, aby celé prihlásenie bolo na jednej
  adrese. Inak by ho Safari a ikona na ploche iPhonu po návrate stratili.
  Z prehliadača k Firebase nejdú žiadne cookies a od Firebase žiadne
  neprídu.
- Stránky TIQR majú prísne pravidlá prehliadača (CSP): skripty len vlastné a
  Google prihlásenie, žiadne vkladanie TIQR do cudzej stránky.
- Do logu nejde žiadny token. Odmietnuté prihlásenie sa zapíše len s
  dôvodom („expired", „unknown key"), bez obsahu.
- **Cloudflare Access** (e-mail + kód pred TIQR) je **nepovinný druhý
  zámok**. Postup je v návode.

## 4. Čo na webe zámerne nejde

| Čo | Prečo |
|---|---|
| export a import CSV, záloha a obnova, uloženie prílohy mailu, diagnostický súbor | súbor by vznikol na serveri, nie v mobile |
| kontrola cien, pripojenie Gmailu, prihlásenie Google Sheets | otvárajú okno na obrazovke servera |
| živá synchronizácia, čítanie mailu | robí ich okno TIQR na serveri samo |
| zapnutie a vypnutie Web access | len v TIQR na serveri |

V mobile to TIQR povie jednou vetou, nič sa nepokazí.

## 5. Návod krok za krokom

Celý návod je v zipe ako **`WEB-A-MOBIL.txt`**. Stručne:

0. **Vydaj 2.117.0** (1-CLICK-UPDATE.bat).
1. **Doména na Cloudflare**: bezplatný účet → Add a domain `tiqr.site`
   (Free) → v Namecheap Nameservers → Custom DNS → dva nameservery
   z Cloudflare → počkať na „Active".
2. **TIQR na serveri**: inštalátor z GitHub Releases (`-setup.exe`) →
   prihlásiť sa tým istým účtom → Settings → Data → Live sync on this
   computer → (Mail: Connect Gmail) → Settings → Integrations → Web access.
   Okno nechať otvorené, Vzdialenú plochu zatvárať krížikom, nie
   odhlásením. Po reštarte: zástupca v `shell:startup` + Autologon.
3. **Tunel**: cloudflared na server → Cloudflare Networking → Tunnels →
   Create a tunnel → príkaz `cloudflared.exe service install …` v PowerShell
   ako správca → Published application `app.tiqr.site` →
   `http://127.0.0.1:8787`.
4. **Google prihlásenie**: Firebase → Authentication → Authorized domains
   `app.tiqr.site`; Google Cloud → Credentials → Web client → redirect URI
   `https://app.tiqr.site/__/auth/handler`.
5. **iPhone**: Safari → app.tiqr.site → prihlásiť sa → Zdieľať → Pridať na
   plochu.

## 6. Čo treba vedieť

- **TIQR na serveri musí bežať a byť prihlásený.** Keď nebeží, mobil napíše
  „TIQR on the server cannot be reached".
- **Zmena z mobilu príde na počítače zvyčajne do minúty**, nie do sekúnd.
  Windows spomaľuje okno, na ktoré sa nikto nepozerá, a s ním aj živú
  synchronizáciu servera. Zmeny z počítačov prídu do mobilu rýchlo ako
  doteraz.
- **Čo sa tu nedalo vyskúšať**, a preto to prvýkrát vyskúšaš na serveri:
  skutočné prihlásenie cez Firebase v TIQR na Windows, skutočné
  prihlasovacie stránky Google cez server a ikona na ploche iPhonu. Ak by
  Google prihlásenie v mobile nešlo, ide prihlásenie e-mailom a heslom; ak
  by nešlo len v ikone na ploche, ide v Safari. Napíš mi presné hlásenie.

## 7. Testy a kontroly

| Kontrola | Výsledok |
|---|---|
| `cargo test --locked` | 1617 prešlo, 0 zlyhalo, 4 ignorované (ako doteraz) |
| `npm test` | 59 prešlo, 0 zlyhalo |
| `npx tsc -b` | bez chyby |
| `npm run build` a `npm run build:web` | v poriadku (web: 1,58 MB, 410 KB po kompresii) |
| `cargo clippy` | 121 starších varovaní ako pred touto zmenou, v `web_access.rs` žiadne; žiadna chyba |

Nové testy v `web_access.rs`:

- `only_a_current_token_of_the_open_account_for_this_project_gets_in` —
  platný token prejde; iný účet, vypršaný, z budúcnosti, iný projekt, iný
  vydavateľ, neznámy kľúč, podvrhnutý podpis, HS256, „alg none" a nezmysel
  neprejdú.
- `files_windows_and_the_servers_own_loops_are_refused_with_a_reason`
- `every_refused_command_is_a_command_this_app_has` — premenovaný príkaz by
  inak ticho prestal byť zakázaný.
- `options_are_off_unless_a_readable_file_says_on`
- `cache_control_max_age_is_read`
- `a_browser_request_reaches_the_windows_command_and_only_with_the_right_token`
  — celá cesta cez HTTP: zdravie, 503 keď nikto nie je prihlásený, 401 bez
  tokenu a so zlým, 403 iný účet, odpoveď príkazu, chyba príkazu, bajty,
  zakázaný príkaz, udalosť z okna doputuje do prehliadača, stránky, 405.
- `firebase_sign_in_pages_come_through_this_address` — prihlasovacie
  stránky idú cez náhradu za Firebase: cesta aj otázka presne, cookies
  nejdú von ani späť, odpoveď nemá pravidlá stránok TIQR, cesta s „..“ sa
  odmietne, stránky TIQR si pravidlá nechajú.
- `firebase_out_of_reach_is_said_plainly`
- Frontend (`src/web/web.test.ts`): rozdelenie udalostí zo servera a
  hlásenia chýb.

Vyskúšané naživo (tu, nie na tvojom serveri): skutočná appka pod Xvfb
odpovedá na 127.0.0.1:8787 (401 bez tokenu, 503 bez prihlásenia, stránky,
ikony, manifest), prihlasovacia obrazovka v prehliadači iPhone 13 a všetky
hlavné obrazovky v mobilnom rozložení (Dashboard, Events, Inventory, Sales,
Pulls, Finance, Mail so správou a nastaveniami, Notes, Settings, detail
udalosti, formulár nového predaja) — žiadna stránka nie je širšia ako displej.
Tlačidlo Google na https adrese vedie na `/__/auth/handler` tej istej
adresy.

## 8. Zmenené súbory

**Nové:** `src-tauri/src/web_access.rs`, `src/web/` (core, event, frames,
answers, session, window, dialog, opener, process, updater, deepLink,
web.test.ts), `src/lib/platform.ts`, `src/lib/usePhone.ts`, tento report.

**Backend:** `lib.rs` (modul, dva príkazy, štart), `commands/database.rs`
(TIQR si zapamätá otvorený účet), `build.rs` (zabudovanie webových
obrazoviek), `Cargo.toml`/`Cargo.lock` (axum, tokio, futures-util),
`tauri.conf.json` (zostaví aj web).

**Frontend:** `vite.config.ts` (webový režim, ikona na plochu),
`package.json` (`build:web`), `Layout.tsx` (dolná lišta, nič z práce servera
v prehliadači), `ui.tsx`, `index.css`, `icons.tsx` (IconMenu), `toast.tsx`,
`useFillHeight.ts`, `MailIntelligence.tsx`, `MailSettingsPane.tsx`,
`Notes.tsx`, `Events.tsx`, `Orders.tsx`, `Settings.tsx` (Web access),
`Welcome.tsx`, `auth.tsx`, `firebase.ts`, `main.tsx`, `api.ts`, `types.ts`,
`.gitignore` (`dist-web`).

**Dokumenty a verzia:** `CHANGELOG.md`, `PROJECT_STATE/*`; verzia 2.117.0
v `package.json`, `package-lock.json`, `tauri.conf.json`, `Cargo.toml`,
`Cargo.lock`.

## 9. Ako vydať

1. Rozbaľ zip do **nového prázdneho** priečinka.
2. Dvojklik na `1-CLICK-UPDATE.bat` a počkaj na „Done". Žltá poznámka
   vymenuje 6 súborov obrázkov z 2.116.1, to je v poriadku.
3. Aktualizáciu nainštaluj na **oboch** počítačoch.
4. Server nastav podľa `WEB-A-MOBIL.txt`.

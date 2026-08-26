# TIQR Manager 2.0.2 — Google Sheets sync, krok 1 (Settings → Integrations)

Report k verzii **2.0.2**. Toto je prvý krok prepojenia appky s tvojimi Google Sheets tabuľkami (pulls aj tickets), o ktoré si žiadal. **Táto verzia rieši len pripojenie samotné** — v Settings pribudla karta Integrations, kde jedným-dvoma klikmi pripojíš tabuľku Pulls a appka si vie overiť, že sa do nej vie pozrieť. Reálne čítanie/zapisovanie riadkov (import pulls z tabuľky do appky a naopak) **ešte nie je súčasťou tejto verzie** — dôvod je v sekcii 3, a presne to je aj dôvod, prečo potrebujem od teba odpovede v sekcii 5 predtým, než do toho pôjdem.

## 1. Ako funguje pripojenie (naozaj 1-2 kliky, pre kohokoľvek)

Pôvodne som navrhoval "Prihlásenie cez Google účet" (OAuth) — to si správne zamietol, lebo by to znamenalo, že každý používateľ appky musí prejsť Google login obrazovkou. Preveril som si to priamo v Google dokumentácii a zistil som, že by to navyše malo aj praktický problém: kým appka nie je oficiálne "verified" u Googlu (čo trvá dni a vyžaduje verejné privacy policy, doménu, demo video), prihlasovacie tokeny by po 7 dňoch prestali fungovať a každý používateľ by musel byť ručne pridaný na "test users" zoznam v Google Console. To je presný opak toho, čo si chcel.

Namiesto toho appka používa **Google Service Account** — technický "robotický" Google účet, ktorý patrí appke samotnej, nie tebe ani žiadnemu používateľovi:

1. Appka má v sebe (zakompilovanú pri builde, nikdy v repozitári) prihlasovaciu identitu `tiqr-sync@tiqr-manager-sync.iam.gserviceaccount.com`.
2. Používateľ appky si len **zdieľa svoju tabuľku s týmto e-mailom** (presne ako keď zdieľaš Google Sheet s kolegom) — toto je jediný "účet" krok a robí sa raz, priamo v Google Sheets, nie v appke.
3. V appke potom stačí vložiť URL tabuľky + názov záložky (tab) a kliknúť **Connect**. Žiadne prihlasovacie okno, žiadny redirect do prehliadača, žiadne čakanie na schválenie.

Toto funguje identicky pre teba aj pre kohokoľvek iného, kto appku bude niekedy používať — nikto sa nemusí nikde "prihlasovať cez Google", len zdieľať svoju tabuľku a vložiť odkaz.

## 2. Čo je nové v tejto verzii

- **Settings → Integrations** (nová karta v Settings, vidno ju aj na Settings home) — pre "Pulls" zobrazuje stav (Connected/Not connected), políčka na URL/ID tabuľky a názov záložky, tlačidlá **Connect/Save**, **Test connection**, **Disconnect**, a e-mail service accountu, s ktorým treba tabuľku zdieľať.
- **`src-tauri/src/google_sheets.rs`** (nový súbor) — vlastný, ručne napísaný klient na Google Sheets API v4 (autentifikácia cez service account JWT, `get_values`/`update_values`/`append_values`). Zámerne som nepoužil hotové `google-sheets4`/`yup-oauth2` knižnice — sú stavané pre async kód, kým celá appka je synchrónna; vlastný tenký klient je menší, jednoduchší na údržbu a sedí so zvyškom kódu.
- **`src-tauri/src/commands/sheets_sync.rs`** (nový súbor) — 4 nové Tauri commands: `get_sheets_connection_status`, `set_sheets_connection`, `clear_sheets_connection`, `test_sheets_connection`. Všetky sú univerzálne pre ľubovoľný "data source" (string kľúč, dnes `"pulls"`, `"tickets"` pribudne pri ďalšom kroku bez toho, aby sa tu čokoľvek menilo).
- **Migrácia `008_sheet_sync.sql`** — nová tabuľka `sheet_sync_links`, zatiaľ prázdna/nepoužívaná appkou (pripravená na budúci krok — párovanie riadku appky s riadkom v tabuľke).
- Pripojenie sa ukladá do existujúcej `app_settings` tabuľky (rovnaký mechanizmus ako napr. téma vzhľadu) — žiadna nová infraštruktúra navyše.

## 3. Čo táto verzia zámerne NEROBÍ (a prečo)

- **Nečíta ani nezapisuje žiadny riadok pulls.** Tlačidlo "Test connection" len overí, že appka vie prečítať bunku A1 danej záložky — nič viac. Skutočné obojsmerné porovnávanie/import/export je ďalší krok a potrebuje presne vedieť, ktorý stĺpec tvojej tabuľky znamená čo (sekcia 5) — stavať to na hádanom mapovaní by presne zopakovalo problém, ktorému sa táto appka historicky vyhýba (radšej sa opýtať, než pokaziť dáta).
- **V Settings vidno zatiaľ len "Pulls", nie "Tickets"** — vybral si "najprv Pulls" a keďže sync riadkov pre Tickets ešte neexistuje, nedáva zmysel ponúkať pripojenie tabuľky, ktorá by aj tak zatiaľ nič nerobila. Karta pre Tickets pribudne v ďalšom kroku (backend je už teraz univerzálny, takže to bude len pridanie UI).
- **Appka bez nastaveného GitHub Secretu (sekcia 6) hlási "Google Sheets sync isn't available in this build"** namiesto toho, aby spadla alebo niečo pokazila — bezpečný, jasne pomenovaný stav, nie tichá chyba.

## 4. Bezpečnosť service account kľúča

Tvoj repozitár je verejný (updater si sťahuje `latest.json` bez prihlásenia), takže čokoľvek raz commitnuté do gitu je navždy verejne čitateľné, aj keby si to neskôr "zmazal". Kľúč preto **nikdy nie je súčasťou zdrojového kódu ani zipu, ktorý dostávaš**:

- Skutočný kľúč pridáš **raz** do GitHub repozitára ako "repository secret" (presný postup nižšie, sekcia 6).
- `src-tauri/build.rs` ho pri builde (len na GitHub Actions serveri) zakompiluje priamo do appky.
- Bežný lokálny `cargo build`/`npm run tauri dev` (napr. keby si niekedy staval appku sám na svojom počítači) tento secret nemá nastavený — appka to nepokazí, len nahlási "sync isn't available", presne ako v bode vyššie.
- `.gitignore` som doplnil o vzory (`*service-account*.json`, `credentials.json`, ...) ako poistku pre prípad, že by si si niekedy stiahnutý `.json` kľúč omylom uložil priamo do priečinka appky.

## 5. Čo potrebujem od teba pred ďalším krokom (mapovanie stĺpcov Pulls)

Poslal si mi reálne 3 riadky z tvojej tabuľky. Porovnal som ich s tým, čo appka o Pulls už dnes vie (`pulls` tabuľka, `buyer_name`/`event_name`/`event_date`/`quantity`/`platform_id`/`seats`/`more_info`/`price_cents`/`currency`/`transfer_done`). Toto mapovanie mi vychádza **jasné, netreba sa na nič pýtať**:

| Tvoj stĺpec | Ide do | Poznámka |
|---|---|---|
| `pull` (raxik, logickk, ...) | `buyer_name` | Presne sedí — appka to už dnes definuje ako "pre koho sa pulluje". |
| `Event name` | `event_name` | Priamy zápis. |
| `Ks` (2x, 2x, ...) | `quantity` | Len odstránim "x" a prevediem na číslo. |
| `Platform` (axs, oetickets, mlb) | `platform_id` | Rovnaká logika ako pri CSV importe — appka platformu podľa mena nájde alebo si ju sama vytvorí, keď ešte neexistuje. |
| `More info` (e-maily) | `more_info` | Voľný text, appka do neho nijako nezasahuje. |
| `Transfer` (Áno/Nie) | `transfer_done` | Áno → hotovo, Nie → nehotovo. |

Toto ale potrebujem od teba potvrdiť/vysvetliť, lebo appka to dnes nevie mapovať bezpečne sama:

1. **Stĺpec "Seats"** — v tvojich reálnych dátach tam nie sú čísla sedadiel, ale hodnoty ako `SlabeRuky22.` a `Markiboss1111.`, čo vyzerá skôr ako **heslo k účtu** (možno k účtu, cez ktorý sa ťahá pull na axs/oetickets?) než sedadlo. Ak je to heslo/citlivý údaj, chcem sa uistiť, že to appka bude ukladať niekam vhodne (a nie do poľa, ktoré sa v appke volá a zobrazuje ako "Seats"). Vieš mi povedať, čo tam presne je?
2. **Prázdny stĺpec** medzi "Seats" a "Transfer" (bez názvu, vo všetkých 3 riadkoch prázdny) — má nejaký účel, alebo ho appka môže úplne ignorovať?
3. **Mena stĺpca "Price"** — appka pri Pulls potrebuje aj menu (EUR/USD/...), tabuľka žiadnu neuvádza. Mám predpokladať EUR vždy, alebo sa to mení riadok od riadku?
4. **Stĺpec "date"** — v prvom riadku je doslova `IDK`, v ostatných `13.1.2026`, čo je skôr dátum, kedy si pull zaevidoval/založil, než dátum podujatia (ten je už v "event date") alebo termín odovzdania (13.1. je dávno pred podujatím 14.8.). Čo presne tento stĺpec znamená?

Kým toto nepotvrdíš, nezačínam písať kód, ktorý by riadky reálne čítal/zapisoval — presne v duchu "radšej sa opýtať, než hádať" pri dátach, ktoré sa týkajú peňazí a citlivých údajov.

## 6. Nastavenie GitHub Secret (urobíš raz, trvá minútu)

1. Otvor `https://github.com/biznismarko9-source/tiqr-manager/settings/secrets/actions`.
2. **New repository secret**.
3. Name: `GOOGLE_SERVICE_ACCOUNT_JSON`
4. Value: celý obsah `.json` súboru s kľúčom, ktorý si mi poslal — skopírovaný a vložený tak, ako je (je to čistý JSON text, nie súbor na nahratie).
5. **Add secret**.

Bez tohto kroku appka naďalej funguje úplne normálne vo všetkom ostatnom — len Integrations sekcia hlási, že sync nie je v tomto builde dostupný.

## 7. Zdieľanie tabuľky (na reálne odskúšanie)

Až budeš mať sekciu 5 potvrdenú a ja spravím reálny sync, budem to potrebovať vyskúšať oproti skutočnej tabuľke. V tomto sandboxe appku spustiť neviem (Tauri potrebuje grafické okno) a navyše odtiaľto nedovidím na `googleapis.com` (sekcia 9) — reálne odskúšanie preto urobíš ty, na svojom počítači, po vydaní tejto verzie. Keď na to dôjde, budem potrebovať buď kópiu (safety copy) tvojej reálnej tabuľky, alebo aspoň testovaciu tabuľku s podobnou štruktúrou, zdieľanú s `tiqr-sync@tiqr-manager-sync.iam.gserviceaccount.com` (stačí "Viewer" na test, "Editor" keď budeme testovať aj zápis).

## 8. Testy

```
cargo test --lib -> 217 passed; 0 failed; 3 ignored
```

217 = pôvodných 196 (z 2.0.1) + 21 nových: `google_sheets::tests` (7 — JWT podpisovanie, parsovanie service account JSON, percent-encoding rozsahu), `commands::sheets_sync::tests` (11 — ukladanie/čítanie pripojenia, oddelenosť pulls/tickets, validácie), `db::migration_008_tests` (3 — migrácia na čistej aj existujúcej databáze).

Dôležité obmedzenie: **tento sandbox nemá prístup na `googleapis.com`** (overil som `curl -v` — proxy vracia 403 na CONNECT tunel, kým napr. `registry.npmjs.org` prejde bez problémov). Reálne volanie Google Sheets API som teda nemohol v tomto prostredí odskúšať naživo. Namiesto toho som JWT podpisovanie overil **offline** — v teste si appka sama vygeneruje dočasný RSA kľúčový pár (cez `openssl` príkazový riadok), podpíše ním token presne tak, ako by to urobila voči Googlu, a overí podpis späť. To dokazuje, že kryptografia/formát je správny; samotné pripojenie na tvoju tabuľku over cez "Test connection" tlačidlo, keď appku dostaneš (sekcia 7).

## 9. Build

```
cargo check --lib -> čisto, 0 warningov, presne 1 verzia reqwest v strome závislostí
npx tsc -b        -> 0 chýb
npm run build     -> OK ("tiqr-manager@2.0.2 build" v hlavičke)
```

## 10. Zmenené/nové súbory a verzia

**Nové (backend):** `src-tauri/src/google_sheets.rs`, `src-tauri/src/commands/sheets_sync.rs`, `src-tauri/migrations/008_sheet_sync.sql`
**Backend upravené:** `Cargo.toml` (pribudli `reqwest`, `jsonwebtoken`, `percent-encoding`, `base64`), `build.rs`, `error.rs`, `db.rs` (registrácia migrácie + testy), `models.rs`, `commands/mod.rs`, `lib.rs`
**Frontend upravené:** `components/icons.tsx` (`IconLink`), `lib/types.ts`, `lib/api.ts`, `pages/Settings.tsx` (nová karta Integrations + `SheetsConnectionCard`)
**CI/ostatné:** `.github/workflows/build-windows.yml` (nový secret `GOOGLE_SERVICE_ACCOUNT_JSON` v oboch build jobs + vysvetľujúci komentár), `.gitignore` (poistka proti náhodnému commitu kľúča)
**Verzia (6 miest):** `package.json`, `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `src-tauri/Cargo.lock`, `release.ps1` (`$Version` + commit message), `1-CLICK-UPDATE.bat` (title/echo text) — všetkých 6 na `2.0.2`, `package-lock.json` sa zosynchronizoval sám cez `npm install`.

## STOP

2.0.2 hotové a overené (217/217 backend testov, čisté `tsc`/`build`) — appka vie pripojiť a otestovať Google Sheets tabuľku pre Pulls, jedným-dvoma klikmi, bez Google prihlasovania. Nič sa zatiaľ reálne nesynchronizuje. Čakám na odpovede k sekcii 5 (najmä stĺpec "Seats") a na nastavenie GitHub Secretu (sekcia 6) — až potom idem stavať skutočný obojsmerný sync riadkov. Nezačínam nič ďalšie bez toho.

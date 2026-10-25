# TIQR Manager 2.116.1 — bez obrázkov udalostí

Tvoje správy:

- *„tu malú šípku z loga treba odstrániť, tie fotky odstráň kompletne"*
- *„tie pri events atď."*

---

## 1. Obrázky udalostí sú preč

Obrázky z 2.116.0 sú odstránené úplne. Už nie sú:

- v zozname **Events** ani na **stránke udalosti** (zmizlo aj tlačidlo
  „Picture"),
- v **Inventory**, v **objednávke**, v **Sales**, v **predaji** ani na
  **Dashboarde**,
- vo formulári **novej udalosti** a pri **úprave** (výber obrázka aj stĺpec
  „Picture" pri pridávaní viacerých udalostí naraz),
- v **Settings → Integrations** (karta „Event pictures").

Na pozadí sa už nič nehľadá a TIQR sa nepýta **Deezeru ani Wikipedie** na
nič. Tieto obrazovky vyzerajú presne ako v 2.115.1.

Zo stability z 2.116.0 ostáva všetko:

- obrazovka s chybou spadne sama a ponúkne „Try again",
- po páde sa pri ďalšom štarte ukáže poznámka,
- chyby sa zapisujú do `logs/tiqr.log` bez mailov, tokenov a hesiel,
- Settings → Support → Diagnostic report,
- opravy (Discord dátum, Detect categories, migrácie, Escape).

## 2. Čo sa stane s uloženými obrázkami (migrácia 048)

Pri prvom štarte 2.116.1 migrácia 048 zmaže všetko, čo obrázky uložili:

- **automatické nálezy** – tabuľka `artwork_lookups` sa zruší,
- **ručne vybrané obrázky** – tabuľka `event_artworks` sa vyprázdni,
- **ich stopy v synchronizácii** (čakajúce na odoslanie, záznamy o verziách,
  záznamy o zmazaní) a **prepínač** obrázkov.

**Nič iné sa nemení.** Udalosti, objednávky, lístky, predaje a financie
ostanú presne také, aké boli. Migrácia prebehne celá naraz, alebo vôbec:
keď sa štart preruší v polovici, nič sa nezmení a migrácia sa spustí znova.

**Prečo jedna prázdna tabuľka ostáva.** Počítač, ktorý má ešte 2.116.0, pri
zlučovaní zálohy z Google Drive prechádza tabuľky jednu po druhej a pri
chýbajúcej tabuľke zastaví celé zlúčenie. Preto `event_artworks` ostáva,
prázdna a nepoužívaná. Neskoršia verzia ju môže zrušiť, keď už nikde nebude
2.116.0.

## 3. Dva počítače počas prechodu

Aktualizáciu nainštaluj na **oboch** počítačoch. Kým ju má len jeden:

- ten s 2.116.0 obrázky ešte ukazuje a hľadá,
- obrázky, ktoré od neho prídu cez živú synchronizáciu, ten s 2.116.1 ticho
  zahodí. Nič sa neuloží a nehlásia sa ako konflikty,
- zálohy z Drive sa dajú zlúčiť oboma smermi.

Keď sa aktualizuje aj druhý, jeho vlastná migrácia 048 vymaže obrázky aj
tam.

## 4. Šípka na ikone na ploche

Malá šípka vľavo dole **nie je v logu TIQR**. Windows ju kreslí na každú
skratku na ploche, aby bolo vidno, že je to len odkaz na aplikáciu. Preto ju
nová verzia TIQR odstrániť nemôže. Dá sa len skryť nastavením vo Windows, a
to naraz pre všetky skratky v počítači.

V zipe je na to priečinok **`SIPKY-NA-PLOCHE`**:

- **`SKRYT-SIPKY.bat`** – šípku skryje. Windows sa opýta na práva správcu
  (potvrď „Áno"). Do `C:\ProgramData\TIQR-Manager` uloží úplne priehľadný
  obrázok a nastaví Windows, aby ho kreslil namiesto šípky. Šípky zmiznú po
  reštarte počítača (alebo po odhlásení a prihlásení).
- **`VRATIT-SIPKY.bat`** – vráti šípky Windows späť a priehľadný obrázok
  zmaže.

Tieto dva súbory nie sú súčasťou aplikácie ani repozitára, `1-CLICK-UPDATE.bat`
ich nikam nenahrá.

Ako bolo overené: časť, ktorá vytvára priehľadný obrázok, prebehla tu v
PowerShelli. Vznikol presne obrázok 16×16, ktorý je celý priehľadný, a keď
sa súbor nedá zapísať, skript skončí chybou a nič nenastaví. Zápis do
Windows registrov sa tu skúsiť nedá, ten prebehne až u teba.

## 5. Testy a kontroly

| Kontrola | Výsledok |
|---|---|
| `cargo test --locked` | 1609 prešlo, 0 zlyhalo, 4 ignorované (ako doteraz) |
| `npm test` | 57 prešlo, 0 zlyhalo (64 mínus 7 testov odstráneného úložiska obrázkov) |
| `npx tsc -b` | bez chyby |
| `npm run build` | v poriadku |
| `cargo clippy` | 121 starších varovaní, ako pred touto zmenou; žiadna chyba |

Nové testy:

- `removing_the_pictures_takes_what_they_stored_and_nothing_else` – databáza
  zo 2.116.0 s obrázkami, s ich stopami v synchronizácii a so spúšťačmi,
  ktoré tam dala 2.116.0. Po migrácii sú obrázky a ich stopy preč, spúšťače
  tiež a nič nové sa nezaradilo na odoslanie. Udalosť, jej záznamy v
  synchronizácii a iné nastavenia ostali.
- `a_new_file_ends_without_the_picture_tables_but_the_empty_one` – nová
  databáza.
- `a_picture_sent_by_a_computer_not_yet_updated_is_set_aside_quietly` – obrázok
  aj jeho zmazanie z počítača s 2.116.0. Nič sa neuloží, nie je to konflikt
  a nič sa neposiela späť.

Odstránené testy patrili odstránenému kódu: 33 v module obrázkov, 1 v živej
synchronizácii, 1 pre migráciu 047 a 7 vo frontende.

## 6. Zmenené súbory

**Zmazané:** `src-tauri/src/commands/event_artwork.rs`,
`src/components/EventArt.tsx`, `src/components/ArtworkPicker.tsx`,
`src/lib/artworkStore.ts`, `src/lib/artworkStore.test.ts`,
`src/lib/eventArtwork.ts`.

**Nové:** `src-tauri/migrations/048_remove_event_artwork.sql`, tento report.

**Upravené – backend:** `db.rs` (migrácia 048, test), `lib.rs`,
`commands/mod.rs`, `commands/cloud_merge.rs`, `commands/live_sync.rs`
(`RETIRED_TABLES`, test), `commands/database.rs` (počet migrácií 48),
`commands/mail_view.rs` (späť na 2.115.1), `diag.rs` (bez počtov obrázkov).

**Upravené – frontend:** `Events.tsx`, `EventRowsModal.tsx`,
`EventDetail.tsx`, `Orders.tsx`, `OrderDetail.tsx`, `Sales.tsx`,
`SaleDetail.tsx`, `Dashboard.tsx`, `Settings.tsx`, `Layout.tsx`, `api.ts`,
`types.ts`, `icons.tsx`. Tieto súbory sú až na hlásenie chýb z 2.116.0 také
ako v 2.115.1, `EventDetail.tsx`, `types.ts` a `icons.tsx` presne také.
Ešte `diagnostics.ts` a `diagnostics.test.ts` (príklad v teste).

**Dokumenty a verzia:** `CHANGELOG.md`, `PROJECT_STATE/CURRENT_STATE.md`,
`KNOWN_BUGS.md`, `PROTECTED_AREAS.md`; verzia 2.116.1 v `package.json`,
`package-lock.json`, `tauri.conf.json`, `Cargo.toml`, `Cargo.lock`.

## 7. Ako vydať

1. Rozbaľ zip do **nového prázdneho** priečinka (pravý klik → Extrahovať
   všetko).
2. Dvojklik na `1-CLICK-UPDATE.bat` a počkaj na „Done".
3. Aktualizáciu nainštaluj na **oboch** počítačoch.
4. Ak chceš skryť šípky, na počítači s Windows spusti
   `SIPKY-NA-PLOCHE\SKRYT-SIPKY.bat`, potvrď „Áno" a reštartuj počítač.

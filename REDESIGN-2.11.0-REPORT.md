# TIQR Manager 2.11.0 — Installer + Auto-Updater + Release Pipeline

Report k tvojmu zadaniu. Fáza 0 (zmapovať, čo existuje) je vlastne polovica
príbehu — väčšina toho, čo si žiadal, **už existovala a fungovala**. Nestaval
som druhý release systém.

---

## 1. Čo existovalo pred taskom

Toto všetko som našiel hotové a **nechal nedotknuté**:

| Vec | Stav |
|---|---|
| Tauri updater plugin (Rust + JS) | ✅ nainštalovaný a zapojený |
| `plugins.updater.pubkey` | ✅ **reálny** public key |
| Updater endpoint | ✅ GitHub Releases `latest.json` |
| `createUpdaterArtifacts: true` | ✅ |
| Windows NSIS installer | ✅ `installMode: currentUser` (per-user) |
| WebView2 | ✅ `downloadBootstrapper` |
| GitHub Actions | ✅ dve cesty: manuálny unsigned test build + signed release na `v*` tag |
| `tauri-action` publish | ✅ s `uploadUpdaterJson: true` |
| `TAURI_SIGNING_PRIVATE_KEY` secret | ✅ zdokumentovaný |
| `release.ps1` + `1-CLICK-UPDATE.bat` | ✅ vrátane guardu na workflow súbor |
| `lib/updater.ts` | ✅ check / download / install / relaunch |
| `UpdateOverlay.tsx` | ✅ full-screen branded overlay |
| Settings → Software | ✅ check, install, release notes, verzia |
| Launch-time check | ✅ v `Layout.tsx` |

**Takže „pridaj Tauri updater" a „pridaj GitHub Releases pipeline" už boli
hotové.** Neduplikoval som ich.

---

## 2. Čo sa reálne zmenilo

### 🔴 Hlavná diera: macOS neexistoval vôbec

Workflow sa volal `build-windows.yml` a **buildoval iba Windows**. Žiadny
`.dmg` sa nikdy nevytvoril ani nepublikoval. To bola skutočná práca.

| Zmena | Prečo |
|---|---|
| Workflow premenovaný na `release.yml`, buildí **obe** platformy | názov už klamal |
| macOS: **universal** `.dmg` (Apple Silicon + Intel v jednom) | chcel si jeden download na platformu |
| `release.ps1` guard premenovaný spolu s ním | musia sedieť — viď nižšie |
| Delete starého release → **vlastný job pred maticou** | oprava reálneho bugu |
| Matica **serializovaná** (`max-parallel: 1`) | oba legy prepisujú `latest.json` |
| Nový job **`verify-release`** | manifest je jediný artefakt, ktorého rozbitie nikto nezbadá |

---

## 3. Windows installer

Nezmenený model — bol už správny:

- **NSIS**, `installMode: currentUser` (per-user, bez admin práv)
- **WebView2: `downloadBootstrapper`** — nechal som ho

K tvojej otázke o `webviewInstallMode`: `downloadBootstrapper` znamená, že
installer si runtime stiahne sám, ak na stroji nie je. Na Windows 11 tam je
vždy, na Windows 10 skoro vždy. Offline varianta (`embedBootstrapper` /
`fixedRuntime`) by pridala ~150 MB do každého installeru kvôli scenáru, ktorý
u teba prakticky nenastane. **Nechal som to tak, ako si písal — neprepínam na
obrovský offline runtime iba kvôli zadaniu.**

Z pohľadu používateľa je installer self-contained: žiadny Node, Rust, npm,
DLL ani setup.

---

## 4. macOS installer

- Štandardný **Tauri DMG** (`targets: "all"` už bolo nastavené), žiadny vlastný
  shell script
- **Universal binary** cez `--target universal-apple-darwin` — jeden `.dmg`
  pre Apple Silicon aj Intel
- Rust targety `aarch64-apple-darwin` + `x86_64-apple-darwin` sa v CI
  inštalujú podmienene iba na macOS legu
- DMG obsahuje app + Applications shortcut (štandardný Tauri layout)

---

## 5. Updater

Existujúci, nedotknutý v jadre. Flow: check → verify signature → download →
install → relaunch. To je presne to, čo `lib/updater.ts` už robilo.

Čo som pridal:

- **`UPDATE_CHECK_INTERVAL_MS = 6h`** — launch check zostal, plus pomalé
  opakovanie pre session, ktorá ostane otvorená dni. Žiadny polling.
- **`getLastUpdateCheck()`** — in-memory záznam kedy sa naposledy kontrolovalo
  a čo sa našlo, aby Dashboard aj Settings vedeli stav zobraziť **bez toho,
  aby samy spustili ďalší check**.

Zlyhaný check je ticho všade okrem Settings, kde si o neho výslovne požiadal.
Appka funguje offline normálne.

---

## 6. GitHub Actions

```
v* tag
  ↓
prepare-release          zmaže starý release pre tento tag
  ↓
release (matica, po jednom)
  ├─ windows-latest  →  NSIS .exe  + updater artifact + .sig
  └─ macos-latest    →  universal .dmg + updater artifact + .sig
  ↓                     oboje do JEDNÉHO releasu + latest.json
verify-release           overí, že to naozaj sedí
```

### Tri štrukturálne opravy, ktoré matica odhalila

1. **Delete starého releasu bol vnútri build jobu.** Pri jednej platforme to
   bolo v poriadku. S maticou by **druhý runner zmazal release, ktorý prvý
   práve publikoval.** Presunuté do `prepare-release`, ktorý beží prvý.
2. **`max-parallel: 1`.** Oba legy publikujú do toho istého releasu a oba
   prepisujú `latest.json`. Sériové spustenie odstráni race a manifest je
   deterministický. Release je zriedkavý — pár minút navyše za správnosť.
3. **`verify-release`.** Overí, že release má `.exe`, `.dmg` aj `latest.json`,
   že manifest pokrýva **obe** platformy a že každý záznam má neprázdny podpis
   aj URL.

Bod 3 stojí za vysvetlenie: pri fetch-nutí `tauri-action` dokumentácie som
zistil, že **nikde nie je explicitne napísané**, či sa `latest.json` naprieč
matice legmi merguje alebo prepisuje. V praxi sa merguje (inak by každá
multi-platform Tauri appka mala rozbitý updater). Ale namiesto toho, aby som
sa na to spoľahol, som z toho spravil **CI kontrolu** — ak by merge nefungoval,
build spadne s jasnou chybou namiesto toho, aby ti používatelia potichu
prestali dostávať updaty.

Test build (tlačidlo v Actions) je teraz tiež pre obe platformy, unsigned,
a **nikdy nevytvorí release ani nesiahne na `latest.json`**.

---

## 7. GitHub Releases

Jediný distribučný kanál. Žiadny update server, žiadny cloud, žiadny hosting.

Artefakty:

| Platforma | Install | Updater |
|---|---|---|
| Windows | `*-setup.exe` | `*.nsis.zip` + `.sig` |
| macOS | `*.dmg` | `*.app.tar.gz` + `.sig` |
| — | | `latest.json` |

---

## 8. Signing (updater)

**Nakonfigurované a funkčné.** `TAURI_SIGNING_PRIVATE_KEY` (+ password) je v
GitHub Secrets, matching public key je v `tauri.conf.json` — tam patrí, je to
verifikačný kľúč.

Private key som **nevypísal, nevygeneroval nový a nikam necommitol.** Nie je v
žiadnom artefakte ani v tomto reporte.

**Neplatný podpis = update sa nenainštaluje.** Žiadna unsigned cesta
neexistuje.

---

## 9. Code signing status — čítaj pozorne

Toto je iné než updater signing a **nie je nakonfigurované.**

| | Stav | Dôsledok |
|---|---|---|
| **Updater signing** | ✅ hotové | in-app update je bezpečný |
| **Windows code signing** | ❌ nie | SmartScreen: „Windows protected your PC" → More info → Run anyway |
| **macOS code signing + notarization** | ❌ nie | Gatekeeper: „unidentified developer" → prvé spustenie right-click → Open |

Obe potrebujú **platené externé credentials** (komerčný CA certifikát; Apple
Developer účet). Workflow už **posiela všetkých 6 Apple premenných** dovnútra,
takže keď secrets pridáš, nič iné meniť netreba.

**Nevytvoril som falošný ani self-signed certifikát.** Self-signed by
vyprodukoval to isté varovanie, len by to *vyzeralo* podpísané — to je horšie
než nič.

Installer je plne funkčný aj bez podpisu. Je to **release limitation**, nie
chyba.

---

## 10. Secrets, ktoré treba nastaviť

**Povinné (už máš):** `TAURI_SIGNING_PRIVATE_KEY`,
`TAURI_SIGNING_PRIVATE_KEY_PASSWORD`

**Voliteľné (už máš):** `GOOGLE_SERVICE_ACCOUNT_JSON`, `GOOGLE_OAUTH_CLIENT_ID`
/`_SECRET`, `FIREBASE_GOOGLE_OAUTH_CLIENT_ID`/`_SECRET`, `ANTHROPIC_API_KEY`

**Chýbajúce, ak chceš podpísaný Mac build:** `APPLE_CERTIFICATE`,
`APPLE_CERTIFICATE_PASSWORD`, `APPLE_SIGNING_IDENTITY`, `APPLE_ID`,
`APPLE_PASSWORD`, `APPLE_TEAM_ID`

**Chýbajúce pre Windows:** code-signing certifikát (mechanizmus závisí od typu
certifikátu — dopíšem, keď ho budeš mať)

Celé je to rozpísané v novom **`RELEASE.md`**, bez jedinej reálnej hodnoty.

---

## 11. Version management

Nezmenil som mechanizmus — **9 výskytov v 7 súboroch**, ako doteraz.
`release.ps1` tvrdo zastaví, ak prvé tri nesedia. `RELEASE.md` má tabuľku
všetkých siedmich vrátane oboch lockfile-ov.

Pravidlo „verzia ide vždy dopredu" platí — Tauri updater porovnáva čísla
priamo a downgrade ani opakovanie neponúkne.

---

## 12. DB safety

**Update nahrádza aplikáciu, nikdy nie databázu.**

- SQLite súbor zostáva na mieste
- nová verzia spustí svoje forward-only migrácie pri štarte, ako pri
  hocijakom inom otvorení
- **žiadna nová migrácia** kvôli updateru (ďalšia nová je stále **027**)
- žiadna reset cesta

Ak update zlyhá (prerušený download, výpadok, neplatný podpis): nič sa
nenainštaluje a stará verzia beží ďalej. Neplatný podpis sa odmietne **pred**
aplikovaním. Žiadny vlastný binary patcher.

---

## 13. Update UI

**Jedno UI, nie dve.**

- **Dashboard**: malý pill vedľa zvončeka — `Up to date` alebo
  `Update to vX.Y.Z`. Číta iba výsledok launch checku a **linkuje** do
  Settings. **Nespúšťa vlastný check a nič neinštaluje.**
- **Settings → Software**: jediné miesto s download/install flow a
  `UpdateOverlay`. Pribudlo Current version / Latest version / Last checked
  k existujúcemu checku, inštalácii a release notes.

„Latest version" je len verzia, ktorú appka reálne dostala z GitHubu — pred
prvým checkom je „Not checked yet", nie odhad.

---

## 14. Čo bolo overené reálne

| | |
|---|---|
| Workflow YAML sa parsuje | ✅ (`pyyaml`) |
| Job graf | ✅ `prepare-release` → `release` (matica 2) → `verify-release` |
| `release.ps1` nemá zastaralý odkaz na starý názov workflow | ✅ 0 výskytov |
| README nemá zastaralý odkaz | ✅ opravené 2 miesta |
| Verzia vo všetkých 7 súboroch | ✅ 2.11.0 |
| `$CommitMsg` bez úvodzoviek | ✅ (guard z 2.7.0) |
| TS súbory: zátvorky + importy | ✅ všetky sa rozlišujú |
| `tauri-action` input názvy | ✅ overené proti `action.yml` v repozitári akcie |
| `src-tauri/src` nedotknuté | ✅ žiadna business logika |

---

## 15. Čo NEBOLO možné overiť

- **Žiadny build.** Nie je tu Node ani Rust. `npx tsc -b`, `npm run build`,
  `cargo check` — ani jedno.
- **macOS build nikdy nebežal.** Ani tu, ani v CI. **Prvý tagovaný beh tohto
  workflowu je jeho prvý test.** Konkrétne neoverené: či universal target
  prejde, kde presne skončí `.dmg` (preto ho hľadám cez `find`, nie natvrdo),
  a či `latest.json` naozaj obsahuje obe platformy — presne preto existuje
  `verify-release`.
- **Install ani update test** na čistom stroji.
- **Code signing** — bez credentials sa nedá otestovať.

Neclaimujem production-ready signing ani notarization. Nie sú.

---

## 16. Presný release postup

```
1. Zvýš verziu (7 súborov — tabuľka v RELEASE.md)
2. Spusti 1-CLICK-UPDATE.bat
3. Sleduj Actions — musí prejsť aj verify-release
4. Skontroluj release page: .exe + .dmg + latest.json
5. Nainštaluj .exe na Windows
6. Otvor .dmg na Macu
7. Zo starej verzie over, že sa update ponúkne a nainštaluje
```

**Používateľ:**
Windows: stiahni `.exe` → nainštaluj → spusti.
Mac: stiahni `.dmg` → pretiahni do Applications → spusti (prvýkrát
right-click → Open).
Update: otvor appku → Dashboard ukáže update → klik → reštart.

---

## Čo sa zámerne NEZMENILO

Žiadna business logika: Orders · Tickets · Sales · Listings · Finance ·
Fulfillment · Attention · Calendar · Price Checker · refund/resell ·
`batch_id` · money/integer cents. `src-tauri/src` sa v tomto kole nedotklo.

Žiadny nový cloud service, žiadny update server, žiadny redesign, žiadne nové
business features.

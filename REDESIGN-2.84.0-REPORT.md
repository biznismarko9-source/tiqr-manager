# TIQR Manager 2.84.0 — Settings

**Dátum:** 11.10.2026
**Zadanie:** podstatné zlepšenie Settings, s dôrazom na Data a Integrations

---

## 0. Prostredie

node, npm, cargo ani Homebrew tu nie sú. **Žiadny build, type-check, lint,
testy ani spustenie.** Zadanie hovorí *"Do not claim visual verification
unless you actually ran and inspected the application"* — nespustil som ju,
teda to netvrdím.

## 1. Podstatná zmena vs. doladenie — to, čo si výslovne pýtal

| podsekcia | čo dostala |
|---|---|
| **Integrations** | **podstatná prestavba** — zo štyroch voľných kariet na dve pomenované skupiny |
| **Data** | **podstatná prestavba** — štyri karty preusporiadané do dvoch pomenovaných skupín |
| Lookups | **nič** |
| Notifications | **nič** |
| Software | **nič** |
| Account | **nič** |
| Support | **nič** |

Dôvod toho nuly je vecný, nie z lenivosti — bod 4 nižšie.

## 2. Integrations: štyri karty boli v skutočnosti dve integrácie

Toto je ten nález, ktorý stál za audit.

`GoogleSignInCard`, karta **Pulls** a karta **Orders & Sales** sú **jedna**
integrácia: jeden Google účet a dva hárky pod ním. Lenže každá ukazovala
vlastný odznak „Connected", takže stránka čítala ako **tri** služby. AI kľúč
je genuínne druhá, nesúvisiaca — a v kóde to hovoril len komentár, nie
stránka.

Teraz:

- **Google Sheets** — *„One Google account, and the sheets it syncs with."*
  Účet hore, dva hárky pod ním.
- **AI** — *„Optional. Your own Anthropic key, used by the AI import."*

Pridané boli **len obaly**. Ani jedna karta, tlačidlo, handler ani popis sa
nedotkol.

## 3. Data: nebolo vidieť, ktoré karty hryzú

Štyri karty — CSV, Check my data, Sync, Backup & restore — stáli v jednom
nerozlíšenom stĺpci. Dve sú bežný nástroj, dve vedia **prepísať celú
databázu**. Stránka to nijako nehovorila.

Teraz:

- **Everyday** — CSV, Check my data.
  *„Nothing here can overwrite your data."*
- **Keeping your data safe** — Sync, Backup & restore.
  *„...and the one action that can replace it. Read each button before
  pressing it."*

Karty sa preusporiadali, ale **vzájomné poradie v skupine ostalo**: Sync je
stále priamo nad Backup & restore, presne ako to žiada komentár z 2.12.0
(*„the same concern — moving this database between machines"*). Nechcel som
rozbiť úvahu, ktorá tam bola zapísaná s dôvodom.

Pre teba konkrétne to má váhu: 1. októbra ťa stála dáta práve tá druhá
skupina.

## 4. Prečo zvyšných päť nedostalo nič

Pozrel som si ich, a majú **po jednej karte**:

| | |
|---|---|
| Notifications | 1 karta |
| Software | 1 karta |
| Account | 1 karta |
| Support | 1 komponenta (`SupportCards`) |
| Lookups | zoznam s vlastnými záložkami |

Nemajú čo zoskupovať. Vytvárať v nich skupiny by znamenalo vymýšľať
štruktúru tam, kde je jedna vec — presne ten druh dekorácie, ktorú zadanie
zakazuje.

Je to zároveň potvrdenie, že tvoj inštinkt bol správny: problém existoval
presne v tých dvoch sekciách, ktoré si menoval, a nikde inde.

## 5. Zmenené súbory

| súbor | čo |
|---|---|
| `src/pages/Settings.tsx` | nová `SettingsGroup`; Data a Integrations prestavané; `type ReactNode` do importu |

Jeden súbor. Žiadna zdieľaná komponenta, žiadna iná sekcia.

## 6. Overenie presunu — najrizikovejšia vec v tomto vydaní

Presúvalo sa ~600 riadkov v 4 352-riadkovom súbore. Preto som to overil
viac než obvykle:

| kontrola | výsledok |
|---|---|
| `<Card>` otvorených / zatvorených | **17 / 17** |
| `<SettingsGroup>` otvorených / zatvorených | **4 / 4** |
| Zátvorky oproti zálohe | **0 / 0 / 0** |
| `setImportOpen` | 4 pred → 4 po |
| `doDownloadTemplate` | 3 → 3 |
| `setExportConfig` | 4 → 4 |
| `runDataChecks` | 2 → 2 |
| `cloudSyncPush` | 1 → 1 |
| „Backup database" / „Restore from backup" | 1 → 1 / 1 → 1 |
| Stratené riadky | len tie, ktoré som zámerne nahradil (import, starý komentár, dva wrappery) |
| Nevyriešené identifikátory | žiadne |

Tri veci kontrolór najprv nahlásil — `HTMLInputElement`, `String`
a `TestResultLine`. Prvé dve sú vstavané typy z generík, tretia je arrow
komponenta vnorená vo funkcii, ktorú môj regex nezachytí. Overil som všetky
tri, než som ich odpísal.

## 7. NEOVERENÉ

- **Build, type-check, lint, testy.**
- **Settings som nevidel bežať.** Ani hub, ani jednu podsekciu, ani jeden
  dialóg.
- Import, export, zálohovanie, obnova, sync, kontrola dát, Google Sheets,
  AI kľúč, notifikácie, aktualizácie, účet, podpora — **kódom nedotknuté,
  ale ani jedno otestované za behu.**
- **Porovnanie so screenshotom som nerobil pohľadom**, len čítaním kódu.
  Screenshot je navyše starší než kód: nemá kartu „Check my data", ktorá
  pribudla v 2.73.0.

## 8. Riziko

Presun 600 riadkov je väčšie riziko než čokoľvek, čo som v tejto sekcii
robil predtým. Zmiernil som ho tým, že som nerozdeľoval ani nemenil obsah
kariet — len som ich presunul ako celé bloky aj s ich komentármi — a potom
spočítal, že všetko sedí. Ale je to spočítané, nie videné.

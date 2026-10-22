# TIQR Manager 2.115.1 — rad a sedadlá v správe o predaji sú skryté

Tvoja správa:

- *„ak príde new sale, tak vo webhooku by malo byť hide row a seats"*

---

## Čo sa zmenilo

Správa o novom predaji v Discorde (kanál Sales) mala riadok so sedadlami
celý na očiach:

> 💺 Sec 112 · Row F · 14, 15

**Teraz sú rad a sedadlá skryté ako spoiler.** V Discorde je na ich mieste
tmavý pruh a ukážu sa až po kliknutí. Sekcia ostáva viditeľná:

> 💺 Sec 112 · ▇▇▇▇▇▇▇▇ *(po kliknutí „Row F · 14, 15")*

- **Platí pre každú správu o predaji:** novú, predaj z mailu aj odoslanú
  tlačidlom „Send to Discord".
- **Ak sa predalo viac blokov sedadiel**, každý má skrytý svoj rad a svoje
  sedadlá.
- **Keď predaj nemá rad ani sedadlá** (len počet lístkov), nie je čo
  skrývať a ukáže sa napríklad „Sec GA · 2 tickets".
- **Správy o nákupe (kanál Inventory) ostávajú ako doteraz.** Žiadosť sa
  týkala predaja. Ak to chceš aj pri nákupoch, napíš.

## Dá sa vypnúť

**Settings → Integrations → Discord → Sales:** pribudol prepínač *„Hide the
row and seats - shown on a click in Discord, the section stays in sight"*.

- Je zapnutý od začiatku.
- Vypnutý vráti správu do pôvodnej podoby.
- Nastavenie je rovnaké na oboch počítačoch, ako ostatné nastavenia
  Discordu.

Náhľad správy v nastaveniach Discordu teraz ukazuje skryté časti rovnako ako
Discord: tmavý pruh, ktorý sa po kliknutí odkryje. To platí aj pre meno a
e-mail kupujúceho v súhrnoch mailov, ktoré boli skryté už od 2.113.1.

## Testy

- `cargo test --locked`: 1596 prešlo, 0 zlyhalo (4 ignorované, ako doteraz).
  Nové a upravené testy:
  - správa o predaji má rad a sedadlá ako spoiler a sekciu viditeľnú;
  - po vypnutí sa ukáže ako predtým;
  - samotný počet lístkov sa neskrýva;
  - znaky `||` v názve radu nemôžu spoiler predčasne ukončiť;
  - správa o nákupe sa nezmenila.
- `npm test`: 53 prešlo. `npm run build` prešiel.
- **Náhľad správy je overený na snímke:** pred kliknutím je pruh, po
  kliknutí sa ukáže „Row GA · 1, 2".

## Súbory

- `src-tauri/src/commands/discord.rs`, `notifications.rs`, `models.rs`
- `src/components/Discord.tsx`, `src/pages/Settings.tsx`, `src/lib/types.ts`
- `CHANGELOG.md`, `PROJECT_STATE/*`, verzia v piatich súboroch.

Žiadna migrácia (ďalšia je 047). Nastavenie je v uloženej konfigurácii
Discordu. Staršia konfigurácia ho nemá, a preto sa skrýva rovnako.

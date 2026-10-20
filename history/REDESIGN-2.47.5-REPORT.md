# TIQR Manager 2.47.5 — štyri slovenské slová, ktoré môj sken nevidel

Mal si pravdu a viem presne, prečo som to prehliadol.

---

## Prečo to 2.47.3 nenašlo

Môj sken hľadal slovenčinu **podľa diakritiky** — dĺžňov a mäkčeňov — plus
podľa zoznamu slov, ktorý som si napísal. Slovo, ktoré nemá ani jedno a ja som
naň nepomyslel, mu prešlo popod ruky.

Štyri také tam boli:

| Slovo | Kde to vidíš |
|---|---|
| `nie` | prepínač **Pull** v New Order — zhasnutá gulička |
| `Kupec` | popis poľa v **New Sale** |
| `Ks` (2×) | hlavička stĺpca v zozname **Pulls** |
| `e.g. Doprava` | placeholder kategórie v Settings |

Teraz je to **`no`**, **`Buyer`**, **`Qty`** a **`e.g. Transport`**.

## Ako som to našiel poriadne

Nie lepším zoznamom slov — ten vie nájsť len to, na čo si už myslel.

Vypísal som **všetkých 733 textov, ktoré appka zobrazuje** (texty medzi
značkami plus každý `placeholder`, `title`, `aria-label` a `label`), z toho
**399 krátkych** — nadpisy, tlačidlá, popisy polí — a prečítal som ich.
Presne tam tie štyri sedeli.

Zapísal som to do `PROTECTED_AREAS.md`, aby sa ďalší jazykový audit nerobil
grepom na diakritiku.

---

## Jedna vec, ktorú by si mal overiť

Písal si, že slovenčina je v **New Event, Order aj Pull**. Ja som slovenčinu
našiel len v **Order** (`nie`) a **Sale** (`Kupec`).

**New Event a New Pull sú v zdrojáku po anglicky už od 2.47.3** — overil som to
teraz znova strojovo, sú úplne čisté.

Takže ak v nich stále vidíš slovenčinu, beží ti **build starší ako 2.47.3**.
Skontroluješ to v **Settings → Software updates → „Current version"**. Ak tam
je 2.47.2 alebo menej, to je vysvetlenie a stačí nainštalovať tento build.

---

## Čo som overil

- **Štyri opravy** — každá presne na jednom mieste (okrem `Ks`, ktoré bolo 2×).
- **Všetky štyri formuláre strojovo čisté** — New Order, New Sale, New Event,
  New Pull: žiadna diakritika, žiadne `nie`/`ano`/`Ks`/`Kupec`.
- **Zoznam súborov** oproti 2.47.4: **0 pribudlo, 0 zmizlo, 4 zdrojové zmenené**
  (+ verzia a dokumentácia).
- **Zátvorky** proti 2.47.4 — identické vo všetkých štyroch.
- **Verzia na 9 miestach v 7 súboroch.**

---

**Verzia:** 2.47.5 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.

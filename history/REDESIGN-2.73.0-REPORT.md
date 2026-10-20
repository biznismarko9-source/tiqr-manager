# TIQR Manager 2.73.0 - všetky tri veci

**Dátum:** 9.10.2026
**Zadanie:** *"vsetky 3"*

---

## 1. Sync dokončený

V `decide_auto` vetva **"druhý stroj má novšie dáta a tento nemá nič
neodoslané"** vracia odteraz **Merge**, nie Pull.

Presne táto vetva ťa 1.10. pripravila o tri objednávky, šesť lístkov a dva
predaje. `local_dirty` tvrdil `false`, hoci deň práce čakal neodoslaný, a Pull
prepísal celý súbor staršou kópiou z Drive.

**Merge to spraviť nemôže.** Aplikuje tie isté tombstony, ktoré by niesol
Pullov súbor - takže keď naozaj niečo zmažeš na druhom stroji, zmaže sa to aj
tu - ale **nechá riadky, ktoré táto strana má a druhá nikdy nevidela**.

A keď je ten príznak **správny**, sú obe možnosti rovnocenné: ak naozaj nemáš
nič neodoslané, nie sú žiadne lokálne riadky, ktoré by Merge zachovával, takže
skončíš s tou istou databázou, akú by dal Pull. **Nič sa tým nestratilo.**

`decide_auto` už Pull nevracia vôbec. Variant ostáva pre ručné tlačidlo
"Sync down", ktoré si spúšťaš sám.

**Čo stále nie je opravené:** prečo je ten príznak nepravdivý. Je to zapísané
v `KNOWN_BUGS.md`. Spolu s 2.68.0 (kde sa Pull a Merge začali pýtať) to však
znamená, že jeho nepravdivosť už nie je fatálna - najhoršie, čo spraví, je, že
preskočí Push, ktorý sa mal stať. To nestratí nič, len nechá stroje nezladené.

## 2. Testy v CI

Nový job **`rust-tests`** v `release.yml`.

Tieto testy existovali celý čas a nikdy ich nikto nespustil. `tauri build`
kompiluje bez `#[cfg(test)]`, takže testový modul, ktorý sa ani nepreloží,
vyzerá presne ako zelený build - a vyzeral, štyri vydania po sebe.

Beží na macOS, nie na Linuxe: Tauri tam potrebuje webkit2gtk a ďalšie
systémové balíky, a kontrola, ktorá padne na chýbajúcom balíku, nikoho nič
nenaučí. macOS ten crate už v ostatných joboch stavia, takže tam toolchain
preukázateľne funguje.

**Zámerne to zatiaľ nie je podmienka vydania.** Tie testy nikto nevidel prejsť
mesiace. Spraviť z nich bránu pred prvým zeleným behom by znamenalo riskovať,
že ti zablokujem vydanie kvôli niečomu, na čo sa ani jeden z nás ešte
nepozrel. Keď uvidíme, že job zozelenel, pridám `needs: rust-tests` a vtedy to
bránou bude. To je tá pointa.

Frontend vlastný job nepotrebuje - `npm run build` je `tsc -b && vite build`,
takže TypeScript sa kontroluje v každom builde.

## 3. "Check my data"

V **Settings → Data**, nad Backup & restore. Deväť kontrol:

| kontrola | čo hľadá |
|---|---|
| Currency that isn't a real code | `US $` namiesto `USD` - kvôli tomu nešiel convert |
| Paid order with no cost on it | zaplatená objednávka za 0,00 |
| Ticket in a different currency than its order | blokuje konverziu |
| A negative amount | záporná cena, poplatok alebo náklad |
| Sale pointing at a ticket that is gone | osirelý predaj |
| Ticket marked sold with no sale behind it | zmazaný predaj bez vrátenia lístka |
| Ticket still for sale although it has been sold | dá sa predať druhýkrát |
| Order whose quantity doesn't match its tickets | rozpočet nákladu na lístok je zlý |
| The same code used twice | nejednoznačné všade, kde sa číta |

**Nikdy nič nemení.** Každá kontrola je `SELECT`. Opraviť ktorúkoľvek znamená
rozhodnúť, aká hodnota je tá správna - a to je tvoje rozhodnutie o tvojich
peniazoch, nie pravidlo, ktoré má tlačidlo hádať. Preto tam nie je žiadne
"oprav všetko" a zámerne som sa oň ani nepokúšal.

Kontrola, ktorá nič nenašla, sa vracia tiež, a panel to povie nahlas:
*"All 9 checks passed - nothing to look at."* To je odpoveď, ktorú budeš
dostávať najčastejšie, a prázdny panel ju dať nevie.

### Čo to nájde u teba práve teraz

- **Currency that isn't a real code: 2** → `YELIVE-001 (US $)`
- **Paid order with no cost on it: 3** → `NFL-001, NFL-002, NFL-003`

Zvyšných sedem kontrol: nula.

## 4. Čo je otestované a čo nie

**Naozaj spustené:**

- **Všetkých deväť SQL dotazov som pustil proti kópii tvojej databázy.**
  Bežia bez chyby a vrátia presne tie čísla vyššie. Toto nie je čítanie kódu,
  toto je skutočný beh.
- `GLOB '[A-Z][A-Z][A-Z]'` funguje v SQLite tak, ako som potreboval - overené
  na reálnej hodnote `US $` aj na `USD`.
- YAML `release.yml` je po zmene platný (preparsované).

**Overené čítaním:**

- Zátvorky v `Settings.tsx`, `cloud_sync.rs` a novom `data_health.rs`:
  rozdiel oproti netknutým zálohám 0 vo všetkých druhoch.
- `IconLoader` som najprv použil a **neexistuje** - vymenené za `Spinner`,
  ktorý je v tej istej karte o pár riadkov nižšie.
- `db::test_conn()` už existoval; nepísal som si vlastnú kópiu (tá moja by
  nezapínala foreign keys).
- Príkaz je zaregistrovaný v `commands/mod.rs` aj v `lib.rs`.
- Nikde nie je `deny(warnings)`, takže `CloudSyncAutoAction::Pull`, ktorý sa
  už nikde nekonštruuje, build nezhodí.

**Neoverené:**

- **Neskompilované** - node ani cargo tu nie sú.
- Panel som nevidel bežať. Po builde: Settings → Data → **Run checks**.
- `rust-tests` job je prvý beh naslepo. Keď padne, pošli mi výstup - to je
  presne to, na čo tam je.

## 5. Čo si pozri sám

Tie tri NFL objednávky. Buď sú naozaj zadarmo - a potom je všetko v poriadku -
alebo tam chýba nákupná cena a každý výpočet zisku na tých šiestich lístkoch
ti ukazuje stopercentnú maržu.

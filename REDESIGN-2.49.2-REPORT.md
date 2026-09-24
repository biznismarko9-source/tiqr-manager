# TIQR Manager 2.49.2 — povolenie jedným klikom priamo z hlášky

Napísal si „doteraz to fungovalo, urob to tak aby to fungovalo aj teraz bez
zmien". Beriem to vážne, tak ti poviem celú pravdu — aj tú nepríjemnú časť.

---

## Najprv som hľadal skutočnú chybu v kóde

Boli dve veci, ktoré by presne takto vyzerali, a obe som preveril:

**1. Nepýta si appka pri obnove tokenu užší rozsah?** Pri obnovovaní tokenu sa
totiž **dá** vypýtať menej, než máš povolené — a výsledok by vyzeral presne
takto. **Nepýta.** Posiela len token a nič iné, takže dostane všetko, čo máš
povolené.

**2. Neprepisuje ti druhé prihlásenie to prvé?** Appka má dve rôzne prihlásenia
cez Google — jedno cez Settings → Integrations (Sheets + Drive) a jedno to
tlačidlo „Continue with Google" (len identita, žiadny Drive). Keby si to druhé
ukladalo na to isté miesto, prepísalo by ti tokeny a Drive by prestal fungovať.
**Neprepisuje.** Do toho miesta píše jediné miesto v kóde — to prvé prihlásenie.

Obidve boli reálni podozriví. Obidve sú čisté.

## Čo zostáva — a prečo ti nemôžem sľúbiť „bez zmien"

**To, aké povolenia token má, sa rozhodlo v momente, keď si ho schválil.**
Neskôr sa to už zmeniť nedá — ani obnovením, ani ničím z appky.

Tvoj token je starší než verzia, ktorá si Drive začala pýtať (2.12.0). Preto
funguje na Sheets a padá na Drive.

**Žiadny kód nevie do tokenu pridať povolenie, ktoré mu nikdy nikto nedal.**
To je pravidlo Googlu, nie chyba appky. Keby som ti napísal, že som to vyriešil
„bez zmien", klamal by som ti — a ty by si to o päť minút zistil sám.

**Jedno schválenie u Googlu je nevyhnutné. Presne jedno, raz.**

## Čo som teda spravil

Zrušil som to hľadanie a chodenie po menu.

V tej červenej hláške, ktorá ti to hlási, je teraz rovno tlačidlo
**„Allow Google Drive access"**:

1. klikneš naň,
2. otvorí sa Google, schváliš (nechaj všetky políčka zaškrtnuté),
3. **sync sa rozbehne okamžite** — presne ten istý, ktorý predtým padal.

Žiadne Settings, žiadne hľadanie, žiadne reštartovanie. Klik a hotovo.

Tlačidlo púšťa **ten istý sync**, ktorý beží na časovači — nie druhú kópiu
tej logiky, ktorá by sa od nej časom rozišla.

## A aby sa to už nemohlo zopakovať

Spolu s tým, čo som pridal v 2.49.1: keby si pri tom schvaľovaní niektoré
políčko odškrtol, appka to **odmietne uložiť** a rovno ti povie, ktoré
povolenie chýba. Nedostaneš sa znova do stavu, kde prihlásenie vyzerá, že
prešlo, a pritom je od začiatku nepoužiteľné.

---

## Čo som overil

- **Obe teórie o chybe v kóde** vyvrátené čítaním, nie dohadom: obnovenie
  tokenu nepýta užší rozsah; `REFRESH_TOKEN_KEY` má v celom projekte **jediné**
  miesto, ktoré doň zapisuje.
- **Hláška a tlačidlo sa nemôžu rozísť** — frontend rozpoznáva presne tú vetu,
  ktorú generuje backend.
- **Zátvorky v `Layout.tsx`** proti 2.48.1 — **bez posunu**. Rozdiel v značkách
  je len `null` a `void` z typu nového `useRef`, nie JSX.
- **Žiadny nepoužitý import.**
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad a samotné schvaľovanie u Googlu.

---

**Verzia:** 2.49.2 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.

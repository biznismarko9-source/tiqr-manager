# TIQR Manager 2.50.0 — euro sa už nekonvertuje na euro, a mena je na tebe

---

## 1. Tá chyba — a je to doslova jeden znak

Mal si úplnú pravdu, že to je bug. Príčina:

V databáze bol pri tých objednávkach uložený **symbol `€`**, nie kód `EUR`.

A kontrola „je to už v eurách, netreba konvertovať?" porovnávala **doslova**
s textom `"EUR"`. Lenže `"€"` sa `"EUR"` nerovná. Takže objednávka zadaná
v eurách prešla filtrom a poslala sa konvertovať **z eur na eurá** — a služba
na kurzy, celkom správne, žiadnu menu s názvom `€` nepozná.

Odtiaľ to `404 not found` a to `2 skipped`.

Mimochodom, appka **mala** poistku presne na tento prípad — „keď je zdroj
a cieľ tá istá mena, nerob nič". Nezabrala z toho istého dôvodu: porovnávala
`€` s `EUR`.

### Ako je to opravené

Appka teraz rozpoznáva symboly ako plnohodnotné kódy:

| Symbol | Kód |
|---|---|
| € | EUR |
| $ | USD |
| £ | GBP |
| Kč | CZK |
| zł | PLN |
| Ft | HUF |
| lei | RON |
| лв | BGN |
| ₺ | TRY |

Čo je **už v cieľovej mene, sa nikdy neponúkne na konverziu** — ani keď je to
uložené ako symbol, ani malými písmenami.

**`kr` som naschvál nechal tak, ako je.** Je to švédska, nórska **aj** dánska
koruna. Tipovať, ktorá to je, by znamenalo prepočítať ti peniaze zlým kurzom —
to radšej nechám tak, nech to vidíš.

Ešte jedna vec: to porovnávanie som presunul **zo SQL do kódu**. SQL vie opraviť
veľké/malé písmená, ale so symbolom nespraví nič — presne preto to predtým
prešlo.

## 2. Preferovaná mena

**Settings → Lookups → Preferred currency**, na výber **EUR / USD / GBP**.

Podľa toho sa riadi **všetko „Convert to…" v celej appke**: banner na
Dashboarde, tlačidlo pri novej objednávke, na detaile objednávky, v Sales aj
vo Finance.

Keď to prepneš, **popisky sa prekreslia naraz všade** — nemusíš nič reštartovať.
A čo je dôležitejšie: popisok a to, čo konverzia naozaj spraví, čítajú **to isté
miesto**, takže sa nemôžu rozísť.

**Default je EUR.** Ak to nikdy neotvoríš, appka sa správa presne ako doteraz.

Uloží sa len jedna z tých troch mien — čokoľvek iné sa odmietne. Je to
nastavenie, podľa ktorého sa presúvajú reálne peniaze, takže mu appka musí
vedieť veriť.

---

## Čo som overil

- **Rozdeľovanie mien som prepísal do Pythonu a spustil na 7 prípadoch**,
  vrátane tvojho presného: objednávka uložená ako `€` s cieľom EUR → **nič na
  konverziu**. Ďalej: mix `€`/GBP, `usd` a `USD` sa spoja do jednej skupiny,
  pri cieli GBP sa konvertujú naopak eurá, `Kč` sa pozná ako CZK. Všetky sedia.
- **Štyri nové Rust testy** priamo v projekte, vrátane toho tvojho prípadu
  a prípadu, keď je preferovaná mena GBP.
- **Zoznam súborov** oproti 2.49.2: 1 pribudol, 0 zmizlo, 11 zmenených.
- **Zátvorky vo všetkých jedenástich** (4 Rust + 7 TypeScript) — **bez posunu**.
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad (Rust ani Node tu nie sú) a samotné volanie na
kurzovú službu.

**Jedna poznámka:** toto opravuje, že sa `€` už nikdy nebude konvertovať. Tie
staré riadky, kde je uložený symbol namiesto kódu, tam **zostanú tak, ako sú** —
zámerne som ich hromadne neprepisoval, lebo to je zásah do uložených dát a na
to sa ťa najprv spýtam. Appka s nimi odteraz zaobchádza správne. Ak chceš, viem
spraviť jednorazové upratanie, ktoré ich prepíše na `EUR`.

---

**Verzia:** 2.50.0 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.

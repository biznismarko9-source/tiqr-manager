# TIQR Manager 2.51.1 — Notes je pod Finance

---

## Najprv odpoveď na „kde je tá nová vec"

**Je hotová** — vyšla ako **2.51.0** v predošlej správe. Ak si ju v appke
nevidel, je to preto, že tá verzia ešte nebola zbuildená a nainštalovaná;
posielam ti zdroják, appka sa stavia až cez CI.

## Čo je v tomto builde

Presunul som ju tam, kam si povedal: **Finance → piata záložka**.

**Overview · Transactions · Accounts · Reports · Notes**

Vlastná položka v bočnom paneli je preč aj s ikonkou, ktorá existovala len
kvôli nej. Panel tým zostáva plochý — to je pravidlo, ktoré si si sám vypýtal
v 2.43.0 (všetko vždy vidieť, nič sa neskrýva): táto zmena riadok **uberá**,
nie pridáva.

**S dátami sa nestalo nič.** Tie isté tabuľky, tá istá migrácia 031, to isté
synchronizovanie medzi počítačmi. Čo si v 2.51.0 stihol napísať, tam ostáva.

Jedna drobnosť v rámci presunu: Finance si kreslí vlastnú hlavičku stránky,
takže Notes si drží už len svoju vlastnú lištu — hľadanie a tlačidlo
**New sheet**.

---

## Čo som overil

- **Po presune nezostal ani jeden mŕtvy odkaz** — trasa `/notes`, import
  stránky aj nevyužitá ikonka sú preč, strojovo overené.
- **Každý import sa dá dohľadať** po zmene ciest o úroveň nižšie.
- **Žiadny nepoužitý import** v presunutej stránke.
- **Zátvorky** v `Notes.tsx` aj `Finance.tsx` — vyvážené.
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad (Rust ani Node tu nie sú, prvý reálny build je
CI).

---

**Verzia:** 2.51.1 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové (031 prišla v 2.51.0), ďalšia voľná je 032.

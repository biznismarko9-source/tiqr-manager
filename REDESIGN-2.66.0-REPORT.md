# TIQR 2.66.0 — tri tiché chyby v Settings

Pýtal si sa, čo tie chyby vlastne sú. **Než som ti ich popísal, overil som
každú priamo v kóde** — našiel ich agent a nechcel som ti tvrdiť, že máš
chyby, na základe cudzieho čítania. Všetky tri obstáli.

Pri tej príležitosti sa ukázalo, že **štvrtá vec, ktorú som ti opakovane
tvrdil, pravda nebola** — o tom na konci.

---

## 1. Upozornenia sa pri prvom zapnutí neuložili

**Toto bola z tých troch najhoršia**, lebo vyzerala ako úspech.

Karta počítala `configured` takto:

```ts
const configured = desktopEnabled || ntfyEnabled;
```

Lenže `desktopEnabled` a `ntfyEnabled` sú **rozpísané checkboxy formulára**,
nie uložený stav. A vykreslenie je:

```tsx
{configured && !editing ? <zhrnutie/> : <formulár/>}
```

`editing` štartuje na `false` a pri načítaní sa nikdy nenastaví.

Takže na čistej inštalácii:

1. Nič nie je nastavené → `configured` false → ukáže sa **formulár**
2. Klikneš prvý checkbox → `configured` naskočí na **true**
3. `editing` je stále false → podmienka splnená → **vykreslí sa zhrnutie**
4. **Tlačidlo Save zmizlo spolu s formulárom**
5. Odznak ukazuje „Enabled"

Neuložilo sa nič. A nemáš ako to zistiť, lebo obrazovka tvrdí opak.

**Oprava** — jeden riadok, ktorý opraví všetky tri použitia naraz:

```ts
const configured = !!status && (status.desktopEnabled || status.ntfyEnabled);
```

„Je to nastavené" je fakt o tom, **čo je uložené**, nie o tom, čo práve píšeš.
Odznak má ukazovať živý stav, prepínanie formulár/zhrnutie sa nemá hýbať pod
rukami, a Cancel má byť len vtedy, keď je kam sa vrátiť.

Zaujímavé: `AiFeaturesCard` o tisíc riadkov vyššie to nikdy nemala, lebo robí
`setEditing(!c)` z **načítanej** hodnoty. Vzor na to existoval, len sa sem
nedostal.

---

## 2. Obnova staršej kópie z Drive nemala potvrdenie

`doRestoreRevision` sa volalo **rovno z tlačidla**. Prepísalo celú databázu
a o 900 ms reštartovalo appku.

Pritom obnova zo súboru **hneď vedľa** sa vždy pýtala — ten ConfirmDialog
tam je od začiatku. Rovnaký dôsledok, iná cesta, a len jedna z nich sa pýtala.

**Oprava:** pridaný ConfirmDialog rovnakej váhy. Pomenuje dátum tej revízie
a povie, že to, čo si medzitým urobil a nesynchronizoval, prežije len
v bezpečnostnej kópii — ktorá sa naozaj robí a objaví sa pod Restore points.

---

## 3. „Combine both" tvrdilo, že sa nič nemaže

Text hovoril:

> *Combining keeps everything — nothing is replaced and nothing is deleted.*

**Nie je to pravda od 2.20.0**, kde pribudlo spracovanie tombstonov. Zlúčenie
odstráni riadky, ktoré si **zmazal na druhom počítači**. Presne to počíta
`MergeOutcome.totalDeleted`.

A to pole bolo v type napísané **dve polia pod** komentárom, ktorý tvrdil, že
sa nič nemaže.

Horšie: **`totalDeleted` sa nikde nezobrazovalo.** Ani vo výsledkovom paneli,
ani v toaste. Takže zlúčenie mohlo zmazať riadky úplne ticho.

**Opravené na štyroch miestach:**
- text pri tlačidle — teraz hovorí, že zlúčenie nič neprepíše, **ale aplikuje
  zmazania, ktoré si urobil na druhom počítači**
- výsledkový panel — pribudol riadok, koľko toho odstránilo a prečo
- toast — to isté, jednou vetou navyše
- doc komentár `MergeOutcome` v `types.ts` — to falošné tvrdenie je preč

---

## 4. Oprava toho, čo som ti tvrdil ja

Opakovane som písal, že **Mixed Money** je najdôležitejšia vec, lebo príznak
zmiešanej meny vraj nikto nečíta a zoznam eventov vypisuje zisk zo sčítania
eur a korún.

**Nie je to pravda a nikdy nebola.**

- `Events.tsx` posiela Cost, Revenue aj Profit cez `formatMoneyOrMixed`,
  ktorý pri `currency === null` vypíše **„Mixed"**
- `EventDetail.tsx` má na to celú vetvu s jantárovým vysvetlením
- `Recap.tsx` to rieši tiež

Ako som sa k tomu dostal: prečítal som komentár pri type, **domyslel si, že
to nič nečíta**, a nenašiel som si konzumentov. Potom som tú istú vetu napísal
do zadania agentovi, agent mi ju vrátil ako zistenie, a ja som si ju odobril
vlastným tvrdením.

Opravené v `REDESIGN-OVERVIEW.md`, v `EVENTS-LAB-LOG.md` aj v zipe.

---

## Čo som netestoval

Na tomto Macu nie je `npm` ani `cargo`, takže **nič z toho som nespustil**.

Overené je: že každý vložený blok je sám o sebe vyvážený (zátvorky, hranaté,
zložené — všetkých šesť na nule), a že všetky tri chyby existovali presne tak,
ako som ich popísal — prečítané v zdroji, nie prevzaté.

Celosúborový sken zátvoriek hlási problém, ale hlási ho **aj v súboroch,
ktorých som sa nedotkol** — nevie si poradiť s JSX. Nie je to teda dôkaz ani
v jednu stranu a nevydávam ho za jeden.

**Prvý skutočný kompilátor, ktorý tento kód uvidí, je CI.**

---

## Súbory

- `src/pages/Settings.tsx` — `configured` zo `status`, ConfirmDialog na Drive
  revízie, text pri zlúčení, `totalDeleted` v paneli aj v toaste
- `src/lib/types.ts` — doc komentár `MergeOutcome`

Deväť výskytov verzie v siedmich súboroch na **2.66.0**.

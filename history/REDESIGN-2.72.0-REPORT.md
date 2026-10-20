# TIQR Manager 2.72.0

**Dátum:** 9.10.2026
**Zadania:** restrictions po anglicky a len základné · v sales ukazovať len budúce eventy

---

## 1. Restrictions po anglicky

Celý zoznam prepísaný a skrátený:

| kód | čo sa zobrazí v zozname | skratka v bunke |
|---|---|---|
| `rv` | **Restricted view** | Restricted |
| `18` | **18+** | 18+ |
| `16` | **16+** | 16+ |
| `id` | **ID required** | ID |
| `aw` | **Away fans only** | Away end |
| `nr` | **No re-entry** | No re-entry |
| `st` | **Standing** | Standing |
| `wc` | **Wheelchair space** | Wheelchair |

Prvých päť sú tie, ktoré stoja peniaze, keď sa prehliadnu - preto sú prvé,
rovnako ako predtým.

**Čo pribudlo:** `Away fans only`. To je tá futbalová, ktorá na resale reálne
bolí - miesto v sektore hostí predané domácemu fanúšikovi je refund aj otočený
kupec pri vstupe. Pri tvojich eventoch (ESPAÑA–INGLATERRA, Liverpool, UEFA) to
nie je teória.

**Čo vypadlo:** `Maloletý len s dospelým`. 16+ a 18+ už vekovú otázku nesú
a pýtal si základný zoznam, nie úplný.

**Prečo sa to dalo prepísať naraz:** overil som, že v databáze **nemáš ani
jeden lístok** s nastaveným restriction - nula riadkov s neprázdnym
`restrictions_json`. Takže vypadnutím kódu sa nič neosirotilo. Keby tam niečo
bolo, vypadnutý kód by sa stále zobrazil (zobrazí sa sám seba), len by sa
nedal nanovo vybrať.

`clean_restrictions` v Ruste nemá vlastný zoznam povolených kódov - len
trimuje, zmenšuje písmená a dedupuje. Takže zoznam žije na jednom mieste
(`src/lib/restrictions.ts`) a toto bola čisto frontendová zmena.

Popisky polí `Obmedzenia` -> `Restrictions` v oboch editoroch.

**Ak chceš ďalšie,** povedz a dopíšem - napadajú ma ešte *Home fans only*,
*Membership required* (treba klubovú kartu) a *Obstructed view* ako jemnejší
stupeň než restricted. Nedal som ich tam sám, lebo si pýtal základné.

## 2. V sale forme len to, čo sa ešte neodohralo

Zoznam objednávok vo formulári predaja teraz ukazuje len eventy s dátumom
**dnes alebo neskôr**.

**Dve veci sa zámerne neskrývajú:**

1. **Objednávka bez dátumu** (TBD event) nemá dátum, ktorý by mohol byť
   v minulosti - ostáva v zozname. Neskrývať niečo preto, že údaj chýba.
2. **Keď niečo napíšeš do hľadania**, zhody sa zobrazia bez ohľadu na dátum.

Tá druhá je dôležitá a chcem ju povedať nahlas: keby filter platil aj na
hľadanie, **nemal by si ako zapísať oneskorený predaj** na event, ktorý sa už
odohral. To sa stáva. Takže predvolený zoznam je "čo môžeš teraz predávať",
ale keď si niečo vypýtaš menom, dostaneš to.

Tlačidlo **"Add sale"** na detaile objednávky funguje aj pre odohratý event -
tá záchrana hľadá v nefiltrovanom výsledku.

## 3. Čo je otestované a čo nie

**Overené:**

- 0 lístkov s nastaveným restriction v tvojej databáze (kópia, originál som
  neotvoril na zápis) - preto bolo bezpečné zoznam prepísať.
- `clean_restrictions` (tickets.rs:205) nemá allow-list, takže Rust sa
  nemusel meniť.
- `todayIso` už bolo v `SaleRowsModal` importované; `eventDate` je na
  `OrderRecord` a je nullable (types.ts:30).
- Zátvorky v `restrictions.ts` aj `SaleRowsModal.tsx`: rozdiel oproti
  netknutým zálohám 0/0/0.
- Verzia 2.72.0 vo všetkých 9 výskytoch v 7 súboroch.

**Neoverené:**

- **Neskompilované** - node ani cargo tu nie sú, prvý kompilátor je CI.
- Ani jednu z týchto dvoch zmien som nevidel bežať. Po builde sa pozri na
  dve veci: či v New order dropdowne vidíš anglické názvy, a či sa ti
  v New sale v zozname objednávok neobjaví event, ktorý už bol.

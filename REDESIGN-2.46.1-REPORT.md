# TIQR Manager 2.46.1 — riadky sa zmestia a Ks je zase číslo

> „tie ks su zle nastavene odstran tam to puzdro a urob to tak ze tam vidno ten
> pocet a urob to tak aby sa tam vsetko zmestilo miesta je tam dost a kus pomen
> ten design nech je taky ostry a profesionalny"

Mal si pravdu v oboch veciach. Zmeral som to.

---

## 1. Ks už nie je zhasnuté políčko

Keď napíšeš sedadlá, počet kusov z nich vychádza — to je správne a nemenil som
to. Ale v 2.46.0 som ti to číslo ukazoval **v read-only políčku**, a to je zlé:
zhasnuté políčko vyzerá ako ovládací prvok, ktorý ťa odmieta. Číslo má vyzerať
ako odpoveď, nie ako zakázané pole.

Teraz je tam **samotné číslo**, zarovnané doprava, tabuľkovými číslicami. Keď
sedadlá nenapíšeš, políčko na písanie počtu je tam ďalej presne ako predtým.

## 2. Naozaj sa to nezmestilo

Toto nebol dojem, dá sa to spočítať. Okno je `max-w-6xl` s `px-5`, čo dáva
**~1112 px** využiteľnej šírky. Sčítal som šírky stĺpcov:

| Formulár | Predtým | Pretekalo o | Teraz |
|---|---|---|---|
| Inventory | 1296 px | 184 px | **1112 px** |
| **Pulls** | **1426 px** | **314 px** | **1112 px** |
| Events | 1206 px | 94 px | **1112 px** |
| Sales | 966 px | — | 1054 px |

Pull pretekal o 314 px. Formulár, ktorý pretečie, to nepovie nahlas — len
stlačí každú bunku, až celok vyzerá lacno. Presne to si videl.

Teraz má každý formulár rozpočet: **28 px číslo riadku + 1020 px dáta + 64 px
ikonky = 1112**. Zapísal som to do `PROTECTED_AREAS.md`, aby sa pri pridaní
stĺpca muselo najprv ubrať inde — čo som ja v 2.46.0 neurobil.

## 3. Ostrejší design

- **Hustejšie bunky.** Prepol som ich na kompaktný krok, ktorý appka **už má**
  (`px-2 py-2` namiesto `px-3 py-2.5`). Nevymýšľal som nové CSS, len som použil
  to, čo tu bolo — a práve tým sa získala časť tej šírky.
- **Čísla a ich hlavičky zarovnané rovnako, doprava.** Číslo vpravo pod
  nadpisom vľavo je klasická známka tabuľky, ktorú nikto poriadne nenastavil.
  Platí to pre Ks a Cena/ks v Inventory, Ks a Tvoja odmena v Pulls, a pre
  všetky štyri peňažné stĺpce v Sales.
- **Drobnejšie ikonky** a užšie krajné stĺpce — číslo riadku 28 px, akcie 64 px.

---

## Čo som overil

- **Šírky spočítané strojovo**, formulár po formulári: 1112 / 1112 / 1112 /
  1054. Počet hlavičiek **sedí s počtom buniek** vo všetkých štyroch.
- **Zoznam súborov** oproti 2.46.0: **0 pribudlo, 0 zmizlo, 5 zmenených.**
- **Zátvorky aj JSX značky** proti 2.46.0 — **identické** vo všetkých piatich.
- **Žiadny nepoužitý import**, každý import dohľadateľný v exporte.
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad (Node ani Rust tu nie sú) a ako presne ti to
sadne oku — šírky sú spočítané na plnú šírku okna, takže ak budeš mať appku v
menšom okne, tabuľka sa odroluje nabok namiesto stlačenia. To je zámer.

---

**Verzia:** 2.46.1 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.

# TIQR Manager 2.47.4 — Profit karta už nie je fialová

> „na light mode je tam chybna farba pri tom profit widgete"

---

## Čo tam bolo

Karta Profit mala `border-brand-300 bg-brand-50/40`. V tom jednom riadku boli
**dve chyby naraz**.

### 1. Rámik nikdy neexistoval

`.card` má v CSS natvrdo `border: 0`. Takže `border-brand-300` nastavilo
**farbu rámika, ktorý nemá žiadnu hrúbku** — nenakreslilo to nič, nikdy, od
verzie 2.13.3. Jediné, čo z tej dvojice reálne fungovalo, bola výplň.

### 2. Tá výplň je sýta levanduľa

`brand-50` je `#e4ddfd`. Ako **jediná zafarbená dlaždica v rade štyroch
bielych** to nečítaš ako „toto je dôležité", ale ako „toto je vybrané". A
zelené číslo zisku na nej navyše s tou farbou bojovalo.

Na dark mode to problém nebol — tam je tá istá vec `brand-500/[0.06]`, čo je
sotva viditeľný závoj. Preto ti to vadilo len na svetlom.

## Čo je tam teraz

**Neutrálny tenký krúžok** (`ring-1`) namiesto výplne. Číslo zostáva väčšie —
24 px oproti 19 px na ostatných kartách — takže hlavný údaj nájdeš okamžite,
ale nič sa nebije.

Krúžok som použil naschvál namiesto rámika: **nepotrebuje hrúbku, aby
existoval**, takže sa nemôže zopakovať chyba č. 1. A na rozdiel od farby pozadia
ho nemôže prebiť vlastné pozadie karty.

## Čoho som sa nedotkol

Prešiel som všetky fialové miesta v appke. Ostatné použitia `bg-brand-50` sú
**lišty výberu, čipy a oznamy** — tie majú byť fialové, lebo to sú akcie
a upozornenia, nie údajové dlaždice. Nechal som ich.

`emphasis` má v celej appke **jediného volajúceho** — práve tú Profit kartu —
takže sa zmenila presne tá jedna dlaždica a nič iné.

---

## Čo som overil

- **Zoznam súborov** oproti 2.47.3: **0 pribudlo, 0 zmizlo, 1 zmenený.**
- **Zátvorky** proti 2.47.3 — identické.
- **Jediný volajúci `emphasis`** strojovo overený.
- **Verzia na 9 miestach v 7 súboroch.**

**Čo overiť nedokážem:** preklad a ako presne ti ten krúžok sadne oku. Ak chceš
Profit kartu zvýrazniť inak — napríklad zelenkastým nádychom podľa toho, či si
v pluse — poviem si a spravím to.

---

**Verzia:** 2.47.4 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.

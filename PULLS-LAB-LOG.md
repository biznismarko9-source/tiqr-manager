# PULLS LAB — denník rozhodnutí

Preview: https://claude.ai/artifact/5v4exfBmEyUqGKzVWoUiJJ
Sesterské denníky: [NOTES-LAB-LOG.md](NOTES-LAB-LOG.md) ·
[FINANCE-LAB-LOG.md](FINANCE-LAB-LOG.md) · [SETTINGS-LAB-LOG.md](SETTINGS-LAB-LOG.md) ·
[DASHBOARD-LAB-LOG.md](DASHBOARD-LAB-LOG.md)

---

**01 — Register sa pri každej oblasti neotvára znova.**
Tento súbor kopíruje THE REGISTER z Notes labu bez zmeny jedinej
hodnoty. To je zmysel toho, že sme ho zapísali.

**02 — Najprv čítať kód, až potom kresliť.**
Settings lab stál na domnienke o synchronizácii, domnienka bola zlá a
musela sa opravovať potom. Tu som najprv prečítal skutočný kód.

**03 — Čo Pulls naozaj je.**
Nie je to skladová vec, je to **služba** vedľa prepredaja, v dvoch
zrkadlových smeroch:

- **Pulls given** — použiješ svoj prístup a kúpiš lístky *pre niekoho
  iného*, **na jeho kartu**. Cena lístka teda nikdy nie sú tvoje
  peniaze. Tvoj je len **poplatok**.
- **Pulls received** — naopak, niekto pullne pre teba a ty mu platíš.

**04 — Dve nezávislé povinnosti, v ľubovoľnom poradí.**
*Preposlané* a *zaplatené* sa dokončia každá inokedy. Preto je v
tabuľke stĺpec „Paid · Done". Zaujímavé sú zmiešané stavy, a
najnebezpečnejší je **preposlané ale nezaplatené** — lístky sú preč a
peniaze nie sú dnu.

**05 — Nič v Pulls sa nikdy nesčítava, a poplatky nevidí Finance.**
Zoznam nemá jediný súčet. Poplatky z received sú v kóde označené ako
*informational only* — nedostanú sa do Profit ani Revenue. Takže reálne
peniaze, ktoré si zarobil aj zaplatil, nevidí žiadny súčet v apke.

**06 — Sheet je len jedným smerom, a to je v poriadku.**
`sync_pulls` číta sheet → apka. Spätný zápis zámerne nie je postavený.
Keď sa ten istý pull zmení na oboch stranách, **zistí sa to a nahlási —
nikdy sa ticho neprepíše.** `Paid` v sheete nie je vôbec, takže sheet a
apka sa o peniazoch rozchádzajú zámerne. Žiaden návrh tu nepíše do
tvojho živého sheetu.

**07 — Desať návrhov.**
The List (to, čo máš, plus štyri súčty) · By Person · **Either Order**
(štyri skutočné stavy ako rozloženie; hore vľavo tá kopa, čo nesmie
ticho rásť, s vekom najstaršej položky) · **Three Day Line** (jediné
hodiny, ktoré si nenastavuješ, sú os) · **Collection Run** (jedna osoba
naraz, text do schránky — nikam sa nič neposiela) · **One Way In**
(stôl na sheet; **žiadne tlačidlo push, zámerne**; zoznam polí, ktoré
existujú len v apke) · **Mirror Split** (obe strany naraz, obe nohy tej
istej láskavosti) · **Pull Sheet** (More info konečne dostal výšku —
tam žije skutočný pokyn) · **Phone Capture** (formulár *je* obrazovka,
a stráži „ešte dva pre Andreja") · **State Trail** (zmeny namiesto
riadkov; kde sú obe značky, vypíše rozostup medzi nimi).

**08 — Dvanásť funkcií, najsilnejšie hore.**

1. **Fees Owed** — jediná otázka, na ktorú apka dnes nevie odpovedať.
   Súčty **po menách, nikdy nie spolu** — jedno zlúčené číslo cez EUR a
   CZK by bola lož. A pod tým riadok „2 riadky sa nepočítajú".
2. **Needs Me Today** — tá trojdňová oranžová stíchne presne vtedy, keď
   je riziko najväčšie: po evente sa poplatok nezaplatený šesť týždňov
   prestane hlásiť navždy. A mlčí aj o najhoršom stave — máš jeho
   peniaze a lístky si neposlal.
3. **Receipts** — `paid` prestane byť áno/nie a stane sa sumou s
   dátumom. Bez toho nie je pravdivé ani 1, ani 2, ani 8. Migrácia
   starých fajok nesie dátum **„neznámy"**, nie vymyslený.
4. **Totals Audit** — ten istý incident s € znova, tentoraz namierený na
   číslo, ktorému by si veril pri peniazoch. Chytí aj zlyhanie samotného
   varovania: pull s dátumom, ktorý sa nedá prečítať, nikdy nezoranžovie.
5. **Buyer Page** · 6. **Chase** (text do schránky, **apka neposiela
   nič**) · 7. **Event Cancelled or Moved** · 8. **Pull Fees into
   Finance** (vypnuté, len prijaté peniaze, a riadok so zlou menou sa
   **nahlas odmietne**, nie ticho preskočí) · 9. **Sync Preview** ·
   10. **The Other Machine** · 11. **Who I Owe** · 12. **Same Person,
   Different Spellings**.

Nič z toho nebeží na pozadí, nič nepíše do živého sheetu, nič si
nedomýšľa číslo, ktoré si nezadal, a nič neradí.

---

## Rozhodnuté

**Pulls ostáva tak, ako je.** 30.09. — lab je hotový a slúži ako
zoznam, ku ktorému sa dá vrátiť. Nič sa teraz nestavia.

## Otvorené

- Ktorý návrh a ktoré funkcie, ak sa k tomu vrátime.
- Či poplatky z Pulls majú ísť do Finance. Úprimná odpoveď je: **áno pre
  prijaté peniaze, nikdy pre dohodnuté** — preto to musí stáť na
  Receipts a nie na tej fajke.

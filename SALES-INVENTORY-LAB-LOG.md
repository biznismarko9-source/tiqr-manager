# SALES & INVENTORY LAB — denník rozhodnutí

Preview: https://claude.ai/artifact/6H1cSsSDq9CfUWyG7sh66j
Sesterské denníky: [EVENTS-LAB-LOG.md](EVENTS-LAB-LOG.md) ·
[PULLS-LAB-LOG.md](PULLS-LAB-LOG.md) · [NOTES-LAB-LOG.md](NOTES-LAB-LOG.md) ·
[FINANCE-LAB-LOG.md](FINANCE-LAB-LOG.md) · [SETTINGS-LAB-LOG.md](SETTINGS-LAB-LOG.md) ·
[DASHBOARD-LAB-LOG.md](DASHBOARD-LAB-LOG.md)

---

**01 — Piata oblasť, register sa stále nepohol.**

**02 — Restrictions už existujú a žijú na LÍSTKU. To nie je náhoda.**
Pýtal si si možnosť vybrať restriction pri novej objednávke. Dátový
model ich zámerne drží na **sedadle**, nie na objednávke — a migrácia
037 to hovorí vlastnými slovami:

> *one order can easily be two clear-view seats and two behind a pillar*

Takže odpoveď nie je presunúť ich na objednávku. Odpoveď je: **pri
vytváraní nastavíš predvoľbu, ktorú prevezme každý vygenerovaný
lístok, a ktorýkoľvek jeden sa od nej môže líšiť.** To robí návrh 02.
Máš tak jednoklikový prípad aj čestný prípad na jednej obrazovke.

Zmena predvoľby vymaže jednotlivé výnimky, aby si tie dve veci nikdy
ticho neodporovali.

**03 — Štyri z ôsmich obmedzení stoja peniaze, keď sa prehliadnu.**
Apka si ich sama označuje: obmedzený výhľad, 18+, 16+, na meno/doklad.
Sedadlo s obmedzeným výhľadom predané ako normálne je reklamácia; 18+
lístok predaný niekomu, kto ho nemôže použiť, je reklamácia **a
hádka**. V každom výbere tu idú prvé a majú výstražný tón.

**04 — Sales majú tri nezávislé stavy.**
Sold · Deliv. · Paid, dokončia sa v ľubovoľnom poradí. Najhoršia
kombinácia je **doručené a nezaplatené** — lístky sú preč, peniaze nie
sú dnu — a to je jediná, ktorá sa zhoršuje sama od seba. V zozname je
stav vypísaný slovami vedľa tých troch fajok.

**05 — Tri čestné riešenia tej tenzie, a každý návrh povie, ktoré berie.**
1. **Iba opečiatkovať** — pri vytváraní sa kódy zapíšu na N lístkov a na
   objednávke neostane nič. Nemôže vzniknúť rozpor, lebo existuje len
   jedna kópia pravdy. Cena: nedá sa opečiatkovať znova neskôr.
2. **Šablóna na objednávke** — kúpi to opätovné pečiatkovanie, ale cena
   je vysoká: **druhá kópia dát, ktorá vyzerá autoritatívne**. Presne
   taký tvar mal incident s €. Ak sa to robí, tá šablóna nesmie **nikdy**
   vstupovať do žiadneho súčtu ani filtra.
3. **Bloky** — objednávka je zoznam rovnorodých blokov, množstvo je ich
   súčet. Najvernejšie tomu, ako naozaj nakupuješ. Najdrahšie.

Návrhy 02 a 03 berú riešenie 1, návrh 04 berie riešenie 3.

**06 — Pätnásť návrhov.**
*Inventory:* The List · **New Order** · **Ticket Grid** (klik na hlavičku
stĺpca nastaví kód všetkým, klik na jednu bunku spraví výnimku — ten istý
ovládač v dvoch výškach) · **Blocks** · **Cassette** (osem obmedzení
v deviatich znakoch; veľké písmeno = všetky lístky, malé = niektoré) ·
**Expand to Seats** · **Ledger** (každý súčet povie, z koľkých riadkov je)
· **Checks Rail**.

*Sales:* The List · **Owed Ledger** (dva záväzky, dvoje hodiny) ·
**Missing First** (pásma, ktoré žiadne triedenie nerozbije) · **Delivery
Ladder** (ktoré obmedzenie viaže — okno platformy, alebo samotný event) ·
**Group Expander** (kde sa lístky v skupine nezhodnú, vypíše *mixed(2)* —
nikdy priemer) · **State Columns** (každý stav má vlastný dátum, takže
poradie dokončenia je čitateľné) · **Disclosure Review**.

**07 — Dvanásť funkcií.**
Dve dvojice z tých dvoch správ boli to isté a **zlúčil som ich, namiesto
aby som nimi nafukoval zoznam**: audit meny a čestné súčty sú jedna vec
(*Currency Guard*), a kontrola zverejnenia obmedzenia je jedna funkcia,
nie jedna na každú oblasť.

Najsilnejšie: **Restriction Stamp** (a jeho dry-run odmietne prepísať
lístok, ktorý si ručne upravil) · **Undisclosed Sale Check** · **Currency
Guard** (súčet, ktorý povie, z koľkých riadkov je — lebo incident s €
sa neprejavil ako zlé číslo, ale ako **neprítomnosť**, a tá nemá pixely)
· **Delivery Deadline** · **Double-Sold Seat**.

**08 — USADENÉ: Restriction Stamp a Bulk Change Preview.**
30.09. Obe sú o tom istom z opačných koncov: **spraviť správnu vec
mnohým riadkom naraz**, a **nespraviť tú zlú**.

*Restriction Stamp* zapíše sadu obmedzení na každý lístok, ktorý
objednávka vygeneruje, a **odmietne prepísať lístok, ktorý si upravil
ručne** — to je presne to, čo ho robí bezpečným voči modelu, ktorý drží
pravdu na sedadle.

*Bulk Change Preview* ukáže, čo by hromadná zmena spravila, **skôr než
to spraví** — koľko riadkov by sa stalo protirečivými, koľko by sa
zapísalo ako oneskorené — a poslednú vie vrátiť späť.

Obe majú dry run, a to nie je náhoda: sú to jediné dve funkcie tu, čo
sa dotýkajú veľa riadkov naraz, a presne taká akcia nemá byť nikdy tichá.

---

## Otvorené

- Ktoré návrhy a ktoré funkcie.
- Ktoré z tých troch riešení tenzie ideme reálne stavať.

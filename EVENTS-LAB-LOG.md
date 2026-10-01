# EVENTS LAB — denník rozhodnutí

Preview: https://claude.ai/artifact/L8wr36ZAcH716omD7uBk9q
Sesterské denníky: [PULLS-LAB-LOG.md](PULLS-LAB-LOG.md) ·
[NOTES-LAB-LOG.md](NOTES-LAB-LOG.md) · [FINANCE-LAB-LOG.md](FINANCE-LAB-LOG.md) ·
[SETTINGS-LAB-LOG.md](SETTINGS-LAB-LOG.md) · [DASHBOARD-LAB-LOG.md](DASHBOARD-LAB-LOG.md)

Dve úrovne: **zoznam všetkých eventov** a **vnútro jedného eventu**.

---

**01 — Štvrtá oblasť a register sa nepohol.**
THE REGISTER je ten istý súbor — Notes, Pulls a teraz obe úrovne
Events. Ani jedna hodnota sa nezmenila. Presne na to design system je,
a preto tieto laby teraz trvajú popoludnie a nie týždeň.

**02 — Tvoje vlastné rozhodnutie o stĺpcoch sa rešpektuje.**
V kóde je napísané, že tabuľka eventov ukazuje Event, Date, Status,
Tickets, Stock, Cost, Revenue, Profit „a nič iné" — a že Available,
Days, Margin a ROI si odtiaľ **zámerne vyhodil v 2.33.0**. Bolo to tvoje
rozhodnutie, takže žiadny návrh tu nedáva margin ani ROI späť do hlavnej
tabuľky. Sú na stránke eventu, kde si povedal, že patria.

**03 — OPRAVA: zmiešaná mena je už vyriešená, tvrdil som to zle.**
Napísal som sem, že príznak `currency` nič nečíta. Nie je to pravda.
`Events.tsx` posiela Cost, Revenue aj Profit cez `formatMoneyOrMixed`
(vypíše **„Mixed"**), `EventDetail.tsx` má na to jantárovú vetu, Recap
tiež. Je to hotové.

Návrhy v tomto labe, ktoré zmiešanú menu ukazujú ako **mixed** a odmietajú
kresliť pruh, teda nič neopravujú — **zhodujú sa s tým, čo apka už robí**.
To je stále dobré (Currency Lanes a Order Batches idú ďalej než dnešok),
ale nie je to oprava chyby.

**04 — Riziko nesmie byť farba.**
Register dovoľuje presne jeden akcent a ten patrí výberu. Takže
v žiadnom návrhu tu nie je červený riadok ani oranžové varovanie —
riziko nesie **poradie, sekcia a pozícia**. Ukázalo sa to ako lepšie
obmedzenie, než to znelo: zoznam zoradený podľa toho, čo horí, povie
viac než zoznam s farebnými bodkami.

**05 — Dvadsať návrhov, desať na každú úroveň.**

*Zoznam:* The Table · **Today Line** (vlasová čiara, čo sa prilepí na
ten okraj, ku ktorému scrolluješ; na nej počet eventov, ktorým dátum
prešiel a stav je stále „upcoming") · **Unsold Gate** (dni a nepredané
ako **jeden objekt** — tá dvojica je riziko a žiadny stĺpec to nepovie)
· **Cost Outstanding** (pruh je **hotovostná pozícia, nie pomer**, a pri
zmiešanej mene sa **odmietne nakresliť**) · **Status Board** (tri
oddelené súčty, lebo *cancelled nie je menšie completed*) · **Currency
Lanes** (príznak meny ako organizačná os; v pruhu je mena jediná, takže
tie súčty sú jediné naprieč eventmi, ktoré sú aritmeticky pravdivé) ·
**Close Out** (obrazovka, ktorej účelom je vyprázdniť sa) · **Dense
Stack** · **Stock Spine** · **Working List** (margin a ROI ostávajú mimo
stĺpcov, ale prestanú vyžadovať prekliknutie).

*Vnútro:* One Page · **Capital Position** (kedy tvoj výdaj vrcholil,
a či si už na nule) · **Ticket Lives** (lístok ako objekt so životom) ·
**Books Tie-Out** (súhrn proti riadkom pod ním; buď je ten rozdiel nula
alebo nie) · **Date Line** · **Open Loops** · **Order Batches** (mena
patrí objednávke, takže tu problém zmiešanej meny prestane existovať) ·
**Event Log** · **Event Day** (veľké písmo, zoradené sekcia–rad–sedadlo,
teda v poradí lístkov v ruke) · **Cost Bands** (ktorá tvoja nákupná cena
je zaseknutá).

**06 — Dvanásť funkcií.**
**Currency Check** je ten incident s € namierený na celú databázu — to je
tá skutočná. **Mixed Money** je v zozname zbytočné: apka to už rieši
(viď 03).

Ďalej: Unsold Before The Date · Money Not Landed · Does This Event Add
Up? · Close The Event (a v náhľade vidno, ako zisk spadne z 510 na 90,
keď sa mŕtvy sklad konečne odpíše) · Same Event Twice · Cancelled or
Moved · What Changed Since The Date · Bottom Line For This List · How
This One Went · Do Both Machines Agree?

**07 — USADENÉ: 01 a 01.**
30.09. — *The Table* na zozname a *One Page* vnútri eventu. Teda tvojich
osem stĺpcov tak, ako si ich zvolil, plus čestnosť pri zmiešanej mene —
a vnútro eventu **bez záložiek**.

---

## Otvorené

- Ktorý návrh na ktorej úrovni a ktoré funkcie.
- Čo s tým príznakom zmiešanej meny urobíme v produkcii. To je jediná
  vec z celého labu, ktorá opravuje číslo, ktoré je **zlé už teraz**.

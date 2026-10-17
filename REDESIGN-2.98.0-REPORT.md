# TIQR Manager 2.98.0 — päť zo siedmich

Dal si sedem vecí. **Päť je hotových a overených**, dve nechávam na vlastný
kus a nižšie píšem prečo — nie je to odkladanie, je to iná veľkosť práce.

---

## 1. Lístky v evente → objednávky

**Toto bola skutočná chyba, nie vzhľad.**

Odkaz na lístku viedol na `/orders?code=<kód LÍSTKA>`. Lenže Inventory hľadá
kódy **objednávok**. Preto ťa klik hodil do Inventory a nenašiel nič — presne
ako si písal.

Prečo ten odkaz vôbec mieril nabok: **lístok nemá vlastnú stránku.** Nebolo
kam ho poslať. Objednávka stránku má, takže riadok je teraz objednávka a klik
ťa na ňu hodí.

| | pred | po |
|---|---|---|
| riadok | lístok | **objednávka** |
| stĺpce | Ticket · Seat · Cost · Listing · Status | **Order · Tickets · Seats · Cost · Status** |
| klik | Inventory, nič nenájde | **detail objednávky** |

Dve veci, ktoré som pri tom ustrážil:

- Zoskupenie kľúčuje na **`orderId`**, nie na kód. Kód je zobrazovací reťazec
  a dve objednávky by ho teoreticky mohli mať rovnaký.
- Mena sa nastaví **len keď sa všetky lístky v objednávke zhodnú.** Jedna
  objednávka môže niesť dve meny — vtedy sa vypíše **„Mixed"**, nesčíta sa.
  Rovnaké pravidlo, aké drží zvyšok appky.

Stavy sa zlučujú tiež: objednávka s troma lístkami ukáže „2 sold · 1
available", nie tri riadky.

---

## 2. Odstránené

- **Potential Profit** (detail eventu) — bolo to posledné miesto v appke, kde
  svietili peniaze, **ktoré nikto nezarobil**. Čo by lístok mohol utŕžiť, nie
  je číslo, ktoré patrí vedľa skutočných.
- **Paid a Outstanding** (detail objednávky)
- **celý pás „Order summary"** — čím sa zoznam lístkov posunul hore, rovno
  pod hlavičku objednávky

Výpočty za nimi som odstránil tiež. Boli to tri `reduce` a jeden `Set` cez
všetky lístky eventu **pri každom renderi** pre číslo, ktoré sa už nekreslí.

**Jedna výnimka:** `salesSummary` sa stále načítava a nič ho nečíta. Nechal
som ho zámerne — je to jediné volanie, ktoré pozná skutočné tržby objednávky,
a je to prvá vec, ktorú bude niekto chcieť späť. Napísal som to do kódu, aby
to nebolo záhadou.

---

## 3. Pulls na spodok navigácie

---

## 4. Dve moje chyby, ktoré som chytil pred vydaním

**Phantom typ.** Do importu som pridal `SaleEntry` — typ, ktorý v projekte
**neexistuje**. Neskompilovalo by to. Našiel som to tak, že som si každý
importovaný typ overil proti `types.ts`, nie že by som si bol istý.

**Zlý komponent.** Nový výpočet `ticketsByOrder` mi pristál v `SalesTab`,
pritom sa používa v `OverviewTab`. Chyba „Cannot find name" a zhodený build.
Vkladal som ho na komentárovú kotvu — a tá kotva bola v **susednej funkcii**.

Poučenie, ktoré si zapisujem: pri vkladaní na kotvu treba overiť, v ktorom
komponente to naozaj skončí. Napísal som si na to kontrolu, čo vypíše
deklaráciu aj použitia aj s ich komponentom.

---

## 5. Čo ostáva — a prečo je to jedna práca, nie dve

Editačné okno v Pulls a zoznam podľa mena.

Keď som to otváral, našiel som **skutočnú príčinu** toho, čo ti na tom okne
vadí:

| sekcia | detail stránka |
|---|---|
| Events | `events/:id` ✓ |
| Inventory | `orders/:id` ✓ |
| Sales | `sales/:id` ✓ |
| **Pulls** | **žiadna** |

Každá ostatná sekcia má stránku, na ktorú klikneš a tam veci robíš. Pulls
jediné nemá — **preto sa všetko tlačí do jedného vysokého okna.** Nie je to
zle naštýlované okno, je to chýbajúca stránka.

Riešenie je dať Pulls `pulls/:id` ako majú ostatné. To je nová route, nová
stránka a presun formulára z okna na ňu — a **zoznam podľa mena patrí do
toho istého prechodu**, lebo bude z tej stránky vychádzať.

Spravím to ako vlastné vydanie. Rozpísať 1 736-riadkový súbor naslepo na
konci dlhého kola je presne to, čím som ti už dvakrát zhodil build.

---

## 6. Overenie

- vyrenderované pred/po (lístky verzus objednávky, vrátane riadku s „Mixed")
- vyváženie zátvoriek celého `src/`
- **kontrola scope**: deklarácia aj obe použitia v tom istom komponente
- **každý importovaný typ overený**, že naozaj existuje
- obe triedy komentárovej chyby: 0

Checkery som pred použitím opäť pustil na úmyselne pokazený súbor.

**Nie je to kompilácia.** CI je prvý kompilátor.

---

## 7. Verzia

2.97.0 → **2.98.0**, 9 výskytov v 7 súboroch plus `CURRENT_STATE.md`.

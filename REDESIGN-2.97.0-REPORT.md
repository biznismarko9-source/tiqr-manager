# TIQR Manager 2.97.0 — Inventory číta ako Events

## Čo si chcel

Aby riadok v Inventory vyzeral ako v Events: názov, vedľa neho kategória,
a pod ním miesto.

## Čo som našiel skôr, než som začal písať

Väčšina toho tam **už bola** — len sa nepoužívala:

| | stav |
|---|---|
| mesto pod názvom | **už od 2.60.0** |
| kategóriový odznak | dáta **od 2.0.27**, kreslili to Orders, Events aj Sales — Inventory nie |
| venue | **jediné naozaj nové** |

`Ticket.eventCity` má v kóde dokonca komentár *„shown under the event name in
Inventory"*. Takže polovica tvojej požiadavky bola hotová a druhá polovica
bola jeden chýbajúci stĺpec.

---

## Čo som spravil

**Odznak** — čisté UI, nula nových dát. Použil som presne ten istý guard,
aký už má Orders (`categoryName && categoryColorSlot !== null`), aby sa tie
obrazovky nerozišli.

**Venue** — `events e` už bolo v dotaze joinuté, takže:

- jeden stĺpec do `SELECT` (`e.venue as event_venue`)
- jedno pole do štruktúry `Order`
- jeden `row.get`
- jedno pole do TypeScript typu

**Žiadna migrácia. Žiadny nový join. Žiadna zmena výpočtu.**

**`eventPlace()`** spája venue a mesto **rovnakým spôsobom ako Events.tsx**.
To je zámer: keby som to napísal druhýkrát vlastnou rukou, tie dve obrazovky
sa raz rozídu v tom, ako to formátujú. Event, ktorý nemá ani jedno,
nevykreslí druhý riadok vôbec — nie prázdny. (Vidíš to na NFL Mexico, ktoré
má venue a nemá mesto.)

---

## Jedna vec, ktorú som si overil pred zásahom

Pridať pole do Rust štruktúry je bezpečné **len keď sa tá štruktúra skladá na
jednom mieste**. `Order` sa skladá na jedinom (`orders.rs:60`) — overené
greppom pred úpravou.

V **2.86.0** ma zhodil presne tento typ zmeny (E0063: chýbajúce pole), lebo
vtedy sa štruktúra skladala na viacerých miestach a ja som doplnil len jedno.
Ten počet sa oplatí overiť zakaždým.

---

## Overenie

Vyrenderované pred/po s tvojimi eventmi.

Checkery som musel postaviť nanovo (nová session ich zmazala) a **najprv som
ich pustil na úmyselne pokazený súbor** — vyváženie zátvoriek padlo na
vloženej zložke, komentárová kontrola na komentári, čo sa sám ukončí. Až
potom som im veril na skutočných súboroch.

Výsledok: `src/` vyvážené celé, obe triedy komentárovej chyby 0, typy a
prop-y odznaku overené proti jeho skutočnej signatúre.

**Rust som nemal čím skompilovať.** Tie tri zmeny sú malé a prečítal som si
ich spätne, ale prvý kompilátor bude CI.

---

## Verzia

2.96.0 → **2.97.0**, 9 výskytov v 7 súboroch plus `CURRENT_STATE.md`.

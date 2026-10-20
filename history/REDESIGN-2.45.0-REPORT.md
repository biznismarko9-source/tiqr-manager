# TIQR Manager 2.45.0 — jeden vyplňovač všade, a otvára sa nad zoznamom

Povedal si dve veci naraz a obe sú v tomto balíku:

> „vsade events, inventory, sales a pulls budu podobne tie vyplnovace udajov"

> „nechcem aby to ked kliknes na new order, sales, pull atd ako svoje okno ale
> tak aby to bolo nad orders, sales pulls atd a podtym rozmazane vidis ostatne
> orders"

A k tomu ešte:

> „vo finance ako je income vs expense widget tak urobit taky isty graf ako
> ktory je na dashboarde v overview"

---

## 1. Štyri formuláre, jeden tvar

Events, Inventory, Sales aj Pulls majú teraz **ten istý vyplňovač**: tabuľka
riadkov, pod ňou tlačidlo „pridaj ďalší", a dole jedna lišta — vľavo ti povie,
čo presne vznikne, vpravo Zrušiť a Vytvoriť.

Spoločné je len to: tabuľka, tlačidlo, lišta. Stĺpce si každý formulár drží
svoje, lebo objednávka a event nemajú čo mať rovnaké políčka.

### Inventory (nová objednávka)

Jedenásť polí, nič viac: **ks · typ · sektor · rad · sedadlá · platforma ·
cena za kus · mena · pull · poznámka**, plus event raz hore.

- **Pull je gulička.** Zhasnutá = nie. Rozsvietená = pullnuté a hneď vedľa
  vyskočí políčko na meno.
- **Dátum nákupu sa nepýta** — opečiatkuje sa dnešným dňom pri otvorení.
- **Čo nesedí, je vlastná objednávka.** Dva riadky sa zlúčia len vtedy, keď sa
  zhoduje sektor, rad, cena, mena, typ, platforma aj pull — **a sedadlá idú
  tesne za sebou**. `14-15` + `16` je jedna objednávka 14-16; `14-15` + `19`
  sú dve. Cena sa porovnáva ako **peniaze**, takže „201,00" a „201.00" je tá
  istá cena.

### Pulls

Jeden riadok = jeden pull. Event sa **píše**, nevyberá — nie je to tvoja
objednávka a v Events ho mať nemusíš. Ďalší riadok si event, dátum, platformu,
odmenu aj menu prevezme, takže traja ľudia na ten istý večer sú tri mená a nič
iné.

Cena lístka tu nie je a ani nebude — nie sú to tvoje peniaze. Je tam **tvoja
odmena**, presne ako doteraz.

### Sales

Tu sú riadky **skutočné lístky**, takže „ďalší riadok" znamená **„prihoď
lístky z ďalšej objednávky"**. Nájdeš objednávku, pridáš jej lístky, a
predajná cena sa predvyplní z toho, za koľko sú vylistované. Zisk vidíš hneď
v riadku. Platforma, mena, stav platby a kupec sú raz hore pre celý predaj.

### Events

Jeden riadok = jeden event, takže **turné napíšeš na jeden raz**. Ďalší riadok
prevezme kategóriu a krajinu — to majú spoločné; názov, miesto, mesto a dátum
sú to, čo sa mení.

Poznámky tu nie sú zámerne — to je text ku konkrétnemu eventu, nie stĺpec, a
zostáva na detaile eventu.

---

## 2. Žiadne vlastné okno — je to nad zoznamom

New Order bola od 2.42.0 celá stránka. **Už nie je.** Všetky štyri sú okno
**nad** svojím zoznamom a pod ním vidíš rozmazané ostatné riadky — presne ako
to robili Sales a Pulls predtým.

`/orders/new` zmizlo. Stará linka ťa pošle na Inventory a otvorí ti okno.
„New order for this event" z detailu eventu ťa tiež pošle na Inventory a event
ti predvyplní.

**Editovanie som nechal tak, ako bolo.** Event upravuješ na jeho detaile, pull
v jeho okne. Oba pôvodné formuláre sú nedotknuté.

---

## 3. Finance: Income vs Expenses je graf z Dashboardu

Dvojité stĺpce sú preč. Je tam **ten istý graf ako na Dashboard → Overview** —
tá istá plynulá krivka s výplňou, tá istá zvislá čiara a bodka pri prejdení
myšou, tie isté prepínače vpravo hore.

Prepínače sú tri: **Income · Expenses · Net**.

Čísla sa nemenili — sú to presne tie, čo kreslili stĺpce, len po mesiacoch ako
krivka. **Net** je príjmy mínus výdavky, teda presne karta „Net Cash Flow", ale
mesiac po mesiaci.

**Veľké číslo nad grafom je súčet za obdobie z kariet nad ním**, nie súčet
nakreslených mesiacov. Graf sa pri „All time" zastaví na posledných 24
mesiacoch, takže keby som sčítaval stĺpce, ukazovali by ti dve rôzne sumy na
jednej obrazovke.

---

## Čo som overil

- **Zoznam súborov** oproti balíku 2.43.0: 4 pribudli (štyri formuláre),
  1 zmizol (`OrderNew.tsx`), 16 sa zmenilo — nič viac, nič navyše.
- **Vyváženosť JSX značiek** oproti 2.43.0, súbor po súbore: jediné rozdiely
  sú presne tie zámenené okná (`EventFormModal` → `EventRowsModal` atď.).
- **Každý import sa dá dohľadať** — strojovo som overil, že všetko, čo tieto
  súbory importujú, ten druhý súbor naozaj exportuje.
- **Žiadny nepoužitý import** v dotknutých súboroch.
- **`/orders/new` nikde nezostalo** a všetky štyri nové okná sú naozaj
  namontované.
- **Verzia je na 9 miestach v 7 súboroch**, všetky posunuté.

**Čo overiť nedokážem:** preklad (Node ani Rust na tomto Macu nie sú, prvý
reálny preklad je až CI) a ako to sadne oku pri reálnych dátach.

**Dve veci, na ktoré ťa chcem upozorniť:**

1. **Tie štyri formuláre sú po slovensky**, kým stránky okolo nich sú po
   anglicky — postavil som ich podľa toho slovenského preview, ktoré si si
   vybral. Ak to chceš zjednotiť, poviem si a prepnem to.
2. **`OrderFormModal` a `SaleFormModal`** už nikto nevolá, ale nechal som ich
   v kóde nedotknuté. Zmazať ich viem, ale nerobím to sám od seba.

---

## Prečo 2.45.0 a nie 2.44.0

`tiqr-manager-2.44.0.zip` už máš v Downloads a má **iný obsah** — tú finance
migráciu, ktorú sme opustili, keď si ma poslal späť na 2.43.0. Aby ti tam
neležali dva rôzne súbory s rovnakým názvom, ide táto verzia o krok ďalej.
Rovnako sme to riešili kedysi pri 2.4.0 → 2.4.1.

---

**Verzia:** 2.45.0 (9 miest v 7 súboroch).
**Migrácie:** žiadne nové, ďalšia voľná je 031.

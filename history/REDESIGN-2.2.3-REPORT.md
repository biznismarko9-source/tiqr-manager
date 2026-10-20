# TIQR Manager 2.2.3 — Event Workspace: Listings tab, Tasks preč, tabuľky na celú šírku

Reagujem na tvoju poslednú správu (2 screenshoty + zadanie):

> *"Vsimol som si ze v tom inventory tie tabulky niesu cele, viac ich natiahni nech su od kraja po kraj"*
>
> *"nakoniec tie Tasks uplne zmazeme, netreba nam to"*
>
> *"taktiez overview by som spojil s inventory, cize budeme tam mat len zlozky overiew, sales, market
> finance, potom chod na toto: Chcem teraz v Event Workspace odstrániť záložku Tasks a nahradiť ju:
> Overview | Inventory | Listings | Sales | Market | Finance. Pridaj novú záložku Listings [...] Iba
> prehľad existujúcich listing údajov, ktoré už TIQR má. [...] ticket, marketplace, listing price,
> currency, status, listing URL, last updated/checked [...] Hore sprav jednoduchý summary: Active
> listings, Listed value, Lowest price, Highest price [...] Najprv skontroluj aktuálny projekt a zisti,
> aké listing dáta už reálne existujú. Ak niektorý údaj nemáme, nevymýšľaj ho."*

Postupne, po poriadku.

## 1. Tabuľky na celú šírku

Presne ako si napísal — Orders a Tickets v Inventory (aj Sales a Finance v ich vlastných záložkách) mali
strop na 1400px šírky, takže na širšom okne zostal vpravo prázdny pás a tabuľka nesiahala od kraja po kraj.
Tento strop som odstránil zo všetkých 4 tabuliek v Event Workspace — presne tá istá oprava, akú dostal celý
layout appky v 2.0.31. Jedna vec na vedomie: pri veľmi širokom monitore sa teoreticky môže niektorý stĺpec
roztiahnuť nezvyčajne — ak sa to niekde bude javiť škaredo, daj vedieť a doladím to (napr. pevnejšie pomery
stĺpcov), zatiaľ som to nerobil, lebo si to nežiadal a bola by to zbytočná práca navyše.

## 2. Tasks záložka — odstránená

Táto záložka bola od začiatku len prázdny placeholder bez akejkoľvek logiky, dát alebo typu v pozadí — takže
jej odstránenie neznamenalo mazať žiadnu funkcionalitu, len prestať ju zobrazovať a vymazať ten kúsok kódu.

## 3. Overview + Inventory — nechal som ich oddelene

Všimol som si, že si na začiatku správy spomenul spojenie Overview a Inventory do jednej záložky, ale
hneď potom si napísal finálny, presne vymenovaný zoznam záložiek: **Overview | Inventory | Listings | Sales
| Market | Finance** — teda so 6 samostatnými záložkami vrátane oboch pôvodných zvlášť. Riadil som sa týmto
druhým, presnejším zadaním. Ak si predsa len chcel Overview a Inventory spojiť do jednej záložky, napíš mi
to a rád to prerobím — je to jednoduchá zmena.

## 4. Nová záložka Listings

Najprv som si — presne ako si žiadal — overil, aké z tých 7 polí (ticket, marketplace, listing price,
currency, status, listing URL, last updated/checked) TIQR reálne má. Prešiel som celý typ `Ticket`, všetky
`ALTER TABLE tickets` naprieč všetkými 21 migráciami (jediná migrácia, čo niečo pridala nad rámec pôvodnej
schémy z verzie 001, je migrácia 010 — a pridala len `resale_status` a `delivery_status`, nič
marketplace/URL/timestamp) a aj existujúcu logiku Price Checkera (`YourTicketGroup`), ktorá je tomuto
najbližšie — ani tam tieto tri polia nie sú.

Výsledok: **4 zo 7 polí reálne existujú** — ticket, listing price, currency, status. Tie sú v novej záložke.
**3 zo 7 neexistujú nikde v appke** — marketplace, listing URL, last updated/checked. Tie som teda
nevymýšľal, presne ako si žiadal. Priamo v záložke je k tomu jedna vetička, aby to nebolo len ticho
chýbajúce: *"Marketplace, listing URL and last checked date aren't tracked in TIQR yet..."* — ak budeš chcieť
tieto tri veci naozaj sledovať, treba na to reálne rozšíriť dáta (napr. nové stĺpce na tickete alebo
prepojenie na marketplace), nie im tu len pridať prázdny stĺpec navyše. Dôsledok: keďže listing URL neexistuje,
nie je čo kliknúť, aby sa otvorila stránka — akonáhle by táto dáta existovala, viem to doplniť veľmi rýchlo.

Hore v záložke je aj tvoj požadovaný summary — **Active listings, Listed value, Lowest price, Highest
price** — počítané z tých istých tiketov tohto eventu, čo už majú stav "Listed".

Nič nové na pozadí — záložka používa presne to isté pole `tickets`, čo si Inventory už aj tak načítava,
len ho filtruje na `status === "listed"`. Žiadne nové API, žiadny nový pricing systém, žiadna marketplace
automatizácia — presne ako si žiadal v "Dôležité".

## Čo som overil

Táto verzia je celá na frontende — žiadna zmena v Ruste, žiadna nová migrácia.

```
npx tsc -b         -> 0 chýb
npm run build      -> OK ("tiqr-manager@2.2.3 build" v hlavičke)
```

Existujúca sada Rust testov nie je touto verziou ovplyvnená (nemenil som žiadny `.rs` súbor), takže som ju
znova nespúšťal — bola zelená naposledy pri 2.2.1/2.2.2 a nič na backende sa odvtedy nezmenilo.

## Zmenené súbory

- `src/pages/EventDetail.tsx` — odstránená záložka Tasks, nová záložka Listings, odstránený `max-w-[1400px]`
  strop zo 4 tabuliek (Orders, Tickets, Sales, Finance)

**Verzia (9 miest v 7 súboroch):** `2.2.3`.

## STOP

2.2.3 hotové, otestované a zabalené. Skontroluj:

1. **Ktorýkoľvek Event → Inventory** — over, že tabuľky Orders aj Tickets teraz siahajú od kraja po kraj
   okna, aj pri väčšom okne.
2. **Ktorýkoľvek Event → záložky hore** — over, že vidíš presne 6 záložiek v poradí Overview, Inventory,
   Listings, Sales, Market, Finance a že Tasks už nikde nie je.
3. **Listings záložka** — otvor ju pri evente, kde máš aspoň jeden tiket so stavom "Listed", over že summary
   (Active listings/Listed value/Lowest/Highest) aj tabuľka dole sedia s tým, čo očakávaš.
4. Ak si naozaj chcel Overview a Inventory spojiť do jednej záložky (bod 3 vyššie), daj vedieť.

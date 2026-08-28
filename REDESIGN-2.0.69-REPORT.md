# TIQR Manager 2.0.69 — Inline editovanie štítkov + preč s duplicitným stĺpcom

## Čo si napísal

*"ten resale status rovno moze ist prec lebo status ukazuje to iste, taktiez tieto statusy si chcem vsade
vediet editnut aj v dashborade nielen v apke. graf je dobry"*

(reakcia na screenshot z Order Detail, kde stĺpce Status a Resale status ukazovali to isté — "Sold")

Spýtal som sa ťa, kde presne chceš vedieť meniť tieto štítky priamo (bez otvárania Edit formulára) —
odpovedal si: **v tabuľkách Order Detail aj Sale Detail (odporúčané)**.

## 1. Order Detail — preč s duplicitným "Resale status"

Mal si pravdu — na tejto konkrétnej tabuľke stĺpec "Resale status" (tvoje ručné značenie) v praxi vždy
ukazoval to isté ako existujúci stĺpec "Status" (skutočný stav lístka) hneď vedľa neho. Stĺpec som odstránil
úplne. Dôležité: **na Sale Detail zostáva "Status" presne tak, ako bol** — tam žiadny iný "Status" stĺpec
nie je, takže tam k žiadnej duplicite nedochádza.

## 2. Status/Delivery status/Payout status — teraz editovateľné priamo v tabuľke

Predtým jediný spôsob, ako zmeniť niektorý z týchto štítkov, bol otvoriť "Edit", nájsť správne pole vo
formulári, zmeniť ho a kliknúť "Save". Teraz stačí **kliknúť priamo na farebný štítok** — otvorí sa
rozbaľovací zoznam s možnosťami, vyberieš novú hodnotu a **uloží sa to okamžite**, žiadne "Save" navyše.
Platí to na oboch obrazovkách:

- **Sale Detail** — Status (Listed/Unlisted/Sold), Delivery status (Delivered/Not delivered), Payout status
  (pending/paid).
- **Order Detail** — Status (skutočný stav — Available/Listed/Cancelled), Delivery status, Payout status.

Dve výnimky, kde štítok zámerne **nie je** takto klikateľný — v oboch prípadoch preto, že zmena tejto
konkrétnej hodnoty má vlastné, dôležitejšie pravidlá, ktoré by jednoduchý rozbaľovací zoznam obišiel:

- **Order Detail, Status pri predanom lístku.** Keď je lístok "Sold", jeho skutočný stav sa nedá zmeniť
  týmto spôsobom vôbec — presne tak, ako to už dnes funguje pri hromadnom tlačidle nad tabuľkou. Von zo
  "Sold" sa dá dostať jedine cez refund na Sale Detail.
- **Sale Detail, Payout status pri refundovanom riadku.** Refund ostáva svoj vlastný, samostatný krok
  (tlačidlo "Refund", s dôvodom) — nie je to len prepnutie pending/paid, takže sa to takto meniť nedá.

Iné pole (Section/Row/Seat/Notes atď.) sa naďalej mení len cez existujúci Edit formulár — nič iné sa
neposúva, len týchto 5 konkrétnych kombinácií (3 na Sale Detail, 2 skutočne editovateľné + 1 uzamknuté na
Order Detail).

## Ako to funguje technicky (v skratke)

Delivery status aj Payout status už appka vedela meniť hromadne (2.0.67) — len nie pre presne jeden
konkrétny lístok/predaj priamo z riadku. Pridal som teda 2 nové, úzko zamerané príkazy na backende
(menia **len** jeden konkrétny stĺpec, nič iné) a jedno nové prepojenie (lístok → jeho aktuálny predaj), aby
Order Detail vedelo zmeniť Payout status bez toho, aby muselo sťahovať celý zoznam predajov.

## Čo som overil

```
cargo test --lib   -> 693 testov (685 + 8 nových), 0 zlyhaní, 3 ignorované
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

Nové testy overujú presne tie isté hranice, čo appka už dnes stráži pri hromadných akciách — že sa zmení
len vybraný lístok, že sa dá prepínať medzi všetkými 3 hodnotami Status a späť, že sa neplatná hodnota
odmietne, že chýbajúce ID zruší celú zmenu (nič sa nezmení), a že sa dá meniť aj pri predanom lístku (kde to
má zmysel).

## Zmenené súbory

**Backend:**
- `src-tauri/src/commands/tickets.rs` — nový `bulk_update_ticket_resale_status_impl` + príkaz; nový priamy
  príkaz `bulk_update_ticket_delivery_status` (samotná logika existovala už od 2.0.67, len nemala vlastný
  príkaz); nové pole `sale_id` na `Ticket`; 8 nových testov.
- `src-tauri/src/models.rs` — 2 nové vstupné typy, nové pole `sale_id`.
- `src-tauri/src/lib.rs` — registrácia 2 nových príkazov.

**Frontend:**
- `src/components/ui.tsx` — nová `InlineStatusSelect` (štítok, čo je zároveň editovacie pole).
- `src/pages/SaleDetail.tsx`, `src/pages/OrderDetail.tsx` — všetky status štítky prepojené na inline
  editovanie; Order Detail navyše bez stĺpca "Resale status" a s upravenými šírkami stĺpcov.
- `src/pages/Tickets.tsx` — `RESALE_STATUS_OPTIONS`/`DELIVERY_STATUS_OPTIONS` teraz zdieľané (exportované),
  aby nový dropdown ponúkal presne tie isté možnosti ako existujúci Edit formulár.
- `src/lib/types.ts`, `src/lib/api.ts` — nové typy a volania pre 2 nové príkazy.

**Verzia (8 miest):** `2.0.69`.

## STOP — nič, čo by som potreboval spätne overiť

Šírky stĺpcov na Order Detail som len prepočítal nazad (stĺpec zmizol, priestor sa vrátil Seat) — nemenil
som nič, čo by si už predtým musel kontrolovať. Ak ti pri klikaní na štítky niečo nebude sedieť (napr. že
rozbaľovací zoznam vyzerá inak, než by si čakal — je to normálny systémový dropdown, nie vlastný appky
štýl), napíš mi presne kde a čo.

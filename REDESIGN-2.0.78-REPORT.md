# TIQR Manager 2.0.78 — Mobile push teraz cez ntfy (nie Pushover)

## Čo si napísal a čo som s tým spravil

Po 2.0.77 (Pushover len s user key, appka mala mať vlastný zabudovaný token) si napísal:

> *"nie, musi to fungovat len s user key bez api klucu, ked poslem user key tak nejak automaticky zistit api key v pozadi aby to fungovalo, len user key nic viac nastavovania"*

Toto som si overil priamo v oficiálnej dokumentácii Pushover API - a bohužiaľ, **toto sa s Pushoverom nedá spraviť vôbec, nech to naprogramujem akokoľvek**. Pushover pri každom posielaní vyžaduje dve úplne nezávislé veci: tvoj osobný "user key" a k tomu ešte samostatný "aplikačný token". Jedno sa z druhého odvodiť nedá - nie je to obmedzenie tejto appky, je to takto navrhnuté na ich strane a nedá sa to obísť. Takže aj tá "appka má vlastný zabudovaný token" verzia z 2.0.77 by ťa aj tak nakoniec donútila spraviť tú jednu registráciu (buď ty, alebo ja s tvojím tokenom) - presne to, čo si nechcel.

Namiesto toho, aby som ti povedal "nedá sa" a nechal to tak, som ti cez otázku v appke ponúkol reálnu alternatívu - **ntfy** - a vysvetlil prečo. Vybral si:

> *"Prejsť na ntfy (odporúčam)"*

a hneď potom si to potvrdil:

> *"ked tak pouzime inu apku, len nech je tiez zadarmo a funguje len pomocou jednej veci, ktoru posleme"*

Presne to ntfy spĺňa - je to zadarmo a naozaj potrebuje len jednu vec.

## Čo je ntfy a prečo je to iné

ntfy.sh je verejná, zadarmo dostupná služba na posielanie push notifikácií na telefón. Na rozdiel od Pushoveru **nepotrebuje absolútne žiadnu registráciu** - ani appka, ani ty. Jediná vec, ktorá existuje, je "topic" (téma) - ľubovoľný text, ktorý si sám vymyslíš. Appka pošle upozornenie jednoducho na adresu `ntfy.sh/tvoj-topic`, bez hesla, bez tokenu, bez prihlasovania.

**Dôležité - priamo z toho vyplýva jedno pravidlo:** keďže táto verejná služba nemá žiadne prihlasovanie, samotný názov topicu JE celá ochrana - kto ten názov pozná, ten vidí (a môže poslať) tvoje notifikácie. Preto:
- Zvoľ si topic, ktorý nie je ľahko uhádnuteľný (nie "tiqr" alebo "marko", ale niečo ako `tiqr-marko-8k2f9x`).
- V appke sa tento topic správa presne ako predtým Pushover user key - nikdy sa nezobrazí späť, len uvidíš, že je uložený.

## Ako to nastavíš (jedna vec, nič viac)

1. Nainštaluj si zadarmo appku **ntfy** na telefón (Google Play / App Store, hľadaj "ntfy").
2. V appke ntfy si "prihlás odber" (subscribe) na tvoj vlastný topic - vymysli si nejaký text, napr. `tiqr-marko-8k2f9x`.
3. V TIQR Manageri choď do **Settings → Notifications**, zapni "ntfy (mobile push)" a do políčka "Topic" napíš presne ten istý text.
4. Ulož a skús "Send test" - na telefóne by ti mala prísť skúšobná notifikácia.

Desktop notifikácie a zvonček na Dashboarde sú bez zmeny - fungujú ako doteraz.

## Čo som overil

```
cargo test --lib   -> 730 testov, 0 zlyhaní, 3 ignorované (27 v notifications module)
npx tsc -b         -> 0 chýb
npm run build      -> OK
```

Medzi tými 27 testami je aj jeden špecificky na to, že appka, ktorá má uložené nastavenia ešte v starom tvare (s emailom aj s Pushoverom z predošlých verzií), sa po tejto zmene nerozbije - staré polia jednoducho ignoruje a nové ntfy nastavenia si prečíta normálne.

Odoslanie na skutočný ntfy server som odtiaľto (z tohto sandboxu) nemohol vyskúšať naživo - rovnaké obmedzenie ako doteraz pri Google Sheets/Pushover. Krok "Send test" v appke je presne na to, aby si si to vyskúšal ty priamo.

## Čo sa zmenilo oproti 2.0.77

- **Pushover je preč úplne** - žiadny "user key", žiadny zabudovaný token, žiadny `PUSHOVER_API_TOKEN` secret v GitHub Actions (ten som tiež odstránil, keďže už nie je na čo).
- Settings → Notifications teraz namiesto "Pushover (mobile push)" ukazuje "ntfy (mobile push)" s jedným políčkom "Topic".

## Zmenené súbory

**Backend (Rust):**
- `src-tauri/src/commands/notifications.rs` — Pushover preč, nová funkcia na odoslanie cez ntfy (obyčajný POST na `ntfy.sh/<topic>`, bez prihlasovania).
- `src-tauri/src/models.rs` — dátové typy zjednotené na `ntfy` namiesto `pushover`.
- `src-tauri/src/lib.rs` — premenovaný testovací príkaz (`test_ntfy_notification`).
- `src-tauri/build.rs`, `.github/workflows/build-windows.yml` — odstránený `PUSHOVER_API_TOKEN` (pre ntfy netreba žiadny build secret).

**Frontend:**
- `src/lib/types.ts`, `src/lib/api.ts` — premenované typy/funkcie z Pushover na ntfy.
- `src/pages/Settings.tsx` — sekcia Notifications teraz ukazuje "ntfy (mobile push)" s políčkom "Topic" a vysvetlením vyššie.
- `src/components/Layout.tsx` — drobná oprava komentára (žiadna funkčná zmena).

**Verzia (9 miest v 7 súboroch):** `2.0.78`.

# TIQR Manager 2.0.64 — AI kategorizácia: lacnejšie, spoľahlivejšie, a návod na nastavenie

## Čo si napísal

*"chcem, aby to bolo co najlacnejsie takze cast centa za kazdu pracu, taktiez chcem aby to bolo bezchybne,
uprav to tak, aby to tak bolo a taktiez daj navod ako na to ai"*

Rozdelil som to na tri časti nižšie: čo je teraz lacnejšie (a prečo to aj tak bolo od začiatku len zlomok
centa), čo je teraz spoľahlivejšie, a presný návod na AI kľúč.

## 1. Lacnejšie

Dobrá správa: aj pred touto úpravou to bola naozaj len časť centa za jeden dopyt — Claude Haiku (model,
ktorý appka používa) stojí 1 $ za milión vstupných a 5 $ za milión výstupných tokenov, a jeden dopyt má
rádovo stovku vstupných tokenov a pár výstupných. Skrátil som teraz text, čo appka posiela modelu, na
najkratšiu verziu, čo ešte spoľahlivo funguje — to zníži vstupné tokeny na jeden dopyt približne o tretinu.

Konkrétne prepočítané (odhadom, keďže presný počet tokenov vie len samotný model):
- **Pred:** ~130-140 vstupných tokenov/dopyt → ~0,00014 $ (asi 0,014 centu)
- **Po:** ~85-95 vstupných tokenov/dopyt → ~0,0001 $ (asi 0,01 centu)

Pri 50 nových/nejasných eventoch za mesiac je to okolo **pol centa mesačne** — reálne nemerateľné pri tvojom
objeme. A pripomínam: appka sa AI pýta LEN na eventy, čo si kľúčové slová nevyriešia samé zadarmo (Grand
Prix, Festival, Musical, Comedy) — nie na každý event.

## 2. Bezchybnejšie

Tu musím byť úprimný na rovinu, presne ako doteraz pri všetkom: **žiadny systém, čo rozpoznáva mená, nevie
byť matematicky garantovane 100% presný** — pri naozaj neznámom/obskúrnom mene sa môže AI pomýliť alebo
povedať "neviem", presne ako by sa mohol pomýliť aj človek, čo to meno nikdy nepočul. To by ti nemal
sľubovať nikto poctivo.

Čo som ale spravil, je odstrániť každú CHYBU, čo viem odstrániť — teda situácie, kde appka zlyhá zbytočne,
nie preto, že by AI meno nepoznalo:

1. **Opakovanie pri výpadku:** ak Anthropic práve na chvíľu preťažený (bežná vec pri akomkoľvek API) alebo
   appka narazí na dočasný chybový stav, appka to teraz skúsi ešte raz po krátkej pauze, namiesto toho, aby
   sa hneď vzdala a nechala event bez kategórie zbytočne. Pri trvalej chybe (zlý kľúč, zlá požiadavka) sa to
   už znova neskúša — to by len minulo peniaze druhýkrát na tú istú, garantovane rovnakú chybu.
2. **Časový limit:** appka teraz na tento konkrétny dopyt čaká najviac 20 sekúnd — predtým nebol žiadny
   limit. Keďže sa toto môže zavolať viackrát za sebou (raz na každý nový/nekategorizovaný event), jeden
   "zaseknutý" dopyt by inak mohol zaseknúť celú synchronizáciu.
3. **Dlhší strop na odpoveď:** appka teraz dovolí modelu odpovedať dlhším textom (predtým prísny strop mohol
   orezať dlhšie meno vlastnej kategórie, čo by si si sám vytvoril, a appka by ho potom nesprávne vyhodnotila
   ako "nepoznám"). **Toto nestojí nič navyše** — Anthropic účtuje len to, čo model skutočne napíše, nie
   povolený strop, a správna odpoveď je vždy len krátky názov kategórie.

Všetko ostatné ostáva presne také bezpečné, ako to už bolo: čo appka nevie s istotou určiť, necháva bez
kategórie — nikdy neuhádne naslepo, nikdy nezapíše kategóriu, čo v appke reálne neexistuje.

## 3. Návod: ako nastaviť AI

Bez tohto appka funguje ďalej normálne — len druhý krok (AI) sa preskočí a appka rozpozná len
Motorsport/Festival/Theatre/Comedy podľa kľúčových slov, zvyšok necháva bez kategórie, presne ako doteraz.

1. **Vytvor si účet a API kľúč.** Choď na **platform.claude.com** (toto je oficiálna Anthropic konzola pre
   API — nie claude.ai, kde bežne s appkou Claude chatuješ, to je iný účet/iný účel). Prihlás sa/zaregistruj
   sa, a v ľavom menu choď na **Settings → API Keys** (priamo: platform.claude.com/settings/keys). Klikni
   "Create Key", daj mu meno (napr. "TIQR Manager"), a skopíruj si vygenerovaný kľúč — zobrazí sa len raz,
   tak si ho hneď ulož niekam bezpečne.
2. **Dobi kredit.** API sa platí vopred (predplatené kredity) — v konzole nájdeš sekciu Billing/Plans, kde
   priradíš platobnú kartu a dobiješ si kredit. Anthropic priamo neuvádza povinné minimum, takže pokojne
   stačí dobiť si menšiu sumu (napr. 5-10 $) na vyskúšanie — pri tvojej spotrebe (časť centa na event) ti to
   vydrží veľmi dlho. Bez kreditu kľúč vytvoríš, ale volania budú zlyhávať (čo appka aj tak berie bezpečne —
   len by sa AI krok stále preskakoval).
3. **Pridaj kľúč do GitHubu.** V tvojom repozitári na GitHube choď na **Settings → Secrets and variables →
   Actions → New repository secret**. Meno secretu: `ANTHROPIC_API_KEY`, hodnota: kľúč z kroku 1 (presne
   ako je, bez úvodzoviek). Ulož.
4. **Spusti nový release.** Ďalší beh `release.ps1` (alebo test build cez GitHub Actions) tento kľúč
   automaticky zabuduje do appky — presne tým istým spôsobom, ako appka už dnes zabuduje kľúč pre Google
   Sheets. Nemusíš meniť nič v appke samotnej.

Po tomto appka vie začať naozaj rozpoznávať aj mená ako "Celine Dion" alebo neznáme športové zápasy.

## Čo som overil

```
cargo test --lib   -> 652 testov (650 + 2 nové), 0 zlyhaní, 3 ignorované
npx tsc -b         -> 0 chýb
npm run build      -> OK (frontend touto úpravou nebol dotknutý)
```

2 nové testy pokrývajú presne rozhodnutie "oplatí sa to skúsiť ešte raz, alebo nie" (dočasná chyba áno,
trvalá chyba nie).

## Zmenené súbory

**Backend:**
- `src-tauri/src/ai_categorize.rs` — kratší text pre AI dopyt, opakovanie pri dočasnom zlyhaní, časový
  limit 20s, vyšší (ale nič nestojaci) strop na dĺžku odpovede

Frontend beze zmeny.

**Verzia (8 miest):** `2.0.64`.

## STOP

Toto dokončuje tvoju požiadavku na "čo najlacnejšie a bezchybné" v rámci toho, čo je pri AI rozpoznávaní
mien reálne možné garantovať — zvyšok (naozajstná presnosť pri neznámych menách) závisí od toho, čo o danom
mene AI model skutočne vie, nie od appky samotnej. Keď si nastavíš kľúč podľa návodu vyššie, daj vedieť, ako
to na reálnych dátach funguje.

## Zdroje (k návodu vyššie)

- [Claude Platform Docs – Get started](https://platform.claude.com/docs/en/get-started)
- [Anthropic Help Center – How do I pay for my Claude API usage?](https://support.claude.com/en/articles/8977456-how-do-i-pay-for-my-claude-api-usage)
- [Claude Platform Docs – Models overview](https://platform.claude.com/docs/en/about-claude/models/overview)

# English dimension coverage — COMPLETE

Every Duckling dimension that ships English rules is ported and corpus-verified.
There is **no remaining English dimension to port.** See
[`RUST_PORT_PROGRESS.md`](RUST_PORT_PROGRESS.md) for the per-milestone log.

## What's ported (all of it)

| Dimension | Entry point | Validation |
|---|---|---|
| **Time** | `parse` / `parse_locale` | 1069/1069 transcribed EN corpus (100%); 12 English locales; regional + computed + beyond-Duckling holidays; per-instant DST vs authoritative IANA tzdata (tz_truth 287 / 17 zones) |
| **Duration** | `parse_duration` | full `Duration/EN/Corpus.hs` + oracle differential (135) |
| **Numeral** | `parse_numeral` | full `Numeral/EN/Corpus.hs` (105); one documented omission (ruleSkipHundreds1, collides with time-of-day) |
| **Ordinal** | `parse_ordinal` | full `Ordinal/EN/Corpus.hs` (32) |
| **Temperature** | `parse_temperature` | `temperature_corpus` (31+3 neg) |
| **Volume** | `parse_volume` | `volume_corpus` (54+4 neg) |
| **Distance** | `parse_distance` | `distance_corpus` (51+4 neg); composite feet/inch + metric↔imperial fold + ambiguous "m" |
| **Quantity** | `parse_quantity` | `quantity_corpus` (50+4 latent+4 neg); mg/kg scaling, product, latent, intervals |
| **AmountOfMoney** | `parse_amountofmoney` | `amountofmoney_corpus` (140+5 latent+3 neg); ~50 currencies, cents composition, lakh/crore/billion, EN/US "grand"+coins |
| **Email** | `parse_email` | full `Email/EN/Corpus.hs` (8+8 neg) |
| **Url** | `parse_url` | `Url/Rules.hs` + negatives (7+6 neg) |
| **CreditCardNumber** | `parse_creditcard` | full `Corpus.hs`, Luhn (12+11 neg) |
| **PhoneNumber** | `parse_phonenumber` | full `Corpus.hs` (16+3 neg) |
| **Time + all dims** | `parse_all` | cross-dimension range-domination surface (behavioral) |

Cross-checked against the source: Duckling's default EN set
(`Dimensions/EN.hs` → `allDimensions`) is **Distance, Duration, Numeral, Ordinal,
Quantity, Temperature, Time, Volume** — all covered — plus the five requestable
dimensions (AmountOfMoney + the four regex dims), also covered. The only other
item in the source, **TimeGrain**, is an internal building block Time consumes
(`timegrain::en`), not a user-facing entity, and Duckling doesn't emit it for EN
by default either.

## Beyond-Duckling extensions (intentional, product-driven)

Additions that *exceed* upstream Duckling, kept additive so the Duckling corpus
stays 100% and clearly marked in-code:

- **Modern / regional holidays** — Juneteenth, Australian holidays, post-2020
  additions Duckling's frozen data lacks.
- **Hyphenated `<number>-<unit>`** — "a 3-inch pipe", "an 8-ounce glass"
  (`hyphenated_units_corpus`, 15). Isolated value dims only → zero Time-corpus risk.

## Orthogonal / optional items — resolved

Not dimension work; each is deliberately settled rather than left open:

- **`unique`-mode ranking (8/1069 gaps): permanent, understood ceiling — not
  "fixed".** `contains` mode (Duckling's real corpus semantics) is **1069/1069**;
  the stricter full-span `unique` bar leaves 8. Diagnosed: **5×** "for a quarter
  past …" resolve correctly via the best entity — Duckling doesn't full-span the
  leading "for" *either*, so forcing it would **diverge**; **3×** "Fri, Jul 18,
  2014 07:00 PM" already resolve to the correct instant, differing only in *span*
  (a weekday-carrying date + connector-less time doesn't fold). Closing the 3 needs
  a core Time-intersect change for span-only gain on newswire-style inputs — judged
  disproportionate vs. the 1069/1069 risk. **Decision: leave the parser as-is;
  contains-mode is the fidelity metric.**
- **Timezone coverage: sufficient.** `tz_truth` spans 17 IANA zones covering every
  offset behavior — 45-minute (Kathmandu, Chatham), half-hour + DST (Adelaide,
  St. John's), DST in both hemispheres, extreme (Kiritimati +14:00) — validated
  per-instant against authoritative tzdata. More zones would be padding.
- **Future-vs-past resolution convention: a design choice, quantified.** Against
  TempEval-3 gold, absolute-date resolution agrees ~92%; the relative-date gap is
  Duckling's deliberate *next-future* default vs newswire TimeML's *in-context past*
  (see [`TE3_TIME_EVAL.md`](TE3_TIME_EVAL.md)). Correct for a forward-looking speech
  assistant; not a bug.

## Evaluation beyond the corpora

Three external benchmarks corroborate the port (harnesses in `rust/eval/`):
[GMB recognition](NER_TIME_EVAL.md) (~93% time-expression recall on real news),
[Recognizers-Text](RECOGNIZERS_EVAL.md) (temperature 96.9% value-agreement vs a
peer library), [TempEval-3](TE3_TIME_EVAL.md) (absolute-date resolution vs human
gold). All differential disagreements were verified against the live oracle to
reproduce upstream Duckling.

## Status

**The English port of the whole Duckling library is complete and resolved.** Every
English dimension is ported and green; Time holds at 1069/1069; the orthogonal
items are settled by decision. Further work (additional languages, more
beyond-Duckling forms, a past-reference resolution mode) is net-new product scope,
not finishing the port.

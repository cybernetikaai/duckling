# Porting the remaining English dimensions

Status of the Duckling → Rust port and what it would take to finish the English
dimensions. See [`RUST_PORT_PROGRESS.md`](RUST_PORT_PROGRESS.md) for the detailed
per-milestone log of what's already done.

## What's ported today

| Dimension | Status | Validation |
|---|---|---|
| **Time** (`parse` / `parse_locale`) | emitted | 1069/1069 transcribed EN corpus (100%); 12 English locales; regional + computed + beyond-Duckling holidays; per-instant DST vs IANA tzdata |
| **Duration** (`parse_duration`) | emitted | full `Duration/EN/Corpus.hs` + oracle differential (135 checks) |
| **Ordinal** (`parse_ordinal`) | emitted | full `Ordinal/EN/Corpus.hs` (32) |
| **Time + Duration** (`parse_all`) | emitted | cross-dimension range domination vs oracle (96 cases) |
| **Numeral** (`parse_numeral`) | emitted (corpus-complete) | full Numeral/EN/Corpus.hs (105 inputs) — integers/written/composition, decimals, comma-groups, K/M/G suffixes, fractions, zero-words, dozen, negatives, skip-hundreds, parentheticals, big compounds, Indian numbering (numeral_corpus). Only ruleSkipHundreds1 ("nine thirty"->930) omitted — collides with time-of-day in the shared rule set (documented). |
| **Email** (`parse_email`) | emitted ✅ | full `Email/EN/Corpus.hs` — email_corpus (8 + 8 neg) |
| **Url** (`parse_url`) | emitted ✅ | `Url/Rules.hs` + Corpus negatives — url_corpus (7 + 6 neg) |
| **CreditCardNumber** (`parse_creditcard`) | emitted ✅ | full `CreditCardNumber/Corpus.hs` (Luhn) — creditcard_corpus (12 + 11 neg) |
| **PhoneNumber** (`parse_phonenumber`) | emitted ✅ | full `PhoneNumber/Corpus.hs` — phonenumber_corpus (16 + 3 neg) |

## Remaining work

### Bucket A — standalone regex dimensions — ✅ DONE

Email, Url, CreditCardNumber, and PhoneNumber are ported and corpus-verified
(see the table above). Language-agnostic ones live at the crate root
(`src/url.rs`, `src/creditcard.rs`, `src/phonenumber.rs`); Email is under
`src/email/en.rs` (its " at "/" dot " forms are English). All are separate
`parse_*` entry points sharing a `dim_rules` cache + `emit_entities` helper, so
they never touch the Time ranker.

### Bucket B — value/unit dimensions, all Numeral-dependent (remaining)

| Dimension | Haskell | Numeral refs | Estimate |
|---|---|---|---|
| ~~Temperature~~ ✅ | 161 L | done — temperature_corpus (31+3 neg) | — |
| ~~Volume~~ ✅ | 166 L | done — volume_corpus (54+4 neg) | — |
| ~~Distance~~ ✅ | 213 L | done — distance_corpus (51+4 neg); incl. composite feet/inch + metric↔imperial fold + ambiguous "m" | — |
| Quantity | 256 L | 21 | ~1 day |
| AmountOfMoney | 432 L | 20 | ~1.5 days |

**Shared prerequisite: finish Numeral** (~1 day) — their corpora test `$1.2M`,
`1.5 million`, `30 lakh`, etc., which the current Time-subset numeral doesn't cover.
After that, each is: a unit table + regex + compose-with-Numeral + interval handling
("between 5 and 10 km") + a transcribed corpus. The interval/compose machinery
already exists from Time. **~6 days including the Numeral completion.**

## Effort summary

- **All nine English dimensions: ~1.5–2 weeks** of focused work.
- **Quick wins (Email / Url / CreditCardNumber / PhoneNumber): ~2 days**, day one.
- Numeral completion (~1 day) unlocks all of Bucket B and closes the `parse_numeral`
  asymmetry with Ordinal.

## Common plumbing (per dimension)

1. A `Token` variant for the dimension.
2. Rule builder(s) producing that token (regex-only for Bucket A; compose-with-Numeral
   for Bucket B).
3. A `parse_<dim>` emitter → the Duckling JSON value shape (via `serde`).
4. A transcribed corpus fixture + an oracle differential test.

The chart parser, ranker, and `ResolveContext` are reused unchanged.

## Architecture fit

- **Bucket A** (agnostic regex) lives at crate root, consistent with the
  language-agnostic core (`engine`, `ranking`, `types`, `grain`).
- **Bucket B** follows the `<dimension>/en.rs` per-language pattern established for
  Numeral/Ordinal/Duration/Time (unit words are language-specific; the value type +
  math are agnostic in `<dimension>/mod.rs`). See `rust/README.md` → "Project layout
  & adding a language".

## Scope note

These are **new dimensions**, beyond the original "English time parsing" mandate.
For a speech → time → timezone product, Time/Duration are core; the four regex
dimensions are cheap and broadly useful for general NLU extraction; the value
dimensions (money/distance/temperature/volume/quantity) are worth it only if the
product actually needs to extract those quantities.

## Suggested order

1. ~~`parse_numeral` (close the Ordinal asymmetry)~~ — ✅ done.
2. ~~Email → Url → CreditCardNumber → PhoneNumber (quick wins)~~ — ✅ done.
3. ~~Finish Numeral~~ ✅ — corpus-complete (105/105); one documented omission
   (ruleSkipHundreds1). Prerequisite for Bucket B now cleared.
4. ~~Temperature~~ ✅ → ~~Volume~~ ✅ → ~~Distance~~ ✅ → Quantity → AmountOfMoney.
   **Next: Quantity** (Duckling/Quantity/EN — `<numeral> <unit> [of <product>]`,
   e.g. "2 cups of sugar", "3 grams", intervals). Same isolated-rule-set pattern:
   its own `numeral + quantity` set via `dim_rules`, so zero Time-corpus risk.

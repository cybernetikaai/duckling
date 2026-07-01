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
| **Numeral** | **foundational** (no standalone `parse_numeral` yet) | ported to the forms Time/Duration need — integers, written numbers, informal quantifiers ("a couple"), decimals, composition. **Not corpus-complete**: magnitude suffixes (`3M`/`100K`/`30 lakh`) and some fraction forms are deferred. |

## The remaining nine — two buckets

Duckling's other English dimensions split cleanly by difficulty, which tracks one
axis: **does the dimension consume Numeral?**

### Bucket A — standalone regex, no Numeral (the easy wins)

These are **language-agnostic** in Duckling (defined in a top-level `Rules.hs`, not
under `EN/`), so in this crate they'd live at the root (`src/url.rs`,
`src/creditcard.rs`, …), touch nothing existing, and can't perturb the Time ranker.

| Dimension | Haskell | Shape | Estimate |
|---|---|---|---|
| Email | 41 L | one regex → `{value}` | ~½ day |
| Url | 63 L | one regex → `{value, domain, path, …}` | ~½ day |
| CreditCardNumber | 72 L | regex + **Luhn** checksum + issuer detection | ~½ day |
| PhoneNumber | 53 L | regex (country code / extension) → `{value}` | ~½ day |

**~2 days for all four**, fully parallel, near-zero risk.

### Bucket B — value/unit dimensions, all Numeral-dependent

| Dimension | Haskell | Numeral refs | Estimate |
|---|---|---|---|
| Temperature | 161 L | yes (`isValueOnly`) | ~½–1 day |
| Volume | 166 L | 7 | ~½–1 day |
| Distance | 213 L | 13 | ~1 day |
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

1. `parse_numeral` (close the Ordinal asymmetry) — ~15 min.
2. Email → Url → CreditCardNumber → PhoneNumber (quick wins, one commit each,
   corpus + oracle-verified).
3. Finish Numeral (K/M/G/lakh + remaining fractions), validated vs
   `Numeral/EN/Corpus.hs`.
4. Temperature → Volume → Distance → Quantity → AmountOfMoney.

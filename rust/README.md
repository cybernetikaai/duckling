# Duckling → Rust (English NLU entity parser)

A behavior-compatible Rust port of [Duckling's](https://github.com/facebook/duckling)
English parsing. It takes natural-language text ("tomorrow at 5pm", "in half an
hour", "$20 and 43c", "2 lbs of coffee", "20 degrees") and resolves it to
structured values, matching Duckling's output. Built for parsing user speech and
coercing the result into a target timezone.

Every Duckling EN dimension that has a corpus is ported and green against it:

- **Time**: 1069/1069 of Duckling's transcribed EN corpus (100%), validated against
  the live oracle across spoken/ASR, interval, holiday, and timezone surfaces.
- **Duration**, **Numeral**, **Ordinal**: full `*/EN/Corpus.hs` + oracle differential.
- **Value/unit dimensions**: **Temperature**, **Volume**, **Distance** (incl.
  composite "7 feet 10 inches"), **Quantity** (incl. "2 cups of sugar", mg/kg
  scaling), **AmountOfMoney** (~50 currencies, "$20 and 43c" cents composition).
- **Regex dimensions**: **Email**, **Url**, **CreditCardNumber** (Luhn),
  **PhoneNumber**.
- **`parse_all`**: extract *every* dimension from one utterance, merged by
  cross-dimension range domination.
- **Timezone/DST**: offsets verified per-instant against authoritative IANA tzdata
  (287 cases across 17 zones, incl. 45-minute and DST-in-exotic-zone offsets).

## Requirements

- Rust (stable), edition 2024 → **Rust 1.85+**. Install via [rustup](https://rustup.rs).
- No **build-time** system dependencies; all crates are pure-Rust (`jiff`,
  `fancy-regex`, `serde`).
- **Runtime (Unix): the system IANA tz database.** `jiff` resolves zones from
  `/usr/share/zoneinfo` (the `tzdata` package) and derives every DST-correct
  offset from it — so the full IANA database is available and stays current with
  the OS. `tzdata` is present on standard Ubuntu/Debian/macOS; a *minimal*
  container (distroless, `scratch`, slimmed Alpine/Ubuntu) must either install
  `tzdata` or build `jiff` with `features = ["tzdb-bundle-always"]` to compile the
  database into the binary. Without either, `TimeZone::get(...)` fails at runtime.

## Build

From the `rust/` directory:

```bash
cd rust

# Build the optimized CLI binary (recommended)
cargo build --release
# → binary at: target/release/duckling

# Or a debug build
cargo build
# → binary at: target/debug/duckling

# Run the full test suite (unit + corpus + differentials)
cargo test
```

Optionally install the binary onto your `PATH`:

```bash
cargo install --path .        # installs `duckling`
```

## Use the CLI

Pass the text as an argument (or pipe it on stdin). Output is a JSON array of
entities: `{dim, body, start, end, value, latent}`.

```bash
duckling "tomorrow at 5pm"
echo "in half an hour" | duckling
```

### Options

| Option | Values | Default | Purpose |
|---|---|---|---|
| `--dims` | `time` \| `duration` \| `ordinal` \| `number` \| `temperature` \| `volume` \| `distance` \| `quantity` \| `amount-of-money` \| `email` \| `url` \| `credit-card-number` \| `phone-number` \| `all` | `time` | Which dimension to extract. `all` = every dimension, merged by range domination. |
| `--ref` | RFC 3339 timestamp | system now | The reference "now" that relative expressions ("tomorrow", "in 2 hours") resolve from. |
| `--tz` | IANA zone (e.g. `America/New_York`) | `UTC` | **Target timezone.** Relative expressions resolve in this zone and output offsets are derived from it. Set it here at parse time to coerce into the target zone — don't convert the result afterward. |
| `--locale` | `en_US` `en_GB` `en_CA` `en_AU` `en_NZ` `en_IN` `en_IE` `en_ZA` `en_PH` `en_BZ` `en_JM` `en_TT` | `en_US` | English locale. Affects numeric date order (US `3/4`→Mar 4, GB `3/4`→Apr 3) and regional holidays. |
| `-h`, `--help` | | | Print help. |

### Examples

```bash
# Time in a target zone, with a fixed reference instant
duckling --tz America/New_York --ref 2013-02-12T04:30:00Z "in 2 hours"
# → [{"dim":"time", ..., "value":{"value":"2013-02-12T01:30:00.000-05:00", ...}}]

# Every dimension from one utterance (range-dominated: noise numerals dropped)
duckling --dims all "pay $20 for 2 lbs of coffee at 3pm"
# → an "amount-of-money" ($20), a "quantity" (2 lb of coffee), and a "time" (3pm)

# A standalone duration
duckling --dims duration "an hour and a half"
# → {"value":90,"unit":"minute","normalized":{"value":5400,"unit":"second"}, ...}

# Money with cents composition
duckling --dims amount-of-money "twenty dollars and 43 cents"   # → $20.43

# A quantity with a product
duckling --dims quantity "3 cups of sugar"     # → {value:3, unit:cup, product:sugar}

# UK day-first dates
duckling --locale en_GB "13/12/2013"          # → 2013-12-13

# A holiday
duckling --ref 2013-01-01T00:00:00Z "thanksgiving"   # → 2013-11-28
```

## Use as a library

Add the crate as a path/git dependency, then call the parse functions. Each
returns `Vec<duckling::Entity>` (serializable to Duckling's JSON via `serde`).

```rust
use duckling::{parse, parse_locale, parse_all, parse_amountofmoney, parse_quantity,
               Locale, ResolveContext};

fn main() {
    // Build a context: a reference instant + the target zone to resolve in.
    let zone = jiff::tz::TimeZone::get("America/New_York").unwrap();
    let reference = "2013-02-12T04:30:00Z".parse::<jiff::Timestamp>().unwrap();
    let ctx = ResolveContext { reference, zone, with_latent: false };

    // Time (US English).
    let times = parse("tomorrow at 5pm", &ctx);

    // Time in another English locale (numeric date order differs).
    let gb = parse_locale("13/12/2013", &ctx, Locale::EnGb);

    // Value dimensions are context-free (no ResolveContext needed).
    let money = parse_amountofmoney("twenty dollars and 43 cents");   // $20.43
    let qty = parse_quantity("3 cups of sugar");                      // 3 cup of sugar

    // Every dimension from one utterance, merged by range domination.
    let all = parse_all("pay $20 for 2 lbs of coffee at 3pm", &ctx);

    for e in times.iter().chain(&gb).chain(&money).chain(&qty).chain(&all) {
        println!("{} [{}..{}] {}", e.dim, e.start, e.end, e.value);
    }
}
```

### Entry points

| Function | Dimension(s) | Notes |
|---|---|---|
| `parse(input, ctx)` | Time | US English (`en_US`). |
| `parse_locale(input, ctx, locale)` | Time | Pick the English locale. |
| `parse_all(input, ctx)` | **all** | Every dimension, merged by cross-dimension range domination (span-contained entities dropped; disjoint all surface). |
| `parse_time_and_duration(input, ctx)` | Time + Duration | The `dims:["time","duration"]` pool, classifier-ranked. |
| `parse_duration(input)` | Duration | Context-free. |
| `parse_numeral(input)` | Numeral | Context-free (corpus-complete). |
| `parse_ordinal(input)` | Ordinal | Context-free. |
| `parse_temperature(input)` | Temperature | Context-free. `{value, unit}` (degree/celsius/fahrenheit). |
| `parse_volume(input)` | Volume | Context-free. ml/hl/l/gallon. |
| `parse_distance(input)` | Distance | Context-free. km/mi/m/cm/in/yd/ft; composite fold. |
| `parse_quantity(input)` / `_opts(input, with_latent)` | Quantity | Context-free. `{value, unit, product?}`. |
| `parse_amountofmoney(input)` / `_opts(input, with_latent)` | AmountOfMoney | Context-free. `{value, unit}` (currency); cents composition. |
| `parse_email(input)` | Email | Context-free. Literal + spelled-out (`a at b dot com`). |
| `parse_url(input)` | Url | Context-free. `{value, domain}`. |
| `parse_creditcard(input)` | CreditCardNumber | Context-free. Luhn-validated; `{value, issuer}`. |
| `parse_phonenumber(input)` | PhoneNumber | Context-free. Normalized `{value}`. |

### `ResolveContext`

```rust
pub struct ResolveContext {
    pub reference: jiff::Timestamp,   // "now" as a true UTC instant
    pub zone: jiff::tz::TimeZone,     // target zone: relative exprs resolve here; output offsets come from here
    pub with_latent: bool,            // false drops latent parses (e.g. a bare "7" as an hour)
}
```

To coerce parsed times into a user's zone, set `zone` to that zone here — the
resolver derives each value's offset per-instant (DST-correct), so you never
convert after the fact.

## Project layout & adding a language

The crate is organized **by dimension, then by language** (mirroring Duckling's
`Dimension/LANG` layout). Each dimension keeps its language-agnostic value type
and shared math in `mod.rs`, and its language-specific rules in a `<lang>` module:

```
src/
  types.rs, engine.rs, ranking.rs, resolve.rs, document.rs, regex.rs   # core (language-agnostic)
  grain.rs                                                             # Grain + calendar math
  numeral/   { mod.rs = NumeralData + accessors,  en.rs = English rules }
  ordinal/   { mod.rs = OrdinalData,               en.rs = English rules }
  duration/  { mod.rs = DurationData + Semigroup,  en.rs = English rules }
  timegrain/ { mod.rs = shim,                      en.rs = English grain words }
  temperature/ volume/ distance/ quantity/ amountofmoney/              # value/unit dims
             { mod.rs = value type + unit enum,    en.rs = English rules }
  url.rs, creditcard.rs, phonenumber.rs            # language-agnostic regex dims (crate root)
  email/     { mod.rs = EmailData,                 en.rs = English "at"/"dot" forms }
  time/
    object.rs, predicate.rs, computed.rs           # language-agnostic time machinery
    en/  { mod.rs = helpers + en_rules(locale);  dates.rs timeofday.rs intervals.rs
           cycles.rs holidays.rs modifiers.rs }    # English Time rules, split by concern
```

Each value/unit and regex dimension parses in its **own rule set** (numeral +
that dimension's rules), isolated from the Time rule set — so adding or changing
one can never perturb the Time corpus. `parse_all` runs them all and merges the
results by range domination.

**To add a language** (e.g. Spanish), add sibling `<lang>` modules and wire them
into `build_rules` in `lib.rs`:

```rust
r.extend(numeral::es::numeral_rules());
r.extend(ordinal::es::ordinal_rules());
r.extend(timegrain::es::timegrain_rules());
r.extend(duration::es::duration_rules());
r.extend(time::es::es_rules(locale));   // time/es/ mirrors time/en/
```

The agnostic pieces (value types, the Semigroup, calendar math, the engine and
ranker) are reused as-is; only the words/regexes are new. **Region** differences
(e.g. en_US vs en_GB date order, regional holidays) stay *data-driven* — a
`Locale` enum threaded through the builders plus region-keyed JSON fixtures — so
they don't need per-region modules.

## Notes

- **Faithful to Duckling**, with a few deliberate, documented divergences (e.g.
  correct per-instant DST offsets on transition boundaries, and beyond-Duckling
  holidays introduced after Duckling's data froze — see `docs/RUST_PORT_PROGRESS.md`).
- Duckling does **dimension extraction, not intent detection**: "the second option"
  yields a Time ("the 2nd"), exactly as upstream Duckling does. Filter by intent in
  your application layer if needed.
- Progress log and design notes: [`docs/RUST_PORT_PROGRESS.md`](../docs/RUST_PORT_PROGRESS.md).

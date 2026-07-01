# Duckling → Rust (English Time/Duration parser)

A behavior-compatible Rust port of [Duckling's](https://github.com/facebook/duckling)
English **Time**, **Duration**, and **Ordinal** parsing. It takes natural-language
text ("tomorrow at 5pm", "in half an hour", "the third of March") and resolves it
to structured values, matching Duckling's output. Built for parsing user speech
and coercing the result into a target timezone.

- **Time**: 1069/1069 of Duckling's transcribed EN corpus (100%), validated against
  the live oracle across spoken/ASR, interval, holiday, and timezone surfaces.
- **Duration**: full `Duration/EN/Corpus.hs` + oracle differential.
- **Ordinal**: full `Ordinal/EN/Corpus.hs`.
- **Timezone/DST**: offsets verified per-instant against authoritative IANA tzdata.

## Requirements

- Rust (stable), edition 2024 → **Rust 1.85+**. Install via [rustup](https://rustup.rs).
- No system dependencies; all crates are pure-Rust (`jiff`, `fancy-regex`, `serde`).

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
| `--dims` | `time` \| `duration` \| `ordinal` \| `all` | `time` | Which dimension(s) to extract. `all` = Time + Duration ranked together. |
| `--ref` | RFC 3339 timestamp | system now | The reference "now" that relative expressions ("tomorrow", "in 2 hours") resolve from. |
| `--tz` | IANA zone (e.g. `America/New_York`) | `UTC` | **Target timezone.** Relative expressions resolve in this zone and output offsets are derived from it. Set it here at parse time to coerce into the target zone — don't convert the result afterward. |
| `--locale` | `en_US` `en_GB` `en_CA` `en_AU` `en_NZ` `en_IN` `en_IE` `en_ZA` `en_PH` `en_BZ` `en_JM` `en_TT` | `en_US` | English locale. Affects numeric date order (US `3/4`→Mar 4, GB `3/4`→Apr 3) and regional holidays. |
| `-h`, `--help` | | | Print help. |

### Examples

```bash
# Time in a target zone, with a fixed reference instant
duckling --tz America/New_York --ref 2013-02-12T04:30:00Z "in 2 hours"
# → [{"dim":"time", ..., "value":{"value":"2013-02-12T01:30:00.000-05:00", ...}}]

# Multiple dimensions from one utterance (Time + Duration, range-dominated)
duckling --dims all "set a timer for 20 minutes and wake me at 7am"
# → a "duration" entity (20 minutes) and a "time" entity (7am)

# A standalone duration
duckling --dims duration "an hour and a half"
# → {"value":90,"unit":"minute","normalized":{"value":5400,"unit":"second"}, ...}

# UK day-first dates
duckling --locale en_GB "13/12/2013"          # → 2013-12-13

# A holiday
duckling --ref 2013-01-01T00:00:00Z "thanksgiving"   # → 2013-11-28
```

## Use as a library

Add the crate as a path/git dependency, then call the parse functions. Each
returns `Vec<duckling::Entity>` (serializable to Duckling's JSON via `serde`).

```rust
use duckling::{parse, parse_locale, parse_duration, parse_ordinal, parse_all,
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

    // Duration and Ordinal are context-free.
    let durs = parse_duration("an hour and a half");
    let ords = parse_ordinal("twenty-fifth");

    // Time + Duration together, ranked by range domination.
    let all = parse_all("set a timer for 20 minutes and wake me at 7am", &ctx);

    for e in times.iter().chain(&gb).chain(&durs).chain(&ords).chain(&all) {
        println!("{} [{}..{}] {}", e.dim, e.start, e.end, e.value);
    }
}
```

### Entry points

| Function | Dimension(s) | Notes |
|---|---|---|
| `parse(input, ctx)` | Time | US English (`en_US`). |
| `parse_locale(input, ctx, locale)` | Time | Pick the English locale. |
| `parse_duration(input)` | Duration | Context-free. |
| `parse_ordinal(input)` | Ordinal | Context-free. |
| `parse_all(input, ctx)` | Time + Duration | One pool, ranked by range domination (widest match wins; disjoint matches all surface). |

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

## Notes

- **Faithful to Duckling**, with a few deliberate, documented divergences (e.g.
  correct per-instant DST offsets on transition boundaries, and beyond-Duckling
  holidays introduced after Duckling's data froze — see `docs/RUST_PORT_PROGRESS.md`).
- Duckling does **dimension extraction, not intent detection**: "the second option"
  yields a Time ("the 2nd"), exactly as upstream Duckling does. Filter by intent in
  your application layer if needed.
- Progress log and design notes: [`docs/RUST_PORT_PROGRESS.md`](../docs/RUST_PORT_PROGRESS.md).

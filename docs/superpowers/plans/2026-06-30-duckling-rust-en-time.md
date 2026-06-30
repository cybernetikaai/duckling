# Duckling → Rust: English Time Parsing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Port Duckling's English **Time** dimension (and the dependencies it needs) from Haskell to an idiomatic Rust crate, validated against Duckling's own test corpus, by porting the tests first and using them to drive every line of logic.

**Architecture:** A new Rust crate lives in `rust/` inside this repo so the Haskell build stays adjacent as a behavioral oracle. Phase 0 turns the Haskell corpus into a language-neutral JSON fixture file (the oracle's output) and stands up a Rust test harness that loads it — producing a wall of failing tests. Every later phase is pure red→green: port the smallest unit of logic that turns a cluster of fixtures green. The engine is a clean reimplementation of Duckling's bottom-up saturating chart parser (not a line-by-line transcription); behavioral fidelity is guaranteed by the fixtures, not by mirroring Haskell internals — **with one exception: timezone/DST correctness, which the corpus does not cover and which is verified by a separate DST-stress fixture set (see the Timezone section).**

**Tech Stack:** Rust (**edition 2024** — current since Rust 1.85, Feb 2025) · **`jiff` (chosen)** — civil + zoned datetime with first-class IANA timezones and explicit DST ambiguity/gap handling, matching Haskell's wall-clock-vs-instant split · `fancy-regex` 0.13 (PCRE-style lookaround) · `serde` / `serde_json` (fixtures) · Haskell `stack` (oracle, incl. real IANA zones via `loadTimeZoneSeries`).

## Global Constraints

- **Crate location:** `rust/` at repo root. Crate name `duckling`. **Edition 2024** (current as of 2026; editions are opt-in language epochs, not release years).
- **Datetime in code samples:** civil values are `jiff::civil::DateTime` built with `jiff::civil::date(y, m, d).at(h, mi, s, 0)`; calendar arithmetic uses `jiff::Span`; per-instant offsets come from `jiff::tz::TimeZone`. (Any `chrono`-style snippet left from an early draft is superseded by this.)
- **Scope:** English (`EN`, no region) **Time** only. Port dependency dimensions (TimeGrain, Numeral, Ordinal, Duration) **only to the extent the EN Time corpus exercises them** — driven by fixtures, never speculatively.
- **Fidelity:** *Behavior-compatible* — match the resolved value, not byte-identical Haskell internals. The comparison key is the Time value JSON **with the `values` array removed** (this mirrors Duckling's own corpus check in `Duckling/Time/Corpus.hs:68-75`).
- **Reference context (verbatim from `Duckling/Testing/Types.hs:71-81`):** reference time = `2013-02-12 04:30:00` at a **constant** `-02:00` offset (no tz transitions); locale = `EN`, no region; `withLatent = false`.
- **Timezone is first-class, not deferred** (see the Timezone section). Internal math stays naive wall-clock (as Duckling does), but `ResolveContext` carries a real IANA zone and the **output offset is computed per-resolved-instant from that zone, never hard-coded**. In-text named-zone parsing (`parseTimezone` + `inTimezone`/`shiftTimezone`) is corpus-tested and ships in the MVP. Correctness is verified **beyond** the corpus with a DST-stress fixture set, because the corpus is DST-free by construction.
- **TDD always:** failing test → run-it-fails → minimal impl → run-it-passes → commit. Conventional commit messages (`feat:`, `test:`, `chore:`). Frequent commits.
- **A rule is "done" only when its fixtures pass** under `cargo test` with output shown.

---

## Strategy: tests first, then logic

```
Haskell corpus (defaultCorpus + negativeCorpus)
        │  run the existing parser once, per example
        ▼
fixtures/en_time_corpus.json   ← the oracle's answers, "values" stripped
        │  loaded by
        ▼
rust/tests/corpus.rs           ← one Rust test per example (all RED at first)
        │  drives
        ▼
port grain → engine → deps → rules → ranking   (each step turns fixtures GREEN)
```

**The assertion ladder** (loosen early, tighten late) lets logic land before ranking exists:

- **Phases 1–4 (`MatchMode::Contains`):** a fixture passes if the expected value appears among the parser's full-range Time results. Multiple competing parses are tolerated.
- **Phase 5 (`MatchMode::Unique`):** a fixture passes only if there is **exactly one** full-range Time result and it equals the expected value. This is Duckling's real corpus semantics (`makeCorpusTest` fails on >1 full-range token) and is what ranking exists to satisfy.

The harness reads the mode from an env var (`DUCKLING_MATCH=contains|unique`, default `contains`) so the bar tightens without editing tests.

## Dependency map — what EN Time pulls in

From `Duckling/Time/EN/Rules.hs` imports and `Duckling/Time/Helpers.hs`:

| Dependency | Why Time needs it | Corpus-exercised subset to port |
| --- | --- | --- |
| **TimeGrain** | grain words ("month", "2 weeks") | full enum + EN grain rules (small) |
| **Numeral** | "in **3** weeks", "**a** week", hour/minute integers | EN integers 0–99, "a/an", written numbers used in the Time corpus |
| **Ordinal** | "the **3rd**", "**first** monday of March" | EN ordinals 1st–31st |
| **Duration** | "**3 weeks**", "in **a year**" | `Numeral × Grain → Duration` + EN duration rules |
| **Engine/Core** | the parser itself | `Token`, `Node`, `Rule`, `Range`, `Document`, regex, saturation, resolve |
| **Ranking** | collapse ambiguous parses to one winner | naive-Bayes scorer + EN model (Phase 5) |

## Timezone: the correctness backbone

> An earlier draft deferred this and called it out-of-scope for the MVP. That was wrong: the corpus converts timezones, and — more dangerously — being corpus-green does **not** prove timezone correctness. Three layers, all real:

1. **In-text named zones** — "8:00 PST", "4pm CET", "15:00 GMT". `parseTimezone` (`Time/TimeZone/Parse.hs`) maps ~150 abbreviations to **fixed** offsets (PST = −480, GMT = 0, CET = +60; DST variants like PDT are *separate* entries). `inTimezone`/`shiftTimezone` (`Helpers.hs:623`) shift the time-of-day by `ctxOffset − providedOffset` minutes into the reference frame. **Corpus-tested (14 EN lines) → MVP.** E.g. under the −02:00 test context, "Thursday 8:00 PST" → `2013-02-14T14:00:00−02:00`.
2. **Reference-zone DST** — production builds the reference via `makeReftime series "America/New_York" utc`, a real IANA series. Both `shiftTimezone`'s `ctxOffset` and the output stamp call `timeZoneFromSeries series instant`, so the offset depends on whether the **resolved instant** is EST or EDT. "in 4 months" / "first Sunday of November 1:30am" can cross a transition and must pick the right offset for the resolved date.
3. **Output offset** — the RFC3339 `±HH:MM` is the reference zone's offset *at the resolved instant*, not a constant.

**The trap:** the test context is a fixed −02:00 offset with **zero transitions**, and named zones are fixed offsets, so the suite never crosses DST. **A port can be 100% corpus-green and still be wrong for any real IANA reference zone across a DST boundary — the normal production case.** Corpus-green ≠ timezone-correct.

**Corrected design:**
- Internal naive-wall-clock math stays (predicates operate on local wall clock, exactly as Duckling does).
- `ResolveContext` carries a real **zone** (IANA id). The −02:00 test context is the degenerate fixed-offset case of the same code path.
- The **output offset is computed per-resolved-instant from the zone** (`zone.offset_at(instant)`), never hard-coded.
- Port `parseTimezone` + `inTimezone`/`shiftTimezone` as an **early** cluster (end of Phase 1 / start of Phase 3), since they're corpus-tested.
- **Beyond-corpus DST verification (Task 0.6)** is the safety net the corpus can't provide.

**Production usage — "coerce into the user's target zone" (the main intended use):** supported natively — **set the reference zone to the target zone at parse time**, and every result comes out in that zone (relative expressions anchor to the user's local clock; in-text zones like "3pm EST" are shifted into it; output offsets are DST-correct per instant). Do **not** parse in one zone and convert the resolved *instant* afterward — results are grain-bearing ("tomorrow" = a *day* at local midnight) and intervals, so instant-conversion silently corrupts them (UTC "tomorrow" midnight → the previous evening in Pacific). API consequences:
1. `ResolveContext.zone` is a **required** parameter — no hidden default. When the target zone is unknown, the caller passes an explicit fallback (UTC or the speaker's detected zone) and **re-parses** if the zone is learned later, rather than converting a stale result.
2. Each resolved value is exposed as / convertible to a `jiff::Zoned` (instant + zone + grain), so a caller who genuinely wants instant-preserving display in another zone can use `zoned.with_time_zone(target)` — DST-correct, and valid *only* for fine-grained absolute instants, never day+ grains or intervals.

**Library decision — RESOLVED: `jiff`.** Civil-vs-zoned matches Haskell's wall-clock-vs-instant split exactly; `jiff::tz::TimeZone` gives per-instant offsets and explicit DST fold/gap handling (closest to `timezone-series`). Internal math uses `jiff::civil::DateTime`; `ResolveContext` carries a `jiff::tz::TimeZone`; the output offset for each resolved instant is read from that zone. `jiff`'s Temporal-style "constrain" overflow also reproduces Haskell's month/year day-clipping for free.

## File structure (Rust crate)

```
rust/
  Cargo.toml
  fixtures/en_time_corpus.json        # generated by the Haskell dumper (Phase 0)
  src/
    lib.rs                            # pub API: parse(input, ctx) -> Vec<Entity>
    grain.rs                          # Grain enum, add(), round(), in_seconds()
    document.rs                       # input text + adjacency rules for matching
    regex.rs                          # fancy-regex wrapper (case-insensitive, position-aware)
    types.rs                          # Token, Dimension, Node, Range, Rule, Pattern, Production
    engine.rs                         # bottom-up saturating matcher
    resolve.rs                        # ResolveContext, value->JSON, Entity
    json.rs                           # rfc3339 + Time value JSON shaping
    time/
      mod.rs
      object.rs                       # TimeObject + time_plus/round/interval/intersect
      predicate.rs                    # Predicate enum + series iterators + runners
      helpers.rs                      # combinators: cycle_nth, intersect, interval, ...
      en_rules.rs                     # English Time rules
    numeral/{mod.rs, types.rs, en_rules.rs}
    ordinal/{mod.rs, types.rs, en_rules.rs}
    duration/{mod.rs, types.rs, en_rules.rs}
    timegrain/{mod.rs, en_rules.rs}
    ranking/{mod.rs, model.rs}        # Phase 5
  tests/
    corpus.rs                         # the golden harness
exe/
  CorpusDump.hs                       # Haskell oracle dumper (Phase 0)
```

---

# Phase 0 — Oracle + corpus extraction (tests first)

**Deliverable:** `rust/fixtures/en_time_corpus.json` committed, and a Rust test harness that loads it and fails on every example. No parsing logic yet.

### Task 0.1: Build the Haskell oracle and confirm the baseline is green

**Files:** none created — environment + verification only.

- [ ] **Step 1: Build the library and test suite**

Run:
```bash
cd /Users/13protons/github/duckling
stack build
```
Expected: completes (first build compiles GHC deps + PCRE; may take many minutes). If `stack` is absent, install via `curl -sSL https://get.haskellstack.org/ | sh`. If the system lacks the PCRE C library, install it (`brew install pcre` on macOS) and rerun.

- [ ] **Step 2: Run the existing EN Time tests to confirm the oracle is trustworthy**

Run:
```bash
stack test duckling:duckling-test 2>&1 | tail -20
```
Expected: the suite reports `All N tests passed` (or the Time group passes). This proves the corpus output we are about to dump is correct.

- [ ] **Step 3: Commit nothing** (verification task). Record in the PR description that the baseline passed.

---

### Task 0.2: Write the corpus dumper executable

**Files:**
- Create: `exe/CorpusDump.hs`
- Modify: `duckling.cabal` (add an `executable` stanza; all corpus modules are already in the library's `exposed-modules`, so `build-depends: duckling` is sufficient)

**Interfaces:**
- Consumes: `Duckling.Core.parse`, `Duckling.Time.EN.Corpus.defaultCorpus`, `Duckling.Time.EN.Corpus.negativeCorpus`, `Duckling.Testing.Types (testContext, testOptions)`.
- Produces: a JSON document on stdout with shape `{ "context": {...}, "positive": [{input, expected|null}], "negative": [string] }`. `expected` is the resolved Time value JSON with the `"values"` key removed; `null` flags an input the oracle did not resolve to exactly one full-range Time entity (logged, not silently dropped).

- [ ] **Step 1: Write the dumper**

```haskell
-- exe/CorpusDump.hs
{-# LANGUAGE OverloadedStrings #-}
module Main (main) where

import           Control.Monad (forM)
import           Data.Aeson (Value(..), object, toJSON, (.=))
import qualified Data.Aeson as A
import qualified Data.Aeson.KeyMap as K
import qualified Data.ByteString.Lazy as LBS
import           Data.Text (Text)
import qualified Data.Text as T
import           Prelude

import           Duckling.Core
import           Duckling.Testing.Types (testContext, testOptions)
import qualified Duckling.Time.EN.Corpus as EN

-- The corpus check compares values with the "values" key deleted; mirror that.
stripValues :: Value -> Value
stripValues (Object o) = Object (K.delete "values" o)
stripValues v          = v

valueJSON :: Entity -> Value
valueJSON e = case value e of RVal _ v -> stripValues (toJSON v)

fullRange :: Text -> Entity -> Bool
fullRange input e = start e == 0 && end e == T.length input

main :: IO ()
main = do
  let (ctx, opts, examples) = EN.defaultCorpus
      (_, _, negatives)      = EN.negativeCorpus
  positive <- forM examples $ \(input, _predicate) -> do
    let ents = filter (fullRange input) (parse input ctx opts [Seal Time])
    pure $ case ents of
      [e] -> object ["input" .= input, "expected" .= valueJSON e]
      _   -> object ["input" .= input, "expected" .= Null,
                     "ambiguousCount" .= length ents]
  let doc = object
        [ "context" .= object
            [ "referenceTime" .= ("2013-02-12T04:30:00.000-02:00" :: Text)
            , "locale"        .= ("en" :: Text)
            , "withLatent"    .= False
            ]
        , "positive" .= positive
        , "negative" .= negatives
        ]
  LBS.putStr (A.encode doc)
```

- [ ] **Step 2: Register the executable in `duckling.cabal`**

Append after the existing `executable duckling-expensive` stanza (the library already exposes the corpus modules, so no module list changes are needed):
```cabal
executable duckling-corpus-dump
  main-is:             CorpusDump.hs
  hs-source-dirs:      exe
  build-depends:       base
                     , duckling
                     , text
                     , aeson
                     , unordered-containers
                     , bytestring
  default-language:    Haskell2010
  ghc-options:         -threaded
```

- [ ] **Step 3: Build the dumper**

Run:
```bash
cd /Users/13protons/github/duckling
stack build duckling:duckling-corpus-dump
```
Expected: builds with no errors. If GHC reports a module is not exposed, add that module name to the library `exposed-modules:` list (block starting `duckling.cabal:34`) and rebuild — but per the cabal audit it is already exposed.

- [ ] **Step 4: Commit**

```bash
git add exe/CorpusDump.hs duckling.cabal
git commit -m "feat: add corpus dumper exe to emit EN time fixtures as JSON"
```

---

### Task 0.3: Generate and commit the fixture file

**Files:**
- Create: `rust/fixtures/en_time_corpus.json` (generated artifact, committed)

- [ ] **Step 1: Generate the fixtures**

Run:
```bash
cd /Users/13protons/github/duckling
mkdir -p rust/fixtures
stack exec duckling-corpus-dump > rust/fixtures/en_time_corpus.json
```
Expected: a multi-hundred-KB JSON file.

- [ ] **Step 2: Sanity-check the content**

Run:
```bash
python3 -c "import json;d=json.load(open('rust/fixtures/en_time_corpus.json'));print('positive',len(d['positive']),'negative',len(d['negative']));print([p for p in d['positive'] if p['input'] in ('today','tomorrow','now','yesterday')])"
```
Expected: prints counts (positive in the hundreds) and shows, e.g.:
```
{'input': 'today', 'expected': {'type': 'value', 'value': '2013-02-12T00:00:00.000-02:00', 'grain': 'day'}}
{'input': 'tomorrow', 'expected': {'type': 'value', 'value': '2013-02-13T00:00:00.000-02:00', 'grain': 'day'}}
{'input': 'now', 'expected': {'type': 'value', 'value': '2013-02-12T04:30:00.000-02:00', 'grain': 'second'}}
{'input': 'yesterday', 'expected': {'type': 'value', 'value': '2013-02-11T00:00:00.000-02:00', 'grain': 'day'}}
```

- [ ] **Step 3: Count how many examples the oracle left as `null`** (ambiguous/unresolved — these are the long tail ranking must later fix)

Run:
```bash
python3 -c "import json;d=json.load(open('rust/fixtures/en_time_corpus.json'));n=[p for p in d['positive'] if p['expected'] is None];print('null:',len(n))"
```
Expected: a small minority. Record the number; these are excluded from `Contains` mode and revisited in Phase 5.

- [ ] **Step 4: Commit**

```bash
git add rust/fixtures/en_time_corpus.json
git commit -m "test: generate EN time golden fixtures from Haskell oracle"
```

---

### Task 0.4: Scaffold the Rust crate

**Files:**
- Create: `rust/Cargo.toml`, `rust/src/lib.rs`

- [ ] **Step 1: Create `rust/Cargo.toml`**

```toml
[package]
name = "duckling"
version = "0.0.0"
edition = "2024"

[dependencies]
jiff = "0.2"          # or latest; civil + zoned datetime, IANA tz, DST handling
fancy-regex = "0.13"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
```

- [ ] **Step 2: Create `rust/src/lib.rs` with the public surface (stubs)**

```rust
pub mod grain;
pub mod document;
pub mod regex;
pub mod types;
pub mod engine;
pub mod resolve;
pub mod json;
pub mod time;

pub use resolve::{Entity, ResolveContext};

/// Parse `input` against the EN Time rules and return resolved entities.
/// Stub until Phase 1; returns empty so the harness compiles and fails loudly.
pub fn parse(_input: &str, _ctx: &ResolveContext) -> Vec<Entity> {
    Vec::new()
}
```

- [ ] **Step 3: Create minimal module stubs so the crate compiles**

Create each of `rust/src/{grain,document,regex,types,engine,json}.rs` and `rust/src/time/mod.rs` containing only a doc comment `//! stub` for now, plus `rust/src/resolve.rs`:
```rust
//! Resolution context and output entity.
use serde::Serialize;

/// Reference instant plus the zone it is interpreted in. The output offset is
/// derived per-resolved-instant from `zone` — never hard-coded. The Duckling
/// test context is just the special case where `zone` is a fixed -02:00 offset.
pub struct ResolveContext {
    pub reference: jiff::Timestamp,   // the "now" as a true UTC instant
    pub zone: jiff::tz::TimeZone,     // fixed -02:00 in tests; e.g. America/New_York in prod
    pub with_latent: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct Entity {
    pub dim: String,
    pub body: String,
    pub start: usize,
    pub end: usize,
    pub value: serde_json::Value,
    pub latent: bool,
}
```

- [ ] **Step 4: Verify it compiles**

Run:
```bash
cd rust && cargo build
```
Expected: builds with warnings about unused stubs, no errors.

- [ ] **Step 5: Commit**

```bash
git add rust/Cargo.toml rust/src
git commit -m "chore: scaffold rust duckling crate"
```

---

### Task 0.5: The golden test harness (every example RED)

**Files:**
- Create: `rust/tests/corpus.rs`

**Interfaces:**
- Consumes: `duckling::parse`, `duckling::ResolveContext`.
- Produces: a single `#[test]` that iterates all positive fixtures (skipping `expected == null`) and asserts behavior per `MatchMode`, plus a negative-corpus test.

- [ ] **Step 1: Write the harness**

```rust
use serde_json::Value;

fn load() -> Value {
    let raw = include_str!("../fixtures/en_time_corpus.json");
    serde_json::from_str(raw).expect("fixtures parse")
}

fn ctx() -> duckling::ResolveContext {
    // Test context: civil 2013-02-12 04:30:00 at a fixed -02:00 zone (no transitions).
    let zone = jiff::tz::TimeZone::fixed(jiff::tz::Offset::constant(-2));
    let reference = jiff::civil::date(2013, 2, 12).at(4, 30, 0, 0)
        .to_zoned(zone.clone()).unwrap().timestamp();
    duckling::ResolveContext { reference, zone, with_latent: false }
}

/// "values" is the alternatives array; behavior-compat ignores it.
fn strip_values(mut v: Value) -> Value {
    if let Value::Object(ref mut o) = v { o.remove("values"); }
    v
}

fn contains_mode() -> bool {
    std::env::var("DUCKLING_MATCH").as_deref() != Ok("unique")
}

#[test]
fn positive_corpus() {
    let data = load();
    let ctx = ctx();
    let mut failures = Vec::new();
    let mut checked = 0usize;
    for ex in data["positive"].as_array().unwrap() {
        let expected = &ex["expected"];
        if expected.is_null() { continue; } // long-tail, revisited in Phase 5
        checked += 1;
        let input = ex["input"].as_str().unwrap();
        let got: Vec<Value> = duckling::parse(input, &ctx)
            .into_iter()
            .filter(|e| e.dim == "time" && e.start == 0 && e.end == input.chars().count())
            .map(|e| strip_values(e.value))
            .collect();
        let exp = strip_values(expected.clone());
        let ok = if contains_mode() {
            got.iter().any(|g| g == &exp)
        } else {
            got.len() == 1 && got[0] == exp
        };
        if !ok { failures.push(format!("{input:?}\n  expected {exp}\n  got      {got:?}")); }
    }
    eprintln!("checked {checked}, {} failing", failures.len());
    assert!(failures.is_empty(), "{} failures:\n{}", failures.len(),
            failures.iter().take(40).cloned().collect::<Vec<_>>().join("\n"));
}

#[test]
fn negative_corpus() {
    let data = load();
    let ctx = ctx();
    let mut failures = Vec::new();
    for ex in data["negative"].as_array().unwrap() {
        let input = ex.as_str().unwrap();
        let n = duckling::parse(input, &ctx).into_iter()
            .filter(|e| e.dim == "time" && e.start == 0 && e.end == input.chars().count())
            .count();
        if n != 0 { failures.push(format!("{input:?} produced {n} time parses")); }
    }
    assert!(failures.is_empty(), "{} negatives leaked:\n{}", failures.len(),
            failures.join("\n"));
}
```

- [ ] **Step 2: Run it — expect the positive test to FAIL loudly, negative to PASS**

Run:
```bash
cd rust && cargo test --test corpus
```
Expected: `negative_corpus` passes (stub parses nothing), `positive_corpus` FAILS reporting hundreds of failures (`expected … got []`). This is the wall of red we drive to green.

- [ ] **Step 3: Commit**

```bash
git add rust/tests/corpus.rs
git commit -m "test: add golden corpus harness (red) with contains/unique modes"
```

---

### Task 0.6: DST-stress fixtures (timezone correctness beyond the corpus)

**Files:**
- Create: `exe/TzStressDump.hs` (oracle exe using a **real** IANA reference zone)
- Create: `rust/fixtures/tz_stress.json`
- Modify: `rust/tests/corpus.rs` (add a `tz_stress` test, same match logic, per-case context)

**Rationale:** the main corpus is DST-free. This set deliberately exercises the paths the corpus cannot: a reference zone with transitions, queries that cross spring-forward / fall-back, and a southern-hemisphere zone (DST flipped). The oracle computes the truth; the Rust port must match it.

**Interfaces:**
- Consumes: `Duckling.Core.parse`, `makeReftime`, `Duckling.Data.TimeZone.loadTimeZoneSeries`.
- Produces: `[{ zone, referenceTimeUtc, input, expected }]` where `expected` is the value JSON (`"values"` stripped). Each case carries its own zone + reference so the Rust harness builds a matching `ResolveContext`.

- [ ] **Step 1: Write `exe/TzStressDump.hs`** — for each (zone, refUtc, input) triple, `parse input (makeReftime series zone refUtc) testOptions [Seal Time]`, take the full-range Time entity, emit `{zone, referenceTimeUtc, input, expected}`. Seed cases (expand as needed):
  - zone `America/New_York`, ref `2013-01-15T12:00:00Z`: `"in 4 months"` (Jan EST → May EDT — offset must change), `"July 4th at noon"`, `"first Sunday of November"`, `"2am on November 3 2013"` (fall-back ambiguous hour).
  - zone `America/New_York`, ref `2013-03-09T12:00:00Z`: `"tomorrow at 2:30am"` (spring-forward gap), `"in 2 days"`.
  - zone `Europe/London`, ref `2013-06-01T12:00:00Z`: `"3pm EST"` (in-text zone differs from reference zone), `"Christmas"`.
  - zone `Australia/Sydney`, ref `2013-06-01T12:00:00Z`: `"in 6 months"` (southern DST flip), `"December 25th"`.
  - zone `America/Los_Angeles`, ref `2017-06-01T12:00:00Z`: `"the first Tuesday of October"` → must be `2017-10-03T00:00:00.000-07:00` (**PDT, not PST** — this is the README's own headline example, and nothing in the corpus covers it; if a port renders it `-08:00`, no corpus test catches it).
- [ ] **Step 2: Register exe in `duckling.cabal`** (mirror Task 0.2; add `other-modules: Duckling.Data.TimeZone`, `build-depends: containers, time, tzdata`/`timezone-olson` as the example exes do — copy the `duckling-expensive` stanza's deps).
- [ ] **Step 3: Generate** — `stack exec duckling-tz-stress-dump > rust/fixtures/tz_stress.json`. Inspect that `"in 4 months"` from a January EST reference yields a **−04:00** (EDT) output offset, proving the oracle shifts across DST.
- [ ] **Step 4: Add the Rust `tz_stress` test** — like `positive_corpus`, but build `ResolveContext` per case from `zone` + `referenceTimeUtc` (requires the chosen tz library; until the zone code lands this test is RED).
- [ ] **Step 5: Commit** — `test: add DST-stress fixtures with real IANA reference zones`.

**Phase 0 exit:** corpus fixtures **and** tz-stress fixtures committed; `cargo test` runs and shows the full red baseline (both sets failing, since no logic exists yet).

---

# Phase 1 — Core engine + first vertical slice

**Deliverable:** the `now` / `today` / `tomorrow` / `yesterday` fixtures pass in `Contains` mode — proving types, engine, regex, resolution, and JSON output end-to-end.

### Task 1.1: `Grain` enum with `add`, `round`, `in_seconds`

**Files:**
- Modify: `rust/src/grain.rs`
- Test: `rust/src/grain.rs` (inline `#[cfg(test)]`)

**Interfaces:**
- Produces: `enum Grain { NoGrain, Second, Minute, Hour, Day, Week, Month, Quarter, Year }` (deriving `Ord` so `NoGrain < … < Year`, matching Haskell); `fn add(dt: jiff::civil::DateTime, g: Grain, n: i64) -> jiff::civil::DateTime`; `fn round(dt: jiff::civil::DateTime, g: Grain) -> jiff::civil::DateTime`; `fn grain_str(g: Grain) -> &'static str`.

- [ ] **Step 1: Write failing tests** (ported from `Duckling/TimeGrain/Types.hs:add` semantics, incl. month-clip)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::{date, DateTime};
    fn d(y:i16,mo:i8,da:i8,h:i8,mi:i8,s:i8)->DateTime{ date(y,mo,da).at(h,mi,s,0) }
    #[test] fn add_day() { assert_eq!(add(d(2013,2,12,4,30,0),Grain::Day,1), d(2013,2,13,4,30,0)); }
    #[test] fn add_month_clip() { assert_eq!(add(d(2013,1,31,0,0,0),Grain::Month,1), d(2013,2,28,0,0,0)); }
    #[test] fn add_year() { assert_eq!(add(d(2016,2,29,0,0,0),Grain::Year,1), d(2017,2,28,0,0,0)); }
    #[test] fn round_day() { assert_eq!(round(d(2013,2,12,4,30,0),Grain::Day), d(2013,2,12,0,0,0)); }
    #[test] fn round_month() { assert_eq!(round(d(2013,2,12,4,30,0),Grain::Month), d(2013,2,1,0,0,0)); }
    #[test] fn round_week_to_monday() { assert_eq!(round(d(2013,2,12,4,30,0),Grain::Week), d(2013,2,11,0,0,0)); }
}
```

- [ ] **Step 2: Run — expect FAIL** (`cannot find function add`)

Run: `cd rust && cargo test grain::`
Expected: compile error / FAIL.

- [ ] **Step 3: Implement**

```rust
use jiff::civil::{date, DateTime, ISOWeekDate, Weekday};
use jiff::Span;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Grain { NoGrain, Second, Minute, Hour, Day, Week, Month, Quarter, Year }

pub fn grain_str(g: Grain) -> &'static str {
    match g {
        Grain::NoGrain => "no-grain", Grain::Second => "second", Grain::Minute => "minute",
        Grain::Hour => "hour", Grain::Day => "day", Grain::Week => "week",
        Grain::Month => "month", Grain::Quarter => "quarter", Grain::Year => "year",
    }
}

// jiff uses Temporal "constrain" overflow for calendar units, so month/year adds
// clip day-of-month exactly like Haskell's addGregorianMonthsClip/YearsClip
// (Jan 31 + 1mo -> Feb 28; Feb 29 + 1yr -> Feb 28).
pub fn add(dt: DateTime, g: Grain, n: i64) -> DateTime {
    let span = match g {
        Grain::NoGrain | Grain::Second => Span::new().seconds(n),
        Grain::Minute  => Span::new().minutes(n),
        Grain::Hour    => Span::new().hours(n),
        Grain::Day     => Span::new().days(n),
        Grain::Week    => Span::new().weeks(n),
        Grain::Month   => Span::new().months(n),
        Grain::Quarter => Span::new().months(3 * n),
        Grain::Year    => Span::new().years(n),
    };
    dt.checked_add(span).expect("datetime add overflow")
}

pub fn round(dt: DateTime, g: Grain) -> DateTime {
    match g {
        Grain::Week => {
            let iso = round(dt, Grain::Day).date().iso_week_date();
            ISOWeekDate::new(iso.year(), iso.week(), Weekday::Monday)
                .unwrap().date().at(0, 0, 0, 0)
        }
        Grain::Quarter => {
            let m = round(dt, Grain::Month);
            add(m, Grain::Month, -(((m.month() as i64) - 1) % 3))
        }
        _ => {
            let mo = if g > Grain::Month  { 1 } else { dt.month() };
            let da = if g > Grain::Day    { 1 } else { dt.day() };
            let h  = if g > Grain::Hour   { 0 } else { dt.hour() };
            let mi = if g > Grain::Minute { 0 } else { dt.minute() };
            let s  = if g > Grain::Second { 0 } else { dt.second() };
            date(dt.year(), mo, da).at(h, mi, s, 0)
        }
    }
}
```

- [ ] **Step 4: Run — expect PASS** (`cargo test grain::`). Expected: 6 passed.
- [ ] **Step 5: Commit** — `git commit -am "feat: port TimeGrain add/round with month-clip semantics"`

---

### Task 1.2: `TimeObject` + interval/intersect helpers

**Files:** Modify `rust/src/time/object.rs` (create), declare in `rust/src/time/mod.rs`.

**Interfaces:**
- Produces: `struct TimeObject { start: jiff::civil::DateTime, grain: Grain, end: Option<jiff::civil::DateTime> }` (derive `Clone, Copy, PartialEq, Debug`); `time_plus(t, g, n)`, `time_end(t)`, `time_interval(kind, a, b)`, `time_intersect(a, b) -> Option<TimeObject>`, `enum IntervalType { Open, Closed }`, `enum IntervalDirection { Before, After }`. Direct ports of `Duckling/Time/Types.hs:806-872`.

- [ ] **Step 1: Failing tests** — port two cases from the Haskell semantics:
```rust
#[test] fn plus_keeps_min_grain() {
    let t = obj(2013,2,12,0,0,0, Grain::Day);
    assert_eq!(time_plus(t, Grain::Day, 1).start, dt(2013,2,13,0,0,0));
}
#[test] fn intersect_day_and_hour() {
    let day = obj(2013,2,12,0,0,0, Grain::Day);
    let hour = obj(2013,2,12,16,0,0, Grain::Hour);
    let i = time_intersect(day, hour).unwrap();
    assert_eq!(i.start, dt(2013,2,12,16,0,0));
    assert_eq!(i.grain, Grain::Hour);
}
```
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement** `TimeObject` and port `timePlus`, `timeEnd`, `timeInterval`, `timeIntersect`, `timeStartsBeforeTheEndOf`, `timeBefore` from `Duckling/Time/Types.hs:806-872` (same branch structure; `Option<jiff::civil::DateTime>` for `end`; `min` of grains via `Ord`).
- [ ] **Step 4: Run — PASS.**
- [ ] **Step 5: Commit** — `feat: port TimeObject + interval/intersect`.

---

### Task 1.3: `Predicate` with lazy series (iterators) + `cycle_nth`

**Files:** Modify `rust/src/time/predicate.rs` (create), declare in `mod.rs`.

**Interfaces:**
- Produces: `enum Predicate { Empty, Series(Rc<SeriesFn>) }` where `type SeriesFn = dyn Fn(TimeObject, &TimeContext) -> (BoxIter, BoxIter)` and `type BoxIter = Box<dyn Iterator<Item = TimeObject>>`; `struct TimeContext { ref_time, min_time, max_time }`; `fn run(&self, t, ctx) -> (BoxIter, BoxIter)`; `fn time_cycle(g)`, `fn cycle_nth(g, n)`. These reproduce `timeCycle`/`cycleNth` behavior (`Duckling/Time/Helpers.hs:104-107` + the cycle machinery). Boxed iterators preserve Haskell's laziness so fine-grain series never materialize unbounded.

- [ ] **Step 1: Failing test**
```rust
#[test] fn cycle_nth_today_tomorrow_yesterday() {
    let ctx = tctx(2013,2,12,4,30,0);
    let head = |p: &Predicate| p.run(ctx.ref_time, &ctx).1.next().unwrap();
    assert_eq!(head(&cycle_nth(Grain::Day, 0)).start,  dt(2013,2,12,0,0,0));
    assert_eq!(head(&cycle_nth(Grain::Day, 1)).start,  dt(2013,2,13,0,0,0));
    assert_eq!(head(&cycle_nth(Grain::Day, -1)).start, dt(2013,2,11,0,0,0));
    assert_eq!(head(&cycle_nth(Grain::Second, 0)).start, dt(2013,2,12,4,30,0));
}
```
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement**
```rust
use std::rc::Rc;
use std::iter::successors;
use crate::grain::{Grain, add as grain_add};
use crate::time::object::{TimeObject, time_round};

pub type BoxIter = Box<dyn Iterator<Item = TimeObject>>;
pub type SeriesFn = dyn Fn(TimeObject, &TimeContext) -> (BoxIter, BoxIter);

#[derive(Clone, Copy)]
pub struct TimeContext { pub ref_time: TimeObject, pub min_time: TimeObject, pub max_time: TimeObject }

#[derive(Clone)]
pub enum Predicate { Empty, Series(Rc<SeriesFn>) }

impl Predicate {
    pub fn run(&self, t: TimeObject, ctx: &TimeContext) -> (BoxIter, BoxIter) {
        match self {
            Predicate::Empty => (Box::new(std::iter::empty()), Box::new(std::iter::empty())),
            Predicate::Series(f) => f(t, ctx),
        }
    }
}

pub fn cycle_nth(g: Grain, n: i64) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, _ctx| {
        let anchor = { let r = time_round(t, g); TimeObject { start: grain_add(r.start, g, n), grain: g, end: None } };
        let fut = successors(Some(anchor), move |p| Some(TimeObject{ start: grain_add(p.start, g, 1), grain: g, end: None }));
        let prev = TimeObject{ start: grain_add(anchor.start, g, -1), grain: g, end: None };
        let past = successors(Some(prev), move |p| Some(TimeObject{ start: grain_add(p.start, g, -1), grain: g, end: None }));
        (Box::new(past) as BoxIter, Box::new(fut) as BoxIter)
    }))
}
```
(`time_cycle(g) = cycle_nth(g, 0)` for now; generalized in Phase 3.)
- [ ] **Step 4: Run — PASS.**
- [ ] **Step 5: Commit** — `feat: port Predicate series + cycle_nth (lazy iterators)`.

---

### Task 1.4: Token / Dimension / Node / Rule / Pattern types

**Files:** Modify `rust/src/types.rs`.

**Interfaces:**
- Produces: `enum Token { RegexMatch(Vec<String>), Time(TimeData), Numeral(i64 /* widened in Phase 2 */), /* more added per dep */ }`; `struct TimeData { pred: Predicate, grain: Grain, latent: bool, form: Option<Form>, direction: Option<IntervalDirection>, holiday: Option<String> }`; `struct Range(pub usize, pub usize)`; `struct Node { range: Range, token: Token, rule: Option<String> }`; `type Production = fn(&[Token]) -> Option<Token>`; `enum PatternItem { Regex(regex::Re), Predicate(fn(&Token)->bool) }`; `struct Rule { name: String, pattern: Vec<PatternItem>, prod: Production }`. The existential GADT `Token` collapses to this enum (Global Constraint: idiomatic Rust).

- [ ] **Step 1: Failing test** — a trivial constructor test:
```rust
#[test] fn token_time_roundtrips() {
    let td = TimeData { pred: Predicate::Empty, grain: Grain::Day, latent:false, form:None, direction:None, holiday:None };
    assert!(matches!(Token::Time(td), Token::Time(_)));
}
```
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement** the types above with `#[derive(Clone)]` where derivable (note `Production` is a fn pointer; `Predicate` is `Clone`).
- [ ] **Step 4: Run — PASS.**
- [ ] **Step 5: Commit** — `feat: core token/node/rule types`.

---

### Task 1.5: `Document` + adjacency

**Files:** Modify `rust/src/document.rs`.

**Interfaces:**
- Produces: `struct Document { text: String, chars: Vec<char> }`; `fn new(&str)`, `fn len(&self) -> usize` (char count); `fn is_adjacent(&self, prev_end: usize, next_start: usize) -> bool` — true if the gap between `prev_end` and `next_start` is only separator characters (whitespace and a small set Duckling treats as adjacent). Simplified port of `Duckling/Types/Document.hs` adjacency; we operate on `char` indices since `fancy-regex` runs on `&str` (byte offsets converted to char indices in `regex.rs`).

- [ ] **Step 1: Failing test**
```rust
#[test] fn adjacency_skips_spaces() {
    let d = Document::new("on  monday");
    assert!(d.is_adjacent(2, 4));   // "on" then two spaces then "monday"
    assert!(!d.is_adjacent(2, 4) == false);
}
#[test] fn adjacency_rejects_letters_between() {
    let d = Document::new("onXmonday");
    assert!(!d.is_adjacent(2, 3));  // 'X' is not a separator
}
```
- [ ] **Step 2–4:** implement (gap chars must all be in `[' ', '\t', '\n', '\r']` plus Duckling's separators; start with whitespace only and widen if fixtures demand), run FAIL→PASS.
- [ ] **Step 5: Commit** — `feat: document with char-index adjacency`.

---

### Task 1.6: Regex layer (`fancy-regex`, case-insensitive, position-aware)

**Files:** Modify `rust/src/regex.rs`.

**Interfaces:**
- Produces: `struct Re(fancy_regex::Regex)`; `fn compile(pattern: &str) -> Re` (prepends `(?i)` to match Haskell's `compCaseless`); `fn match_at(&self, doc: &Document, char_pos: usize) -> Vec<RegexHit>` where `struct RegexHit { start: usize, end: usize, groups: Vec<String> }` (char indices). Mirrors `lookupRegex` (anchored search from a position) by searching the substring from `char_pos` and keeping only hits whose start is adjacent to `char_pos`.

- [ ] **Step 1: Failing test**
```rust
#[test] fn caseless_group_capture() {
    let re = compile("(this|next)");
    let d = Document::new("Next monday");
    let hits = re.match_at(&d, 0);
    assert_eq!(hits[0].groups[0].to_lowercase(), "next");
    assert_eq!((hits[0].start, hits[0].end), (0, 4));
}
```
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement** — convert the doc to a `&str` slice from the char position, run `Regex::captures_from_pos`, translate byte offsets back to char indices, collect groups (`captures.get(i)`), filter by adjacency. Handle `fancy-regex`'s `Result` (lookaround can error) by treating errors as no-match.
- [ ] **Step 4: Run — PASS.**
- [ ] **Step 5: Commit** — `feat: fancy-regex layer (caseless, position-aware)`.

---

### Task 1.7: Engine — bottom-up saturating matcher

**Files:** Modify `rust/src/engine.rs`.

**Interfaces:**
- Consumes: `Document`, `Rule`, `PatternItem`, `Token`, `Node`, `Range`.
- Produces: `fn parse_string(rules: &[Rule], doc: &Document) -> Vec<Node>` — produces every node reachable by saturating the rule set (regex hits + predicate matches over produced nodes, repeated to a fixpoint). Clean reimplementation (Global Constraint), behavior-validated by fixtures.

- [ ] **Step 1: Failing test** — with one hand-built rule that matches the regex `today` and produces a `Token::Time`:
```rust
#[test] fn single_regex_rule_produces_node() {
    let rules = vec![ /* rule: pattern [Regex("today")], prod -> Token::Time(...) */ ];
    let doc = Document::new("today");
    let nodes = parse_string(&rules, &doc);
    assert!(nodes.iter().any(|n| matches!(n.token, Token::Time(_)) && n.range == Range(0,5)));
}
```
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement** the matcher:
  - Seed: for each rule, attempt to match its pattern starting at every char position.
  - `match_pattern(items, doc, pos, stash) -> Vec<Vec<Node>>`: recursively match each `PatternItem`. `Regex` → `re.match_at(doc, pos)`; `Predicate(f)` → every stash node `n` with `is_adjacent(pos, n.range.0) && f(&n.token)`. Each match advances `pos` to the item's end; collect full routes.
  - For each full route, call `rule.prod(&route_tokens)`; on `Some(token)`, build a `Node { range: Range(route_start, route_end), token, rule: Some(name) }`.
  - Maintain a `HashSet`-deduped stash; repeat the whole sweep until no new node is added (fixpoint).
- [ ] **Step 4: Run — PASS.**
- [ ] **Step 5: Commit** — `feat: bottom-up saturating engine`.

---

### Task 1.8: Resolution + Time-value JSON

**Files:** Modify `rust/src/resolve.rs`, `rust/src/json.rs`.

**Interfaces:**
- Produces: `fn resolve_time(td: &TimeData, ctx: &ResolveContext) -> Option<serde_json::Value>` (port of `Resolve TimeData`, `Duckling/Time/Types.hs:105-133`, MVP: ignore `notImmediate`/latent-gating beyond `with_latent`); `json::rfc3339(dt: civil::DateTime, off: jiff::tz::Offset) -> String` (port of `toRFC3339`/`timezoneOffset`, `Types.hs:730-752` — millis padded to 3, offset `±HH:MM`), where **`off` is the offset `ctx.zone` reports for that resolved instant, computed per-instant — this is the DST-correct path, never a constant**; `json::simple_value(t, off)`, `json::interval_value`, `json::open_interval` → the `{type,value,grain}` / `{type,from,to}` shapes (`Types.hs:181-197`).
- Wires `lib::parse` to: build `Document`, run `parse_string(en_rules(), &doc)`, resolve each `Token::Time` node, map to `Entity { dim:"time", body, start, end, value, latent }`.

- [ ] **Step 1: Failing test** (`json::rfc3339`):
```rust
#[test] fn rfc3339_matches_haskell() {
    let dt = jiff::civil::date(2013, 2, 12).at(0, 0, 0, 0);
    let off = jiff::tz::Offset::constant(-2);
    assert_eq!(rfc3339(dt, off), "2013-02-12T00:00:00.000-02:00");
}
```
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement** `rfc3339` (hand-format to guarantee the `.000` + `-02:00` shape), the value shapers, and `resolve_time` (run predicate against `ctx` → take future head else past head → `simple_value`/`open_interval`). Wire `lib::parse`.
- [ ] **Step 4: Run — PASS.**
- [ ] **Step 5: Commit** — `feat: resolution + RFC3339 time-value JSON`.

---

### Task 1.9: The first four rules → corpus slice green

**Files:** Modify `rust/src/time/en_rules.rs` (create), `rust/src/time/helpers.rs` (create, just `now`/`today` builders for now), declare in `mod.rs`; wire `en_rules()` into `lib::parse`.

**Interfaces:**
- Produces: `fn en_rules() -> Vec<Rule>` returning the four instant rules. `now = cycle_nth(Second, 0)`, `today = cycle_nth(Day,0)`, `tomorrow = cycle_nth(Day,1)`, `yesterday = cycle_nth(Day,-1)`, each wrapped in `TimeData` with the right grain. Regexes ported verbatim from `Duckling/Time/EN/Rules.hs:130-145`.

- [ ] **Step 1: Add a focused test** (subset of the corpus, fast feedback):
```rust
#[test] fn instants_resolve() {
    let ctx = ctx(); // shared helper
    for (input, val, grain) in [
        ("now","2013-02-12T04:30:00.000-02:00","second"),
        ("today","2013-02-12T00:00:00.000-02:00","day"),
        ("tomorrow","2013-02-13T00:00:00.000-02:00","day"),
        ("yesterday","2013-02-11T00:00:00.000-02:00","day"),
    ] {
        let got = duckling::parse(input,&ctx);
        assert!(got.iter().any(|e| e.value["value"]==val && e.value["grain"]==grain),
                "{input}: {:?}", got);
    }
}
```
- [ ] **Step 2: Run — FAIL.**
- [ ] **Step 3: Implement** the four rules (regex pattern + production building the `TimeData`) and `en_rules()`.
- [ ] **Step 4: Run focused test — PASS**, then run the full harness to see the dent:
```bash
cd rust && cargo test --test corpus 2>&1 | grep checked
```
Expected: `positive_corpus` still fails overall, but the 4 instant inputs now pass (verify by temporarily grepping failures for "today" — should be absent).
- [ ] **Step 5: Commit** — `feat: EN instant rules (now/today/tomorrow/yesterday) green`.

**Phase 1 exit:** instant fixtures green in `Contains` mode; the architecture (regex → engine → predicate → resolve → JSON → harness) is proven end-to-end. **Capture the velocity here** — rules-per-hour from this slice is the basis for estimating Phases 3–4.

---

# Phase 2 — Dependency dimensions (Numeral, Ordinal, Duration, TimeGrain)

**Goal:** port only the dependency-dimension rules the EN Time corpus exercises, each driven by Time fixtures that currently fail for lack of a number/ordinal/duration/grain token.

**Repeatable task template** (apply per dependency):
1. Pick a failing Time fixture that needs the dependency (e.g. `"in 3 weeks"` needs Numeral + TimeGrain + Duration).
2. Port the dependency's `types.rs` (the data struct + its `Token` enum arm) — TDD with one unit test.
3. Port the EN rule(s) from the Haskell source, regexes verbatim, TDD against the dependency's own corpus values where useful.
4. Run the Time harness; confirm the targeted fixtures advance.
5. Commit per rule.

**Backlog (port in this order — each unlocks Time fixtures):**
- [ ] **TimeGrain** EN rules (`Duckling/TimeGrain/EN/Rules.hs`): grain words → `Token::Grain`. Smallest; do first.
- [ ] **Numeral** EN integers 0–99, "a/an", "couple", "dozen", teen/tens composition (`Duckling/Numeral/EN/Rules.hs` — port only the integer-producing rules the Time corpus hits; skip decimals/large scales unless a fixture needs them). Widen `Token::Numeral` to carry `NumeralData { value: f64, grain: Option<i32> }`.
- [ ] **Ordinal** EN 1st–31st (`Duckling/Ordinal/EN/Rules.hs`).
- [ ] **Duration** `Numeral × Grain → Duration` and "a <grain>" (`Duckling/Duration/EN/Rules.hs` + `Duckling/Duration/Helpers.hs`). Add `Token::Duration(DurationData{ value, grain })`.

**Phase 2 exit:** every Time fixture whose failure was "missing dependency token" now fails only for a missing *Time* rule (verify by spot-checking the harness failure list — no failure should be a bare number/duration input).

> **Scope note (per writing-plans Scope Check):** Phase 2 is itself a multi-subsystem effort. Once Phase 1 proves the patterns, spin Phase 2 out into its own detailed plan (`2026-..-duckling-rust-deps.md`) with one fully-specified task per dependency rule, generated the same way Phase 1 was.

---

# Phase 3 — Full EN Time rule set

**Goal:** drive the bulk of `positive_corpus` green in `Contains` mode by porting the ~142 EN Time rules, in clusters, each cluster turning a recognizable group of fixtures green.

**Repeatable cluster task** (one commit per rule or tight rule-group):
1. Filter the harness failures to the cluster's inputs.
2. Port the rule(s) from `Duckling/Time/EN/Rules.hs` and any new combinator from `Duckling/Time/Helpers.hs` (TDD the combinator separately when non-trivial — `intersect`, `predNth`, `nthDOWOfMonth`, `interval`, `inDuration`, `durationAgo`).
3. Run the harness; confirm the cluster's inputs pass and nothing regressed.
4. Commit.

**Cluster backlog (real groupings from the source, roughly increasing difficulty):**
- [ ] Intersect & absorb (`intersect`, `on <day>`, `in|during <month>`, comma absorption) — `Rules.hs:40-128`.
- [ ] Days of week & months (`mkRuleDaysOfWeek`, `mkRuleMonths`) — table-driven.
- [ ] This / next / last `<day-of-week>` and `<cycle>` (`predNth`, `cycleNth`) — generalize `cycle_nth`/`time_cycle` from Phase 1.
- [ ] Time-of-day: `HH:MM(:SS)`, `h<MM>`, AM/PM, "noon/midnight", "quarter past", "half past" — needs `TimeDatePredicate` + `runHourPredicate`/`runMinutePredicate` (`Time/Types.hs:247-490`).
- [ ] Parts of day: morning/afternoon/evening/night/lunch + "this/in the `<part>`" (`partOfDay`, `runAMPMPredicate`).
- [ ] Day-of-month: ordinal & numeric, "the 3rd", "March 3rd", "3rd of March" (`runDayOfTheMonthPredicate`, `intersectDOM`).
- [ ] Relative by duration: "in 3 weeks", "3 weeks ago", "next/last `<n>` `<grain>`" (`inDuration`, `durationAgo`, `durationAfter/Before`).
- [ ] Intervals: "from X to Y", "between X and Y", "X - Y", "by `<time>`", "until `<time>`" (`interval`, `mkTimeIntervalsPredicate`, open intervals + `IntervalDirection`).
- [ ] Absolute dates: `M/D`, `M/D/Y`, `D Mon Y`, `YYYY-MM-DD` (`yearMonthDay`, `monthDay`) — verbatim regexes incl. the lookbehind/lookahead ones (`fancy-regex` already supports them).
- [ ] Year AD/BC, year-month, quarters, seasons (`yearADBC`, `season`, `seasonPredicate`).
- [ ] **In-text timezones** (`ruleTimezone`, `parseTimezone`, `inTimezone`/`shiftTimezone`, `Time/Types.hs` hasTimezone flag). Port the ~150-entry fixed-offset table and the predicate shift. Must turn the corpus's GMT/PST/CET examples green **and** the tz-stress fixtures (Task 0.6) green — the latter is what proves DST handling, not the former.
- [ ] Holidays (`mkRuleHolidays`, `HolidayHelpers.hs`, `Computed.hs`) — large table; the `holidayBeta` JSON key (`Types.hs:207`).

**Phase 3 exit:** the large majority of non-`null` positive fixtures pass in `Contains` mode. Track the count each session via `cargo test --test corpus 2>&1 | grep checked`.

> Spin Phase 3 into its own plan once Phase 2 lands; the cluster backlog above becomes that plan's task list.

---

# Phase 4 — Ranking (tighten to `Unique`)

**Goal:** flip `DUCKLING_MATCH=unique` and make it pass — i.e. collapse competing full-range parses to the single correct winner, matching Duckling's real corpus semantics.

- [ ] Dump the EN classifier model to JSON: add a Haskell exe (sibling of the corpus dumper) that serializes `Duckling.Ranking.Classifiers.classifiers (makeLocale EN Nothing)` (`ClassData` = prior/unseen/likelihoods/n) to `rust/fixtures/en_classifiers.json`.
- [ ] Port the naive-Bayes scorer (`Duckling/Ranking/Rank.hs:28-47`): `ll`, `posLL`, `score` (recursive over node children), and the `Candidate` partial order (`Ranking/Types.hs:77-105`).
- [ ] Port feature extraction (`Duckling/Ranking/Extraction.hs`) so feature keys match the model's keys exactly (the rule name + child rule-name concatenation scheme). **This is the correctness-critical part** — a feature-key mismatch silently zeroes scores.
- [ ] Apply `rank` in `lib::parse` (filter to winners, dedupe) and run `DUCKLING_MATCH=unique cargo test --test corpus`.

**Phase 4 exit:** `positive_corpus` passes in `Unique` mode for all non-`null` fixtures.

---

# Phase 5 — Hardening

- [ ] Resolve the `null`-expected fixtures from Phase 0 (the originally-ambiguous inputs) now that ranking exists; remove the skip.
- [ ] Add the `latentCorpus` fixtures (`withLatent = true`) — extend the dumper to emit a third set, add a harness test with a latent context.
- [ ] Confirm interval / open-interval / holiday JSON shapes byte-match the oracle (extend behavior-compat to include `from`/`to`/`holidayBeta`).
- [ ] Wire `lib::parse` public API + a small `README` documenting the cleaner Rust API surface.
- [ ] `superpowers:requesting-code-review` before merge.

---

## Self-Review

- **Spec coverage:** Time ✔ (Phases 1,3); dependencies ✔ (Phase 2 — TimeGrain/Numeral/Ordinal/Duration enumerated); test data ✔ (Phase 0 dumps real corpus + negatives); English-first ✔ (Global Constraints pin EN/no-region); tests-before-logic ✔ (Phase 0 produces a red harness before any rule). Ranking ✔ (Phase 4). Latent ✔ (Phase 5). **Timezone ✔ — in-text named zones (Phase 3 cluster, corpus-tested) + DST correctness via real-IANA-zone stress fixtures (Task 0.6), since the corpus is DST-free and cannot validate this.**
- **Placeholder scan:** Phases 0–1 contain complete code and exact commands. Phases 2–5 are deliberately structured as iterative task templates + real enumerated backlogs (not vague steps) because pre-writing exact Rust for ~142 rules + numeral grammar before the foundation exists would be fabrication; the Scope notes direct each into its own fully-specified plan once its predecessor proves the pattern.
- **Type consistency:** `TimeObject{start,grain,end}`, `Predicate{Empty,Series}`, `Token::Time(TimeData)`, `ResolveContext{reference,zone,with_latent}`, and `rfc3339(dt, off)` (offset read per-instant from `ctx.zone`) are used identically across tasks. `Token::Numeral` is explicitly widened from the Phase-1 placeholder to `NumeralData` in Phase 2 (flagged in 1.4 and 2).
- **Behavior-compat key:** every comparison strips `"values"`, matching `Duckling/Time/Corpus.hs:68-75` and the harness `strip_values` — consistent end to end.

---

## Execution Handoff

Plan complete and saved to `docs/superpowers/plans/2026-06-30-duckling-rust-en-time.md`. Two execution options:

1. **Subagent-Driven (recommended)** — I dispatch a fresh subagent per task, review between tasks, fast iteration. Best here because Phase 0/1 tasks are independent and verifiable.
2. **Inline Execution** — execute tasks in this session with checkpoints for review.

Which approach?

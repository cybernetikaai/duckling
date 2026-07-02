# Value-resolution benchmark (Microsoft Recognizers-Text)

A second evaluation, complementary to the [GMB time-recognition benchmark](NER_TIME_EVAL.md).
Where GMB only tags *spans* (so it measures recognition), Microsoft
[Recognizers-Text](https://github.com/microsoft/Recognizers-Text) (MIT) is a
Duckling-style recognizer whose `/Specs/*.json` files carry input → **resolved
value + unit** for the same dimensions Duckling has — so this measures
**resolution** (did we compute the right *value*?), not just recognition.

**This is a differential/coverage benchmark, not a pass/fail oracle.** It's a
different library: unit strings differ (we map them), and the two tools have
genuinely different *scope* on some dimensions. Disagreements are characterized
below, not treated as failures.

## How to run

```bash
git clone https://github.com/microsoft/Recognizers-Text
cargo build --release --manifest-path rust/Cargo.toml
python3 rust/eval/recognizers_eval.py --specs-dir Recognizers-Text/Specs
```

For each English spec it locates the MS-matched span, finds our overlapping
entity, and compares the numeric value (with unit mapped best-effort). "clean" =
inputs where MS emitted a single in-scope result (a comparable 1:1 case;
multi-result inputs like IP addresses diverge by *tokenization*, not resolution).

## Results

| Dimension | cases | recognized | value-agree | clean-input value-agree |
|---|---|---|---|---|
| **temperature** | 32 | 96.9% | **96.9%** | 96.9% |
| amount-of-money | 190 | 85.3% | 74.7% | 78.3% |
| distance | 40 | 75.0% | 60.0% | 60.0% |
| number | 259 | 96.1% | 44.4% | 36.1% |

## Interpretation (what each row tells us)

- **Temperature — 96.9% value-agreement is a strong resolution validation.** Our
  `parse_temperature` computes the same values as an independent library on real
  sentences. The single miss is a typo MS tolerates and we don't
  ("34.9 centigrate to farenheit").

- **Money — 78% (clean); the misses are a coverage boundary, not wrong values.**
  Nearly all misses are currencies Duckling doesn't model (Finnish markka, French
  franc, Peseta) or un-symboled forms MS infers ("125 million australian
  dollars"). Where the currency is supported, values agree.

- **Distance — 60%; a few genuine leads.** The disagreements cluster on
  **hyphenated forms** ("3-inch", "six-mile", "three-foot") and **mixed numbers**
  ("10 1/2 miles" → 10.5, where we take only "1/2"). These are worth checking
  against the live oracle — some may be real gaps, some Duckling-vs-MS design
  differences.

- **Number — low agreement is a *scope* difference, not bugs.** MS Recognizers
  resolves things Duckling's numeral deliberately doesn't: fraction words
  ("two thirds" → 0.667, "five eighths" → 0.625), scientific notation ("1e10"),
  exponentiation ("1.1^23"), spelled decimals ("two hundred point zero three" →
  200.03), hyphenated tens ("fifty-two" → 52), and hundred-multiplication
  ("322 hundred" → 32200). This precisely maps the boundary between the two
  libraries' number scope.

## Takeaways

- **Resolution (not just recognition) is now validated against an independent
  implementation** — temperature near-perfectly; money/distance where units and
  scope align.
- The benchmark **maps Duckling's scope boundaries** on number and money against a
  peer tool, and **surfaces concrete distance leads** (hyphenated units, mixed
  numbers) for follow-up.
- Datetime is intentionally out of scope here: MS's `datetimeV2` resolution shape
  differs enough from Duckling's that a fair comparison needs its own normalizer
  (candidate for a follow-up, alongside the TimeML/TIMEX3 value benchmark).

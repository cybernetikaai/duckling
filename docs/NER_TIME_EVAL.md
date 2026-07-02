# Time-recognition benchmark (GMB NER corpus)

A recognition task built around an external labeled dataset — the Kaggle
["Annotated Corpus for Named Entity Recognition"](https://www.kaggle.com/datasets/abhinavwalia95/entity-annotated-corpus)
(Groningen Meaning Bank, ~48k news sentences with BIO tags: `geo gpe per org tim
art eve nat`). It measures how the port recognizes time expressions in **real
news text**, complementing Duckling's own curated corpus.

**Scope.** Duckling is a structured-value extractor, not a person/place/org
recognizer, so only the **`tim`** tag is in scope. The other tags
(geo/gpe/per/org/…) are classic NER and out of scope by design.

## How to run

The dataset is external (not committed — it's ~19 MB and separately licensed).
Point the harness at your local copy:

```bash
cargo build --release --manifest-path rust/Cargo.toml
python3 rust/eval/ner_time_eval.py /path/to/ner.csv                 # --dims time
python3 rust/eval/ner_time_eval.py /path/to/ner.csv --latent        # + latent (years)
python3 rust/eval/ner_time_eval.py /path/to/ner.csv --dims all --latent
```

It extracts gold `tim` spans (contiguous `B-tim`/`I-tim` runs), batch-parses every
sentence via `duckling --batch`, and scores recognition by **character-span
overlap**. Runs over all 48k sentences in ~16 s (thanks to `--batch`).

## Results

47,959 sentences; 17,258 (36 %) contain a time expression; **20,333 gold `tim`
spans**.

| Configuration | Overlap recall | Exact-span | Sentences fully covered | Port spans / % overlapping gold |
|---|---|---|---|---|
| `--dims time` (default) | **81.5 %** | 36.6 % | 80.4 % | 23,597 / 70.7 % |
| `--dims time --latent` | **92.6 %** | 42.5 % | 91.8 % | 47,194 / 40.7 % |
| `--dims all --latent` | **92.9 %** | 40.3 % | 92.1 % | 61,962 / 34.6 % |

## What the numbers mean

**The latent knob is the headline.** Recall jumps 81.5 % → 92.6 % when latent
parses are enabled, because a bare four-digit **year** ("… in 2001 …") is *latent*
in Duckling and dropped by default. Real prose is full of bare years, so a
speech/NLU pipeline over news-like text should run with latent on — but it pays
in precision: "extra" spans (port time spans not overlapping any gold `tim`) rise
from 29 % to 59 %, because every bare number also becomes a latent time. This is a
genuine product tradeoff, now quantified. (`--dims all` adds only ~0.3 % recall
over time+latent — durations like "two-day" — while flooding extras with numerals,
so `time --latent` is the sweet spot for time recognition.)

**Residual misses are not port bugs.** With max recall, the still-missed gold
spans fall into three buckets, none of which is a port defect:

- **Duckling scope limits, faithfully matched.** Vague/relative words the parser
  correctly does *not* resolve to a concrete instant — "later", "recent",
  "several", "each", "since", "before". And **decades** ("the 1990s", "past
  decade") — the live oracle returns nothing for these too, so the port is
  faithful, not deficient.
- **Gold-label noise.** Places mistagged `tim` in the corpus — "darfur",
  "moscow", "tehran" — which the port (correctly) does not read as times.
- **Partial-span differences.** Exact-span is only ~40 % because overlap-recall
  counts a hit even when boundaries differ (gold "September 8" vs. a different
  port boundary); the resolved value is typically still correct.

**Many "extras" are the port being *more* complete than the gold.** The top
non-overlapping port spans are real time expressions GMB simply didn't tag:
"last week", "this month", "now", "next year", "next week", seasons ("fall",
"winter"), month abbreviations ("jan", "feb"), and ordinals-as-dates ("the first"
→ the 1st, Duckling's documented behavior).

**Zero port-vs-Duckling divergences.** Every surprising case was cross-checked
against the live oracle and matched. Even a spurious-looking hit — "the H" inside
"the **H5N1**" avian-flu strain (the `h`/`H` hour notation over-matching) — is
produced *identically* by Duckling (only the resolved instant differs, because of
different reference dates). The port reproduces Duckling's behavior on real-world
text, quirks included.

## Takeaways

- On real news text the port recognizes **~93 %** of human-labeled time
  expressions (overlap), with residual misses attributable to Duckling's own scope
  or gold-label noise — not port bugs.
- **Run with `--latent`** for prose-heavy input (recovers bare years); accept the
  precision cost or post-filter numerals downstream.
- The differential audit against the oracle found **no behavioral divergences**,
  extending the fidelity evidence from Duckling's curated corpus to messy,
  in-the-wild text.

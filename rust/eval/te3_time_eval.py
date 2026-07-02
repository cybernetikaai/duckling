#!/usr/bin/env python3
"""TempEval-3 (TimeML/TIMEX3) time value-resolution benchmark.

The GMB benchmark measures *recognition* (spans); Recognizers-Text measures
resolution vs a peer library. This measures resolution against **human-annotated
gold**: TimeML `.tml` files tag temporal expressions with an ISO `value` (the
TIMEX3 standard) and each document carries a creation time (DCT) that relative
expressions resolve against. We parse each gold expression with its document's
DCT as `--ref` and check our resolved date against the TIMEX3 value.

Free mirror: `git clone https://github.com/jspotter/TempEval-3`
Gold subset used: `data/TBAQ-cleaned` (TimeBank + AQUAINT).

v1 scope: `type="DATE"` with a fully-specified value (YYYY, YYYY-MM, YYYY-MM-DD),
compared at the gold's granularity. TIME / DURATION / SET and underspecified
values (PRESENT_REF, TXX, P1W, …) are tallied as out-of-scope, not scored.

Usage:
    python3 rust/eval/te3_time_eval.py --data <TempEval-3>/data/TBAQ-cleaned \
            [--bin rust/target/release/duckling]
"""
import argparse
import glob
import os
import re
import json
import subprocess
import collections

ap = argparse.ArgumentParser()
ap.add_argument("--data", required=True, help="path to TBAQ-cleaned (TimeBank+AQUAINT)")
ap.add_argument("--bin", default="rust/target/release/duckling")
args = ap.parse_args()

ATTR = lambda tag, name: (re.search(rf'{name}="([^"]*)"', tag) or [None, None])[1]
TIMEX_TAG = re.compile(r"<TIMEX3\b([^>]*)>(.*?)</TIMEX3>", re.S)
STRIP = re.compile(r"<[^>]+>")
FULLDATE = re.compile(r"^\d{4}(-\d{2}(-\d{2})?)?$")  # YYYY | YYYY-MM | YYYY-MM-DD

# group in-scope DATE expressions by document DCT so each batch shares one --ref
by_dct = collections.defaultdict(list)   # dct -> [(text, gold_value)]
counts = collections.Counter()
for path in glob.glob(os.path.join(args.data, "**", "*.tml"), recursive=True):
    if os.path.basename(path).startswith("._"):
        continue
    doc = open(path, encoding="latin-1").read()
    dct = None
    for attrs, _inner in TIMEX_TAG.findall(doc):
        if ATTR(attrs, "functionInDocument") in ("CREATION_TIME", "PUBLICATION_TIME"):
            dct = ATTR(attrs, "value")
            break
    if not dct or not re.match(r"^\d{4}-\d{2}-\d{2}$", dct):
        counts["doc-no-usable-DCT"] += 1
        continue
    for attrs, inner in TIMEX_TAG.findall(doc):
        typ, val = ATTR(attrs, "type"), ATTR(attrs, "value")
        if ATTR(attrs, "functionInDocument") == "CREATION_TIME":
            continue
        counts[f"type:{typ}"] += 1
        text = STRIP.sub("", inner).strip()
        if typ == "DATE" and val and FULLDATE.match(val) and text:
            by_dct[dct].append((text, val))
        else:
            counts["out-of-scope"] += 1

# An expression is "absolute" if its text pins the YEAR (an explicit 4-digit year
# or a numeric date with a year), so it resolves the same regardless of anchor.
# Everything else ("January", "July 1", "Tuesday", "the following month") is
# "relative": its year/occurrence depends on the anchor, and TimeML resolves it to
# the document's in-context (often *past*) time while Duckling defaults to the next
# *future* one — a resolution-convention difference, not a bug. Scored separately.
# `--latent` surfaces bare years.
ABSOLUTE = re.compile(r"\b\d{4}\b|\d{1,2}/\d{1,2}/\d{2,4}")
# kind -> [recognized, agree, total]
tally = {"absolute": [0, 0, 0], "relative": [0, 0, 0]}
misses = {"absolute": [], "relative": []}
ref_time = lambda d: d + "T12:00:00Z"
for dct, items in by_dct.items():
    out = subprocess.run(
        [args.bin, "--dims", "time", "--ref", ref_time(dct), "--latent", "--batch"],
        input="\n".join(t for t, _ in items), capture_output=True, text=True,
    ).stdout.splitlines()
    for (text, gold), line in zip(items, out):
        kind = "absolute" if ABSOLUTE.search(text) else "relative"
        tally[kind][2] += 1
        try:
            ents = json.loads(line) if line.strip() else []
        except json.JSONDecodeError:
            ents = []
        if not ents:
            if len(misses[kind]) < 6:
                misses[kind].append(f"    no-parse  ref={dct}  {text[:30]!r}  gold={gold}")
            continue
        tally[kind][0] += 1
        val = str(ents[0].get("value", {}).get("value", ""))[:10]  # our YYYY-MM-DD
        if val[: len(gold)] == gold:                               # truncate to gold precision
            tally[kind][1] += 1
        elif len(misses[kind]) < 6:
            misses[kind].append(f"    val-diff  ref={dct}  {text[:26]!r}  gold={gold}  ours={val}")

print(f"documents with usable DCT: {len(by_dct)}")
print(f"\n{'subset':<10} {'exprs':>6} {'recognized':>13} {'value-agree':>13}")
print("-" * 46)
for kind in ("absolute", "relative"):
    r, a, n = tally[kind]
    pct = lambda x: f"{100*x/n:.1f}%" if n else "n/a"
    print(f"{kind:<10} {n:>6} {r:>7} {pct(r):>5} {a:>7} {pct(a):>5}")
print(f"\nout-of-scope (TIME/DURATION/SET/underspecified, not scored): {counts['out-of-scope']}")
print("TIMEX3 type distribution:", {k.split(':')[1]: v for k, v in counts.items() if k.startswith('type:')})
for kind in ("absolute", "relative"):
    print(f"\nsample {kind} misses:")
    for m in misses[kind]:
        print(m)

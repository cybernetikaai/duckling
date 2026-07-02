#!/usr/bin/env python3
"""Differential value-resolution benchmark against Microsoft Recognizers-Text.

Recognizers-Text (github.com/microsoft/Recognizers-Text, MIT) is a Duckling-style
recognizer whose `/Specs/*.json` test files carry input -> **resolved value+unit**
for the same dimensions Duckling has. Unlike the span-only GMB NER corpus, this
lets us check *resolution* (did we compute the right value?), not just recognition.

It is a *different* library, so this is a differential/coverage benchmark, not a
pass/fail oracle: unit strings differ (we map them), and some behaviours diverge
by design. We report, per dimension: recognition (did we emit an entity at that
span) and value-agreement (recognized AND numeric value within tolerance).

Usage:
    git clone https://github.com/microsoft/Recognizers-Text
    python3 rust/eval/recognizers_eval.py --specs-dir Recognizers-Text/Specs \
            [--bin rust/target/release/duckling]
"""
import argparse
import json
import os
import subprocess

ap = argparse.ArgumentParser()
ap.add_argument("--specs-dir", required=True, help="path to Recognizers-Text/Specs")
ap.add_argument("--bin", default="rust/target/release/duckling")
ap.add_argument("--tol", type=float, default=1e-6)
args = ap.parse_args()

# MS unit spelling -> our unit string (best-effort; value is the primary signal).
TEMP_UNITS = {"C": "celsius", "F": "fahrenheit", "Degree": "degree"}
CURR_UNITS = {
    "Dollar": "$", "United States dollar": "USD", "Cent": "cent", "Penny": "cent",
    "Pence": "cent", "British pound": "£", "Pound": "£", "Euro": "EUR",
    "Japanese yen": "JPY", "Chinese yuan": "CNY", "Rupee": "INR",
    "Canadian dollar": "CAD", "Australian dollar": "AUD", "New Zealand dollar": "NZD",
}
LEN_UNITS = {
    "Millimeter": "millimetre", "Centimeter": "centimetre", "Meter": "metre",
    "Kilometer": "kilometre", "Inch": "inch", "Foot": "foot", "Yard": "yard",
    "Mile": "mile",
}

# Each dimension: which spec, our --dims, how to filter/keep MS results, unit map.
DIMS = [
    dict(name="temperature", specs="NumberWithUnit/English/TemperatureModel.json",
         dims="temperature", type_prefix="temperature", subtype=None, umap=TEMP_UNITS),
    dict(name="number", specs="Number/English/NumberModel.json",
         dims="number", type_prefix="number", subtype=None, umap=None),
    dict(name="amount-of-money", specs="NumberWithUnit/English/CurrencyModel.json",
         dims="amount-of-money", type_prefix="currency", subtype=None, umap=CURR_UNITS),
    dict(name="distance", specs="NumberWithUnit/English/DimensionModel.json",
         dims="distance", type_prefix="dimension", subtype="Length", umap=LEN_UNITS),
]


def expected(spec_path, type_prefix, subtype):
    """[(input, [(matched_text, value_float, ms_unit), ...]), ...], results in order."""
    data = json.load(open(spec_path, encoding="utf-8"))
    out = []
    for c in data:
        rs = []
        for r in c.get("Results", []):
            if not (r.get("TypeName") or "").startswith(type_prefix):
                continue
            res = r.get("Resolution") or {}
            if subtype and res.get("subtype") != subtype:
                continue
            val = res.get("value")
            if val is None:
                continue
            try:
                v = float(val)
            except ValueError:
                continue
            rs.append((r.get("Text", ""), v, res.get("unit")))
        if rs:
            out.append((c["Input"], rs))
    return out


def our_value(entity):
    """Numeric value + unit from one of our entities (simple values only)."""
    v = entity.get("value") or {}
    if "value" in v and not isinstance(v["value"], dict):
        try:
            return float(v["value"]), v.get("unit")
        except (TypeError, ValueError):
            return None, None
    return None, None


def batch(bin_path, dims, inputs):
    out = subprocess.run([bin_path, "--dims", dims, "--batch"],
                         input="\n".join(inputs), capture_output=True, text=True).stdout
    res = []
    for line in out.splitlines():
        try:
            res.append(json.loads(line) if line.strip() else [])
        except json.JSONDecodeError:
            res.append([])
    return res


# "clean" = inputs where MS emits a single in-scope result (comparable 1:1);
# multi-result inputs (IPs, version strings) diverge by tokenization, not resolution.
print(f"{'dimension':<16} {'cases':>6} {'recognized':>13} {'value-agree':>13}   clean-input value-agree")
print("-" * 78)
for d in DIMS:
    path = os.path.join(args.specs_dir, d["specs"])
    grouped = expected(path, d["type_prefix"], d["subtype"])
    parsed = batch(args.bin, d["dims"], [inp for inp, _ in grouped])

    n = recog = agree = 0
    cn = cagree = 0
    misses = []
    for (inp, results), ents in zip(grouped, parsed):
        clean = len(results) == 1
        cur = 0  # advance a per-input cursor so repeated texts locate in order
        for text, ms_val, ms_unit in results:
            n += 1
            cn += clean
            pos = inp.find(text, cur)
            if pos < 0:
                pos = inp.find(text)
            span = (pos, pos + len(text)) if pos >= 0 else (0, len(inp))
            cur = span[1]
            hit = next((e for e in ents if e["start"] < span[1] and span[0] < e["end"]), None)
            if hit is None:
                if len(misses) < 5:
                    misses.append(f"      no-entity  {inp[:48]!r}  (ms={ms_val} {ms_unit})")
                continue
            recog += 1
            ov, ou = our_value(hit)
            if ov is not None and abs(ov - ms_val) <= args.tol * max(1.0, abs(ms_val), abs(ov)):
                agree += 1
                cagree += clean
            elif len(misses) < 5:
                misses.append(f"      val-diff   {inp[:42]!r}  ms={ms_val}{ms_unit or ''}  ours={ov}{ou or ''}")
    pct = lambda x, tot: f"{100*x/tot:.1f}%" if tot else "n/a"
    print(f"{d['name']:<16} {n:>6} {recog:>7} {pct(recog,n):>6} {agree:>7} {pct(agree,n):>6}"
          f"      {cagree:>4}/{cn:<4} {pct(cagree,cn):>6}")
    for m in misses:
        print(m)

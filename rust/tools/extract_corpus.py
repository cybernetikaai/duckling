#!/usr/bin/env python3
"""Transcribe Duckling's EN Time corpus (Haskell source) into golden JSON fixtures.

The corpus encodes expected resolved values as deterministic data, e.g.
    examples (datetime (2013, 2, 15, 0, 0, 0) Day) [ "2/15", ... ]
Because the released Haskell test suite is green, these declared values are
exactly what a correct parser emits. The expected JSON value is a pure function
of the datetime tuple(s) + grain + the fixed test context (-02:00, no tz
transitions), so we can compute it here without running Haskell.

Scope: positive `examples (...)` blocks that belong to `defaultCorpus`
(= allExamples ++ custom), excluding the latentCorpus / diffCorpus regions
which use a different context/options. Plus the negativeCorpus string list.
"""
import json
import re
import sys

SRC = "/Users/13protons/github/duckling/Duckling/Time/EN/Corpus.hs"
OUT = "/Users/13protons/github/duckling/rust/fixtures/en_time_corpus.json"
OFFSET = "-02:00"  # constant test-context offset (TimeZoneSeries with no transitions)

GRAINS = {"NoGrain", "Second", "Minute", "Hour", "Day", "Week", "Month",
          "Quarter", "Year"}

text = open(SRC, encoding="utf-8").read()
lines = text.splitlines()

def line_of(idx):
    return text.count("\n", 0, idx) + 1

def find_binding(name):
    m = re.search(rf"^{re.escape(name)} ::", text, re.M)
    return line_of(m.start()) if m else None

neg_line = find_binding("negativeCorpus")
allex_line = find_binding("allExamples")
assert neg_line and allex_line, "could not locate corpus bindings"

TUPLE = re.compile(
    r"\(\s*(-?\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*(\d+)\s*,\s*([\d.]+)\s*\)")
STR = re.compile(r'"((?:[^"\\]|\\.)*)"')

def rfc3339(t):
    y, mo, d, h, mi, s = t
    sec = int(float(s))
    ms = round((float(s) - sec) * 1000)
    return f"{y:04d}-{mo:02d}-{d:02d}T{h:02d}:{mi:02d}:{sec:02d}.{ms:03d}{OFFSET}"

def instant(t, grain):
    return {"value": rfc3339(t), "grain": grain}

def match_balanced(s, open_idx, opener="(", closer=")"):
    """Return index just past the matching closer for the opener at open_idx."""
    depth = 0
    i = open_idx
    while i < len(s):
        c = s[i]
        if c == opener:
            depth += 1
        elif c == closer:
            depth -= 1
            if depth == 0:
                return i
        i += 1
    raise ValueError("unbalanced")

positives = []
negatives = []
skipped = []

for m in re.finditer(r"examples\s*\(", text):
    ln = line_of(m.start())
    # keep defaultCorpus blocks only: custom (before negativeCorpus) or allExamples
    if not (ln < neg_line or ln >= allex_line):
        continue
    paren_open = m.end() - 1  # index of '('
    paren_close = match_balanced(text, paren_open)
    expr = text[paren_open + 1:paren_close]
    # the string list follows: skip ws and `--` line comments, expect '['
    j = paren_close + 1
    while j < len(text):
        if text[j] in " \t\r\n":
            j += 1
        elif text[j:j + 2] == "--":
            nl = text.find("\n", j)
            j = len(text) if nl == -1 else nl + 1
        else:
            break
    if text[j] != "[":
        skipped.append((ln, "no string list", expr[:60]))
        continue
    list_close = match_balanced(text, j, "[", "]")
    list_blob = text[j + 1:list_close]
    inputs = [s.encode().decode("unicode_escape") if "\\" in s else s
              for s in STR.findall(list_blob)]
    if not inputs:
        continue

    ctor = expr.strip().split(None, 1)[0]
    tuples = [tuple(int(x) if "." not in x else float(x) for x in g)
              for g in TUPLE.findall(expr)]
    grain_words = [w for w in re.findall(r"[A-Za-z]+", expr) if w in GRAINS]
    holiday = STR.findall(expr)
    direction = "After" if re.search(r"\bAfter\b", expr) else (
                "Before" if re.search(r"\bBefore\b", expr) else None)

    if not grain_words:
        skipped.append((ln, "no grain", expr[:60]))
        continue
    grain = grain_words[-1].lower()

    if ctor in ("datetime", "datetimeHoliday"):
        if len(tuples) != 1:
            skipped.append((ln, f"{ctor} expected 1 tuple", expr[:60])); continue
        val = {"type": "value", **instant(tuples[0], grain)}
        if ctor == "datetimeHoliday" and holiday:
            val["holidayBeta"] = holiday[0]
    elif ctor in ("datetimeInterval", "datetimeIntervalHoliday"):
        if len(tuples) != 2:
            skipped.append((ln, f"{ctor} expected 2 tuples", expr[:60])); continue
        val = {"type": "interval",
               "from": instant(tuples[0], grain),
               "to": instant(tuples[1], grain)}
        if ctor == "datetimeIntervalHoliday" and holiday:
            val["holidayBeta"] = holiday[0]
    elif ctor == "datetimeOpenInterval":
        if len(tuples) != 1 or direction is None:
            skipped.append((ln, "openInterval shape", expr[:60])); continue
        val = {"type": "interval"}
        val["from" if direction == "After" else "to"] = instant(tuples[0], grain)
    else:
        skipped.append((ln, f"unknown ctor {ctor}", expr[:60])); continue

    for inp in inputs:
        positives.append({"input": inp, "expected": val})

# negativeCorpus: a local `examples = [ "...", ... ]` between neg_line and latentCorpus
neg_region = "\n".join(lines[neg_line - 1: allex_line - 1])
nm = re.search(r"examples\s*=\s*\[", neg_region)
if nm:
    start = nm.end() - 1
    end = match_balanced(neg_region, start, "[", "]")
    negatives = STR.findall(neg_region[start + 1:end])

doc = {
    "context": {"referenceTime": "2013-02-12T04:30:00.000-02:00",
                "locale": "en", "withLatent": False},
    "positive": positives,
    "negative": negatives,
}

import os
os.makedirs(os.path.dirname(OUT), exist_ok=True)
with open(OUT, "w", encoding="utf-8") as f:
    json.dump(doc, f, ensure_ascii=False, indent=1)

print(f"positive examples: {len(positives)}")
print(f"negative examples: {len(negatives)}")
print(f"skipped blocks:    {len(skipped)}")
for s in skipped:
    print("  SKIP", s)

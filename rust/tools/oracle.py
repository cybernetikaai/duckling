#!/usr/bin/env python3
"""Use the live rasa/duckling server as an oracle to (1) cross-check the
transcribed corpus fixtures, and (2) generate DST-stress fixtures (Task 0.6)."""
import json
import urllib.parse
import urllib.request
from datetime import datetime, timezone

FIX = "/Users/13protons/github/duckling/rust/fixtures/en_time_corpus.json"
TZ_OUT = "/Users/13protons/github/duckling/rust/fixtures/tz_stress.json"
URL = "http://localhost:8000/parse"

def reftime_ms(iso_z):
    dt = datetime.strptime(iso_z, "%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc)
    return int(dt.timestamp() * 1000)

def parse(text, tz, ref_ms, lang="en"):
    data = urllib.parse.urlencode({
        "lang": lang, "tz": tz, "reftime": ref_ms,
        "dims": '["time"]', "text": text,
    }).encode()
    with urllib.request.urlopen(URL, data=data, timeout=10) as r:
        return json.load(r)

def strip_values(v):
    return {k: x for k, x in v.items() if k != "values"}

def core(v):  # ignore holidayBeta (rasa image version may differ on holidays)
    return {k: x for k, x in v.items() if k not in ("values", "holidayBeta")}

def full_range_values(entities, text):
    n = len(text)
    return [strip_values(e["value"]) for e in entities
            if e.get("dim") == "time" and e.get("start") == 0 and e.get("end") == n]

# ---- (1) cross-check ---------------------------------------------------------
NORONHA_REF = reftime_ms("2013-02-12T06:30:00Z")  # = 04:30 -02:00 test context
fix = json.load(open(FIX))
pos = fix["positive"]
exact = core_ok = checked = errors = 0
core_mismatches = []
for ex in pos:
    inp, expected = ex["input"], ex["expected"]
    try:
        got = full_range_values(parse(inp, "America/Noronha", NORONHA_REF), inp)
    except Exception as e:
        errors += 1
        continue
    checked += 1
    if any(g == expected for g in got):
        exact += 1
    if any(core(g) == core(expected) for g in got):
        core_ok += 1
    else:
        if len(core_mismatches) < 15:
            core_mismatches.append((inp, core(expected), [core(g) for g in got]))

print("=== CROSS-CHECK (transcription vs live oracle, tz=America/Noronha) ===")
print(f"checked={checked}  exact={exact} ({100*exact//max(checked,1)}%)  "
      f"core(ignoring holidayBeta)={core_ok} ({100*core_ok//max(checked,1)}%)  errors={errors}")
print(f"core mismatches (real signal): {checked - core_ok}")
for inp, exp, got in core_mismatches:
    print(f"  {inp!r}\n    expected {json.dumps(exp)}\n    oracle   {json.dumps(got)}")

# ---- (2) DST-stress fixtures -------------------------------------------------
CASES = [
    ("America/New_York", "2013-01-15T12:00:00Z", "in 4 months"),          # EST->EDT
    ("America/New_York", "2013-01-15T12:00:00Z", "the 4th of July"),       # summer EDT
    ("America/New_York", "2013-01-15T12:00:00Z", "first Sunday of November"),  # fall-back day
    ("America/New_York", "2013-03-09T12:00:00Z", "tomorrow at 2:30am"),    # spring-forward gap
    ("America/New_York", "2013-11-02T12:00:00Z", "tomorrow at 1:30am"),    # fall-back ambiguous
    ("Europe/London",    "2013-06-01T12:00:00Z", "3pm EST"),               # in-text zone != ref
    ("Europe/London",    "2013-06-01T12:00:00Z", "Christmas"),             # winter GMT
    ("Australia/Sydney", "2013-06-01T12:00:00Z", "in 6 months"),           # southern flip
    ("Australia/Sydney", "2013-06-01T12:00:00Z", "December 25th"),         # AEDT +11
    ("America/Los_Angeles", "2017-06-01T12:00:00Z", "the first Tuesday of October"),  # README PDT
]
tz_fixtures = []
print("\n=== DST-STRESS FIXTURES (oracle truth) ===")
for zone, ref_iso, text in CASES:
    try:
        got = full_range_values(parse(text, zone, reftime_ms(ref_iso)), text)
    except Exception as e:
        print(f"  ERROR {zone} {text!r}: {e}"); continue
    expected = got[0] if len(got) == 1 else None
    tz_fixtures.append({"zone": zone, "referenceTimeUtc": ref_iso,
                        "input": text, "expected": expected,
                        "ambiguousCount": (None if expected else len(got))})
    show = expected["value"] if (expected and "value" in expected) else (
           f"interval {expected.get('from',{}).get('value','?')}..{expected.get('to',{}).get('value','?')}"
           if expected else f"<{len(got)} parses>")
    print(f"  [{zone}] now={ref_iso}  {text!r}\n    -> {show}")

json.dump({"cases": tz_fixtures}, open(TZ_OUT, "w"), indent=1)
print(f"\nwrote {len(tz_fixtures)} DST-stress cases to {TZ_OUT}")

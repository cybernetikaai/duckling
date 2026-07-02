#!/usr/bin/env python3
"""Evaluate the Duckling port's Time recognition against a GMB-style NER CSV.

The CSV has columns: Sentence #, Sentence, POS, Tag  (Tag is a Python-list string
of BIO tags: B-tim/I-tim mark time expressions). We extract the gold `tim` spans,
batch-parse each sentence with `duckling --dims time --batch`, and score
recognition by character-span overlap.

Usage:  python3 ner_time_eval.py /path/to/ner.csv [--limit N] [--bin PATH]
Duckling is a structured-value extractor, so only the `tim` tag is in scope
(geo/gpe/per/org/... are classic NER, not Duckling dimensions).
"""
import csv, ast, sys, subprocess, json, argparse, collections
csv.field_size_limit(10**7)

ap = argparse.ArgumentParser()
ap.add_argument("csv")
ap.add_argument("--limit", type=int, default=0)
ap.add_argument("--bin", default="rust/target/release/duckling")
ap.add_argument("--ref", default="2005-06-15T12:00:00Z")
ap.add_argument("--dims", default="time")
ap.add_argument("--latent", action="store_true")
args = ap.parse_args()

def tim_spans(tags):
    """Contiguous B-tim/I-tim runs -> list of (tok_start, tok_end) exclusive."""
    spans, i = [], 0
    while i < len(tags):
        if tags[i] == "B-tim":
            j = i + 1
            while j < len(tags) and tags[j] == "I-tim":
                j += 1
            spans.append((i, j)); i = j
        else:
            i += 1
    return spans

def char_ranges(tokens, tok_spans):
    """Map token-index spans to char ranges in the single-space join."""
    starts, pos = [], 0
    for t in tokens:
        starts.append(pos); pos += len(t) + 1
    ends = [starts[k] + len(tokens[k]) for k in range(len(tokens))]
    return [(starts[a], ends[b - 1]) for (a, b) in tok_spans]

rows = []
with open(args.csv, newline="", encoding="utf-8", errors="replace") as f:
    for row in csv.DictReader(f):
        try:
            tags = ast.literal_eval(row["Tag"])
        except Exception:
            continue
        toks = row["Sentence"].split()
        if not toks or len(toks) != len(tags):
            continue
        text = " ".join(toks)
        gold = char_ranges(toks, tim_spans(tags))
        rows.append((text, gold))
        if args.limit and len(rows) >= args.limit:
            break

# Batch-parse all sentences in one process.
inp = "\n".join(t for t, _ in rows)
proc = subprocess.run(
    [args.bin, "--dims", args.dims, "--ref", args.ref, "--batch"] + (["--latent"] if args.latent else []),
    input=inp, capture_output=True, text=True,
)
out_lines = proc.stdout.splitlines()

def overlaps(a, b):
    return a[0] < b[1] and b[0] < a[1]

gold_total = gold_hit = 0          # gold tim spans; recognized (overlap)
gold_exact = 0                     # gold spans matched exactly by a port span
port_total = port_matched = 0      # port time spans; overlapping >=1 gold
sent_with_gold = sent_gold_all_hit = 0
misses = collections.Counter()
extras = collections.Counter()

for (text, gold), line in zip(rows, out_lines):
    try:
        ents = json.loads(line) if line.strip() else []
    except Exception:
        ents = []
    pspans = [(e["start"], e["end"]) for e in ents]
    port_total += len(pspans)
    for ps in pspans:
        if any(overlaps(ps, g) for g in gold):
            port_matched += 1
        else:
            extras[text[ps[0]:ps[1]].lower()] += 1
    if gold:
        sent_with_gold += 1
        all_hit = True
        for g in gold:
            gold_total += 1
            if any(overlaps(p, g) for p in pspans):
                gold_hit += 1
                if any(p == g for p in pspans):
                    gold_exact += 1
            else:
                all_hit = False
                misses[text[g[0]:g[1]].lower()] += 1
        if all_hit:
            sent_gold_all_hit += 1

def pct(a, b): return f"{100*a/b:.1f}%" if b else "n/a"
print(f"sentences evaluated:        {len(rows)}")
print(f"sentences with a tim span:  {sent_with_gold}")
print()
print(f"gold tim spans:             {gold_total}")
print(f"  recognized (overlap):     {gold_hit}  recall {pct(gold_hit, gold_total)}")
print(f"  matched exactly:          {gold_exact}  exact {pct(gold_exact, gold_total)}")
print(f"  sentences fully covered:  {sent_gold_all_hit}  ({pct(sent_gold_all_hit, sent_with_gold)})")
print()
print(f"port time spans emitted:    {port_total}")
print(f"  overlap a gold tim:       {port_matched}  ({pct(port_matched, port_total)})")
print(f"  no gold overlap (extra):  {port_total - port_matched}  ({pct(port_total-port_matched, port_total)})")
print()
print("top 20 MISSED gold tim expressions (port found nothing overlapping):")
for w, c in misses.most_common(20): print(f"  {c:5}  {w!r}")
print()
print("top 20 EXTRA port spans (no gold tim overlap — number/date GMB didn't tag tim, or spurious):")
for w, c in extras.most_common(20): print(f"  {c:5}  {w!r}")

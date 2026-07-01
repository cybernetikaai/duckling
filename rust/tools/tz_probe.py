#!/usr/bin/env python3
"""Probe the live rasa/duckling oracle for DST-stress cases across zones and
transitions, then merge the unambiguous ones into tz_stress.json."""
import json, urllib.parse, urllib.request
from datetime import datetime, timezone

URL = "http://localhost:8000/parse"
TZ_OUT = "/Users/13protons/github/duckling/rust/fixtures/tz_stress.json"

def ref_ms(iso): return int(datetime.strptime(iso,"%Y-%m-%dT%H:%M:%SZ").replace(tzinfo=timezone.utc).timestamp()*1000)
def parse(text, tz, iso):
    data=urllib.parse.urlencode({"lang":"en","tz":tz,"reftime":ref_ms(iso),"dims":'["time"]',"text":text}).encode()
    with urllib.request.urlopen(URL,data=data,timeout=10) as r: return json.load(r)
def strip(v): return {k:x for k,x in v.items() if k!="values"}
def full(ents,text):
    n=len(text); return [strip(e["value"]) for e in ents if e.get("dim")=="time" and e.get("start")==0 and e.get("end")==n]

# (zone, ref just before a DST transition) -> probes hit the transition day via "tomorrow"
SCEN = [
    ("America/New_York",     "2024-03-09T12:00:00Z"),  # spring fwd next day
    ("America/New_York",     "2024-11-02T12:00:00Z"),  # fall back next day
    ("America/Los_Angeles",  "2024-03-09T20:00:00Z"),
    ("America/Los_Angeles",  "2024-11-02T20:00:00Z"),
    ("Europe/London",        "2024-03-30T12:00:00Z"),
    ("Europe/London",        "2024-10-26T12:00:00Z"),
    ("Europe/Berlin",        "2024-03-30T12:00:00Z"),
    ("Australia/Sydney",     "2024-04-06T12:00:00Z"),  # southern fall back
    ("Australia/Sydney",     "2024-10-05T12:00:00Z"),  # southern spring fwd
    ("Pacific/Auckland",     "2024-04-06T00:00:00Z"),
]
INPUTS = ["tomorrow at 2:30am","tomorrow at 1:30am","tomorrow at 2am","tomorrow at 3am",
          "tomorrow at noon","tomorrow at 3pm","in 2 days at 9am","next monday at 8am"]

cases=[]
for zone,ref in SCEN:
    for inp in INPUTS:
        try: vals=full(parse(inp,zone,ref),inp)
        except Exception: continue
        if len(vals)==1:  # unambiguous single full-range value
            cases.append({"zone":zone,"referenceTimeUtc":ref,"input":inp,"expected":vals[0]})

data=json.load(open(TZ_OUT))
existing={(c["zone"],c["referenceTimeUtc"],c["input"]) for c in data["cases"]}
added=[c for c in cases if (c["zone"],c["referenceTimeUtc"],c["input"]) not in existing]
data["cases"].extend(added)
json.dump(data,open(TZ_OUT,"w"),indent=2)
print(f"probed {len(SCEN)*len(INPUTS)}, unambiguous {len(cases)}, newly added {len(added)}, total now {len(data['cases'])}")
# Show a sample of added cases (esp. transition-day times)
for c in added:
    if "2:30" in c["input"] or "1:30" in c["input"] or "2am" in c["input"]:
        print(" ",c["zone"],c["input"],"->",c["expected"]["value"])

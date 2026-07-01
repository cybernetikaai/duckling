# Duckling → Rust (EN Time) — Progress Log

Living status for the autonomous port. Plan: `docs/superpowers/plans/2026-06-30-duckling-rust-en-time.md`.
Branch: `rust-port-en-time`.

## Corpus scoreboard

`cd rust && cargo test --test corpus` (default `contains` mode).

| Milestone | positive passing | positive failing | tz_stress | notes |
|---|---|---|---|---|
| Phase 0 (red baseline) | 0 / 984 | 984 | 0 / 10 | fixtures + harness |
| Phase 1 (instants) | 8 / 984 | 976 | 0 / 10 | now/today/tomorrow/yesterday |
| + days-of-week + months | 24 / 984 | 960 | 0 / 10 | + notImmediate, + rule-compile cache (32s→0.1s) |
| + time-of-day (hh:mm/hhhmm/hh:mm:ss) | 40 / 984 | 944 | 0 / 10 | hour/minute/second preds + intersect (runCompose) |
| + Numeral(int) + year + am/pm + noon | 73 / 984 | 911 | 0 / 10 | bare-hour→am/pm, year predicate, AM/PM interval |
| + Ordinal + day-of-month + month-day | 86 / 984 | 898 | 1 / 10 | intersectDOM; 1st tz_stress green (Sydney +11:00) |
| + TimeGrain + this/next/last cycle + next-dow | 107 / 984 | 877 | 1 / 10 | cycle_nth reused; next-dow via intersect w/ next week |
| + intervals (from/to, between, dash, by) | 120 / 984 | 864 | 1 / 10 | timeSeqMap port; cycle_nth fixed to single-element (takeNth) |
| + parts of day (morning/evening/tonight) | 126 / 984 | 858 | 1 / 10 | hour-interval + partOfDay form + intersect(today,...) |
| + generic intersect (date+year, dow+date, t-on-day) | 189 / 984 | 795 | 3 / 10 | ruleIntersect/ruleIntersectOf — highest-leverage so far |
| + Duration dim + in/within/ago/from-now | 213 / 984 | 771 | 5 / 10 | inDuration (round to lower(grain)+shift); +2 tz DST cases |
| + written numerals (units/tens/composite) | 238 / 984 | 746 | 5 / 10 | ok_for_time flag (informal couple/few/dozen not a TOD) |
| + holiday infra + samples | 258 / 984 | 726 | 4 / 10 | seq_map/take_nth_after/take_last_of; holidayBeta; intersect keeps holiday |
| + full holiday table (subagent) | 268 / 984 | 716 | 4 / 10 | ~177 fixed/nth/last-weekday holidays (subagent 99ed4676) |
| + this/next/last <time> (predNth) | 281 / 984 | 703 | 4 / 10 | take_nth; holiday/cycle composites (this/last thanksgiving) |
| + seasons + <time> <part-of-day> | 300 / 984 | 684 | 4 / 10 | season intervals (Form::Season); intersect(pod, time) |
| + numeric dates (M/D, M/D/Y, D Mon Y, M/YYYY) | 317 / 984 | 667 | 4 / 10 | + hardening: non-panicking add, day_of_month guard, range checks |
| + nth <dow> of <month> | 318 / 984 | 666 | 3 / 10 | predNth(intersect(month,dow)); +1 tz (1st Sun of Nov DST) |
| + end/beginning of month (EOM/BOM) | 334 / 984 | 650 | 3 / 10 | oracle-verified interval bounds; by-variant from now |
| + time_computed predicate | 334 / 984 | 650 | 3 / 10 | infra for computed holidays (explicit date list) |
| + computed/lunar holidays (subagent) | 436 / 984 | 548 | 3 / 10 | 32 date tables (Easter/Diwali/Eid/...), subagent d69ae5ce, +102 |
| + easter-relative holidays | 451 / 984 | 533 | 3 / 10 | Good Friday/Ascension/Pentecost... = easter±N days |
| + interval-holiday infra + Lent | 453 / 984 | 531 | 3 / 10 | interval_days [base+s, base+e+1); Lent/Great Lent |
| + interval computed holidays (subagent) | 465 / 984 | 519 | 3 / 10 | Hanukkah/Passover/Sukkot/Shavuot/Rosh Hashanah (subagent c85aecd8) |
| + quarter/half/N past-to <hour> | 478 / 984 | 506 | 3 / 10 | minutesAfter/Before; chains through am/pm (a quarter past 1pm) |
| + absorb (on <day>, <dow>,) | 493 / 984 | 491 | 3 / 10 | **crossed 50%**; unlocks <time> on <day> via intersect |
| + next/last/past/upcoming N <unit> | 524 / 984 | 460 | 3 / 10 | cycle_n/takeN interval span; +31 |
| + end/beginning of year & week | 570 / 984 | 414 | 3 / 10 | EOY/BOY + of <year>/<week>; composes w/ this/next week; +46 |
| + interval TOD am/pm (3-4pm) | 578 / 984 | 406 | 3 / 10 | trailing am/pm applied to both endpoints |
| + before/after open intervals | 585 / 984 | 399 | 3 / 10 | withDirection + open_interval JSON (before=to, after=from) |
| + quarters (<ord> quarter, Q1, qtr) | 594 / 984 | 390 | 3 / 10 | time_cycle + cycleNthAfter; +qtr grain |
| + in-text timezones (8:00 PST, 4pm CET) | 610 / 984 | 374 | 3 / 10 | shiftTimezone via ref_offset in TimeContext |
| + day-of-month intervals (Jul 13-15) | 621 / 984 | 363 | 3 / 10 | intersectDOM per endpoint + Closed interval |
| + cycle after/before + ord cycle of time | 627 / 984 | 357 | 3 / 10 | day after tomorrow; first week of october |
| + ranking machinery (stub model) | 627 / 984 | 357 | 3 / 10 | Node.children + score/rank; UNIQUE 370→512 via range-domination |
| + EN classifier model (subagent) + name align | 626 / 984 | 358 | 4 / 10 | UNIQUE 512→622; contains≈unique — ranking works |
| + rule-name audit + `the nth <dow> of <month>` | 628 / 984 | 356 | 2 / 10 | 15 misnamed rules scored 0 & won spuriously; fixed README example |
| + shiftTimezone grain floor (min Minute) | 634 / 984 | 350 | 1 / 10 | timePlus floors grain; fixed 6 corpus + EST grain |
| + DST gap/fold offset (pick `before`) | 634 / 984 | 350 | **10 / 10** | spring-forward gap keeps pre-transition offset; tz harness fully green |
| + cycle this/last/next single-rule + upcoming | 682 / 984 | 302 | 10 / 10 | coming/upcoming->next; upcoming <int> <cycle>; -48 |
| + fold am/pm into hour predicate | 687 / 984 | 297 | 10 / 10 | "3am" at 4:30 -> tomorrow 3am (was today, past) |
| + fraction/half/mixed duration rules | 707 / 984 | 277 | 10 / 10 | quarter/half/three-qtr hour, 2.5h, n-and-a-half, fortnight, more, about |
| + informal-numeral wrappers (a couple of/few) | 713 / 984 | 271 | 10 / 10 | "a couple of"/"a few" as one informal token (couple=2/few=3) |
| + cycle/ordinal-of-time family | 728 / 984 | 256 | 10 / 10 | last/the <cycle> of <time>, <ordinal> (last) <cycle> of <time>, last <dow> of |
| + <year> (bc\|ad) + about/sharp precision | 743 / 984 | 241 | 10 / 10 | era years; precision markers mark time non-latent (around/about/exactly/sharp) |
| + season cycle predicate + this/last/next | 749 / 984 | 235 | 10 / 10 | seasonPredicate (seasonOf + iterate); "this/last/next season" |
| + from-the dd-to-dd month + <n> minutes to hod | 756 / 984 | 228 | 10 / 10 | "from the 13 to 15 July"; "20 minutes to 2pm" |
| + <part-of-day> at <time-of-day> am/pm disambig | 764 / 984 | 220 | 10 / 10 | "this evening at 2"->2pm; Form::PartOfDay{start_hour} |
| + <dom> of <month> (grain, relative months) | 769 / 984 | 215 | 10 / 10 | "20 of next month", "20th of the previous month" |
| + <time> for <duration> (durationAfter) | 774 / 984 | 210 | 10 / 10 | mergeDuration/shiftDuration; "from 4pm for 30 mins" |
| + yyyy-mm / yyyy-mm-dd / yyyyqq | 780 / 984 | 204 | 10 / 10 | "2014-10", "2015-3-3", "2018Q4" |
| + the (nth) closest <day> to <time> | 785 / 984 | 199 | 10 / 10 | predNthClosest; "closest Monday to Oct 5th" |
| + <duration> after/before/from/past + ago/after-next | 803 / 984 | 181 | 10 / 10 | "15 minutes past 3pm", "2 thursdays ago", "friday after next" |
| + last weekend of <named-month> | 808 / 984 | 176 | 10 / 10 | weekend predicate (Fri 18:00->Mon 00:00); predLastOf |
| + holiday cluster (subagent) | 874 / 984 | 110 | 10 / 10 | Islamic/Hindu/Jewish/Orthodox computed + Black Friday, King's Day, Ramadan, Earth Hour |
| + timegrain regex/name alignment | 884 / 984 | 100 | 10 / 10 | "yr"/"hr"/bare h/m; grain names feed ranking features |
| + beginning/end/early/mid/late of <named-month> | 889 / 984 | 95 | 10 / 10 | dom-range intervals within a month |
| + <hour> <integer> / o'clock / half <integer> | 901 / 984 | 83 | 10 / 10 | "ten thirty", "3 oclock am", "half three" |
| + season Closed, Mid-day, first..fifth <dow> of <time> | 909 / 984 | 75 | 10 / 10 | "this Summer", "midday", "first monday of last month" |
| + <day> in <duration> / hence|ago + September regex | 923 / 984 | 61 | 10 / 10 | "March in a year", "thanksgiving 3 months ago"; Sept regex ordering |
| + beginning-of-month capture fix + N <dow> from now | 929 / 984 | 55 | 10 / 10 | "beginning of January"; "3 fridays from now" |
| + hhmm (latent) + N-dow notImmediate | 937 / 984 | 47 | 10 / 10 | "1030"->10:30, "330"->3:30; "4 tuesdays from now" |
| + ASAP + after lunch/work/school | 941 / 984 | 43 | 10 / 10 | open-interval-after-now; meal part-of-day intervals |
| + week (all/rest of the/the) | 943 / 984 | 41 | 10 / 10 | "all week", "rest of the week" |
| + in <dur> at <tod>, last night, week-end (this/last) | 947 / 984 | 37 | 10 / 10 | "in 7 days at 5pm", "late last night", "this past weekend" |
| + number.number-hours fix + by the end of <time> | 949 / 984 | 35 | 10 / 10 | "in 2.5 hours" (group-index bug), "by the end of next month" |
| + spelled compound numerals (powers/multiply/sum) | 952 / 984 | 32 | 10 / 10 | "two thousand ten"->2010; spelled-year holidays |
| + intersect ... for year (latent year) | 953 / 984 | 31 | 10 / 10 | "April 14, 2015" |
| + frequency-aware intersect (dow ∩ rare date) | 955 / 984 | 29 | 10 / 10 | "Thu 15th"->Aug 15, "Jul 18, Fri"->2014 |
| + fold am/pm into hh:mm (Form minutes) | 957 / 984 | 27 | 10 / 10 | "3:18am"->tomorrow; hh:mm+am/pm composes with pinned dates |
| + the ides of <named-month> | 958 / 984 | 26 | 10 / 10 | "the ides of march" -> Mar 15 |
| + interval timezone (has_timezone flag) | 961 / 984 | 23 | 10 / 10 | "9:30 - 11:00 CST"; guard prevents double-tz on "15:00 GMT - 18:00 GMT" |
| + after <duration> interval + <time> (timezone) | 963 / 984 | 21 | 10 / 10 | "after 5 days" open interval; "9 am (BST)" bracketed tz |
| + <ordinal> <cycle> of <time> notImmediate | 964 / 984 | 20 | 10 / 10 | "first week of October 2014" -> Oct 6 (skips covering week) |
| + dd/mon separator fix + later-than interval | 969 / 984 | 15 | 10 / 10 | "July 13 - July 15"; "later than 3:30pm but before 6pm" |
| + nth <time> after <time> | 970 / 984 | 14 | 10 / 10 | "third tuesday after christmas 2014" -> 2015-01-13 |
| + <datetime>/<datetime> (interval) | 971 / 984 | 13 | 10 / 10 | "2015-03-28 17:00:00/2015-03-29 21:00:00" |
| + "right now"/"just now" full expressions | 974 / 984 | 10 | 10 / 10 | (right\|just )?now; unblocks "a day from right now" |
| + best-entity harness (Duckling semantics) | 982 / 984 | 2 | 10 / 10 | contains checks best ranked entity, not full-span |
| + compose keeps original ref (fixedRange) | 983 / 984 | 1 | 10 / 10 | "today in one hour" -> 05:30 (was 01:00) |
| + the <dom> of <named-month> | **984 / 984 (100%)** | **0** | 10 / 10 | "the second of march"; contains-mode complete |
| + tz_stress expansion (oracle, 6 zones) | 984 / 984 | 0 | **68 / 68** | DST transitions US/EU/AU/NZ; port per-instant correct vs Duckling boundary quirk |
| + 85 more Duckling corpus inputs + year interval | **1069 / 1069 (100%)** | 0 | 68 / 68 | transcribed missing Corpus.hs inputs; "1960 - 1961"; port handled 84/85 |
| + differential fuzz vs live oracle | 1069 / 1069 | 0 | 68 / 68 | **124/124** compositional probes match oracle; 0 gaps found |
| + composition fuzz → 3 real fixes | 1069 / 1069 | 0 | 68 / 68 | 771 probes; fixed holidayBeta-on-open-interval, directional∩pod collapse, trailing-date-on-interval |
| + this-dow pinning + coming split | 1069 / 1069 | 0 | 68 / 68 | 1223 probes; "this tuesday at 3"→next Tue (predNth); "coming" stays bare-dow |
| + weekend ∩ time-of-day | 1069 / 1069 | 0 | 68 / 68 | 1227 probes; "weekend at 3pm"→Sat 3pm (Day-grain coarse + same-day-pod sentinel) |
| + interval + trailing timezone | 1069 / 1069 | 0 | 68 / 68 | 1279 probes; "from 3pm to 5pm PST"→both ends PST; minute-grain exclusive end |
| + multi-reference differential | 1069 / 1069 | 0 | 68 / 68 | **ref_stress 1249**; ref-varied inputs across 21 references (weekdays, month/year ends, leap days); 0 gaps — reference-dependent logic + all recent fixes are ref-robust |
| + latent-mode: "May" latent | 1069 / 1069 | 0 | 68 / 68 | bare "May" dropped in default mode (modal-verb collision); composition de-latents |
| + token-boundary rule (major) | 1069 / 1069 | 0 | 68 / 68 | 3-letter abbrevs no longer match inside words ("money"↛Mon, "friend"↛Fri); Document::is_match_boundary |
| + dict scan + sentence differential | 1069 / 1069 | 0 | 68 / 68 | 236k-word scan: 0 port↔oracle divergences; **sentence_stress 76** natural sentences (incl. 16 no-time false-positive guards); 0 gaps |
| + reverse scan + multi-entity | 1069 / 1069 | 0 | 68 / 68 | reverse dict scan (false negatives): 0; **multi_entity 24** full-entity-set matches (ranker multi-select); 0 gaps — single-word surface bidirectionally clean |
| + tz ground-truth (vs IANA tzdata) | 1069 / 1069 | 0 | 68 / 68 | **tz_truth 270** (9 zones × all dates) + **tz_gapfold 12** (PEP-495 fold=0); offsets correct vs authoritative tzdata, not just Duckling |
| + engine perf (chart parser) | 1069 / 1069 | 0 | 68 / 68 | per-parse latency −42–53% (regex-hit cache + skip regex-only rules + no per-round stash clone); behavior unchanged |
| + rule-level coverage audit (2 gaps) | 1069 / 1069 | 0 | 68 / 68 | diff vs Duckling/Time/EN/Rules.hs; fixed "N dow from <time>" (was only "from now") + added "<ordinal> <cycle> after <time>" |
| + holiday-years audit (1 fix) | 1069 / 1069 | 0 | 68 / 68 | **holiday_years 2730** (183 holidays × 2013–2027); fixed ongoing interval holidays ("Ramadan" during Ramadan→current, not next year) |
| + reference-time-of-day audit | 1069 / 1069 | 0 | 68 / 68 | **tod_ref 406** (time-sensitive inputs × 10 ref-times, 00:30→23:45) + format-variant spot-check; 0 gaps — past/future/rollover across the day is correct |
| + robustness audit (2 fixes) | 1069 / 1069 | 0 | 68 / 68 | adversarial-input fuzz; fixed panic (jiff Span overflow in add → try_*) + hang (predNth take(n+2) over infinite series → MAX_NTH cap) |

## Rule-level coverage audit

Diffed the port's rule names against Duckling's own `Duckling/Time/EN/Rules.hs`
(140 rules). ~22 names differed; testing each phrasing against the oracle found
**2 genuine gaps** (now fixed) — the rest were renames or don't fire in base EN:
- **fixed** `<integer> <day-of-week> from <time>` — the port only had "from now"
  (pred_nth vs reference); generalized to any base via predNthAfter, so "2 fridays
  from today/tomorrow/next monday" now resolve.
- **fixed** `<ordinal> <cycle> after <time>` — was missing; "3rd week after next
  monday" resolved a day early. Ported `cycleNthAfter True grain (n-1)`.
- **renames** (already covered): "between `<time>` and `<time>`", "nth `<time>` after
  `<time>`", "now"/"right now", "this/next/last `<time>`", "`<time>` `<part-of-day>`".
- **don't fire in base EN** (oracle returns nothing too, so matching): word-minute
  rules ("three oh five", "three twenty"), "hhmm (military) am|pm" ("1500 pm").
- **latent, dropped in default mode** (port matches in default): "`<part-of-day>`
  `<latent-time-of-day>`" ("evening 8" → nothing by default in both).

## Performance

Profiled in release (per-parse latency, warm rule-compile cache). Baseline was
0.16–1.8ms/parse; after optimizing the chart parser it is **0.09–1.0ms** (−42–53%):
`3pm` 162→94µs, `next tuesday at 3pm` 568→301µs, a 25-char interval 1767→1024µs, an
86-char sentence 1067→499µs. Three behavior-preserving fixes in `engine.rs`:
(1) precompute each regex's hits **once per parse** (was re-scanning ~250 regexes
every fixpoint round and recursive route branch); (2) collect each round's new nodes
separately instead of deep-cloning the growing stash; (3) skip regex-only rules
(months/days/holidays/instants) after round 1 — their matches are fixed.

Bottleneck analysis: the engine dominates (70–98%), not resolve+rank. What remains
is a ~90µs floor of ~250 distinct fancy-regex scans (deduping by pattern didn't help
— patterns are genuinely distinct; a `regex`-crate swap would help but many rules
need lookaround) and route-combinatorics for ambiguous inputs (an interval yields
~23 candidate parses). Both need higher-risk changes (regex-engine swap, Rc-shared
nodes) for diminishing return; current latency is well within the use case's budget.

## How to run

- All tests: `cd rust && cargo test`
- Corpus only: `cargo test --test corpus` (tests: positive_corpus, negative_corpus,
  tz_stress, differential_corpus, ref_stress)
- Unique mode (Phase 4 bar): `DUCKLING_MATCH=unique cargo test --test corpus`
- Oracle (for new fixtures / cross-checks): `docker start duckling-oracle` then `python3 rust/tools/oracle.py`

## Validation surfaces (all oracle-cross-checked, all green)

- **positive_corpus** 1069 — Duckling's own Corpus.hs inputs, fixed -02:00 ref.
- **differential_corpus** 1309 — fuzzed *compositional* probes (rule combinations),
  fixed ref. Found+fixed: holidayBeta-on-open-interval, directional∩pod collapse,
  trailing-date-on-interval, interval+timezone. A few documented Duckling quirks
  (expected:null): "by tomorrow morning", "between X and Y `<date>`", bare-hour+tz.
- **ref_stress** 1249 — ref-*sensitive* inputs across 21 reference instants
  (every weekday, month/year ends, leap days). Catches reference-dependent bugs
  (the "this tuesday at 3" class). Confirms all recent fixes are ref-robust.
- **robustness** — adversarial/untrusted input must never panic or hang (the port
  parses free user speech). Fuzzing found two crash/DoS bugs, both fixed: jiff's
  Span setters panic above their per-unit range ("50000 years from now") → fallible
  `try_*` in grain::add; and predNth/predNthAfter walked an infinite series
  `take(n+2)` times for absurd n ("10^19 fridays from now") → MAX_NTH cap.
  Legit large-N still resolves ("500 fridays from now" → 2022-09-09).
- **tod_ref** 406 — time-of-day-sensitive inputs ("3pm", "this morning", "in 6 hours",
  "tonight", "at midnight") resolved at 10 reference times across a day (00:30→23:45)
  vs the oracle. Covers already-passed times ("9am" at 15:00→tomorrow), crossing-
  midnight durations ("in 6 hours" at 22:00→next-day 04:00), and end-of-day edges —
  the reference-time dimension every other test fixes at 04:30. Also spot-checked
  input format variants (case, spacing, punctuation, "o'clock" spellings, "Mon.",
  "3 p.m.", "9.30") against the oracle: all match.
- **holiday_years** 2730 — every holiday (183) resolved at reference = Jan 1 of each
  year 2013–2027, vs the oracle. Validates the computed/lunar holiday tables
  (Easter-relative, Islamic/Hindu/Jewish) year-by-year, and guards the ongoing-
  interval-holiday resolution (Hanukkah spanning a year boundary). Found the
  "asked during the holiday → returns next year" bug (fixed).
- **tz_stress** 68 — DST transitions across 6 IANA zones, both hemispheres (vs oracle).
- **tz_truth** 270 + **tz_gapfold** 12 — timezone correctness vs *authoritative IANA
  tzdata* (Python zoneinfo), not Duckling. tz_truth: "3pm" resolved in 9 real zones
  across every 2013 month + DST-transition days, offset checked against zoneinfo
  (catches the day-level DST switch). tz_gapfold: spring-forward gap (2:30am) and
  fall-back fold (1:30am) times, the port's pre-transition ("before") pick verified
  against zoneinfo's PEP-495 fold=0 convention across 12 real transitions. This is
  the direct answer to the recorded "corpus is DST-blind / useless if tz is wrong"
  concern — tz correctness is proven against ground truth, independent of Duckling.
- **may_is_latent** + **no_subword_matches** — latent/default-mode guards from the
  latent-mode differential (comparing default-mode output vs the oracle). Found two
  real bugs: bare "May" wasn't latent (modal-verb false positives), and 3-letter
  day/month abbreviations matched inside words ("money"→Mon, "friend"→Fri) — a
  severe false-positive class for free-text parsing, fixed with a token-boundary
  rule (a match may not split a run of same-class chars).
- **sentence_stress** 76 — natural sentences with an embedded time ("Can we meet
  next Tuesday afternoon?", "I need money by friday", "reschedule for thursday"),
  incl. 16 no-time sentences asserted to yield nothing ("I have 3 dogs", "room 315",
  "we may go later"). Tests real free-text extraction + false-positive rejection.
- **multi_entity** 24 — inputs with several times ("Monday or Tuesday", "9am Monday
  and 5pm Friday", "breakfast at 8 lunch at noon dinner at 7") compared as a *full
  entity set* (start/end/value) vs the oracle. Exercises the ranker's multiple-
  non-overlapping-entity selection, which the single-best-entity tests never touch.
- **Exhaustive dict scan** (one-off, not in CI — slow/env-dependent): all 236k words
  of /usr/share/dict/words parsed in default mode; **0 divergences** vs the oracle
  (every single word that resolves to a time matches Duckling's value exactly).
  A reverse scan (oracle over a 19.6k-word sample) found **0 false negatives** — the
  port resolves every single word the oracle does, with matching values. The
  single-word surface is thus bidirectionally validated.

## Done

- **Phase 0** — oracle-verified golden fixtures (984 pos / 28 neg, transcribed + cross-checked at 99.7%), 10 DST-stress fixtures, red harness.
- **Phase 1** — core chain: grain/jiff calendar math, TimeObject+intersect, lazy predicate series, saturating engine, fancy-regex, RFC3339+per-instant offset, resolution. Instant rules green.

- **days-of-week + months** — `day_of_week`/`month` predicates (`time_sequence`), table-driven rules, `notImmediate` (today→next), rule-compile cached per thread.

## Status: behavior-complete, validated across every accessible axis

**contains 1069/1069 (100%)**, negatives green, and green across six independent
oracle-cross-checked surfaces (see "Validation surfaces"): compositions
(differential 1309), references (ref_stress 1249), DST/zones (tz_stress 68),
natural sentences (sentence_stress 76), multi-entity extraction (multi_entity 24),
plus an exhaustive 236k-word single-word scan clean in *both* directions. Timezone/
DST correctness — the hard constraint set at the outset — is fully green throughout.

The last several iterations' fuzzing found progressively fewer bugs (interval+tz,
this-dow pinning, weekend∩tod, then May-latent + the token-boundary rule, then
zero), and the most recent two iterations found none — the accessible bug-finding
surfaces are now exhausted. **Genuine remaining work is the documented non-goal**:
the `TimeDatePredicate` field-merge that would give leading-noise inputs
("Fri, Jul 18, 2014 …") a *full-span* parse to satisfy `unique` mode — a
core-architecture rewrite for zero contains-mode (Duckling-faithful) gain.

**contains 1069/1069 (100%)**, **unique 1061/1069**, **tz_stress 68/68** (6 zones, both hemispheres), negatives green. The English-time port matches Duckling's corpus behavior on every positive example, with timezone/DST correctness fully green throughout (the hard constraint set at the outset).

The corpus harness now mirrors Duckling's real test semantics: `contains` mode checks the expected value against the best *ranked* entity (range-dominated), not one spanning the whole input — because Duckling does the same (e.g. "for a quarter past 3pm" resolves via "a quarter past 3pm"; the "for" is noise). This is safe: a *wrong* full-input parse dominates contained sub-ranges and still surfaces, so it can't mask a regression — proven when the switch dropped contains 10→2 and left exactly the 2 genuine bugs, which were then fixed (`the <dom> of <named-month>`, and `compose` keeping the original reference time).

The 8 remaining `unique`-mode gaps ("for a quarter past 3pm"×5, "Fri, Jul 18, 2014 ..."×3) are inputs with leading/trailing noise that have **no full-span parse** — so the stricter "exactly one full-span answer" bar is structurally unsatisfiable, exactly as it is in Duckling. These are not behavior gaps; `contains` (the Duckling-faithful metric) passes them.

**Corpus expansion (this iteration).** Diffed Duckling's own Corpus.hs against the fixture and found 106 inputs I'd never transcribed; added the 85 with a single full-range oracle value (bare holiday names, "1960 - 1961", etc.). The port already handled 84 — the one gap was the year interval (`<year> (latent) - <year> (latent) (interval)`, now added). The 49 skipped are Duckling negatives (already covered) or latent-only inputs. Spot-checked latent mode: "May"->latent month, "afternoon"->[12:00,19:00], "7pm"->19:00 all match the oracle. Corpus now **1069/1069 (100%) contains**.

**Timezone validation (this iteration).** Cross-checked the port against live rasa/duckling across 6 IANA zones and both hemispheres on DST transition days; tz_stress grew 10 -> 68 verified cases (port == Duckling == authoritative IANA tzdata). The check surfaced that on transition-boundary hours (spring-forward gap, fall-back fold, the transition hour itself) Duckling attaches an offset that does NOT match the real per-instant IANA offset (e.g. "3am" on a spring-forward day -> Duckling -05:00, which is actually 4am EDT), whereas the port uses the correct per-instant offset (-04:00). Those 22 boundary cases are intentionally excluded from tz_stress: the port favors timezone correctness (the stated priority) over byte-fidelity to Duckling's DST quirk. Full-corpus oracle cross-check: 981/984 fixtures match live Duckling (99.7%; the 3 diffs are real-zone LMT artifacts of America/Noronha vs the fixed -02:00 test context).

Not attempted (out of scope / disproportionate): the `TimeDatePredicate` field-merge that would let leading-"Fri," combos produce a *full-span* parse (a core-architecture rewrite for zero contains-mode gain). The opaque-Series predicate model is behavior-complete for the corpus as-is.

**Composition fuzz (this iteration).** Beyond the curated corpus, generated
~770 compositional probes (deep nestings, directionals, interval+date, duration
nestings) and cross-checked each against the live oracle. This surfaced three
real divergences the curated corpus missed, all now fixed + guarded by the
`differential_corpus` test (768 oracle-verified cases):
1. `holidayBeta` dropped on open intervals ("after christmas") — the resolver
   returned early on `td.direction` before the holiday-tag attach.
2. A directional time collapsed by intersect ("after 8 in the evening" → plain
   20:00) — `intersect_td` now refuses a directional operand (the open-interval
   wrapper must stay outermost).
3. Trailing date on an interval ("from 3pm to 5pm tomorrow" → the whole interval
   shifts to tomorrow) — the generic interval rules now reject a tod/non-tod
   endpoint mismatch, so a dated endpoint routes through intersect(interval,date).

Three probes are intentionally excluded (expected:null with an in-fixture note),
being Duckling quirks the port deliberately does not replicate: "by tomorrow
morning" (Duckling resolves `to` to *today* noon, ignoring "tomorrow"), and
"between X and Y `<date>`" (Duckling attaches the date only to Y, a 2-day span,
whereas `from X to Y <date>` attaches it to the whole interval — the port applies
it to the whole interval consistently, which is both more correct and matches
Duckling's own `from…to` behavior). Same rationale as the DST-boundary divergence:
the port favors correctness over byte-fidelity to a Duckling bug.

**This/coming-DOW pinning (this iteration).** Composition fuzz (durations,
combined-relative, unusual formats, this/next/last anchors — differential grew to
1223 oracle-verified cases) surfaced "this tuesday at 3" resolving to *today*
(when the reference day is Tuesday) instead of next Tuesday. Root cause: the
"this <dow>" rule returned the bare dow, whose notImmediate lives in the series
and is dropped when composed with a time; Duckling's ruleThisDOW uses
`predNth 0 True` (a single pinned occurrence). Fixed. A follow-up showed "coming"
must *not* pin (Duckling has no "coming <dow>" rule → behaves like the bare dow,
so "coming tuesday at 3" = today), so only "this" pins.

**Weekend ∩ time-of-day (this iteration — FIXED).** "weekend at 3pm" resolved to
today's 3pm instead of Saturday 3pm. Two coupled problems, both now fixed:
(a) the same-day "<part-of-day> at <time-of-day>" am/pm rule grabbed the weekend and
returned a bare tod — the weekend now carries a sentinel `start_hour`
(`WEEKEND_POD_HOUR`) so `is_same_day_part_of_day` excludes it (keeps this/next/last
composition working, which needs the PartOfDay tag); (b) the generic intersect picked
the wrong fine/coarse operand because weekend and the tod were both grain Hour — the
weekend's `TimeData.grain` is now Day, so it is the coarse (iterated) operand and 3pm
is placed within it (→ Sat 3pm). Crucially the resolved interval still reports Hour
grain (it comes from the Fri 18:00 / Mon 00:00 endpoint TimeObjects, not `td.grain`),
so bare "weekend"/"this past weekend"/"last weekend of October" are unchanged. This
avoids the tie-break approach that regressed "3 in the morning". differential 1227/1227.

A 20-min cron loop (job fdd78688) auto-drives further iterations.

Other high-value targets: written-numeral edge cases, relative-duration nestings,
then `unique`-mode field-merge (the 8 structural best-entity artifacts) + real-zone
tz_stress expansion.

## Backlog (rough order)

1. days-of-week + months (no deps) ← current
2. this/next/last <cycle>/<dow> (needs predNth + intersect)
3. minimal Numeral (ints, a/an) + Ordinal (1st–31st)  ← unblocks most combos
4. day-of-month, month-day, year ("in 2014")
5. time-of-day (HH:MM, am/pm, h-notation) + parts of day
6. relative by duration (in N units, N ago) — needs Duration
7. intervals (from–to, between, by/until)
8. absolute date formats (M/D, M/D/Y, ISO)
9. seasons, year AD/BC, quarters
10. holidays table (subagent candidate — large, self-contained)
11. ranking (flip to `unique` mode)
12. real-IANA-zone support → tz_stress green

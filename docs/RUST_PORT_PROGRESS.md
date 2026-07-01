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

## How to run

- All tests: `cd rust && cargo test`
- Corpus only: `cargo test --test corpus`
- Unique mode (Phase 4 bar): `DUCKLING_MATCH=unique cargo test --test corpus`
- Oracle (for new fixtures / cross-checks): `docker start duckling-oracle` then `python3 rust/tools/oracle.py`

## Done

- **Phase 0** — oracle-verified golden fixtures (984 pos / 28 neg, transcribed + cross-checked at 99.7%), 10 DST-stress fixtures, red harness.
- **Phase 1** — core chain: grain/jiff calendar math, TimeObject+intersect, lazy predicate series, saturating engine, fancy-regex, RFC3339+per-instant offset, resolution. Instant rules green.

- **days-of-week + months** — `day_of_week`/`month` predicates (`time_sequence`), table-driven rules, `notImmediate` (today→next), rule-compile cached per thread.

## In progress

Cumulative thru N-dow-from-now. contains **929/984 (94%)**, unique **926/984**, tz_stress **10/10** (timezone/DST fully green — the hard constraint). The holiday subagent completed (176→110); its Islamic/Hindu/Jewish/Orthodox + fixed-date holidays are committed.

Remaining **35** failures (the hard tail; each needs new infra or is a harness artifact):
- **harness-strictness artifacts** (~7, NOT rule gaps): "right now"/"just now" (entity is "now" [substring]), "for a quarter past 3pm"×5 (entity is "a quarter past 3pm"). Duckling's corpus checks the best-entity value; our `full_range_time_values` requires the entity to span the whole input. Could relax the harness to "expected value among any entity" but that risks masking real gaps — leaving as-is.
- **intersect-ordering (rare date ∩ frequent weekday)** (~4): "Thu 15th", "Jul 18, Fri", "Fri, Jul 18, 2014 07:00 PM" (+2 tz/hh variants) — intersecting a yearly date with a weekday iterates the weekday (frequent) and hits SAFE_MAX (10) before finding the match. Duckling merges TimeDatePredicate fields and orders the composition coarsest-first; my port runCompose-intersects and bounds each side at SAFE_MAX. Fix = reorder equal-grain intersect operands by frequency (or raise the outer bound), but it's broad/risky — deferred.
- **"the second of march"** (1): ranking picks "the <cycle> of <time>" (second=grain) over the correct dom(2); model/ranking nuance.
- **datetime combos** (remainder): "Fri, Jul 18, 2014 07:00 PM" (+19h00/19h) — deep multi-level named-day+comma+date+year+time composition (also blocked by intersect-ordering).
- **timezone-tagged intervals** (~3): "9:30 - 11:00 CST" — needs `<datetime>-<datetime> (interval) timezone` with a `hasNoTimezone` guard (my attempt double-applied tz to already-tz'd ends like "15:00 GMT - 18:00 GMT" and regressed 3 — reverted). Requires adding a `has_timezone` flag to TimeData.
- **hh:mm + am/pm roll** (~2): "3:18am"/"3:18a" resolve to today 3:18 not tomorrow — the am/pm fold is only done for pure hours; hh:mm uses the intersect path (today-leak). Needs folding ampm into the minute-composed time (store minute in the form, or a general compose-classification fix).
- **free-form intervals** (~3): "later than 9:30 but before 11:00 on Thursday", "later than 3:30pm but before 6pm", "tomorrow in between 1-2:30 ish".
- **interval ranking** (~2): "July 13 - July 15", "Aug 8 - Aug 12" — a spurious dd/mon/yyyy parse ("13 - July 15"→2015) outscores the correct interval.
- **misc**: "the ides of march", "first week of october 2014", "third tuesday after christmas 2014", "a day from right now", "today in one hour", "after 5 days", "Thursday 9 am (BST)", "2015-03-28 17:00:00/2015-03-29 21:00:00".

Next best targets: intersect-ordering for rare-date ∩ weekday (unblocks ~4 + the datetime combos) — the biggest remaining lever, but needs care to reorder equal-grain intersect operands by frequency without regressing working composes; then the `has_timezone` flag for interval-tz (~3 CST cases). Most remaining need real infra or are harness artifacts (~7).
A 20-min cron loop (job fdd78688) auto-drives further iterations.

Next high-value targets (by remaining count): `<time> <part-of-day>` &
`<time> on <day>` intersect (unlocks many interval+day combos), written numerals
(one..ninety), relative durations (needs Duration dim), holidays (subagent),
numeric M/D/Y dates, then ranking (unique mode) + real-zone tz_stress.

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

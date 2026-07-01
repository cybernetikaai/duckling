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

Cumulative thru ranking. contains **626/984**, unique **622/984** (converged). Model applied. Next: rule-name audit (unmodeled rules score 0 and can win — fixes tz_stress wrong-year + a few contains); then fraction hours.
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

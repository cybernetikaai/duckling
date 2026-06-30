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

- Next: minimal Numeral (ints + written small numbers + a/an) → unblocks bare hours ("4pm"), years ("in 2014"), day-of-month, durations. Then am/pm + noon/midnight rules.

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

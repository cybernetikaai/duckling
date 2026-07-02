# Duckling → Rust (EN Time) — Progress Log

Living status for the autonomous port. Remaining-work analysis:
[`REMAINING_DIMENSIONS.md`](REMAINING_DIMENSIONS.md). Branch: `rust-port-en-time`.

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
| + `values` alternatives array | 1069 / 1069 | 0 | 68 / 68 | **values_array 60**; emit Duckling's `values` (next-occurrence alternatives); last output feature, oracle-verified |
| + values-array covering/ref cases | 1069 / 1069 | 0 | 68 / 68 | **values_array 69**; alternatives correct across day/year references (ongoing holidays, covering hour, passed tod) |
| + numeric-date locale audit | 1069 / 1069 | 0 | 68 / 68 | US M/D/Y convention matches oracle ("3/4/2015"→Mar 4); D/M out-of-range-month rejected; +8 differential, +4 negatives |
| + large-scale randomized diff | 1069 / 1069 | 0 | 68 / 68 | **random_diff 1500**; random inputs × random references (2010–2022) vs oracle; 0 real divergences — definitive completeness pass |
| + EN_GB locale (day-first dates) | 1069 / 1069 | 0 | 68 / 68 | **gb_locale 19**; `parse_locale` — UK "13/12/2013"→Dec 13, "3/4"→Apr 3; ported from EN/GB/Rules.hs; US unchanged |
| + all-region date conventions | 1069 / 1069 | 0 | 68 / 68 | **region_dates 120**; 12 English regions × 3 conventions (month-first/day-first/ZA-hybrid); Locale enum + per-locale rule cache |
| + regional holidays (11 regions) | 1069 / 1069 | 0 | 68 / 68 | **region_holidays 405**; Guy Fawkes/ANZAC/Melbourne Cup/Heritage Day… ported from per-region Rules.hs (subagent), oracle-verified across 3 years |
| + US-region holidays (gap fix) | 1069 / 1069 | 0 | 68 / 68 | **region_holidays 726**; Independence/Memorial/Labor/Columbus Day, Cinco de Mayo, Juneteenth… (~96) were missing from base (Corpus.hs tests "4th of July" the date, not the holiday); now resolve |
| + tractable "other" holidays | 1069 / 1069 | 0 | 68 / 68 | **region_holidays 753**; +9 relative-date holidays: Election Day, Cyber Monday (days-after-nth-DOW) + Victoria Day, Reconciliation Day (nth-DOW-rel-date, predLastOf vs predNthAfter distinguished) |
| + post-2020 holidays (extension) | 1069 / 1069 | 0 | 68 / 68 | **modern_holidays 8**; holidays introduced after Duckling froze (~2020-03): Juneteenth National Independence Day (US name), Indigenous Peoples' Day spelling variants (US), Truth and Reconciliation / Orange Shirt Day / Emancipation Day / National Indigenous Peoples Day (CA). Deliberately diverges from oracle (returns nothing) — see "Deliberate divergence: beyond-Duckling holidays" below |
| + AU Queen's/King's Birthday (extension) | 1069 / 1069 | 0 | 68 / 68 | **modern_holidays 11**; Australia's Queen's Birthday (2nd Mon June, majority-state convention) + King's Birthday post-2022 rename — a major AU public holiday Duckling's AU rules never included (oracle returns nothing). Faithful AU port confirmed complete (all 28 EN/AU/Rules.hs holidays present) |
| + NZ Matariki/King's + IE St Brigid's (extension) | 1069 / 1069 | 0 | 68 / 68 | **modern_holidays 20**; region audit found 3 more post-2020 public holidays the oracle lacks: NZ **Matariki** (2022, legislated Friday date-table 2022–2052), NZ **King's Birthday** (1st Mon June rename), IE **St Brigid's Day** (2023, exact date-table incl. the 1-Feb-Friday exception). Per-case `ref` pins the table years |
| + spoken-form audit → 2 real fixes | 1069 / 1069 | 0 | 68 / 68 | **spoken_forms 53** (ASR idioms vs oracle). Found 2 faithful-port gaps the curated corpus missed: (1) written ordinals were truncated to first..tenth — ported the full `ruleOrdinals` (…twentieth, thirtieth…ninetieth) + `ruleCompositeOrdinals` ("twenty fifth"→25), fixing "the fifteenth of august", "december twenty fifth"; (2) added `<hour> oh <integer>` ("eight oh five am"→8:05). unique 1061/1069 unchanged |
| + spoken-form audit II (breadth) | 1069 / 1069 | 0 | 68 / 68 | **spoken_forms 105**; +52 forms across 2 refs — 24h spoken ("fourteen thirty"), American "of"=to ("ten of three"→2:50), composite ordinals in dates ("march twenty first"), spelled datetimes, this/next part-of-day, week/month relatives. **0 divergences** — pass-1 fixes generalize; port faithfully matches oracle incl. forms Duckling rejects ("fourteen thirty"/"sixteen hundred"/"twenty twenty"-as-year → [] both sides) |
| + spoken-interval audit → 1 real fix | 1069 / 1069 | 0 | 68 / 68 | **spoken_forms 142**; +37 interval/range forms across 2 refs ("nine to five", "monday to friday", "from half past nine to eleven"). Fixed "from now to 5pm": the tod/non-tod endpoint guard wrongly rejected an instant ("now", grain Second) paired with a tod → refined to allow Second-grain instants (trailing-date case "from 3pm to 5pm tomorrow" still routes correctly; differential 768 green) |
| + spoken-duration composition audit | 1069 / 1069 | 0 | 68 / 68 | **spoken_forms 178**; +36 duration/directional compositions across 2 refs ("half an hour before noon", "twenty minutes after three", "an hour and a half ago", "any time after half nine", "three days before christmas", "within the next hour"). **0 divergences** — the original corpus's duration rules already cover this; the spoken/British variants compose correctly. Spoken surface now thoroughly validated |
| + Duration dimension (new output) | 1069 / 1069 | 0 | 68 / 68 | **duration_corpus 83** (all of Duckling/Duration/EN/Corpus.hs + 5 negatives); new `parse_duration` emits `dim:"duration"` JSON ({value,unit,`<unit>`,normalized}); ported the DurationData Semigroup + composite rules ("2 years and 3 months"→27mo, "an hour and 45 minutes"→105min, "1 year 2 days 3 hours and 4 seconds"). **Bonus Time fix:** the composites also fixed "2 hours and 30 minutes from now" (was dropping "2 hours"→05:00, now 07:00). Kept separate from `parse` (Time), so Time ranker untouched |
| + Duration differential vs oracle | 1069 / 1069 | 0 | 68 / 68 | **duration_corpus 135** (+47 oracle-verified cases, +5 genuine negatives); 64-input differential (abbreviations, fractions, composites, colloquial, more/less, precision) → **0 divergences** incl. partial-match parity ("3.5 weeks"→both drop "3." and yield partial "5 weeks", since decimal-durations only apply to hours/minutes). New dimension validated against the live oracle, not just the transcribed corpus |
| + combined Time+Duration (parse_all) | 1069 / 1069 | 0 | 68 / 68 | **combined_dims 28**; new `parse_all` = the `dims:["time","duration"]` surface, ranking both in one pool by dimension-agnostic range domination. **0 divergences** vs oracle: "in 2 hours"→Time (contained Duration dominated), "…20 minutes and wake me at 7am"→Duration+Time (disjoint), "at 3pm for 2 hours"→one Time. `parse` (Time-only) unchanged → Time corpus untouched |
| + multi-entity sentence differential | 1069 / 1069 | 0 | 68 / 68 | **combined_dims 58**; +30 realistic full-sentence utterances (53 entities) — "wake me at 7am and remind me in 2 hours"→2 Time, "take the medicine every 4 hours for 3 days"→2 Duration, "book it from 9 to 5 on monday". **0 divergences** — `parse_all` extracts the exact oracle entity set (dim+span) from multi-mention speech, the realistic product input |
| + prose false-positive parity | 1069 / 1069 | 0 | 68 / 68 | **combined_dims 96**; +38 prose sentences with temporal homographs/idioms ("the second option", "a quarter of the students", "give me a second", "the third quarter earnings", "wait a minute", "i'll take seconds"→[]). **0 divergences** — `parse_all` matches Duckling's dimension-scoped extraction exactly (incl. its no-intent quirks like "the second"→day-of-month), so no spurious-entity drift in ordinary speech |
| + Ordinal dimension (validate port) | 1069 / 1069 | 0 | 68 / 68 | **ordinal_corpus 32** (all of Ordinal/EN/Corpus.hs); new `parse_ordinal` emits `dim:"ordinal"` `{type,value}`. Directly validates the earlier composite-ordinal port against Duckling — "twenty-fifth"/"twenty—fifth"/"twenty fifth"/"twentyfifth"→25, "thirtyfirst"→31, "seventy-third"→73, "ninetieth"→90 — not just indirectly via Time. All pass; trivial emission (rules already ported), no Time-ranker risk |

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
- **region_dates** 120 + **region_holidays** 726 — all 12 English regions via
  `parse_locale(_, _, EnXx)` (`parse` = EnUs). Numeric-date convention (month-first
  US/CA/PH; day-first GB/AU/NZ/IN/IE/BZ/JM/TT; ZA hybrid) + region-specific holidays
  (subagent-extracted to fixtures/region_holidays.json, built by region_holiday_rules
  from four date kinds: month/day, nth (or last) weekday of month, fixed interval,
  easter offset). Cross-checked vs the oracle with locale=en_XX across 2013/2015/2018.
  US-region holidays (Independence/Memorial/Labor/Columbus Day, Cinco de Mayo,
  Juneteenth, …) — ~96 that were absent from the base (Corpus.hs tests "4th of July"
  the date, not "Independence Day" the holiday) — now resolve.
  Six date kinds now supported: month/day, nth (or last) weekday of month, fixed
  interval, easter offset, **days-after-nth-weekday** (Election Day = 1st Tue after
  1st Mon Nov; Cyber Monday = Mon after Thanksgiving; +Native American Heritage,
  Carl Garner, Grandparents, Military Spouse Day), and **nth-weekday-relative-to-a-
  date** (Victoria Day = last Mon on/before May 25; National Patriots' Day;
  Reconciliation Day — predLastOf inclusive vs predNthAfter strict distinguished).
  **Known gap:** ~20 remaining "other" holidays are skipped (verbatim Haskell kept
  in the fixture), all needing bespoke machinery for rarely-spoken holidays:
  long-weekend intervals (Memorial/Labor Day weekend), predNthClosest-to-a-weekday
  (Emancipation/Tax Day), compound offset-of-relative-date (Admin Professionals'
  Day), multi-day week intervals (NAIDOC/EMS Week), calendar-computed
  (Hosay/Hazrat Ali = Islamic), conditional (Royal Queensland Show), and
  fixed single-year (Super Tuesday 2008). Diminishing value; documented follow-up.
- **gb_locale** 19 — EN_GB day-first numeric dates via `parse_locale(_, _, EnGb)`,
  vs the oracle with locale=en_GB. Day-first positives ("13/12/2013"→Dec 13,
  "3/4/2015"→Apr 3) + out-of-range-month rejections. The US default (`parse`) and
  all its tests are unchanged; the two locale rule sets differ only in the numeric-
  date field order.
- **random_diff** 1500 — large-scale randomized differential: random inputs
  (parameterized templates across every rule family) × random references (date +
  time-of-day, 2010–2022), vs the oracle. Varies input AND reference together (all
  other fuzzing varied one at a time). 0 real divergences. NB: the fixture is
  generated against `tz=Etc/GMT+2` (stable −02:00), because `America/Noronha`
  applied historical DST before ~2008 — a resolved date in a DST year would differ
  from the fixed −02:00 test zone (a test artifact, not a port bug).
- **values_array** 69 — the port emits Duckling's `values` array (up to 3 next-
  occurrence alternatives: 3 for recurring predicates, 1 for single/past ones); this
  test cross-checks the full array element-by-element vs the oracle, incl. the 12h
  interleaving ("10:30" → 10:30/22:30/next-day), covering-point ("half past 4" at
  04:30), holiday/interval/season, past-direction cases, and (via per-case
  references) covering intervals across the day/year: ongoing holidays ("Ramadan"
  during it, "Hanukkah" spanning a year boundary), a season during itself, a passed
  time-of-day ("9am" at 15:00 → tomorrow). Computed from a separate predicate run so
  the primary `value` is unchanged.
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

**Unique-mode re-investigation (later iteration).** Diagnosed the 8 `unique` gaps precisely:
- **5× "for a quarter past …"** — the port's best entity IS the correct value ("a quarter past 3pm"); Duckling does not absorb a leading "for" either, so these have no full-span parse *in Duckling*. Forcing one would be a **divergence**, not a fix. Test artifact, correctly left alone.
- **3× "Fri, Jul 18, 2014 07:00 PM/19h00/19h"** — resolved *value* is already correct (the port emits "Jul 18, 2014 07:00 PM" → the right instant, plus a separate "Fri, Jul 18, 2014" day). The only gap is span: a weekday-carrying date won't intersect a **connector-less** trailing time. Note "Fri, Jul 18, 2014 **at** 7pm" and "tuesday feb 18 2014 at 7pm" both parse full-span — the machinery exists; only the bare-time (no "at") three-way merge is missing. Fixing it means touching the Time intersect rules, which risks the 1069/1069 for a span-only gain on 3 inputs whose values are already right — so still deliberately not done. Values are behavior-complete; the span difference is the only residue.

**Duration dimension — now implemented** (was previously deferred). `parse_duration(input)`
emits standalone Duration entities (`dim:"duration"`, `{value, unit, <unit>,
type, normalized:{value,unit}}`) — the full `dims:["duration"]` surface. It is a
*separate* entry point from `parse` (Time), collecting `Token::Duration` nodes and
ranking them among themselves (range domination), so the Time ranker/classifier is
untouched — no cross-dimension ranking, no risk to the 1069/1069 Time corpus. To
complete it, ported the missing `Duration/EN/Rules.hs` pieces: the DurationData
Semigroup (`withGrain` + combine-at-finer-grain) and the three composite rules
(`<int> <grain> <dur>`, `… ,|and <dur>`, `<dur> ,|and <dur>`), plus the
numeral-and-quarter and dot-minutes rules. Passes all of `Duration/EN/Corpus.hs`
(**duration_corpus 83**, incl. 5 negatives). Side benefit: adding the composite
rules to the shared rule set fixed a latent Time bug — "2 hours and 30 minutes
from now" previously dropped "2 hours" (→05:00) and now composes to 150 min
(→07:00), matching "in 2 hours and 30 minutes"; Time corpus stayed green.

**Duration differential (this iteration → 0 divergences).** Validated the new
dimension against the live oracle beyond the transcribed corpus: 64 inputs
spanning abbreviations ("30 sec", "4 hr", "3 yrs"), fractions ("quarter of an
hour", "2.5 hours", "half a year"), composites ("two hours thirty", "a week and
2 days", "3 hours 15 minutes"), colloquial ("a couple of hours"), more/less
("2 more minutes"), and precision ("about 20 minutes"). **0 false-negatives, 0
false-positives** — the port matches the oracle exactly, including on partial-match
fallbacks: "3.5 weeks" has no full-span parse on either side (the decimal-duration
rule only applies to hours/minutes), so both yield the partial "5 weeks" (dropping
"3."). Locked the 47 oracle-verified full-span cases into **duration_corpus**
(83→135 checks). Confirmed coverage limits are genuine Duckling parity (not port
bugs): "5s"/"3wk"/"6 mos" abbreviations and "several weeks" are unsupported by
Duckling too — kept out of the hard-negative list so a future beyond-Duckling
extension isn't blocked.

**Combined Time+Duration output (this iteration → 0 divergences).** Added
`parse_all(input, ctx)` — the `dims:["time","duration"]` surface — which collects
both Time and Duration nodes and ranks them in one pool by *dimension-agnostic*
range domination (the port's existing `rank` already works this way), exactly as
Duckling: the widest match per position wins, disjoint matches all surface. This
is a *new* entry point; `parse` (Time-only) and `parse_duration` are unchanged, so
the 1069/1069 Time corpus is untouched — zero cross-dimension ranking risk to the
validated path. Verified against the oracle (dims=["time","duration"]) on 28 mixed
utterances: "in 2 hours"→Time (the contained "2 hours" Duration is dominated),
"2 hours"→Duration, "at 3pm for 2 hours"→one Time [0,18] (the `<time> for
<duration>` rule spans it, dominating the inner Duration), "set a timer for 20
minutes and wake me at 7am"→Duration[16,26]+Time[39,45] (disjoint, both surface).
0 divergences; locked as **combined_dims 28**.

**Multi-entity sentence differential (this iteration → 0 divergences).** Tested
`parse_all` on the realistic product input: 30 full spoken sentences each carrying
2+ time/duration mentions (53 entities total) — "wake me at 7am and remind me in
2 hours" (2 Time), "set a timer for 20 minutes then another for half an hour" (2
Duration), "take the medicine every 4 hours for 3 days", "book it from 9 to 5 on
monday", "set an alarm for 6:30am and a reminder for 8pm tonight". `parse_all`
returned the *exact* oracle entity set (dim+span) for all 30 — the ranker
correctly surfaces every non-dominated entity across a sentence, not just one.
Merged into **combined_dims** (28→58 cases).

**Prose false-positive parity (this iteration → 0 divergences).** The Duration
dimension was new and hadn't faced the "no spurious entity in ordinary English"
bar that Time has (sentence_stress, dict scan). Tested `parse_all` on 38 prose
sentences where temporal words appear non-temporally: ordinal homographs ("the
second option is better", "he came in third"), grain homographs ("a quarter of
the students", "the third quarter earnings", "minutes of the meeting"), idioms
("give me a second", "wait a minute", "back in the day", "i'll take seconds"), and
legit embedded durations ("a week is a long time", "it lasted an hour"). **0
divergences** — `parse_all` reproduces Duckling's *dimension-scoped* extraction
exactly, including its intent-blind quirks: "the second option" → Time "the
second" (day-of-month 2nd), "the third quarter" → Time (quarter of year), "a
quarter of the students" → Duration "a quarter" (15 min). These aren't port false
positives; they are faithful Duckling behavior (Duckling extracts dimensions, it
does not detect whether a mention is temporal). "i'll take seconds" → [] on both.
Merged into **combined_dims** (58→96).

**Ordinal dimension — emit + validate (this iteration).** The full ordinal port
(first..ninetieth base + composites + digits) landed earlier but had only been
exercised *indirectly* through Time (dates like "the fifteenth of august"). Added
`parse_ordinal(input)` (emits `dim:"ordinal"`, `{type:"value", value:<int>}`) and
validated it directly against `Ordinal/EN/Corpus.hs` — **ordinal_corpus 32**,
all pass. This confirms the composite-ordinal rules against Duckling's own
authoritative corpus, including every separator the corpus exercises: hyphen
("twenty-fifth"), em-dash ("twenty—fifth"), space ("twenty fifth"), concatenated
("twentyfifth"), and digits ("25th") — all →25; likewise 31/42/73/90. Emission
is a new entry point (rules unchanged), so no Time-ranker risk. Note: the standalone
**Numeral** dimension was considered and declined — validating it needs the full
`Numeral/EN/Rules.hs` (K/M/G/lakh suffixes, "1/5" fractions, "point 77"), a large
port outside the time domain with little product value; the numeral forms the time
path needs are already covered.

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

**Deliberate divergence: beyond-Duckling holidays (EXTENSION).**
Duckling's holiday data froze around 2020-03 (last upstream release), so it
misses holidays created/renamed since — and its per-region tables also omit some
long-standing public holidays entirely. Confirmed against the live oracle:
`national day for truth and reconciliation`, `orange shirt day`, `national
indigenous peoples day`, CA `emancipation day`, `juneteenth national
independence day`, no-apostrophe `indigenous peoples day`, and AU `queen's
birthday` / `king's birthday` all return `[]`. Because our AI speech pipeline
sees these names in real user input, `modern_holiday_rules` (en_rules.rs) adds
them as a small, region-scoped, clearly-labeled extension:

- **US** — *Juneteenth National Independence Day* (federal 2021; the June 19 date
  Duckling already had under `juneteenth`, now under its formal name);
  *Indigenous Peoples' Day* spelling variants (plural / no-apostrophe) the single
  Duckling regex missed — same 2nd-Monday-of-October date.
- **CA** — *National Day for Truth and Reconciliation* / *Orange Shirt Day*
  (federal statutory holiday 2021, Sept 30); *Emancipation Day* (federal 2021,
  Aug 1 — distinct from the US April date, so CA-scoped); *National Indigenous
  Peoples Day* (June 21).
- **AU** — *Queen's Birthday* / *King's Birthday* (2nd Monday of June, the
  majority-state convention; King's Birthday is the post-2022 rename after
  Charles III's accession). A major Australian public holiday Duckling's AU
  rules never included. QLD (1st Mon Oct) and WA (governor-set) observe it on
  other dates and are not represented — the same single-date-per-holiday
  limitation as the faithful port (e.g. AU Labour Day). The faithful AU port
  itself is confirmed complete: all 28 holidays in `Duckling/Time/EN/AU/Rules.hs`
  are present (region_holidays), and AU Boxing/Christmas/New Year/Good
  Friday/Easter Monday resolve via the shared global + computed rules.
- **NZ** — *Matariki* (Māori New Year; public holiday since 2022, observed on a
  legislated Friday each year with no weekday rule — backed by an explicit
  2022–2052 date table per the Te Kāhui o Matariki Public Holiday Act 2022) and
  *King's Birthday* (the post-2022 rename of Duckling's faithful NZ *Queen's
  Birthday*, 1st Monday of June — note this differs from AU's 2nd Monday).
- **IE** — *St Brigid's Day* / *Lá Fhéile Bríde* (public holiday since 2023: the
  first Monday of February, except when 1 Feb is a Friday, when it is 1 Feb).
  Backed by an exact date table so the Friday exception (2030/2036/2041/2047) is
  correct rather than approximated by a weekday rule.

Matariki and St Brigid's Day use legislated date tables (like the computed lunar
holidays) via the new `computed_holiday_td(&[(y,m,d)])` helper; a per-case `ref`
in the fixture pins specific table years so the indexing — and St Brigid's
1-Feb-Friday exception — are locked, not just the default 2013-ref first entry.

These intentionally DIVERGE from the oracle — the same posture as the tz
ground-truth and interval divergences (favor correctness over byte-fidelity to a
frozen dataset). They are verified against official government dates rather than
the oracle, kept out of the faithful `region_holiday_rules` port, region-scoped
so they never leak into oracle-based region tests, and guarded by the
**modern_holidays 11** test. Deliberately NOT added (state-specific / discontinued,
low speech-input value): AU Grand Final Day (VIC-only, date set annually) and
Family & Community Day (ACT, discontinued 2017 → replaced by Reconciliation Day,
which we already have); plus the remaining ~20 Duckling "other" holidays.

**Spoken-form audit (this iteration → 2 real fixes).** Prior fuzzing used
written/templated inputs; this pass targeted the product's actual input
distribution — ASR/spoken-English idioms ("half seven", "eight oh five am", "the
fifteenth of august", "december twenty fifth", "quarter to noon", "the day after
tomorrow", "tomorrow at half past nine", …). Curated ~70, cross-checked each
against the live oracle (en_US, ref 2013-02-12 04:30, wall-clock compare after
aligning the reference — the oracle mis-parses `Etc/GMT+2` to +00:00, so both
sides are reduced to offset-free wall-clock from a common 04:30 ref). This
surfaced **two genuine faithful-port gaps the curated corpus never caught**:

1. **Written ordinals were truncated to first..tenth.** `ordinal.rs` had only
   the 10 small words + digit ordinals; Duckling's `ruleOrdinals` covers
   first..twentieth plus thirtieth/fortieth/…/ninetieth, and `ruleCompositeOrdinals`
   covers twenty-first..ninety-ninth (incl. spaced "twenty fifth"). So "the
   fifteenth of august" resolved to Aug 1 (ordinal dropped) and "december twenty
   fifth" to Dec 20. Ported both rules verbatim (unified under Duckling's real
   rule names, which the trained classifier already references) — now Aug 15 /
   Dec 25. Very common in speech; a clear miss.
2. **No `<hour> oh <minute>` rule.** "eight oh five am" → 5:00 (dropped "eight
   oh"). Ported `ruleHONumeralAlt` (`<hour-of-day> (zero|oh|ou) <integer 1-9>`)
   → 8:05. Bare (non-am/pm) "twelve oh three" stays latent → dropped in default
   mode, matching the oracle exactly.

Both verified across contains (1069/1069) and unique (1061/1069 — same 8
structural artifacts as before, no regression). The 53 oracle-resolved spoken
forms are locked as the **spoken_forms** regression test.

**Spoken-form audit II — breadth (this iteration → 0 new bugs).** Broadened the
proven surface: +52 forms not covered in pass 1, run across two references
(04:30 and 15:00, to exercise past/future rollover). Coverage added: 24-hour
spoken times ("fourteen thirty"), American "of"=to ("ten of three"→2:50, "quarter
of five"→4:45), composite ordinals in date positions ("march twenty first", "the
twenty third of march"), fully-spelled datetimes ("march third at three thirty"),
this/next part-of-day ("this morning", "tomorrow night"), and week/month
relatives ("next weekend", "end of the month"). **0 divergences** — the pass-1
ordinal/oh fixes generalize, and the port faithfully matches the oracle even
where Duckling *rejects* a form: "fourteen thirty", "sixteen hundred", and
"twenty twenty"-as-a-year all return [] on both sides (Duckling has no 24h-spoken
or spelled-year-by-juxtaposition rule). **spoken_forms** grew 53→105 cases.

**Spoken-interval audit (this iteration → 1 real fix).** Extended the differential
to spoken intervals/ranges (38 forms × 2 refs): "nine to five", "monday to
friday", "from half past nine to eleven", "between two and four", month ranges.
One real divergence: **"from now to 5pm"** — the port returned an *open* interval
"from now" plus a separate bare "5pm", instead of the closed interval [now, 5pm].
Cause: `tod_endpoint_mismatch` (added earlier to route "from 3pm to 5pm tomorrow"
through intersect) rejected any tod paired with a non-tod, and "now" (grain
Second, no tod form) tripped it. Refined the guard to allow a Second-grain instant
(only "now"/"right now"/"just now" are Second-grain non-tods; a dated tod like
"5pm tomorrow" carries an hour so it is never Second) — "from now to 5pm" now
forms [now, 5pm] while the trailing-date case is unaffected (differential_corpus
768 green, unique 1061/1069 unchanged). **spoken_forms** grew 105→142.

**Spoken-duration composition audit (this iteration → 0 new bugs).** Fourth pass
on the spoken surface, this time duration/directional compositions (42 forms × 2
refs): "half an hour before noon", "twenty minutes after three", "an hour and a
half ago", "any time after half nine" (British), "three days before christmas",
"within the next hour", "two and a half hours from now". **0 divergences** — the
original corpus already exercises the duration rules (`<duration> after/before/
from/past`, `quarter/half/N past-to <hour>`), so the spoken/British-idiom variants
compose correctly on top. Locked into **spoken_forms** (142→178).

**Spoken surface status:** four differential passes (single times, breadth,
intervals, duration compositions) → 3 real fixes (ordinals, `<hour> oh <min>`,
`from now to <time>`), now 178 oracle-verified forms guarded. Diminishing returns
reached — the remaining productive axes are *different in kind*: the Duration
dimension as first-class output (a genuine capability gap for a speech assistant,
noted below as a scope expansion), and ASR-noise robustness (disfluencies, repeats).

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

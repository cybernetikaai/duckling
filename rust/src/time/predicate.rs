//! Time predicates as lazy (past, future) series of TimeObjects.
//!
//! Duckling's predicates return infinite lazy lists; Rust iterators are the
//! natural equivalent. Atomic predicates (cycle_nth, day_of_week, month, ...)
//! are modeled as Series closures; intersection composes them (Phase 3+).

use std::iter::successors;
use std::rc::Rc;

use crate::grain::{Grain, add, lower, round as grain_round};
use crate::time::object::{
    IntervalType, TimeObject, time_before, time_interval, time_intersect, time_plus, time_plus_end,
    time_round, time_starting_at_the_end_of, time_starts_before_end_of,
};
use jiff::civil::DateTime;

const SAFE_MAX: usize = 10;

/// Upper bound on the nth-occurrence index for predNth/predNthAfter. Real queries
/// never exceed a few thousand ("500 fridays from now" is already extreme); a huge
/// index from untrusted input ("10^19 fridays from now") would otherwise make
/// `future.take(n+2)` walk an infinite series forever. Beyond this the predicate
/// yields nothing (the candidate then resolves to nothing useful and is filtered).
const MAX_NTH: u64 = 10_000;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ampm {
    Am,
    Pm,
}

pub type BoxIter = Box<dyn Iterator<Item = TimeObject>>;
pub type SeriesFn = dyn Fn(TimeObject, &TimeContext) -> (BoxIter, BoxIter);

#[derive(Clone, Copy)]
pub struct TimeContext {
    pub ref_time: TimeObject,
    pub min_time: TimeObject,
    pub max_time: TimeObject,
    /// Reference-zone offset (minutes), for shifting in-text timezones into frame.
    pub ref_offset_minutes: i64,
}

#[derive(Clone)]
pub enum Predicate {
    Empty,
    Series(Rc<SeriesFn>),
}

impl Predicate {
    pub fn run(&self, t: TimeObject, ctx: &TimeContext) -> (BoxIter, BoxIter) {
        match self {
            Predicate::Empty => (Box::new(std::iter::empty()), Box::new(std::iter::empty())),
            Predicate::Series(f) => f(t, ctx),
        }
    }
}

/// past = [anchor-step, anchor-2step, ...], future = [anchor, anchor+step, ...]
fn time_sequence(grain: Grain, step: i64, anchor: TimeObject) -> (BoxIter, BoxIter) {
    let fwd = successors(Some(anchor), move |p| Some(time_plus(*p, grain, step)));
    let first_back = time_plus(anchor, grain, -step);
    let back = successors(Some(first_back), move |p| Some(time_plus(*p, grain, -step)));
    (Box::new(back) as BoxIter, Box::new(fwd) as BoxIter)
}

/// The single nth occurrence of grain `g`, anchored at the reference time
/// (port of `cycleNth` = `takeNth n (timeCycle g)`). Yields exactly one element,
/// classified past/future by whether the query time starts before its end.
/// (Returning the whole series would leak neighbours into interval/intersect.)
pub fn cycle_nth(g: Grain, n: i64) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, ctx: &TimeContext| {
        let base = ctx.ref_time;
        let anchor = TimeObject { start: add(grain_round(base.start, g), g, n), grain: g, end: None };
        if time_starts_before_end_of(t, anchor) {
            (Box::new(std::iter::empty()) as BoxIter, Box::new(std::iter::once(anchor)) as BoxIter)
        } else {
            (Box::new(std::iter::once(anchor)) as BoxIter, Box::new(std::iter::empty()) as BoxIter)
        }
    }))
}

fn weekday_mon1(dt: DateTime) -> i64 {
    // ISO: Monday=1 .. Sunday=7
    dt.weekday().to_monday_zero_offset() as i64 + 1
}

/// Day-of-week predicate, n in 1..=7 (Monday=1). Port of runDayOfTheWeekPredicate.
pub fn day_of_week(n: i64) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, _ctx| {
        let dow = weekday_mon1(t.start);
        let days_until = (n - dow).rem_euclid(7);
        let anchor = TimeObject {
            start: add(grain_round(t.start, Grain::Day), Grain::Day, days_until),
            grain: Grain::Day,
            end: None,
        };
        time_sequence(Grain::Day, 7, anchor)
    }))
}

/// Month predicate, n in 1..=12. Port of runMonthPredicate.
pub fn month(n: i64) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, _ctx| {
        let rounded = time_plus(time_round(t, Grain::Year), Grain::Month, n - 1);
        let anchor = if time_starts_before_end_of(t, rounded) {
            rounded
        } else {
            time_plus(rounded, Grain::Year, 1)
        };
        time_sequence(Grain::Year, 1, anchor)
    }))
}

/// Hour-of-day predicate (port of runHourPredicate). is12h + optional am/pm.
pub fn hour(is12h: bool, ampm: Option<Ampm>, n: i64) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, _ctx| {
        let h = t.start.hour() as i64;
        let step: i64 = if is12h && n <= 12 && ampm.is_none() { 12 } else { 24 };
        let n2 = match ampm {
            Some(Ampm::Am) => n.rem_euclid(12),
            Some(Ampm::Pm) => n.rem_euclid(12) + 12,
            None => n,
        };
        let rounded = time_round(t, Grain::Hour);
        let anchor = time_plus(rounded, Grain::Hour, (n2 - h).rem_euclid(step));
        time_sequence(Grain::Hour, step, anchor)
    }))
}

/// Minute predicate (port of runMinutePredicate).
pub fn minute(n: i64) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, _ctx| {
        let m = t.start.minute() as i64;
        let rounded = time_round(t, Grain::Minute);
        let anchor = time_plus(rounded, Grain::Minute, (n - m).rem_euclid(60));
        time_sequence(Grain::Hour, 1, anchor)
    }))
}

/// Second predicate (port of runSecondPredicate, integer seconds).
pub fn second(n: i64) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, _ctx| {
        let s = t.start.second() as i64;
        let rounded = time_round(t, Grain::Second);
        let anchor = time_plus(rounded, Grain::Second, (n - s).rem_euclid(60));
        time_sequence(Grain::Minute, 1, anchor)
    }))
}

/// Intersection via runCompose (Types.hs:623). `fine` must be the smaller grain.
/// Stays lazy: take_while is bounded by `.take(SAFE_MAX)` so infinite series
/// (e.g. hourly across ±2000y) are never materialized.
pub fn intersect(fine: Predicate, coarse: Predicate) -> Predicate {
    Predicate::Series(Rc::new(move |now: TimeObject, ctx: &TimeContext| {
        let (past2, future2) = coarse.run(now, ctx);
        let min_t = ctx.min_time;
        let max_t = ctx.max_time;
        let ref_off = ctx.ref_offset_minutes;
        let ref_t = ctx.ref_time;
        let f_back = fine.clone();
        let f_fwd = fine.clone();
        let back: Vec<TimeObject> = past2
            .take_while(move |t| time_starts_before_end_of(min_t, *t))
            .take(SAFE_MAX)
            .flat_map(move |time1| compose_one(&f_back, time1, ref_t, ref_off))
            .collect();
        let fwd: Vec<TimeObject> = future2
            .take_while(move |t| time_starts_before_end_of(*t, max_t))
            .take(SAFE_MAX)
            .flat_map(move |time1| compose_one(&f_fwd, time1, ref_t, ref_off))
            .collect();
        (Box::new(back.into_iter()) as BoxIter, Box::new(fwd.into_iter()) as BoxIter)
    }))
}

fn compose_one(fine: &Predicate, time1: TimeObject, ref_time: TimeObject, ref_off: i64) -> Vec<TimeObject> {
    // Duckling's fixedRange keeps the original reference time and only clamps
    // min/max to the segment. Preds that anchor at `now` (dow/month/dom/hour)
    // use `time1`; preds anchored at the reference (in_duration, cycle_nth) must
    // still use the real ref — else "today in one hour" would compute
    // now-relative to midnight of today (01:00) instead of the true now (05:30).
    let fixed = TimeContext {
        ref_time,
        min_time: time1,
        max_time: time1,
        ref_offset_minutes: ref_off,
    };
    let (_p, f) = fine.run(time1, &fixed);
    f.take_while(move |this| time_starts_before_end_of(*this, time1))
        .filter_map(move |t| time_intersect(t, time1))
        .collect()
}

pub fn hour_minute(is12h: bool, h: i64, m: i64) -> Predicate {
    intersect(minute(m), hour(is12h, None, h))
}

pub fn hour_minute_second(is12h: bool, h: i64, m: i64, s: i64) -> Predicate {
    intersect(second(s), intersect(minute(m), hour(is12h, None, h)))
}

/// Year predicate (port of runYearPredicate): a single occurrence, in past or
/// future depending on whether the reference year is before/after n.
pub fn year(n: i64) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, _ctx| {
        let tyear = t.start.year() as i64;
        let y = TimeObject {
            start: add(grain_round(t.start, Grain::Year), Grain::Year, n - tyear),
            grain: Grain::Year,
            end: None,
        };
        if tyear <= n {
            (Box::new(std::iter::empty()) as BoxIter, Box::new(std::iter::once(y)) as BoxIter)
        } else {
            (Box::new(std::iter::once(y)) as BoxIter, Box::new(std::iter::empty()) as BoxIter)
        }
    }))
}

/// Day-of-month predicate (port of runDayOfTheMonthPredicate). Skips months
/// that don't have enough days (e.g. the 31st in February). Stays lazy.
pub fn day_of_month(n: i64) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, _ctx| {
        // No month has >31 days; an out-of-range n would make `enough_days`
        // reject forever (infinite filter). Yield nothing instead.
        if !(1..=31).contains(&n) {
            return (Box::new(std::iter::empty()) as BoxIter, Box::new(std::iter::empty()) as BoxIter);
        }
        let rounded = time_round(t, Grain::Month);
        let dom = t.start.day() as i64;
        let anchor = if dom <= n { rounded } else { time_plus(rounded, Grain::Month, 1) };
        let fwd = successors(Some(anchor), |p| Some(time_plus(*p, Grain::Month, 1)))
            .filter(move |to: &TimeObject| n <= to.start.date().days_in_month() as i64)
            .map(move |to| time_plus(to, Grain::Day, n - 1));
        let prev = time_plus(anchor, Grain::Month, -1);
        let past = successors(Some(prev), |p| Some(time_plus(*p, Grain::Month, -1)))
            .filter(move |to: &TimeObject| n <= to.start.date().days_in_month() as i64)
            .map(move |to| time_plus(to, Grain::Day, n - 1));
        (Box::new(past) as BoxIter, Box::new(fwd) as BoxIter)
    }))
}

/// AM/PM as a 12h interval per day (port of runAMPMPredicate, sans the
/// maybe-shrink-first refinement, which only matters when "now" is inside the
/// interval — add if a corpus case needs it).
pub fn ampm_predicate(is_am: bool) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, _ctx| {
        let n = if is_am { 0 } else { 12 };
        let rounded = time_round(t, Grain::Day);
        let anchor_start = time_plus(rounded, Grain::Hour, n);
        let anchor_end = time_plus(anchor_start, Grain::Hour, 12);
        let anchor = time_interval(IntervalType::Open, anchor_start, anchor_end);
        let fwd = successors(Some(anchor), move |p| Some(time_plus_end(*p, Grain::Hour, 24)));
        let prev = time_plus_end(anchor, Grain::Hour, -24);
        let past = successors(Some(prev), move |p| Some(time_plus_end(*p, Grain::Hour, -24)));
        (Box::new(past) as BoxIter, Box::new(fwd) as BoxIter)
    }))
}

/// "in <duration>" / "<duration> ago" (negative value): round now to
/// lower(grain), then shift by the duration (port of inDuration/shiftDuration).
/// Single occurrence, grain = lower(duration grain).
pub fn in_duration(value: i64, grain: Grain) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, ctx: &TimeContext| {
        let lg = lower(grain);
        let start = add(grain_round(ctx.ref_time.start, lg), grain, value);
        let obj = TimeObject { start, grain: lg, end: None };
        if time_starts_before_end_of(t, obj) {
            (Box::new(std::iter::empty()) as BoxIter, Box::new(std::iter::once(obj)) as BoxIter)
        } else {
            (Box::new(std::iter::once(obj)) as BoxIter, Box::new(std::iter::empty()) as BoxIter)
        }
    }))
}

/// Predicate from an explicit, chronologically-sorted list of dates (port of
/// timeComputed). Used for computed/lunar holidays (Easter, Diwali, …).
pub fn time_computed(dates: Vec<TimeObject>) -> Predicate {
    let dates = std::rc::Rc::new(dates);
    Predicate::Series(Rc::new(move |t: TimeObject, _ctx| {
        let idx = dates.partition_point(|d| time_before(*d, t));
        let mut past: Vec<TimeObject> = dates[..idx].to_vec();
        past.reverse();
        let future: Vec<TimeObject> = dates[idx..].to_vec();
        (Box::new(past.into_iter()) as BoxIter, Box::new(future.into_iter()) as BoxIter)
    }))
}

/// Shift a time-of-day predicate from a named timezone into the reference frame
/// (port of shiftTimezone): each occurrence moves by (ref_offset - provided)
/// minutes. e.g. "8:00 PST" under a -02:00 reference: shift = -120-(-480) = +360.
pub fn shift_timezone(provided_minutes: i64, inner: Predicate) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, ctx: &TimeContext| {
        let shift = ctx.ref_offset_minutes - provided_minutes;
        let (past, future) = inner.run(t, ctx);
        // Duckling's shiftTimezone is `timePlus x Minute`, which floors grain to
        // min(grain, Minute) — a timezone-shifted hour reports minute grain.
        let g = |o: Grain| o.min(Grain::Minute);
        (
            Box::new(past.map(move |o| TimeObject {
                start: add(o.start, Grain::Minute, shift),
                grain: g(o.grain),
                end: o.end.map(|e| add(e, Grain::Minute, shift)),
            })) as BoxIter,
            Box::new(future.map(move |o| TimeObject {
                start: add(o.start, Grain::Minute, shift),
                grain: g(o.grain),
                end: o.end.map(|e| add(e, Grain::Minute, shift)),
            })) as BoxIter,
        )
    }))
}

/// Floor each occurrence's grain to `min(grain, Minute)` without moving the
/// instant. Used so a tz-shifted interval's exclusive end is computed at minute
/// grain (endpoint + 1 minute) rather than +1 hour: "9am to 5pm GMT" ends at
/// 5pm-GMT + 1min, not the hour-exclusive 6pm shifted.
pub fn floor_grain_to_minute(inner: Predicate) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, ctx: &TimeContext| {
        let (past, future) = inner.run(t, ctx);
        let m = |o: TimeObject| TimeObject { grain: o.grain.min(Grain::Minute), ..o };
        (Box::new(past.map(m)) as BoxIter, Box::new(future.map(m)) as BoxIter)
    }))
}

/// The raw cycle of `grain` (all occurrences), rounded to `grain` at each query
/// time. Used as the cyclic predicate for cycleNthAfter (e.g. quarters of a year).
pub fn time_cycle(grain: Grain) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, _ctx| {
        time_sequence(grain, 1, time_round(t, grain))
    }))
}

/// An interval spanning n grain-cycles from the reference (port of cycleN/takeN).
/// n>=0: [start, end) over the next n cycles (skipping the current if
/// not_immediate); n<0: the last |n| cycles, ending at the current cycle.
pub fn cycle_n(not_immediate: bool, grain: Grain, n: i64) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, ctx: &TimeContext| {
        let base = ctx.ref_time;
        let anchor = time_round(base, grain);
        let (past, future) = time_sequence(grain, 1, anchor);
        let slot: Option<TimeObject> = if n >= 0 {
            let mut fut: Vec<TimeObject> = future.take(n as usize + 2).collect();
            if not_immediate
                && fut.first().is_some_and(|a| time_intersect(*a, base).is_some())
                && !fut.is_empty()
            {
                fut.remove(0);
            }
            match (fut.first(), fut.get(n as usize)) {
                (Some(&start), Some(&end)) => Some(time_interval(IntervalType::Open, start, end)),
                _ => None,
            }
        } else {
            let p: Vec<TimeObject> = past.take((-n) as usize + 1).collect();
            match (p.get(((-n) - 1) as usize), p.first()) {
                (Some(&start), Some(&end)) => Some(time_interval(IntervalType::Closed, start, end)),
                _ => None,
            }
        };
        match slot {
            Some(nth) if time_starts_before_end_of(t, nth) => {
                (Box::new(std::iter::empty()) as BoxIter, Box::new(std::iter::once(nth)) as BoxIter)
            }
            Some(nth) => {
                (Box::new(std::iter::once(nth)) as BoxIter, Box::new(std::iter::empty()) as BoxIter)
            }
            None => (Box::new(std::iter::empty()) as BoxIter, Box::new(std::iter::empty()) as BoxIter),
        }
    }))
}

const SAFE_MAX_INTERVAL: usize = 12;

/// Generic timeSeqMap: apply `f` to each occurrence of `g` (bounded), then
/// re-bucket into (past, future) around `now`. `dont_reverse` mirrors the
/// Haskell flag (intervals keep order; nth/last reverse).
fn seq_map<F>(
    dont_reverse: bool,
    f: F,
    g: &Predicate,
    now: TimeObject,
    ctx: &TimeContext,
) -> (Vec<TimeObject>, Vec<TimeObject>)
where
    F: Fn(TimeObject) -> Option<TimeObject>,
{
    let (g_past, g_future) = g.run(now, ctx);
    let past1: Vec<TimeObject> = g_past.take(SAFE_MAX_INTERVAL).filter_map(&f).collect();
    let future1: Vec<TimeObject> = g_future.take(SAFE_MAX_INTERVAL).filter_map(f).collect();

    let ends_after_now = |x: &TimeObject| time_starts_before_end_of(now, *x);

    let sp = past1.iter().position(|x| !ends_after_now(x)).unwrap_or(past1.len());
    let new_future = past1[..sp].to_vec();
    let old_past: Vec<TimeObject> = past1[sp..]
        .iter()
        .cloned()
        .take_while(|x| time_starts_before_end_of(ctx.min_time, *x))
        .collect();

    let bp = future1.iter().position(ends_after_now).unwrap_or(future1.len());
    let new_past = future1[..bp].to_vec();
    let old_future: Vec<TimeObject> = future1[bp..]
        .iter()
        .cloned()
        .take_while(|x| time_starts_before_end_of(*x, ctx.max_time))
        .collect();

    let rev = |mut v: Vec<TimeObject>| {
        if !dont_reverse {
            v.reverse();
        }
        v
    };
    let past = rev(new_past).into_iter().chain(old_past).collect();
    let future = rev(new_future).into_iter().chain(old_future).collect();
    (past, future)
}

/// Interval predicate (runTimeIntervalsPredicate): each pred1 occurrence runs
/// to pred2's first occurrence after it.
pub fn time_intervals(kind: IntervalType, pred1: Predicate, pred2: Predicate) -> Predicate {
    Predicate::Series(Rc::new(move |now: TimeObject, ctx: &TimeContext| {
        let f = |segment: TimeObject| -> Option<TimeObject> {
            let (_p, mut fut2) = pred2.run(segment, ctx);
            fut2.next().map(|first_future| time_interval(kind, segment, first_future))
        };
        let (past, future) = seq_map(true, f, &pred1, now, ctx);
        (Box::new(past.into_iter()) as BoxIter, Box::new(future.into_iter()) as BoxIter)
    }))
}

/// nth occurrence of `cyclic` within/after each `base` occurrence (takeNthAfter).
/// e.g. predNthAfter(3, Thursday, November) = the 4th Thursday = Thanksgiving.
pub fn take_nth_after(n: i64, not_immediate: bool, cyclic: Predicate, base: Predicate) -> Predicate {
    Predicate::Series(Rc::new(move |now: TimeObject, ctx: &TimeContext| {
        if n.unsigned_abs() > MAX_NTH {
            return (Box::new(std::iter::empty()) as BoxIter, Box::new(std::iter::empty()) as BoxIter);
        }
        let f = |t: TimeObject| -> Option<TimeObject> {
            let (mut past, future) = cyclic.run(t, ctx);
            if n >= 0 {
                let fut: Vec<TimeObject> = future.take((n as usize) + 2).collect();
                let drop_n = if not_immediate && fut.first().is_some_and(|a| time_before(*a, t)) {
                    (n as usize) + 1
                } else {
                    n as usize
                };
                fut.into_iter().nth(drop_n)
            } else {
                past.nth(((-n) - 1) as usize)
            }
        };
        let (past, future) = seq_map(false, f, &base, now, ctx);
        (Box::new(past.into_iter()) as BoxIter, Box::new(future.into_iter()) as BoxIter)
    }))
}

/// nth occurrence of `pred` relative to the reference time (port of takeNth).
/// n>=0 picks from the future (skipping the current one if not_immediate and it
/// covers now); n<0 picks from the past. Used by predNth for this/next/last.
// Season boundaries (Spring, Summer, Fall, Winter), ports of seasonStart.
const SEASON_STARTS: [(i8, i8); 4] = [(3, 20), (6, 21), (9, 23), (12, 21)];

fn season_start_date(y: i16, idx: usize) -> jiff::civil::Date {
    let (m, d) = SEASON_STARTS[idx];
    jiff::civil::date(y, m, d)
}
fn next_season_idx(y: i16, idx: usize) -> (i16, usize) {
    if idx == 3 { (y + 1, 0) } else { (y, idx + 1) }
}
fn prev_season_idx(y: i16, idx: usize) -> (i16, usize) {
    if idx == 0 { (y - 1, 3) } else { (y, idx - 1) }
}
/// The season containing `day` (port of seasonOf): the latest season whose
/// start is on/before `day`, falling back to the previous year's Winter.
fn season_of(day: jiff::civil::Date) -> (i16, usize) {
    let y = day.year();
    for idx in [3usize, 2, 1, 0] {
        if season_start_date(y, idx) <= day {
            return (y, idx);
        }
    }
    (y - 1, 3)
}
fn season_obj(s: (i16, usize)) -> TimeObject {
    let (y, idx) = s;
    let start = season_start_date(y, idx);
    let (ny, nidx) = next_season_idx(y, idx);
    let end = season_start_date(ny, nidx).yesterday().unwrap();
    TimeObject { start: start.at(0, 0, 0, 0), grain: Grain::Day, end: Some(end.at(0, 0, 0, 0)) }
}

/// Cycle through the four astronomical seasons (port of seasonPredicate).
/// future starts at the current season; past runs backwards from the previous.
pub fn season_series() -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, _ctx| {
        let cur = season_of(t.start.date());
        let fwd = successors(Some(cur), |&(y, i)| Some(next_season_idx(y, i))).map(season_obj);
        let back = successors(Some(prev_season_idx(cur.0, cur.1)), |&(y, i)| {
            Some(prev_season_idx(y, i))
        })
        .map(season_obj);
        (Box::new(back) as BoxIter, Box::new(fwd) as BoxIter)
    }))
}

/// The n-th closest occurrence of `cyclic` to each occurrence of `base`
/// (port of takeNthClosest): merge the past/future cyclic occurrences by
/// absolute distance, ties going to the future.
pub fn take_nth_closest(n: i64, cyclic: Predicate, base: Predicate) -> Predicate {
    Predicate::Series(Rc::new(move |now: TimeObject, ctx: &TimeContext| {
        let f = |t: TimeObject| -> Option<TimeObject> {
            let (past, future) = cyclic.run(t, ctx);
            let pa: Vec<TimeObject> = past.take(SAFE_MAX).collect();
            let fu: Vec<TimeObject> = future.take(SAFE_MAX).collect();
            let dist = |o: &TimeObject| t.start.duration_until(o.start).abs();
            let (mut pi, mut fi) = (0usize, 0usize);
            let mut res = None;
            for _ in 0..=n.max(0) {
                match (pa.get(pi), fu.get(fi)) {
                    (None, None) => break,
                    (Some(x), None) => {
                        res = Some(*x);
                        pi += 1;
                    }
                    (None, Some(y)) => {
                        res = Some(*y);
                        fi += 1;
                    }
                    (Some(x), Some(y)) => {
                        if dist(x) < dist(y) {
                            res = Some(*x);
                            pi += 1;
                        } else {
                            res = Some(*y);
                            fi += 1;
                        }
                    }
                }
            }
            res
        };
        let (past, future) = seq_map(false, f, &base, now, ctx);
        (Box::new(past.into_iter()) as BoxIter, Box::new(future.into_iter()) as BoxIter)
    }))
}

/// Shift each occurrence of `base` forward by a duration, rounding the anchor
/// to `min(occurrence grain, duration grain)` first (port of mergeDuration).
pub fn merge_duration(base: Predicate, value: i64, grain: Grain) -> Predicate {
    Predicate::Series(Rc::new(move |now: TimeObject, ctx: &TimeContext| {
        let f = move |x: TimeObject| -> Option<TimeObject> {
            let gp = x.grain.min(grain);
            let t2 = if gp == x.grain { x } else { time_round(x, gp) };
            Some(time_plus(t2, grain, value))
        };
        let (past, future) = seq_map(false, f, &base, now, ctx);
        (Box::new(past.into_iter()) as BoxIter, Box::new(future.into_iter()) as BoxIter)
    }))
}

/// Shift each occurrence of `base` forward by a duration, rounding the anchor
/// to the grain just below the duration's (port of shiftDuration, NoGrain case).
pub fn shift_duration(base: Predicate, value: i64, grain: Grain) -> Predicate {
    Predicate::Series(Rc::new(move |now: TimeObject, ctx: &TimeContext| {
        let f = move |x: TimeObject| -> Option<TimeObject> {
            Some(time_plus(time_round(x, lower(grain)), grain, value))
        };
        let (past, future) = seq_map(false, f, &base, now, ctx);
        (Box::new(past.into_iter()) as BoxIter, Box::new(future.into_iter()) as BoxIter)
    }))
}

pub fn take_nth(n: i64, not_immediate: bool, pred: Predicate) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, ctx: &TimeContext| {
        if n.unsigned_abs() > MAX_NTH {
            return (Box::new(std::iter::empty()) as BoxIter, Box::new(std::iter::empty()) as BoxIter);
        }
        let base = ctx.ref_time;
        let (mut past, future) = pred.run(base, ctx);
        let nth = if n >= 0 {
            let fut: Vec<TimeObject> = future.take((n as usize) + 2).collect();
            let drop_n = if not_immediate
                && fut.first().is_some_and(|a| time_intersect(*a, base).is_some())
            {
                (n as usize) + 1
            } else {
                n as usize
            };
            fut.into_iter().nth(drop_n)
        } else {
            past.nth(((-n) - 1) as usize)
        };
        match nth {
            None => (Box::new(std::iter::empty()) as BoxIter, Box::new(std::iter::empty()) as BoxIter),
            Some(o) if time_starts_before_end_of(t, o) => {
                (Box::new(std::iter::empty()) as BoxIter, Box::new(std::iter::once(o)) as BoxIter)
            }
            Some(o) => {
                (Box::new(std::iter::once(o)) as BoxIter, Box::new(std::iter::empty()) as BoxIter)
            }
        }
    }))
}

/// last occurrence of `cyclic` within each `base` occurrence (takeLastOf).
/// e.g. last Monday of May = Memorial Day.
pub fn take_last_of(cyclic: Predicate, base: Predicate) -> Predicate {
    Predicate::Series(Rc::new(move |now: TimeObject, ctx: &TimeContext| {
        let f = |t: TimeObject| -> Option<TimeObject> {
            let (mut past, _future) = cyclic.run(time_starting_at_the_end_of(t), ctx);
            past.next()
        };
        let (past, future) = seq_map(false, f, &base, now, ctx);
        (Box::new(past.into_iter()) as BoxIter, Box::new(future.into_iter()) as BoxIter)
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;
    fn tctx(y: i16, mo: i8, da: i8, h: i8, mi: i8, s: i8) -> TimeContext {
        let now = TimeObject { start: date(y, mo, da).at(h, mi, s, 0), grain: Grain::Second, end: None };
        TimeContext { ref_time: now, min_time: now, max_time: now, ref_offset_minutes: -120 }
    }
    /// Resolution-style pick: first future occurrence, else first past.
    fn future_head(p: &Predicate, ctx: &TimeContext) -> TimeObject {
        let (mut past, mut future) = p.run(ctx.ref_time, ctx);
        future.next().or_else(|| past.next()).unwrap()
    }
    #[test]
    fn cycle_nth_instants() {
        let ctx = tctx(2013, 2, 12, 4, 30, 0);
        assert_eq!(future_head(&cycle_nth(Grain::Day, 0), &ctx).start, date(2013, 2, 12).at(0, 0, 0, 0));
        assert_eq!(future_head(&cycle_nth(Grain::Day, 1), &ctx).start, date(2013, 2, 13).at(0, 0, 0, 0));
        assert_eq!(future_head(&cycle_nth(Grain::Day, -1), &ctx).start, date(2013, 2, 11).at(0, 0, 0, 0));
        assert_eq!(future_head(&cycle_nth(Grain::Second, 0), &ctx).start, date(2013, 2, 12).at(4, 30, 0, 0));
    }
    #[test]
    fn day_of_week_picks_next() {
        let ctx = tctx(2013, 2, 12, 4, 30, 0); // Tuesday
        // next Monday after Tue Feb 12 is Feb 18
        assert_eq!(future_head(&day_of_week(1), &ctx).start, date(2013, 2, 18).at(0, 0, 0, 0));
    }
    #[test]
    fn month_picks_current_february() {
        let ctx = tctx(2013, 2, 12, 4, 30, 0);
        let m = future_head(&month(2), &ctx);
        assert_eq!(m.start, date(2013, 2, 1).at(0, 0, 0, 0));
        assert_eq!(m.grain, Grain::Month);
    }
    #[test]
    fn hour_minute_composes() {
        let now = TimeObject { start: date(2013, 2, 12).at(4, 30, 0, 0), grain: Grain::Second, end: None };
        let ctx = TimeContext {
            ref_time: now,
            min_time: TimeObject { start: date(2011, 2, 12).at(4, 30, 0, 0), ..now },
            max_time: TimeObject { start: date(2015, 2, 12).at(4, 30, 0, 0), ..now },
            ref_offset_minutes: -120,
        };
        let h = future_head(&hour_minute(true, 4, 23), &ctx);
        assert_eq!(h.start, date(2013, 2, 12).at(4, 23, 0, 0));
        assert_eq!(h.grain, Grain::Minute);
    }
}

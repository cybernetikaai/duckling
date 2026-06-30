//! Time predicates as lazy (past, future) series of TimeObjects.
//!
//! Duckling's predicates return infinite lazy lists; Rust iterators are the
//! natural equivalent. Atomic predicates (cycle_nth, day_of_week, month, ...)
//! are modeled as Series closures; intersection composes them (Phase 3+).

use std::iter::successors;
use std::rc::Rc;

use crate::grain::{Grain, add, round as grain_round};
use crate::time::object::{
    IntervalType, TimeObject, time_interval, time_intersect, time_plus, time_plus_end, time_round,
    time_starts_before_end_of,
};
use jiff::civil::DateTime;

const SAFE_MAX: usize = 10;

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

/// The series of all occurrences of grain `g`, shifted so the future head is
/// `round(now, g) + n*g`. Resolution picks the future head (else past head).
pub fn cycle_nth(g: Grain, n: i64) -> Predicate {
    Predicate::Series(Rc::new(move |t: TimeObject, _ctx: &TimeContext| {
        let anchor = {
            let r = time_round(t, g);
            TimeObject { start: add(r.start, g, n), grain: g, end: None }
        };
        let future = successors(Some(anchor), move |p| {
            Some(TimeObject { start: add(p.start, g, 1), grain: g, end: None })
        });
        let prev = TimeObject { start: add(anchor.start, g, -1), grain: g, end: None };
        let past = successors(Some(prev), move |p| {
            Some(TimeObject { start: add(p.start, g, -1), grain: g, end: None })
        });
        (Box::new(past) as BoxIter, Box::new(future) as BoxIter)
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
        let f_back = fine.clone();
        let f_fwd = fine.clone();
        let back: Vec<TimeObject> = past2
            .take_while(move |t| time_starts_before_end_of(min_t, *t))
            .take(SAFE_MAX)
            .flat_map(move |time1| compose_one(&f_back, time1))
            .collect();
        let fwd: Vec<TimeObject> = future2
            .take_while(move |t| time_starts_before_end_of(*t, max_t))
            .take(SAFE_MAX)
            .flat_map(move |time1| compose_one(&f_fwd, time1))
            .collect();
        (Box::new(back.into_iter()) as BoxIter, Box::new(fwd.into_iter()) as BoxIter)
    }))
}

fn compose_one(fine: &Predicate, time1: TimeObject) -> Vec<TimeObject> {
    let fixed = TimeContext { ref_time: time1, min_time: time1, max_time: time1 };
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

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;
    fn tctx(y: i16, mo: i8, da: i8, h: i8, mi: i8, s: i8) -> TimeContext {
        let now = TimeObject { start: date(y, mo, da).at(h, mi, s, 0), grain: Grain::Second, end: None };
        TimeContext { ref_time: now, min_time: now, max_time: now }
    }
    fn future_head(p: &Predicate, ctx: &TimeContext) -> TimeObject {
        p.run(ctx.ref_time, ctx).1.next().unwrap()
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
        };
        let h = future_head(&hour_minute(true, 4, 23), &ctx);
        assert_eq!(h.start, date(2013, 2, 12).at(4, 23, 0, 0));
        assert_eq!(h.grain, Grain::Minute);
    }
}

//! Time predicates as lazy (past, future) series of TimeObjects.
//!
//! Duckling's predicates return infinite lazy lists; Rust iterators are the
//! natural equivalent. Atomic predicates (cycle_nth, day_of_week, month, ...)
//! are modeled as Series closures; intersection composes them (Phase 3+).

use std::iter::successors;
use std::rc::Rc;

use crate::grain::{Grain, add, round as grain_round};
use crate::time::object::{TimeObject, time_plus, time_round, time_starts_before_end_of};
use jiff::civil::DateTime;

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
}

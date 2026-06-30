//! Time predicates as lazy (past, future) series of TimeObjects.
//!
//! Duckling's predicates return infinite lazy lists; Rust iterators are the
//! natural equivalent. `cycle_nth` is the seed for now/today/tomorrow/...;
//! the TimeDate/Intersect/Intervals variants arrive in Phase 3.

use std::iter::successors;
use std::rc::Rc;

use crate::grain::{Grain, add};
use crate::time::object::{TimeObject, time_round};

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
}

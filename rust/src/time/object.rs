//! TimeObject + interval/intersect helpers (port of Duckling/Time/Types.hs:806-872).

use crate::grain::{Grain, add, round as grain_round};
use jiff::civil::DateTime;

#[derive(Clone, Copy, PartialEq, Debug)]
pub struct TimeObject {
    pub start: DateTime,
    pub grain: Grain,
    pub end: Option<DateTime>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IntervalType {
    Open,
    Closed,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IntervalDirection {
    Before,
    After,
}

pub fn time_round(t: TimeObject, g: Grain) -> TimeObject {
    TimeObject { start: grain_round(t.start, g), grain: g, end: None }
}

pub fn time_plus(t: TimeObject, g: Grain, n: i64) -> TimeObject {
    TimeObject { start: add(t.start, g, n), grain: t.grain.min(g), end: None }
}

/// Shift the whole interval (start and end) by n*g; preserves length.
pub fn time_plus_end(t: TimeObject, g: Grain, n: i64) -> TimeObject {
    TimeObject {
        start: add(t.start, g, n),
        grain: t.grain.min(g),
        end: t.end.map(|e| add(e, g, n)),
    }
}

pub fn time_end(t: TimeObject) -> DateTime {
    t.end.unwrap_or_else(|| add(t.start, t.grain, 1))
}

pub fn time_before(a: TimeObject, b: TimeObject) -> bool {
    a.start < b.start
}

pub fn time_starts_before_end_of(a: TimeObject, b: TimeObject) -> bool {
    a.start < time_end(b)
}

pub fn time_interval(kind: IntervalType, a: TimeObject, b: TimeObject) -> TimeObject {
    let g = a.grain.min(b.grain);
    let g2 = if a.grain < Grain::Day && b.grain < Grain::Day { g } else { b.grain };
    let end = match kind {
        IntervalType::Open => b.start,
        IntervalType::Closed => b.end.unwrap_or_else(|| add(b.start, g2, 1)),
    };
    TimeObject { start: a.start, grain: g, end: Some(end) }
}

/// Intersection of two TimeObjects; resulting grain/end are the smallest.
pub fn time_intersect(a: TimeObject, b: TimeObject) -> Option<TimeObject> {
    let (t1, t2) = if a.start > b.start { (b, a) } else { (a, b) };
    let e1 = time_end(t1);
    let e2 = time_end(t2);
    let g = t1.grain.min(t2.grain);
    if e1 <= t2.start {
        None
    } else if e1 < e2 || (t1.start == t2.start && e1 == e2 && t1.end.is_some()) {
        Some(TimeObject { start: t2.start, end: t1.end, grain: g })
    } else {
        Some(TimeObject { grain: g, ..t2 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jiff::civil::date;
    fn obj(y: i16, mo: i8, da: i8, h: i8, mi: i8, s: i8, g: Grain) -> TimeObject {
        TimeObject { start: date(y, mo, da).at(h, mi, s, 0), grain: g, end: None }
    }
    #[test]
    fn plus_advances_start() {
        let t = obj(2013, 2, 12, 0, 0, 0, Grain::Day);
        assert_eq!(time_plus(t, Grain::Day, 1).start, date(2013, 2, 13).at(0, 0, 0, 0));
    }
    #[test]
    fn intersect_day_and_hour() {
        let day = obj(2013, 2, 12, 0, 0, 0, Grain::Day);
        let hour = obj(2013, 2, 12, 16, 0, 0, Grain::Hour);
        let i = time_intersect(day, hour).unwrap();
        assert_eq!(i.start, date(2013, 2, 12).at(16, 0, 0, 0));
        assert_eq!(i.grain, Grain::Hour);
    }
}

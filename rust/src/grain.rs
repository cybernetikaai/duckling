//! Time grain + calendar arithmetic (port of Duckling/TimeGrain/Types.hs).

use jiff::Span;
use jiff::civil::{DateTime, ISOWeekDate, Weekday, date};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub enum Grain {
    NoGrain,
    Second,
    Minute,
    Hour,
    Day,
    Week,
    Month,
    Quarter,
    Year,
}

/// One grain finer (port of TimeGrain.lower).
pub fn lower(g: Grain) -> Grain {
    match g {
        Grain::NoGrain | Grain::Second => Grain::Second,
        Grain::Minute => Grain::Second,
        Grain::Hour => Grain::Minute,
        Grain::Day => Grain::Hour,
        Grain::Week => Grain::Day,
        Grain::Month => Grain::Day,
        Grain::Quarter => Grain::Month,
        Grain::Year => Grain::Month,
    }
}

pub fn grain_str(g: Grain) -> &'static str {
    match g {
        Grain::NoGrain => "no-grain",
        Grain::Second => "second",
        Grain::Minute => "minute",
        Grain::Hour => "hour",
        Grain::Day => "day",
        Grain::Week => "week",
        Grain::Month => "month",
        Grain::Quarter => "quarter",
        Grain::Year => "year",
    }
}

/// jiff uses Temporal "constrain" overflow for calendar units, so month/year
/// adds clip day-of-month exactly like Haskell's addGregorianMonthsClip /
/// YearsClip (Jan 31 + 1mo -> Feb 28; Feb 29 + 1yr -> Feb 28).
pub fn add(dt: DateTime, g: Grain, n: i64) -> DateTime {
    let span = match g {
        Grain::NoGrain | Grain::Second => Span::new().seconds(n),
        Grain::Minute => Span::new().minutes(n),
        Grain::Hour => Span::new().hours(n),
        Grain::Day => Span::new().days(n),
        Grain::Week => Span::new().weeks(n),
        Grain::Month => Span::new().months(n),
        Grain::Quarter => Span::new().months(3 * n),
        Grain::Year => Span::new().years(n),
    };
    // Out-of-range results (e.g. a phone number mis-parsed as a huge year) must
    // not panic; return dt unchanged so such candidates simply resolve to nothing
    // useful and get filtered (never full-range for those inputs).
    dt.checked_add(span).unwrap_or(dt)
}

pub fn round(dt: DateTime, g: Grain) -> DateTime {
    match g {
        Grain::Week => {
            let iso = round(dt, Grain::Day).date().iso_week_date();
            ISOWeekDate::new(iso.year(), iso.week(), Weekday::Monday)
                .unwrap()
                .date()
                .at(0, 0, 0, 0)
        }
        Grain::Quarter => {
            let m = round(dt, Grain::Month);
            add(m, Grain::Month, -(((m.month() as i64) - 1) % 3))
        }
        _ => {
            let mo = if g > Grain::Month { 1 } else { dt.month() };
            let da = if g > Grain::Day { 1 } else { dt.day() };
            let h = if g > Grain::Hour { 0 } else { dt.hour() };
            let mi = if g > Grain::Minute { 0 } else { dt.minute() };
            let s = if g > Grain::Second { 0 } else { dt.second() };
            date(dt.year(), mo, da).at(h, mi, s, 0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn d(y: i16, mo: i8, da: i8, h: i8, mi: i8, s: i8) -> DateTime {
        date(y, mo, da).at(h, mi, s, 0)
    }
    #[test]
    fn add_day() {
        assert_eq!(add(d(2013, 2, 12, 4, 30, 0), Grain::Day, 1), d(2013, 2, 13, 4, 30, 0));
    }
    #[test]
    fn add_month_clip() {
        assert_eq!(add(d(2013, 1, 31, 0, 0, 0), Grain::Month, 1), d(2013, 2, 28, 0, 0, 0));
    }
    #[test]
    fn add_year_clip() {
        assert_eq!(add(d(2016, 2, 29, 0, 0, 0), Grain::Year, 1), d(2017, 2, 28, 0, 0, 0));
    }
    #[test]
    fn round_day() {
        assert_eq!(round(d(2013, 2, 12, 4, 30, 0), Grain::Day), d(2013, 2, 12, 0, 0, 0));
    }
    #[test]
    fn round_month() {
        assert_eq!(round(d(2013, 2, 12, 4, 30, 0), Grain::Month), d(2013, 2, 1, 0, 0, 0));
    }
    #[test]
    fn round_week_to_monday() {
        assert_eq!(round(d(2013, 2, 12, 4, 30, 0), Grain::Week), d(2013, 2, 11, 0, 0, 0));
    }
}

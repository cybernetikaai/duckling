//! English (`en`) Time rules.
//!
//! `en_rules(locale)` (bottom of this file) assembles the full rule set. Shared
//! helpers and TimeData constructors live here; the rule *builders* are grouped
//! by concern into submodules (each `use super::*` to reach these helpers):
//!   - `dates`     — days-of-week, months, numeric dates, day-of-month, EOM/EOY
//!   - `timeofday` — clock times, am/pm, parts of day, "past/to"
//!   - `intervals` — from/to, dash, between, directional (before/after), dom ranges
//!   - `cycles`    — this/next/last, nth-of, quarters, cycle-after/before
//!   - `holidays`  — global + per-region (data-driven) + beyond-Duckling holidays
//!   - `modifiers` — durations, seasons, absorption, timezones, intersect, precision
//!
//! Locale differences are numeric-date field order + regional holidays only
//! (both data-driven, threaded via the `Locale` param) — no per-region logic.
//! To add another language, add a sibling `time/<lang>/` module like this one.

pub(super) use crate::grain::Grain;
pub(super) use crate::regex::compile;
pub(super) use crate::time::object::{IntervalDirection, IntervalType};
pub(super) use crate::time::predicate::{
    Ampm, Predicate, ampm_predicate, cycle_n, cycle_nth, day_of_month, day_of_week,
    floor_grain_to_minute, hour, hour_minute, hour_minute_second, in_duration, intersect,
    merge_duration, minute, month, season_series, shift_duration, shift_timezone, take_last_of,
    take_nth, take_nth_after, take_nth_closest, time_cycle, time_intervals, year as year_pred,
};
pub(super) use crate::types::{Form, Locale, PatternItem, Rule, TimeData, Token};

mod cycles;
mod dates;
mod holidays;
mod intervals;
mod modifiers;
mod timeofday;

fn regex_groups(tokens: &[Token]) -> Option<&Vec<String>> {
    match tokens.first() {
        Some(Token::RegexMatch(g)) => Some(g),
        _ => None,
    }
}

fn mk_latent(mut td: TimeData) -> TimeData {
    td.latent = true;
    td
}
fn not_latent(mut td: TimeData) -> TimeData {
    td.latent = false;
    td
}

fn get_int_value(t: &Token) -> Option<i64> {
    match t {
        Token::Numeral(n) => crate::numeral::int_value(n),
        Token::Ordinal(o) => Some(o.value),
        _ => None,
    }
}

fn is_a_month(t: &Token) -> bool {
    matches!(t, Token::Time(td) if matches!(td.form, Some(Form::Month { .. })))
}
fn is_a_day_of_week(t: &Token) -> bool {
    matches!(t, Token::Time(td) if matches!(td.form, Some(Form::DayOfWeek)))
}
fn is_a_grain(t: &Token) -> bool {
    matches!(t, Token::TimeGrain(_))
}
fn grain_of(t: &Token) -> Option<Grain> {
    if let Token::TimeGrain(g) = t {
        Some(*g)
    } else {
        None
    }
}
fn cycle_nth_td(g: Grain, n: i64) -> TimeData {
    TimeData {
        pred: cycle_nth(g, n),
        grain: g,
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}
fn is_month_or_dow(t: &Token) -> bool {
    is_a_month(t) || is_a_day_of_week(t)
}
fn is_dom_ordinal(t: &Token) -> bool {
    matches!(t, Token::Ordinal(o) if (1..=31).contains(&o.value))
}
fn is_dom_integer(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if crate::numeral::ok_for_time(n)
        && crate::numeral::int_value(n).is_some_and(|v| (1..=31).contains(&v)))
}
fn is_dom_value(t: &Token) -> bool {
    is_dom_ordinal(t) || is_dom_integer(t)
}

fn day_of_month_td(n: i64) -> TimeData {
    TimeData {
        pred: day_of_month(n),
        grain: Grain::Day,
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}

/// Intersect a month/day-of-week time with a day-of-month value (port of intersectDOM).
fn intersect_dom(td: &TimeData, dom_token: &Token) -> Option<TimeData> {
    let n = get_int_value(dom_token)?;
    // For a day-of-week target, iterate the day-of-month (monthly, rarer) and
    // compose the weekday within it — otherwise "Thu 15th" iterates Thursdays
    // and hits SAFE_MAX before reaching the 15th that is a Thursday.
    let pred = if matches!(td.form, Some(Form::DayOfWeek)) {
        intersect(td.pred.clone(), day_of_month(n))
    } else {
        intersect(day_of_month(n), td.pred.clone())
    };
    Some(TimeData {
        pred,
        grain: Grain::Day,
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: td.holiday.clone(),
        has_timezone: false,
    })
}

fn is_integer_between(lo: i64, hi: i64) -> Box<dyn Fn(&Token) -> bool> {
    Box::new(move |t| {
        matches!(t, Token::Numeral(n)
            if crate::numeral::ok_for_time(n)
                && crate::numeral::int_value(n).is_some_and(|v| (lo..=hi).contains(&v)))
    })
}

fn is_a_time_of_day(t: &Token) -> bool {
    matches!(t, Token::Time(td) if matches!(td.form, Some(Form::TimeOfDay { .. })))
}

/// The generic (loose) interval rules pair any two times. Reject when exactly
/// one endpoint is a bare time-of-day and the other is not: a coherent time-of-
/// day interval has both ends as tods (handled by the dedicated tod rules), so a
/// bare-tod paired with a *dated* time (e.g. "from 3pm to 5pm tomorrow", where
/// "5pm tomorrow" is 5pm intersected with a day) is the trailing-date-on-interval
/// reading — leave it to intersect(interval, date), which resolves correctly.
///
/// Exception: a Second-grain *instant* ("now"/"right now"/"just now") paired with
/// a tod is a valid interval with no date to distribute ("from now to 5pm" →
/// [now, 5pm]), and Duckling forms it. A dated tod endpoint is never Second-grain
/// (it carries an hour/minute), so keying the exception on Second grain admits
/// "now" without re-admitting the trailing-date case.
fn tod_endpoint_mismatch(a: &TimeData, b: &TimeData) -> bool {
    let a_tod = matches!(a.form, Some(Form::TimeOfDay { .. }));
    let b_tod = matches!(b.form, Some(Form::TimeOfDay { .. }));
    if a_tod == b_tod {
        return false;
    }
    let non_tod = if a_tod { b } else { a };
    non_tod.grain != Grain::Second
}

fn is_month_or_year(t: &Token) -> bool {
    matches!(t, Token::Time(td) if matches!(td.form, Some(Form::Month { .. })) || td.grain == Grain::Year)
}

fn hour_td(is12h: bool, n: i64) -> TimeData {
    TimeData {
        pred: hour(is12h, None, n),
        grain: Grain::Hour,
        latent: false,
        not_immediate: false,
        form: Some(Form::TimeOfDay {
            hours: Some(n as i8),
            minutes: None,
            is12h,
        }),
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}

fn year_td(n: i64) -> TimeData {
    // 2-digit years map to 1950..2049 (port of `year` helper).
    let y = if n <= 99 {
        (n + 50).rem_euclid(100) + 1950
    } else {
        n
    };
    TimeData {
        pred: year_pred(y),
        grain: Grain::Year,
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}

/// Apply am/pm to a time-of-day. Duckling merges the AMPM field into the hour
/// predicate (runHourPredicate tdAMPM), so a bare hour becomes a step-24 series
/// anchored at the next matching hour — "3am" at 4:30 resolves to *tomorrow*
/// 3am, not today's already-past 3am. Only a separate-interval intersect would
/// mis-file today's occurrence as future, so fold ampm into hour() directly for
/// pure hours; hh:mm etc. fall back to the interval intersect.
fn time_of_day_ampm(is_am: bool, td: &TimeData) -> TimeData {
    let ampm = if is_am { Ampm::Am } else { Ampm::Pm };
    // Fold ampm into the hour predicate (Duckling merges tdAMPM) for both a
    // pure hour and an hh:mm, so the resolved time rolls to the next matching
    // occurrence rather than leaking today's already-past one — and composes
    // cleanly when a specific date pins the day ("Jul 18, 2014 07:00 PM").
    // Only a pure hour (grain Hour) or an hh:mm (minutes set) is folded; hh:mm:ss
    // keeps its seconds via the fallback.
    match td.form {
        Some(Form::TimeOfDay {
            hours: Some(h),
            minutes,
            is12h,
        }) if minutes.is_some() || td.grain == Grain::Hour => {
            let hp = hour(is12h, Some(ampm), h as i64);
            let (pred, grain) = match minutes {
                Some(m) => (intersect(minute(m as i64), hp), Grain::Minute),
                None => (hp, Grain::Hour),
            };
            return TimeData {
                pred,
                grain,
                latent: false,
                not_immediate: false,
                form: Some(Form::TimeOfDay {
                    hours: None,
                    minutes: None,
                    is12h: false,
                }),
                direction: None,
                holiday: td.holiday.clone(),
                has_timezone: false,
            };
        }
        _ => {}
    }
    // Fallback (hh:mm:ss, or no known hour): intersect the am/pm half-day.
    TimeData {
        pred: intersect(td.pred.clone(), ampm_predicate(is_am)),
        grain: td.grain,
        latent: false,
        not_immediate: false,
        form: Some(Form::TimeOfDay {
            hours: None,
            minutes: None,
            is12h: false,
        }),
        direction: None,
        holiday: td.holiday.clone(),
        has_timezone: false,
    }
}

fn tod(
    pred: Predicate,
    grain: Grain,
    hours: Option<i64>,
    minutes: Option<i64>,
    is12h: bool,
) -> TimeData {
    TimeData {
        pred,
        grain,
        latent: false,
        not_immediate: false,
        form: Some(Form::TimeOfDay {
            hours: hours.map(|h| h as i8),
            minutes: minutes.map(|m| m as i8),
            is12h,
        }),
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}

/// A rule whose regex matches an instant phrase and produces `cycle_nth(g, n)`.
fn instant(name: &str, g: Grain, n: i64, re: &str) -> Rule {
    Rule {
        name: name.to_string(),
        pattern: vec![PatternItem::Regex(compile(re))],
        prod: Box::new(move |_| Some(Token::Time(TimeData::new(cycle_nth(g, n), g)))),
    }
}

/// Build a rule that matches a regex and produces a fixed Time token.
fn time_rule<F>(name: &str, re: &str, make: F) -> Rule
where
    F: Fn() -> TimeData + 'static,
{
    Rule {
        name: name.to_string(),
        pattern: vec![PatternItem::Regex(compile(re))],
        prod: Box::new(move |_| Some(Token::Time(make()))),
    }
}

fn is_an_hour_of_day(t: &Token) -> bool {
    matches!(t, Token::Time(td)
        if matches!(td.form, Some(Form::TimeOfDay { hours: Some(_), .. })) && td.grain > Grain::Minute)
}
fn hour_minute_td(is12h: bool, h: i64, m: i64) -> TimeData {
    tod(
        hour_minute(is12h, h, m),
        Grain::Minute,
        Some(h),
        Some(m),
        is12h,
    )
}
fn minutes_after(n: i64, td: &TimeData) -> Option<TimeData> {
    if let Some(Form::TimeOfDay {
        hours: Some(h),
        is12h,
        ..
    }) = td.form
    {
        Some(hour_minute_td(is12h, h as i64, n))
    } else {
        None
    }
}
fn minutes_before(n: i64, td: &TimeData) -> Option<TimeData> {
    if let Some(Form::TimeOfDay {
        hours: Some(h),
        is12h,
        ..
    }) = td.form
    {
        let h = h as i64;
        let (hh, i12) = if h == 0 {
            (23, is12h)
        } else if h == 1 && is12h {
            (12, true)
        } else {
            (h - 1, is12h)
        };
        Some(hour_minute_td(i12, hh, 60 - n))
    } else {
        None
    }
}

fn is_not_latent(t: &Token) -> bool {
    matches!(t, Token::Time(td) if !td.latent)
}
fn grain_finer_than(g: Grain) -> Box<dyn Fn(&Token) -> bool> {
    Box::new(move |t| matches!(t, Token::Time(td) if td.grain < g))
}
fn is_grain_of_year(t: &Token) -> bool {
    matches!(t, Token::Time(td) if td.grain == Grain::Year)
}
fn is_a_time(t: &Token) -> bool {
    matches!(t, Token::Time(_))
}
fn now_td() -> TimeData {
    cycle_nth_td(Grain::Second, 0)
}
fn today_td() -> TimeData {
    cycle_nth_td(Grain::Day, 0)
}
fn is_a_part_of_day(t: &Token) -> bool {
    matches!(t, Token::Time(td) if matches!(td.form, Some(Form::PartOfDay { .. })))
}
/// Sentinel start_hour marking a multi-day part-of-day (the weekend). Out of the
/// 0..24 real-hour range so `is_same_day_part_of_day` can exclude it.
const WEEKEND_POD_HOUR: i8 = 24;
/// A part-of-day confined to a single day (morning/evening/…), excluding the
/// multi-day weekend. The "<part-of-day> at <time-of-day>" am/pm rule applies only
/// to these — for the weekend, "at 3pm" must intersect (→ Sat 3pm), not disambiguate.
fn is_same_day_part_of_day(t: &Token) -> bool {
    matches!(t, Token::Time(td)
        if matches!(td.form, Some(Form::PartOfDay { start_hour }) if start_hour != WEEKEND_POD_HOUR))
}
fn part_of_day(start_hour: i64, mut td: TimeData) -> TimeData {
    td.form = Some(Form::PartOfDay {
        start_hour: start_hour as i8,
    });
    td
}
fn pod_start_hour(td: &TimeData) -> Option<i64> {
    match td.form {
        Some(Form::PartOfDay { start_hour }) => Some(start_hour as i64),
        _ => None,
    }
}

/// Intersect two TimeData (finer grain drives the composition).
fn intersect_td(a: &TimeData, b: &TimeData) -> Option<TimeData> {
    if matches!(a.pred, Predicate::Empty) || matches!(b.pred, Predicate::Empty) {
        return None;
    }
    // An open-ended interval ("after 8", "before 3pm") is a half-line, not a
    // point-set to intersect with a part-of-day/date. The directional wrapper
    // must stay outermost, so refuse to intersect a directional operand —
    // otherwise "after 8 in the evening" wrongly collapses to a plain 20:00.
    if a.direction.is_some() || b.direction.is_some() {
        return None;
    }
    // `intersect(fine, coarse)` iterates the coarse predicate (bounded by
    // SAFE_MAX), composing the fine one within each occurrence. Normally the
    // finer grain is the inner predicate. But a day-of-week is high-frequency
    // (weekly), so when it shares a grain with a rarer operand (a specific date
    // like "Jul 18"), make the day-of-week the inner one — otherwise iterating
    // weeks hits SAFE_MAX before reaching e.g. the "Jul 18" that is a Friday.
    // A day-of-week shares the Day grain with a rarer same-grain operand (a
    // specific date like "Jul 18"); make the weekly dow the inner predicate so
    // the rarer one is iterated (else iterating weeks hits SAFE_MAX). Restricted
    // to equal grain: for a finer-grain operand the dow may be a recurring
    // time-of-day (e.g. "Thursday 8:00 PST") where iterating the dow is correct.
    let a_dow = matches!(a.form, Some(Form::DayOfWeek));
    let b_dow = matches!(b.form, Some(Form::DayOfWeek));
    let (fine, coarse) = if a.grain == b.grain && a_dow != b_dow {
        if a_dow { (a, b) } else { (b, a) }
    } else if a.grain <= b.grain {
        (a, b)
    } else {
        (b, a)
    };
    Some(TimeData {
        pred: intersect(fine.pred.clone(), coarse.pred.clone()),
        grain: a.grain.min(b.grain),
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: a.holiday.clone().or_else(|| b.holiday.clone()),
        has_timezone: false,
    })
}

fn month_td(n: i64) -> TimeData {
    TimeData {
        pred: month(n),
        grain: Grain::Month,
        latent: false,
        not_immediate: false,
        form: Some(Form::Month { month: n as i8 }),
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}
fn month_day_td(m: i64, d: i64) -> TimeData {
    // fixed calendar date; intersect always succeeds here
    intersect_td(&month_td(m), &day_of_month_td(d)).expect("month_day")
}
fn nth_dow_of_month_td(n: i64, dow: i64, m: i64) -> TimeData {
    TimeData {
        pred: take_nth_after(n - 1, true, day_of_week(dow), month(m)),
        grain: Grain::Day,
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}
fn last_dow_of_month_td(dow: i64, m: i64) -> TimeData {
    TimeData {
        pred: take_last_of(day_of_week(dow), month(m)),
        grain: Grain::Day,
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}
/// The n-th day-of-week relative to a fixed calendar date (predNthAfter/predLastOf
/// on a monthDay): n=0 → first DOW on/after the date (Reconciliation Day = 1st Mon
/// on/after May 26); n=-1 → last DOW on/before it (Victoria Day = last Mon on/before
/// May 25).
fn nth_dow_rel_date_td(n: i64, dow: i64, m: i64, d: i64) -> TimeData {
    TimeData {
        pred: take_nth_after(n, false, day_of_week(dow), month_day_td(m, d).pred),
        grain: Grain::Day,
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}
fn mk_holiday(name: &str, mut td: TimeData) -> TimeData {
    td.holiday = Some(name.to_string());
    td
}

fn holiday_rule(name: &'static str, re: &str, make: impl Fn() -> TimeData + 'static) -> Rule {
    Rule {
        name: format!("holiday: {name}"),
        pattern: vec![PatternItem::Regex(compile(re))],
        prod: Box::new(move |_| Some(Token::Time(mk_holiday(name, make())))),
    }
}

/// Build an interval TimeData (port of the `interval` helper).
fn interval_td(kind: IntervalType, td1: &TimeData, td2: &TimeData) -> Option<TimeData> {
    if matches!(td1.pred, Predicate::Empty) || matches!(td2.pred, Predicate::Empty) {
        return None;
    }
    Some(TimeData {
        pred: time_intervals(kind, td1.pred.clone(), td2.pred.clone()),
        grain: td1.grain.min(td2.grain),
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: None,
        has_timezone: false,
    })
}

fn with_direction(dir: IntervalDirection, mut td: TimeData) -> TimeData {
    td.direction = Some(dir);
    td.latent = false;
    td
}

fn dom_interval(m: &TimeData, t1: &Token, t2: &Token) -> Option<TimeData> {
    let d1 = intersect_dom(m, t1)?;
    let d2 = intersect_dom(m, t2)?;
    interval_td(IntervalType::Closed, &d1, &d2)
}

fn hour_interval(h1: i64, h2: i64) -> Option<TimeData> {
    interval_td(IntervalType::Open, &hour_td(false, h1), &hour_td(false, h2))
}

fn is_grain_of_time_day(t: &Token) -> bool {
    matches!(t, Token::Time(td) if td.grain == Grain::Day)
}

/// Named timezone -> fixed offset in minutes (subset of parseTimezone).
const TZ: &[(&str, i64)] = &[
    ("GMT", 0),
    ("UTC", 0),
    ("WET", 0),
    ("BST", 60),
    ("CET", 60),
    ("WAT", 60),
    ("WEST", 60),
    ("CEST", 120),
    ("EET", 120),
    ("SAST", 120),
    ("EEST", 180),
    ("EAT", 180),
    ("MSK", 180),
    ("IST", 330),
    ("PST", -480),
    ("PDT", -420),
    ("MST", -420),
    ("MDT", -360),
    ("CST", -360),
    ("CDT", -300),
    ("EST", -300),
    ("EDT", -240),
    ("AST", -240),
    ("ADT", -180),
    ("AKST", -540),
    ("AKDT", -480),
    ("HST", -600),
    ("JST", 540),
    ("KST", 540),
    ("AEST", 600),
    ("AEDT", 660),
    ("ACST", 570),
    ("AWST", 480),
    ("NZST", 720),
    ("NZDT", 780),
];
fn tz_offset(name: &str) -> Option<i64> {
    let u = name.to_uppercase();
    TZ.iter().find(|(n, _)| *n == u).map(|&(_, o)| o)
}
fn in_timezone_td(provided: i64, td: &TimeData) -> TimeData {
    TimeData {
        pred: shift_timezone(provided, td.pred.clone()),
        grain: td.grain,
        latent: false,
        not_immediate: false,
        form: td.form,
        direction: td.direction,
        holiday: td.holiday.clone(),
        has_timezone: true,
    }
}

fn has_no_timezone(t: &Token) -> bool {
    matches!(t, Token::Time(td) if !td.has_timezone)
}

/// Build a closed interval and shift the whole thing into `tz_name`. The endpoints
/// are first floored to minute grain so the interval's exclusive end is the second
/// endpoint + 1 minute (e.g. "9am to 5pm GMT" → 5pm-GMT + 1min = 15:01, not the
/// hour-exclusive 18:00 shifted to 16:00). Shifting the *whole resolved* interval
/// (rather than the two recurring endpoint predicates) keeps both ends on the same
/// day — shifting endpoints independently lets a large offset pair them across a
/// day boundary.
fn interval_timezone(tz_name: &str, a: &TimeData, b: &TimeData) -> Option<Token> {
    let off = tz_offset(tz_name)?;
    let a_min = TimeData {
        pred: floor_grain_to_minute(a.pred.clone()),
        grain: Grain::Minute,
        ..a.clone()
    };
    let b_min = TimeData {
        pred: floor_grain_to_minute(b.pred.clone()),
        grain: Grain::Minute,
        ..b.clone()
    };
    let iv = interval_td(IntervalType::Closed, &a_min, &b_min)?;
    Some(Token::Time(in_timezone_td(off, &iv)))
}

fn is_a_duration(t: &Token) -> bool {
    matches!(t, Token::Duration(_))
}
fn duration_of(t: &Token) -> Option<(i64, Grain)> {
    if let Token::Duration(d) = t {
        Some((d.value, d.grain))
    } else {
        None
    }
}

/// A time shifted forward by a duration (port of durationAfter): the end of a
/// "<time> for <duration>" interval. NoGrain times use shiftDuration, others
/// mergeDuration; the result takes the duration's grain.
fn duration_after_td(value: i64, grain: Grain, td: &TimeData) -> TimeData {
    let pred = if td.grain == Grain::NoGrain {
        shift_duration(td.pred.clone(), value, grain)
    } else {
        merge_duration(td.pred.clone(), value, grain)
    };
    TimeData::new(pred, grain)
}
fn cycle_n_td(grain: Grain, n: i64) -> TimeData {
    TimeData {
        pred: cycle_n(true, grain, n),
        grain,
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}

fn in_duration_td(value: i64, grain: Grain) -> TimeData {
    TimeData {
        pred: in_duration(value, grain),
        grain: crate::grain::lower(grain),
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}

/// The grain-window [now+v, now+v+1) (port of inDurationInterval); negate v for
/// durationIntervalAgo. Intersecting a day/month time with it selects the
/// occurrence in that window: "March in a year" -> March of next year.
fn in_duration_interval_td(value: i64, grain: Grain) -> Option<TimeData> {
    interval_td(
        IntervalType::Open,
        &in_duration_td(value, grain),
        &in_duration_td(value + 1, grain),
    )
}

/// Approximation of isOkWithThisNext: holidays and date-like forms can take
/// "this/next/last"; bare time-of-day cannot.
fn is_ok_with_this_next(t: &Token) -> bool {
    matches!(t, Token::Time(td) if td.holiday.is_some()
        || matches!(td.form, Some(Form::DayOfWeek) | Some(Form::Month { .. }) | Some(Form::PartOfDay { .. }) | Some(Form::Season)))
}

fn season_td(sm: i64, sd: i64, em: i64, ed: i64) -> Option<TimeData> {
    // End is a full day (its exclusive bound is the following midnight), which
    // our Closed interval reports as the "to" value — matching Duckling.
    let mut td = interval_td(
        IntervalType::Closed,
        &month_day_td(sm, sd),
        &month_day_td(em, ed),
    )?;
    td.form = Some(Form::Season);
    Some(td)
}

fn pred_nth_td(n: i64, not_immediate: bool, td: &TimeData) -> TimeData {
    TimeData {
        pred: take_nth(n, not_immediate, td.pred.clone()),
        grain: td.grain,
        latent: false,
        not_immediate: false,
        form: td.form,
        direction: None,
        holiday: td.holiday.clone(),
        has_timezone: false,
    }
}

fn is_ordinal(t: &Token) -> bool {
    matches!(t, Token::Ordinal(_))
}
fn is_grain_quarter(t: &Token) -> bool {
    matches!(t, Token::TimeGrain(Grain::Quarter))
}
fn cycle_nth_after_td(not_immediate: bool, grain: Grain, n: i64, base: &TimeData) -> TimeData {
    TimeData {
        pred: take_nth_after(n, not_immediate, time_cycle(grain), base.pred.clone()),
        grain,
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}

/// The last occurrence of a cycle grain within `base` (port of cycleLastOf).
fn cycle_last_of_td(grain: Grain, base: &TimeData) -> TimeData {
    TimeData::new(take_last_of(time_cycle(grain), base.pred.clone()), grain)
}

/// The last occurrence of a cyclic time within `base` (port of predLastOf);
/// grain comes from the cyclic predicate, e.g. "last Monday of March".
fn pred_last_of_td(cyclic: &TimeData, base: &TimeData) -> TimeData {
    TimeData::new(
        take_last_of(cyclic.pred.clone(), base.pred.clone()),
        cyclic.grain,
    )
}

/// The n-th occurrence of a cyclic time at/after `base` (port of predNthAfter,
/// notImmediate=true), e.g. "first monday of last month". Grain from cyclic.
fn pred_nth_after_td(n: i64, cyclic: &TimeData, base: &TimeData) -> TimeData {
    let mut td = TimeData::new(
        take_nth_after(n, true, cyclic.pred.clone(), base.pred.clone()),
        cyclic.grain,
    );
    td.holiday = cyclic.holiday.clone();
    td
}

/// The recurring weekend interval Fri 18:00 → Mon 00:00 (port of `weekend`).
/// Day-grain (not Hour) so that intersecting with a time-of-day treats the
/// weekend as the *coarse* operand — "weekend at 3pm" iterates the weekend and
/// places 3pm within it (→ Sat 3pm). The resolved interval still reports Hour
/// grain, taken from the Fri 18:00 / Mon 00:00 endpoints, not this TimeData grain.
fn weekend_td() -> TimeData {
    let fri = intersect(hour(false, None, 18), day_of_week(5));
    let mon = intersect(hour(false, None, 0), day_of_week(1));
    TimeData::new(time_intervals(IntervalType::Open, fri, mon), Grain::Day)
}

/// The n-th closest occurrence of `td1` to `td2` (port of predNthClosest),
/// e.g. "the closest Monday to Oct 5th". Grain/holiday come from td1.
fn pred_nth_closest_td(n: i64, td1: &TimeData, td2: &TimeData) -> TimeData {
    let mut td = TimeData::new(
        take_nth_closest(n, td1.pred.clone(), td2.pred.clone()),
        td1.grain,
    );
    td.holiday = td1.holiday.clone();
    td
}

fn is_grain_month_or_coarser(t: &Token) -> bool {
    matches!(t, Token::Time(td) if td.grain >= Grain::Month)
}

fn dom_of_this_month(d: i64) -> Option<TimeData> {
    intersect_td(&day_of_month_td(d), &cycle_nth_td(Grain::Month, 0))
}
fn dom_of_next_month(d: i64) -> Option<TimeData> {
    intersect_td(&day_of_month_td(d), &cycle_nth_td(Grain::Month, 1))
}

fn day_of_week_td(n: i64) -> TimeData {
    TimeData {
        pred: day_of_week(n),
        grain: Grain::Day,
        latent: false,
        not_immediate: false,
        form: Some(Form::DayOfWeek),
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}
fn is_grain_of_time(g: Grain) -> Box<dyn Fn(&Token) -> bool> {
    Box::new(move |t| matches!(t, Token::Time(td) if td.grain == g))
}

fn month_num(s: &str) -> Option<i64> {
    let s = s.to_lowercase();
    [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ]
    .iter()
    .position(|p| s.starts_with(p))
    .map(|i| (i + 1) as i64)
}
fn valid_md(m: i64, d: i64) -> bool {
    (1..=12).contains(&m) && (1..=31).contains(&d)
}
fn year_month_day_td(y: i64, m: i64, d: i64) -> Option<TimeData> {
    if !valid_md(m, d) {
        return None;
    }
    intersect_td(&month_day_td(m, d), &year_td(y))
}
fn year_month_td(y: i64, m: i64) -> Option<TimeData> {
    if !(1..=12).contains(&m) {
        return None;
    }
    intersect_td(&month_td(m), &year_td(y))
}
fn parse_i(g: &[String], i: usize) -> Option<i64> {
    g.get(i)?.parse().ok()
}

/// A day-of-month range within a named month, [sd..ed] Open; ed == -1 means
/// "to the last day of the month" (cycleLastOf Day). Shared by the
/// beginning/end-of-<named-month> and early/mid/late-<named-month> rules.
fn month_dom_range(month: &TimeData, sd: i64, ed: i64) -> Option<TimeData> {
    let start = intersect_td(month, &day_of_month_td(sd))?;
    let end = if ed == -1 {
        cycle_last_of_td(Grain::Day, month)
    } else {
        intersect_td(month, &day_of_month_td(ed))?
    };
    // Duckling uses interval Open here, but its end object is a full day whose
    // exclusive bound is the following midnight — which is what our Closed
    // interval yields (end-of-end-day), so the reported "to" matches.
    interval_td(IntervalType::Closed, &start, &end)
}

pub fn en_rules(locale: Locale) -> Vec<Rule> {
    let mut rules = vec![
        instant(
            "now",
            Grain::Second,
            0,
            r"(right |just )?now|at\s+the\s+moment|atm",
        ),
        instant("today", Grain::Day, 0, r"todays?|at\s+this\s+time"),
        instant("tomorrow", Grain::Day, 1, r"tmrw?|tomm?or?rows?"),
        instant("yesterday", Grain::Day, -1, r"yesterdays?"),
    ];
    rules.extend(dates::days_of_week());
    rules.extend(dates::months());
    rules.extend(timeofday::time_of_day_rules());
    rules.extend(timeofday::past_to_rules());
    rules.extend(timeofday::numeral_dependent_rules());
    rules.extend(dates::day_of_month_rules());
    rules.extend(cycles::cycle_and_relative_rules());
    rules.extend(intervals::interval_rules());
    rules.extend(timeofday::part_of_day_rules());
    rules.extend(modifiers::duration_rules());
    rules.extend(holidays::holiday_rules());
    rules.extend(modifiers::season_rules());
    rules.extend(dates::numeric_date_rules(locale));
    rules.extend(holidays::region_holiday_rules(locale));
    rules.extend(holidays::modern_holiday_rules(locale));
    rules.extend(dates::end_beginning_of_month_rules());
    rules.extend(dates::end_beginning_year_week_rules());
    rules.extend(cycles::this_next_last_time_rules());
    rules.extend(cycles::nth_dow_of_time_rules());
    rules.extend(cycles::quarter_rules());
    rules.extend(cycles::cycle_after_before_rules());
    rules.extend(timeofday::time_pod_rules());
    rules.extend(crate::time::computed::computed_holiday_rules());
    rules.extend(crate::time::computed::computed_holiday_shift_rules());
    rules.extend(crate::time::computed::computed_interval_holiday_rules());
    rules.push(crate::time::computed::earth_hour_rule());
    rules.extend(modifiers::absorb_rules());
    rules.extend(modifiers::timezone_rules());
    rules.extend(intervals::dom_interval_rules());
    rules.extend(intervals::direction_rules());
    rules.extend(modifiers::precision_and_era_rules());
    rules.extend(dates::named_month_part_rules());
    rules.extend(modifiers::intersect_rules());
    rules
}

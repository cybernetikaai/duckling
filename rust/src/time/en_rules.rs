//! English Time rules.
//! Phase 1: instants. + days-of-week, months.

use crate::grain::Grain;
use crate::regex::compile;
use crate::time::object::{IntervalDirection, IntervalType};
use crate::time::predicate::{
    Ampm, Predicate, ampm_predicate, cycle_nth, day_of_month, day_of_week, hour, hour_minute,
    hour_minute_second, in_duration, intersect, merge_duration, minute, month, season_series,
    shift_duration, shift_timezone, take_last_of, take_nth, take_nth_after, take_nth_closest,
    time_cycle, time_intervals, year as year_pred, cycle_n,
};
use crate::types::{Form, PatternItem, Rule, TimeData, Token};

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
fn tod_endpoint_mismatch(a: &TimeData, b: &TimeData) -> bool {
    let a_tod = matches!(a.form, Some(Form::TimeOfDay { .. }));
    let b_tod = matches!(b.form, Some(Form::TimeOfDay { .. }));
    a_tod != b_tod
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
        form: Some(Form::TimeOfDay { hours: Some(n as i8), minutes: None, is12h }),
        direction: None,
        holiday: None,
        has_timezone: false,
    }
}

fn year_td(n: i64) -> TimeData {
    // 2-digit years map to 1950..2049 (port of `year` helper).
    let y = if n <= 99 { (n + 50).rem_euclid(100) + 1950 } else { n };
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
    if let Some(Form::TimeOfDay { hours: Some(h), minutes, is12h }) = td.form {
        if minutes.is_some() || td.grain == Grain::Hour {
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
                form: Some(Form::TimeOfDay { hours: None, minutes: None, is12h: false }),
                direction: None,
                holiday: td.holiday.clone(),
                has_timezone: false,
            };
        }
    }
    // Fallback (hh:mm:ss, or no known hour): intersect the am/pm half-day.
    TimeData {
        pred: intersect(td.pred.clone(), ampm_predicate(is_am)),
        grain: td.grain,
        latent: false,
        not_immediate: false,
        form: Some(Form::TimeOfDay { hours: None, minutes: None, is12h: false }),
        direction: None,
        holiday: td.holiday.clone(),
        has_timezone: false,
    }
}

fn tod(pred: Predicate, grain: Grain, hours: Option<i64>, minutes: Option<i64>, is12h: bool) -> TimeData {
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

fn days_of_week() -> Vec<Rule> {
    // (name, n [Mon=1..Sun=7], regex)
    let days: [(&str, i64, &str); 7] = [
        ("Monday", 1, r"mondays?|mon\.?"),
        ("Tuesday", 2, r"tuesdays?|tues?\.?"),
        ("Wednesday", 3, r"wed?nesdays?|wed\.?"),
        ("Thursday", 4, r"thursdays?|thu(rs?)?\.?"),
        ("Friday", 5, r"fridays?|fri\.?"),
        ("Saturday", 6, r"saturdays?|sat\.?"),
        ("Sunday", 7, r"sundays?|sun\.?"),
    ];
    days.iter()
        .map(|&(name, n, re)| {
            time_rule(name, re, move || TimeData {
                pred: day_of_week(n),
                grain: Grain::Day,
                latent: false,
                not_immediate: true,
                form: Some(Form::DayOfWeek),
                direction: None,
                holiday: None,
                has_timezone: false,
            })
        })
        .collect()
}

fn months() -> Vec<Rule> {
    let ms: [(&str, i64, &str); 12] = [
        ("January", 1, r"january|jan\.?"),
        ("February", 2, r"february|feb\.?"),
        ("March", 3, r"march|mar\.?"),
        ("April", 4, r"april|apr\.?"),
        ("May", 5, r"may"),
        ("June", 6, r"june|jun\.?"),
        ("July", 7, r"july|jul\.?"),
        ("August", 8, r"august|aug\.?"),
        ("September", 9, r"september|sept?\.?"),
        ("October", 10, r"october|oct\.?"),
        ("November", 11, r"november|nov\.?"),
        ("December", 12, r"december|dec\.?"),
    ];
    ms.iter()
        .map(|&(name, n, re)| {
            time_rule(name, re, move || TimeData {
                pred: month(n),
                grain: Grain::Month,
                latent: false,
                not_immediate: false,
                form: Some(Form::Month { month: n as i8 }),
                direction: None,
                holiday: None,
                has_timezone: false,
            })
        })
        .collect()
}

fn is_an_hour_of_day(t: &Token) -> bool {
    matches!(t, Token::Time(td)
        if matches!(td.form, Some(Form::TimeOfDay { hours: Some(_), .. })) && td.grain > Grain::Minute)
}
fn hour_minute_td(is12h: bool, h: i64, m: i64) -> TimeData {
    tod(hour_minute(is12h, h, m), Grain::Minute, Some(h), Some(m), is12h)
}
fn minutes_after(n: i64, td: &TimeData) -> Option<TimeData> {
    if let Some(Form::TimeOfDay { hours: Some(h), is12h, .. }) = td.form {
        Some(hour_minute_td(is12h, h as i64, n))
    } else {
        None
    }
}
fn minutes_before(n: i64, td: &TimeData) -> Option<TimeData> {
    if let Some(Form::TimeOfDay { hours: Some(h), is12h, .. }) = td.form {
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

/// quarter/half/N past|to <hour-of-day> (ruleHODHalf/Quarter, ruleNumeral/Half/
/// Quarter To/After HOD). e.g. "half past 3", "quarter to 3", "20 past 3".
fn past_to_rules() -> Vec<Rule> {
    fn after_rule(name: &str, re: &str, n: i64) -> Rule {
        Rule {
            name: name.into(),
            pattern: vec![
                PatternItem::Regex(compile(re)),
                PatternItem::Predicate(Box::new(is_an_hour_of_day)),
            ],
            prod: Box::new(move |tokens| match tokens {
                [_, Token::Time(td)] => minutes_after(n, td).map(Token::Time),
                _ => None,
            }),
        }
    }
    fn before_rule(name: &str, re: &str, n: i64) -> Rule {
        Rule {
            name: name.into(),
            pattern: vec![
                PatternItem::Regex(compile(re)),
                PatternItem::Predicate(Box::new(is_an_hour_of_day)),
            ],
            prod: Box::new(move |tokens| match tokens {
                [_, Token::Time(td)] => minutes_before(n, td).map(Token::Time),
                _ => None,
            }),
        }
    }
    vec![
        // "ten thirty", "3 15", "three twenty" -> hour + minutes (latent-preserving).
        Rule {
            name: "<hour-of-day> <integer>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_an_hour_of_day)),
                PatternItem::Predicate(is_integer_between(10, 59)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(hod), num] => {
                    let (h, is12h) = match hod.form {
                        Some(Form::TimeOfDay { hours: Some(h), is12h, .. }) => (h as i64, is12h),
                        _ => return None,
                    };
                    let td = hour_minute_td(is12h, h, get_int_value(num)?);
                    Some(Token::Time(if hod.latent { mk_latent(td) } else { td }))
                }
                _ => None,
            }),
        },
        Rule {
            name: "<time-of-day> o'clock".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_a_time_of_day)),
                PatternItem::Regex(compile(r"o.?clock")),
            ],
            prod: Box::new(|tokens| match tokens.first()? {
                Token::Time(td) => Some(Token::Time(not_latent(td.clone()))),
                _ => None,
            }),
        },
        Rule {
            name: "half <integer> (UK style hour-of-day)".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"half")),
                PatternItem::Predicate(Box::new(is_an_hour_of_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td)] => minutes_after(30, td).map(Token::Time),
                _ => None,
            }),
        },
        // <hour> half / <hour> quarter
        Rule {
            name: "<hour-of-day> half".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_an_hour_of_day)),
                PatternItem::Regex(compile(r"half")),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td), _] => minutes_after(30, td).map(Token::Time),
                _ => None,
            }),
        },
        Rule {
            name: "<hour-of-day> quarter".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_an_hour_of_day)),
                PatternItem::Regex(compile(r"(a|one)? ?quarter")),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td), _] => minutes_after(15, td).map(Token::Time),
                _ => None,
            }),
        },
        before_rule("half to|till|before <hour-of-day>", r"half (to|till|before|of)", 30),
        before_rule("quarter to|till|before <hour-of-day>", r"(a|one)? ?quarter (to|till|before|of)", 15),
        after_rule("half after|past <hour-of-day>", r"half (after|past)", 30),
        after_rule("quarter after|past <hour-of-day>", r"(a|one)? ?quarter (after|past)", 15),
        // <integer> to|past <hour-of-day>
        Rule {
            name: "<integer> to|till|before <hour-of-day>".into(),
            pattern: vec![
                PatternItem::Predicate(is_integer_between(1, 59)),
                PatternItem::Regex(compile(r"to|till|before|of")),
                PatternItem::Predicate(Box::new(is_an_hour_of_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [num, _, Token::Time(td)] => minutes_before(get_int_value(num)?, td).map(Token::Time),
                _ => None,
            }),
        },
        Rule {
            name: "<integer> minutes to|till|before <hour-of-day>".into(),
            pattern: vec![
                PatternItem::Predicate(is_integer_between(1, 59)),
                PatternItem::Predicate(Box::new(|t| matches!(t, Token::TimeGrain(Grain::Minute)))),
                PatternItem::Regex(compile(r"to|till|before|of")),
                PatternItem::Predicate(Box::new(is_an_hour_of_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [num, _, _, Token::Time(td)] => {
                    minutes_before(get_int_value(num)?, td).map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "integer after|past <hour-of-day>".into(),
            pattern: vec![
                PatternItem::Predicate(is_integer_between(1, 59)),
                PatternItem::Regex(compile(r"after|past")),
                PatternItem::Predicate(Box::new(is_an_hour_of_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [num, _, Token::Time(td)] => minutes_after(get_int_value(num)?, td).map(Token::Time),
                _ => None,
            }),
        },
    ]
}

fn time_of_day_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "hh:mm".into(),
            pattern: vec![PatternItem::Regex(compile(r"((?:[01]?\d)|(?:2[0-3]))[:.]([0-5]\d)"))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                let h: i64 = g.first()?.parse().ok()?;
                let m: i64 = g.get(1)?.parse().ok()?;
                let is12h = h != 0 && h < 12;
                Some(Token::Time(tod(hour_minute(is12h, h, m), Grain::Minute, Some(h), Some(m), is12h)))
            }),
        },
        Rule {
            name: "hhhmm".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(?<!/)((?:[01]?\d)|(?:2[0-3]))h(([0-5]\d)|(?!\d))",
            ))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                let h: i64 = g.first()?.parse().ok()?;
                let m: i64 = g.get(1).and_then(|s| s.parse().ok()).unwrap_or(0);
                Some(Token::Time(tod(hour_minute(false, h, m), Grain::Minute, Some(h), Some(m), false)))
            }),
        },
        Rule {
            name: "hhmm (latent)".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"((?:[01]?\d)|(?:2[0-3]))([0-5]\d)(?!.\d)",
            ))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                let h: i64 = g.first()?.parse().ok()?;
                let m: i64 = g.get(1)?.parse().ok()?;
                Some(Token::Time(mk_latent(tod(
                    hour_minute(h < 12, h, m),
                    Grain::Minute,
                    Some(h),
                    Some(m),
                    h < 12,
                ))))
            }),
        },
        Rule {
            name: "hh:mm:ss".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"((?:[01]?\d)|(?:2[0-3]))[:.]([0-5]\d)[:.]([0-5]\d)",
            ))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                let h: i64 = g.first()?.parse().ok()?;
                let m: i64 = g.get(1)?.parse().ok()?;
                let s: i64 = g.get(2)?.parse().ok()?;
                let is12h = h < 12;
                Some(Token::Time(tod(
                    hour_minute_second(is12h, h, m, s),
                    Grain::Second,
                    Some(h),
                    None,
                    is12h,
                )))
            }),
        },
    ]
}

/// Rules that consume Numeral tokens (years, bare hours) and the rules that
/// build on them (am/pm, at-TOD, noon/midnight).
fn numeral_dependent_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "year (latent)".into(),
            pattern: vec![PatternItem::Predicate(is_integer_between(25, 10000))],
            prod: Box::new(|tokens| {
                let n = get_int_value(tokens.first()?)?;
                Some(Token::Time(mk_latent(year_td(n))))
            }),
        },
        Rule {
            name: "in|during <named-month>|year".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"in|during")),
                PatternItem::Predicate(Box::new(is_month_or_year)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td)] => Some(Token::Time(not_latent(td.clone()))),
                _ => None,
            }),
        },
        Rule {
            name: "time-of-day (latent)".into(),
            pattern: vec![PatternItem::Predicate(is_integer_between(0, 23))],
            prod: Box::new(|tokens| {
                let n = get_int_value(tokens.first()?)?;
                Some(Token::Time(mk_latent(hour_td(n < 13, n))))
            }),
        },
        Rule {
            name: "at <time-of-day>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"at|@")),
                PatternItem::Predicate(Box::new(is_a_time_of_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td)] => Some(Token::Time(not_latent(td.clone()))),
                _ => None,
            }),
        },
        Rule {
            name: "<time-of-day> am|pm".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_a_time_of_day)),
                PatternItem::Regex(compile(r"(in the )?([ap])(\s|\.)?(m?)\.?")),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td), Token::RegexMatch(g)] => {
                    let is_am = g.get(1).map(|s| s.eq_ignore_ascii_case("a")).unwrap_or(false);
                    let m_empty = g.get(3).map(|s| s.is_empty()).unwrap_or(true);
                    if td.latent && m_empty {
                        Some(Token::Time(mk_latent(time_of_day_ampm(is_am, td))))
                    } else if let Some(Form::TimeOfDay { hours: Some(h), .. }) = td.form {
                        if h < 13 {
                            Some(Token::Time(time_of_day_ampm(is_am, td)))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            }),
        },
        Rule {
            name: "noon|midnight|EOD|end of day".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(noon|midni(ght|te)|(the )?(EOD|end of (the )?day))",
            ))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                let noon = g.first()?.eq_ignore_ascii_case("noon");
                Some(Token::Time(hour_td(false, if noon { 12 } else { 0 })))
            }),
        },
        Rule {
            name: "Mid-day".into(),
            pattern: vec![PatternItem::Regex(compile(r"(the )?mid(\s)?day"))],
            prod: Box::new(|_| Some(Token::Time(hour_td(false, 12)))),
        },
    ]
}

/// Day-of-month + month-day rules (need Ordinal/Numeral). Ports of the
/// ruleDOM* / ruleNamedDOM* / ruleMonthDOM* family.
fn day_of_month_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "<day-of-month> (ordinal)".into(),
            pattern: vec![PatternItem::Predicate(Box::new(is_dom_ordinal))],
            prod: Box::new(|tokens| {
                let n = get_int_value(tokens.first()?)?;
                Some(Token::Time(mk_latent(day_of_month_td(n))))
            }),
        },
        Rule {
            name: "the <day-of-month> (number)".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"the")),
                PatternItem::Predicate(Box::new(is_dom_integer)),
            ],
            prod: Box::new(|tokens| {
                let n = get_int_value(tokens.get(1)?)?;
                Some(Token::Time(mk_latent(day_of_month_td(n))))
            }),
        },
        Rule {
            name: "the <day-of-month> (ordinal)".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"the")),
                PatternItem::Predicate(Box::new(is_dom_ordinal)),
            ],
            prod: Box::new(|tokens| {
                let n = get_int_value(tokens.get(1)?)?;
                Some(Token::Time(day_of_month_td(n)))
            }),
        },
        Rule {
            name: "<named-month>|<named-day> <day-of-month> (ordinal)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_month_or_dow)),
                PatternItem::Predicate(Box::new(is_dom_ordinal)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td), dom] => intersect_dom(td, dom).map(Token::Time),
                _ => None,
            }),
        },
        Rule {
            name: "<named-month> <day-of-month> (non ordinal)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_a_month)),
                PatternItem::Predicate(Box::new(is_dom_integer)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td), dom] => intersect_dom(td, dom).map(Token::Time),
                _ => None,
            }),
        },
        // "the ides of March" -> the 15th (Mar/May/Jul/Oct) or 13th otherwise.
        Rule {
            name: "the ides of <named-month>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"the ides? of")),
                PatternItem::Predicate(Box::new(is_a_month)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td)] => {
                    let m = match td.form {
                        Some(Form::Month { month }) => month as i64,
                        _ => return None,
                    };
                    let dom = if [3, 5, 7, 10].contains(&m) { 15 } else { 13 };
                    intersect_td(td, &day_of_month_td(dom)).map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "<day-of-month> (ordinal or number) of <named-month>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_dom_value)),
                PatternItem::Regex(compile(r"of|in")),
                PatternItem::Predicate(Box::new(is_a_month)),
            ],
            prod: Box::new(|tokens| match tokens {
                [dom, _, Token::Time(td)] => intersect_dom(td, dom).map(Token::Time),
                _ => None,
            }),
        },
        // With a leading "the" (ruleTheDOMOfMonth) — a full-span dom parse for
        // "the second of march" that outranks the-cycle-of-<second> by score.
        Rule {
            name: "the <day-of-month> (ordinal or number) of <named-month>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"the")),
                PatternItem::Predicate(Box::new(is_dom_value)),
                PatternItem::Regex(compile(r"of|in")),
                PatternItem::Predicate(Box::new(is_a_month)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, dom, _, Token::Time(td)] => intersect_dom(td, dom).map(Token::Time),
                _ => None,
            }),
        },
        // Grain-based variant (ruleDOMOfTimeMonth): accepts any month-grained
        // time, e.g. "20 of next month", "20th of the previous month".
        Rule {
            name: "<day-of-month> (ordinal or number) of <month>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_dom_value)),
                PatternItem::Regex(compile(r"of( the)?")),
                PatternItem::Predicate(is_grain_of_time(Grain::Month)),
            ],
            prod: Box::new(|tokens| match tokens {
                [dom, _, Token::Time(td)] => intersect_dom(td, dom).map(Token::Time),
                _ => None,
            }),
        },
        Rule {
            name: "<day-of-month> (ordinal or number) <named-month>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_dom_value)),
                PatternItem::Predicate(Box::new(is_a_month)),
            ],
            prod: Box::new(|tokens| match tokens {
                [dom, Token::Time(td)] => intersect_dom(td, dom).map(Token::Time),
                _ => None,
            }),
        },
    ]
}

/// this/next/last <cycle> and this/next <day-of-week>.
fn cycle_and_relative_rules() -> Vec<Rule> {
    vec![
        // One rule (ruleCycleThisLastNext): the matched word selects the offset.
        // Single alternation so "upcoming" matches wholly rather than letting
        // "coming" partial-match at offset 2. coming/upcoming/next -> +1.
        Rule {
            name: "this|last|next <cycle>".into(),
            pattern: vec![
                PatternItem::Regex(compile(
                    r"(this|current|coming|next|(the( following)?)|last|past|previous|upcoming)",
                )),
                PatternItem::Predicate(Box::new(is_a_grain)),
            ],
            prod: Box::new(|tokens| {
                let g = grain_of(tokens.get(1)?)?;
                let word = match &tokens[0] {
                    Token::RegexMatch(m) => m.first()?.to_lowercase(),
                    _ => return None,
                };
                let n = match word.as_str() {
                    "this" | "current" | "the" => 0,
                    "coming" | "next" | "upcoming" | "the following" => 1,
                    "last" | "past" | "previous" => -1,
                    _ => return None,
                };
                Some(Token::Time(cycle_nth_td(g, n)))
            }),
        },
        // "upcoming 2 weeks" -> cycleNth(week, 2) (a single cycle, not an interval).
        Rule {
            name: "upcoming <integer> <cycle>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"upcoming")),
                PatternItem::Predicate(Box::new(|t| get_int_value(t).is_some_and(|v| v >= 0))),
                PatternItem::Predicate(Box::new(is_a_grain)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, num, grain] => {
                    Some(Token::Time(cycle_nth_td(grain_of(grain)?, get_int_value(num)?)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "<integer> upcoming <cycle>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(|t| get_int_value(t).is_some_and(|v| v >= 0))),
                PatternItem::Regex(compile(r"upcoming")),
                PatternItem::Predicate(Box::new(is_a_grain)),
            ],
            prod: Box::new(|tokens| match tokens {
                [num, _, grain] => {
                    Some(Token::Time(cycle_nth_td(grain_of(grain)?, get_int_value(num)?)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "this|next <day-of-week>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(this|next|coming)")),
                PatternItem::Predicate(Box::new(is_a_day_of_week)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::RegexMatch(g), Token::Time(td)] => {
                    let word = g.first().map(|s| s.to_ascii_lowercase()).unwrap_or_default();
                    if word == "next" {
                        // the day-of-week falling in next week
                        Some(Token::Time(TimeData {
                            pred: intersect(td.pred.clone(), cycle_nth(Grain::Week, 1)),
                            grain: Grain::Day,
                            latent: false,
                            not_immediate: false,
                            form: td.form,
                            direction: None,
                            holiday: None,
                            has_timezone: false,
                        }))
                    } else if word == "this" {
                        // "this <dow>": predNth 0 notImmediate — a *single* pinned
                        // occurrence (the upcoming dow), so it survives intersection
                        // with a time-of-day. A bare dow's notImmediate lives in the
                        // series and is dropped when composed, which would let "this
                        // tuesday at 3" fall back to today when today is Tuesday.
                        Some(Token::Time(pred_nth_td(0, true, td)))
                    } else {
                        // "coming <dow>": Duckling has no dedicated rule; it behaves
                        // like the bare dow (notImmediate in the series), so "coming
                        // tuesday at 3" composes to today's Tuesday, unlike "this".
                        Some(Token::Time(not_latent(td.clone())))
                    }
                }
                _ => None,
            }),
        },
    ]
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
    td.form = Some(Form::PartOfDay { start_hour: start_hour as i8 });
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

/// Open-ended intervals: "until/before <time>" (to), "after/from <time>" (from).
fn direction_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "until <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(
                    r"(anytime |sometimes? )?(before|(un)?til(l)?|through|up to)",
                )),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td)] => {
                    Some(Token::Time(with_direction(IntervalDirection::Before, td.clone())))
                }
                _ => None,
            }),
        },
        Rule {
            name: "from|since|after <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"from|since|(anytime |sometimes? )?after")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td)] => {
                    Some(Token::Time(with_direction(IntervalDirection::After, td.clone())))
                }
                _ => None,
            }),
        },
    ]
}

fn dom_interval(m: &TimeData, t1: &Token, t2: &Token) -> Option<TimeData> {
    let d1 = intersect_dom(m, t1)?;
    let d2 = intersect_dom(m, t2)?;
    interval_td(IntervalType::Closed, &d1, &d2)
}

/// Day-of-month intervals within a month (ruleIntervalMonthDDDD family):
/// "July 13 to 15", "23rd to 26th Oct", "from 13 to 15 of July".
fn dom_interval_rules() -> Vec<Rule> {
    let sep = || compile(r"\-|to|th?ru|through|(un)?til(l)?");
    let dv = || PatternItem::Predicate(Box::new(is_dom_value));
    let am = || PatternItem::Predicate(Box::new(is_a_month));
    vec![
        Rule {
            name: "<month> dd-dd (interval)".into(),
            pattern: vec![am(), dv(), PatternItem::Regex(sep()), dv()],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(m), t1, _, t2] => dom_interval(m, t1, t2).map(Token::Time),
                _ => None,
            }),
        },
        Rule {
            name: "dd-dd <month> (interval)".into(),
            pattern: vec![dv(), PatternItem::Regex(sep()), dv(), am()],
            prod: Box::new(|tokens| match tokens {
                [t1, _, t2, Token::Time(m)] => dom_interval(m, t1, t2).map(Token::Time),
                _ => None,
            }),
        },
        Rule {
            name: "dd-dd of <month> (interval)".into(),
            pattern: vec![dv(), PatternItem::Regex(sep()), dv(), PatternItem::Regex(compile(r"of")), am()],
            prod: Box::new(|tokens| match tokens {
                [t1, _, t2, _, Token::Time(m)] => dom_interval(m, t1, t2).map(Token::Time),
                _ => None,
            }),
        },
        Rule {
            name: "from <month> dd-dd (interval)".into(),
            pattern: vec![PatternItem::Regex(compile(r"from")), am(), dv(), PatternItem::Regex(sep()), dv()],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(m), t1, _, t2] => dom_interval(m, t1, t2).map(Token::Time),
                _ => None,
            }),
        },
        Rule {
            name: "from the <day-of-month> (ordinal or number) to the <day-of-month> (ordinal or number) <named-month> (interval)".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"from( the)?")),
                dv(),
                PatternItem::Regex(compile(r"\-|to( the)?|th?ru|through|(un)?til(l)?")),
                dv(),
                am(),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, t1, _, t2, Token::Time(m)] => dom_interval(m, t1, t2).map(Token::Time),
                _ => None,
            }),
        },
        Rule {
            name: "from the <day-of-month> (ordinal or number) to the <day-of-month> (ordinal or number) of <named-month> (interval)".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"from( the)?")),
                dv(),
                PatternItem::Regex(compile(r"\-|to( the)?|th?ru|through|(un)?til(l)?")),
                dv(),
                PatternItem::Regex(compile(r"of")),
                am(),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, t1, _, t2, _, Token::Time(m)] => dom_interval(m, t1, t2).map(Token::Time),
                _ => None,
            }),
        },
    ]
}

fn interval_rules() -> Vec<Rule> {
    let sep = r"\-|to|th?ru|through|(un)?til(l)?";
    vec![
        // "1960 - 1961" (ruleIntervalYearLatent): two bare 4-digit years, y1<y2.
        Rule {
            name: "<year> (latent) - <year> (latent) (interval)".into(),
            pattern: vec![
                PatternItem::Predicate(is_integer_between(1000, 10000)),
                PatternItem::Regex(compile(sep)),
                PatternItem::Predicate(is_integer_between(1000, 10000)),
            ],
            prod: Box::new(|tokens| match tokens {
                [a, _, b] => {
                    let (y1, y2) = (get_int_value(a)?, get_int_value(b)?);
                    (y1 < y2)
                        .then(|| interval_td(IntervalType::Closed, &year_td(y1), &year_td(y2)))
                        .flatten()
                        .map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "<datetime> - <datetime> (interval)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_not_latent)),
                PatternItem::Regex(compile(sep)),
                PatternItem::Predicate(Box::new(is_not_latent)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(a), _, Token::Time(b)] if !tod_endpoint_mismatch(a, b) => {
                    interval_td(IntervalType::Closed, a, b).map(Token::Time)
                }
                _ => None,
            }),
        },
        // "2015-03-28 17:00:00/2015-03-29 21:00:00" (ruleIntervalSlash). The
        // sameGrain guard keeps "/" from matching mismatched-grain operands.
        Rule {
            name: "<datetime>/<datetime> (interval)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_not_latent)),
                PatternItem::Regex(compile(r"/")),
                PatternItem::Predicate(Box::new(is_not_latent)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(a), _, Token::Time(b)] if a.grain == b.grain => {
                    interval_td(IntervalType::Closed, a, b).map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "from <datetime> - <datetime> (interval)".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"from")),
                PatternItem::Predicate(Box::new(is_a_time)),
                PatternItem::Regex(compile(sep)),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(a), _, Token::Time(b)] if !tod_endpoint_mismatch(a, b) => {
                    interval_td(IntervalType::Closed, a, b).map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "between <time> and <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"between")),
                PatternItem::Predicate(Box::new(is_a_time)),
                PatternItem::Regex(compile(r"and")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(a), _, Token::Time(b)] if !tod_endpoint_mismatch(a, b) => {
                    interval_td(IntervalType::Closed, a, b).map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "<time-of-day> - <time-of-day> (interval)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(|t: &Token| {
                    is_not_latent(t) && is_a_time_of_day(t)
                })),
                PatternItem::Regex(compile(r"\-|:|to|th?ru|through|(un)?til(l)?")),
                PatternItem::Predicate(Box::new(is_a_time_of_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(a), _, Token::Time(b)] => {
                    interval_td(IntervalType::Closed, a, b).map(Token::Time)
                }
                _ => None,
            }),
        },
        // "later than 3:30pm but before 6pm" / "from 9 to 11" (ruleIntervalTODFrom).
        Rule {
            name: "from <time-of-day> - <time-of-day> (interval)".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(later than|from|(in[\s-])?between)")),
                PatternItem::Predicate(Box::new(is_a_time_of_day)),
                PatternItem::Regex(compile(r"((but )?before)|\-|to|th?ru|through|(un)?til(l)?")),
                PatternItem::Predicate(Box::new(is_a_time_of_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(a), _, Token::Time(b)] => {
                    interval_td(IntervalType::Closed, a, b).map(Token::Time)
                }
                _ => None,
            }),
        },
        // "hh(:mm) - <tod> am|pm": am/pm on the trailing time applies to both.
        Rule {
            name: "hh(:mm) - <time-of-day> am|pm".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(?:from )?((?:[01]?\d)|(?:2[0-3]))([:.]([0-5]\d))?")),
                PatternItem::Regex(compile(r"\-|to|th?ru|through|(un)?til(l)?")),
                PatternItem::Predicate(Box::new(is_a_time_of_day)),
                PatternItem::Regex(compile(r"(in the )?([ap])(\s|\.)?m?\.?")),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::RegexMatch(g1), _, Token::Time(td2), Token::RegexMatch(g4)] => {
                    let h: i64 = g1.first()?.parse().ok()?;
                    let m = g1.get(2).and_then(|s| s.parse::<i64>().ok());
                    let is_am = g4.get(1).map(|s| s.eq_ignore_ascii_case("a")).unwrap_or(false);
                    let td1 = match m {
                        Some(mm) => hour_minute_td(true, h, mm),
                        None => hour_td(true, h),
                    };
                    let a = time_of_day_ampm(is_am, &td1);
                    let b = time_of_day_ampm(is_am, td2);
                    interval_td(IntervalType::Closed, &a, &b).map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "by <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"by")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td)] => {
                    interval_td(IntervalType::Open, &now_td(), td).map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "<time> for <duration>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_not_latent)),
                PatternItem::Regex(compile(r"for")),
                PatternItem::Predicate(Box::new(is_a_duration)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td1), _, dur] => {
                    let (v, g) = duration_of(dur)?;
                    interval_td(IntervalType::Closed, td1, &duration_after_td(v, g, td1))
                        .map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "from <time> for <duration>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(from|starting|beginning|after|starting from)")),
                PatternItem::Predicate(Box::new(is_not_latent)),
                PatternItem::Regex(compile(r"for")),
                PatternItem::Predicate(Box::new(is_a_duration)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td1), _, dur] => {
                    let (v, g) = duration_of(dur)?;
                    interval_td(IntervalType::Closed, td1, &duration_after_td(v, g, td1))
                        .map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "for <duration> from <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"for")),
                PatternItem::Predicate(Box::new(is_a_duration)),
                PatternItem::Regex(compile(r"(from|starting|beginning|after|starting from)")),
                PatternItem::Predicate(Box::new(is_not_latent)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, dur, _, Token::Time(td1)] => {
                    let (v, g) = duration_of(dur)?;
                    interval_td(IntervalType::Closed, td1, &duration_after_td(v, g, td1))
                        .map(Token::Time)
                }
                _ => None,
            }),
        },
        // A time shifted by a duration: "15 minutes past 3pm", "10 mins before 5".
        Rule {
            name: "<duration> after|before|from|past <time>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_a_duration)),
                PatternItem::Regex(compile(r"(after|before|from|past)")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [dur, Token::RegexMatch(g), Token::Time(td)] => {
                    let (v, gr) = duration_of(dur)?;
                    let signed = if g.first()?.eq_ignore_ascii_case("before") { -v } else { v };
                    Some(Token::Time(duration_after_td(signed, gr, td)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "<integer> <named-day> ago|back".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(|t| get_int_value(t).is_some_and(|v| v >= 0))),
                PatternItem::Predicate(Box::new(is_a_day_of_week)),
                PatternItem::Regex(compile(r"ago|back")),
            ],
            prod: Box::new(|tokens| match tokens {
                [num, Token::Time(td), _] => {
                    Some(Token::Time(pred_nth_td(-get_int_value(num)?, false, td)))
                }
                _ => None,
            }),
        },
        // "3 fridays from now" -> the 3rd upcoming friday (predNth n-1).
        Rule {
            name: "<integer> <named-day> from now|hence".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(|t| get_int_value(t).is_some_and(|v| v >= 1))),
                PatternItem::Predicate(Box::new(is_a_day_of_week)),
                PatternItem::Regex(compile(r"from now|hence")),
            ],
            prod: Box::new(|tokens| match tokens {
                [num, Token::Time(td), _] => {
                    // notImmediate: on a Tuesday, "4 tuesdays from now" skips today.
                    Some(Token::Time(pred_nth_td(get_int_value(num)? - 1, true, td)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "<time> before last|after next".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_a_time)),
                PatternItem::Regex(compile(r"(before last|after next)")),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td), Token::RegexMatch(g)] => {
                    let after_next = g.first()?.eq_ignore_ascii_case("after next");
                    Some(Token::Time(pred_nth_td(1, after_next, td)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "last weekend of <named-month>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"last\s(week(\s|-)?end|wkend)\s(of|in)")),
                PatternItem::Predicate(Box::new(is_a_month)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(m)] => Some(Token::Time(pred_last_of_td(&weekend_td(), m))),
                _ => None,
            }),
        },
        // "March in a year", "thanksgiving in 9 months": the day/month time
        // intersected with the window one duration from now.
        Rule {
            name: "<day> in <duration>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(|t| {
                    matches!(t, Token::Time(td) if td.grain == Grain::Day || td.grain == Grain::Month)
                })),
                PatternItem::Regex(compile(r"in")),
                PatternItem::Predicate(Box::new(|t| matches!(t, Token::Duration(d) if d.grain > Grain::Hour))),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td), _, dur] => {
                    let (v, g) = duration_of(dur)?;
                    intersect_td(td, &in_duration_interval_td(v, g)?).map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "<day> <duration> hence|ago".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(|t| {
                    matches!(t, Token::Time(td) if td.grain == Grain::Day || td.grain == Grain::Month)
                })),
                PatternItem::Predicate(Box::new(is_a_duration)),
                PatternItem::Regex(compile(r"(from now|hence|ago)")),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td), dur, Token::RegexMatch(g)] => {
                    let (v, gr) = duration_of(dur)?;
                    let signed = if g.first()?.eq_ignore_ascii_case("ago") { -v } else { v };
                    intersect_td(td, &in_duration_interval_td(signed, gr)?).map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "by the end of <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"by (the )?end of")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td)] => {
                    interval_td(IntervalType::Closed, &now_td(), td).map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "in <duration> at <time-of-day>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"in")),
                PatternItem::Predicate(Box::new(|t| matches!(t, Token::Duration(d) if d.grain > Grain::Hour))),
                PatternItem::Regex(compile(r"at")),
                PatternItem::Predicate(Box::new(is_a_time_of_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, dur, _, Token::Time(td)] => {
                    let (v, g) = duration_of(dur)?;
                    intersect_td(td, &in_duration_interval_td(v, g)?).map(Token::Time)
                }
                _ => None,
            }),
        },
        // "all week" / "rest of the week" / "the week" (ruleWeek). End is two
        // days before next week's start.
        Rule {
            name: "week".into(),
            pattern: vec![PatternItem::Regex(compile(r"(all|rest of the|the) week"))],
            prod: Box::new(|tokens| {
                let m = regex_groups(tokens)?.first()?.to_lowercase();
                // End = two days before next week's start; a Day object whose
                // exclusive bound (Closed) is the reported "to" (Feb 17), which
                // is what the corpus expects for both "all" and "rest".
                let end = cycle_nth_after_td(true, Grain::Day, -2, &cycle_nth_td(Grain::Week, 1));
                let start = if m == "all" { cycle_nth_td(Grain::Week, 0) } else { today_td() };
                let period = interval_td(IntervalType::Closed, &start, &end)?;
                Some(Token::Time(if m == "the" { mk_latent(period) } else { period }))
            }),
        },
    ]
}

fn hour_interval(h1: i64, h2: i64) -> Option<TimeData> {
    interval_td(IntervalType::Open, &hour_td(false, h1), &hour_td(false, h2))
}

fn part_of_day_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "as soon as possible".into(),
            pattern: vec![PatternItem::Regex(compile(r"asap|as\ssoon\sas\spossible"))],
            prod: Box::new(|_| {
                Some(Token::Time(with_direction(IntervalDirection::After, now_td())))
            }),
        },
        Rule {
            name: "last night".into(),
            pattern: vec![PatternItem::Regex(compile(r"(late )?last night"))],
            prod: Box::new(|tokens| {
                let m = regex_groups(tokens)?.first()?.to_lowercase();
                let hours = if m == "late " { 3 } else { 6 };
                let start = duration_after_td(-hours, Grain::Hour, &today_td());
                let iv = interval_td(IntervalType::Open, &start, &today_td())?;
                Some(Token::Time(part_of_day(24 - hours, iv)))
            }),
        },
        Rule {
            name: "week-end".into(),
            pattern: vec![PatternItem::Regex(compile(r"(week(\s|-)?end|wkend)s?"))],
            prod: Box::new(|_| {
                // Tag as a part-of-day (sentinel start_hour) so this/last/next <time>
                // compose it (Duckling's mkOkForThisNext), while marking it multi-day
                // so the same-day am/pm rule skips it. Resolution unchanged.
                let mut td = weekend_td();
                td.form = Some(Form::PartOfDay { start_hour: WEEKEND_POD_HOUR });
                Some(Token::Time(td))
            }),
        },
        Rule {
            name: "after lunch/work/school".into(),
            pattern: vec![PatternItem::Regex(compile(r"after[\s-]?(lunch|work|school)"))],
            prod: Box::new(|tokens| {
                let m = regex_groups(tokens)?.first()?.to_lowercase();
                let (s, e) = match m.as_str() {
                    "lunch" => (13, 17),
                    "work" => (17, 21),
                    "school" => (15, 21),
                    _ => return None,
                };
                let iv = interval_td(IntervalType::Open, &hour_td(false, s), &hour_td(false, e))?;
                Some(Token::Time(part_of_day(s, intersect_td(&today_td(), &iv)?)))
            }),
        },
        Rule {
            name: "part of days".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(morning|after ?noo?n(ish)?|evening|night|(at )?lunch)",
            ))],
            prod: Box::new(|tokens| {
                let m = regex_groups(tokens)?.first()?.to_lowercase();
                let (h1, h2) = if m.contains("morning") {
                    (0, 12)
                } else if m.contains("evening") || m.contains("night") {
                    (18, 0)
                } else if m.contains("lunch") {
                    (12, 14)
                } else {
                    (12, 19) // afternoon
                };
                Some(Token::Time(part_of_day(h1, mk_latent(hour_interval(h1, h2)?))))
            }),
        },
        Rule {
            name: "early morning".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"early ((in|hours of) the )?morning",
            ))],
            prod: Box::new(|_| Some(Token::Time(part_of_day(0, mk_latent(hour_interval(0, 9)?))))),
        },
        Rule {
            name: "in|during the <part-of-day>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(in|during)( the)?")),
                PatternItem::Predicate(Box::new(is_a_part_of_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td)] => Some(Token::Time(not_latent(td.clone()))),
                _ => None,
            }),
        },
        Rule {
            name: "this <part-of-day>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"this")),
                PatternItem::Predicate(Box::new(is_a_part_of_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td)] => {
                    let start = pod_start_hour(td)?;
                    intersect_td(&today_td(), td).map(|t| Token::Time(part_of_day(start, t)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "tonight".into(),
            pattern: vec![PatternItem::Regex(compile(r"(late )?toni(ght|gth|te)s?"))],
            prod: Box::new(|tokens| {
                let m = regex_groups(tokens)?.first()?.to_lowercase();
                let h = if m.contains("late") { 21 } else { 18 };
                let evening = hour_interval(h, 0)?;
                intersect_td(&today_td(), &evening).map(|t| Token::Time(part_of_day(h, t)))
            }),
        },
        // "this evening at 2" -> the part-of-day disambiguates the bare hour's
        // am/pm: PM unless the pod starts before noon, or the hour is 12 (->AM).
        Rule {
            name: "<part-of-day> at <time-of-day>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_same_day_part_of_day)),
                PatternItem::Regex(compile(r"at|@")),
                PatternItem::Predicate(Box::new(is_a_time_of_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(pod), _, Token::Time(tod)] => {
                    let start = pod_start_hour(pod)?;
                    let hours = match tod.form {
                        Some(Form::TimeOfDay { hours: Some(h), is12h: true, .. }) => h as i64,
                        _ => return None,
                    };
                    let is_am = start < 12 || hours == 12;
                    Some(Token::Time(time_of_day_ampm(is_am, tod)))
                }
                _ => None,
            }),
        },
    ]
}

fn is_grain_of_time_day(t: &Token) -> bool {
    matches!(t, Token::Time(td) if td.grain == Grain::Day)
}

/// Absorb connective words so the surrounded time can intersect (ruleAbsorbOnDay,
/// ruleAbsorbOnADOW, ruleAbsorbCommaTOD). e.g. "on Thursday", "Monday,".
fn absorb_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "on <day>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"on")),
                PatternItem::Predicate(Box::new(is_grain_of_time_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td)] => Some(Token::Time(td.clone())),
                _ => None,
            }),
        },
        Rule {
            name: "on a <named-day>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"on a")),
                PatternItem::Predicate(Box::new(is_a_day_of_week)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td)] => Some(Token::Time(td.clone())),
                _ => None,
            }),
        },
        Rule {
            name: "absorption of , after named day".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_a_day_of_week)),
                PatternItem::Regex(compile(r",")),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td), _] => Some(Token::Time(td.clone())),
                _ => None,
            }),
        },
    ]
}

/// Named timezone -> fixed offset in minutes (subset of parseTimezone).
const TZ: &[(&str, i64)] = &[
    ("GMT", 0), ("UTC", 0), ("WET", 0),
    ("BST", 60), ("CET", 60), ("WAT", 60), ("WEST", 60),
    ("CEST", 120), ("EET", 120), ("SAST", 120),
    ("EEST", 180), ("EAT", 180), ("MSK", 180),
    ("IST", 330),
    ("PST", -480), ("PDT", -420), ("MST", -420), ("MDT", -360),
    ("CST", -360), ("CDT", -300), ("EST", -300), ("EDT", -240),
    ("AST", -240), ("ADT", -180), ("AKST", -540), ("AKDT", -480), ("HST", -600),
    ("JST", 540), ("KST", 540), ("AEST", 600), ("AEDT", 660),
    ("ACST", 570), ("AWST", 480), ("NZST", 720), ("NZDT", 780),
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

/// "<time-of-day> <timezone>" (ruleTimezone): shift the time into the frame.
fn timezone_rules() -> Vec<Rule> {
    let alt = TZ.iter().map(|(n, _)| *n).collect::<Vec<_>>().join("|");
    let tz_re = format!(r"\b({alt})\b");
    vec![
        Rule {
            name: "<time> timezone".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(|t| is_not_latent(t) && is_a_time_of_day(t))),
                PatternItem::Regex(compile(&tz_re)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td), Token::RegexMatch(g)] => {
                    let off = tz_offset(g.first()?)?;
                    Some(Token::Time(in_timezone_td(off, td)))
                }
                _ => None,
            }),
        },
        // "9 am (BST)": timezone in parentheses (ruleTimezoneBracket).
        Rule {
            name: "<time> (timezone)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(|t| {
                    is_not_latent(t) && is_a_time_of_day(t) && has_no_timezone(t)
                })),
                PatternItem::Regex(compile(&format!(r"\(({alt})\)"))),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td), Token::RegexMatch(g)] => {
                    let off = tz_offset(g.first()?)?;
                    Some(Token::Time(in_timezone_td(off, td)))
                }
                _ => None,
            }),
        },
        // "9:30 - 11:00 CST": one trailing timezone applies to both ends. The
        // hasNoTimezone guards skip already-tz'd ends ("15:00 GMT - 18:00 GMT",
        // handled per-end) so the tz isn't applied twice.
        Rule {
            name: "<datetime> - <datetime> (interval) timezone".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(|t| is_a_time_of_day(t) && has_no_timezone(t))),
                PatternItem::Regex(compile(r"\-|to|th?ru|through|(un)?til(l)?")),
                PatternItem::Predicate(Box::new(|t| is_a_time_of_day(t) && has_no_timezone(t))),
                PatternItem::Regex(compile(&tz_re)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(a), _, Token::Time(b), Token::RegexMatch(g)] => {
                    let off = tz_offset(g.first()?)?;
                    // Build the interval from the raw ends first (resolves the
                    // 12h hh:mm ambiguity to the daytime occurrence), then shift
                    // the whole interval into the timezone.
                    let iv = interval_td(IntervalType::Closed, a, b)?;
                    Some(Token::Time(in_timezone_td(off, &iv)))
                }
                _ => None,
            }),
        },
    ]
}

/// Generic intersection of two adjacent times (ports of ruleIntersect /
/// ruleIntersectOf). Composes dates+years, dow+month-day, time-on-day, etc.
fn intersect_rules() -> Vec<Rule> {
    vec![
        // "April 14, 2015": intersect a non-latent time with a (latent) year.
        Rule {
            name: "intersect by \",\", \"of\", \"from\" for year".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_not_latent)),
                PatternItem::Regex(compile(r"of|from|,")),
                PatternItem::Predicate(is_grain_of_time(Grain::Year)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(a), _, Token::Time(b)] => {
                    intersect_td(a, b).map(|t| Token::Time(not_latent(t)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "intersect".into(),
            pattern: vec![
                PatternItem::Predicate(grain_finer_than(Grain::Year)),
                PatternItem::Predicate(Box::new(|t| is_not_latent(t) || is_grain_of_year(t))),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(a), Token::Time(b)] if !a.latent || !b.latent => {
                    intersect_td(a, b).map(|t| Token::Time(not_latent(t)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "intersect by \",\", \"of\", \"from\", \"'s\"".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_not_latent)),
                PatternItem::Regex(compile(r"of|from|for|'s|,|@")),
                PatternItem::Predicate(Box::new(is_not_latent)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(a), _, Token::Time(b)] => {
                    intersect_td(a, b).map(|t| Token::Time(not_latent(t)))
                }
                _ => None,
            }),
        },
    ]
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

/// Relative-duration rules (ports of ruleIntervalForDurations / inDuration etc).
fn duration_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "in|within|after <duration>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(in|within|after)")),
                PatternItem::Predicate(Box::new(is_a_duration)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::RegexMatch(g), dur] => {
                    let (v, gr) = duration_of(dur)?;
                    let w = g.first()?.to_lowercase();
                    match w.as_str() {
                        "within" => interval_td(IntervalType::Open, &now_td(), &in_duration_td(v, gr))
                            .map(Token::Time),
                        // "after 5 days" -> open interval starting at that point.
                        "after" => Some(Token::Time(with_direction(
                            IntervalDirection::After,
                            in_duration_td(v, gr),
                        ))),
                        _ => Some(Token::Time(in_duration_td(v, gr))),
                    }
                }
                _ => None,
            }),
        },
        Rule {
            name: "<duration> hence|ago".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_a_duration)),
                PatternItem::Regex(compile(r"(from now|hence|ago)")),
            ],
            prod: Box::new(|tokens| match tokens {
                [dur, Token::RegexMatch(g)] => {
                    let (v, gr) = duration_of(dur)?;
                    let signed = if g.first()?.eq_ignore_ascii_case("ago") { -v } else { v };
                    Some(Token::Time(in_duration_td(signed, gr)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "last|past|next <duration>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"([lp]ast|next)")),
                PatternItem::Predicate(Box::new(is_a_duration)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::RegexMatch(g), dur] => {
                    let (v, gr) = duration_of(dur)?;
                    let w = g.first()?.to_lowercase();
                    let n = if w == "last" || w == "past" { -v } else { v };
                    Some(Token::Time(cycle_n_td(gr, n)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "in <number> (implicit minutes)".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"in")),
                PatternItem::Predicate(is_integer_between(0, 60)),
            ],
            prod: Box::new(|tokens| {
                let n = get_int_value(tokens.get(1)?)?;
                Some(Token::Time(in_duration_td(n, Grain::Minute)))
            }),
        },
    ]
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
    let mut td = interval_td(IntervalType::Closed, &month_day_td(sm, sd), &month_day_td(em, ed))?;
    td.form = Some(Form::Season);
    Some(td)
}

fn season_rules() -> Vec<Rule> {
    let mut rules = vec![Rule {
        name: "last|this|next <season>".into(),
        pattern: vec![PatternItem::Regex(compile(
            r"(this|current|next|last|past|previous) seasons?",
        ))],
        prod: Box::new(|tokens| {
            let w = regex_groups(tokens)?.first()?.to_lowercase();
            let n = match w.as_str() {
                "this" | "current" => 0,
                "last" | "past" | "previous" => -1,
                "next" => 1,
                _ => return None,
            };
            let mut td = TimeData::new(take_nth(n, false, season_series()), Grain::Day);
            td.form = Some(Form::Season);
            Some(Token::Time(td))
        }),
    }];
    let seasons: [(&str, &str, i64, i64, i64, i64); 4] = [
        ("summer", r"summer", 6, 21, 9, 23),
        ("fall", r"fall|autumn", 9, 23, 12, 21),
        ("winter", r"winter", 12, 21, 3, 20),
        ("spring", r"spring", 3, 20, 6, 21),
    ];
    rules.extend(seasons.iter().map(|&(name, re, sm, sd, em, ed)| Rule {
        name: format!("season {name}"),
        pattern: vec![PatternItem::Regex(compile(re))],
        prod: Box::new(move |_| season_td(sm, sd, em, ed).map(Token::Time)),
    }));
    rules
}

fn time_pod_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "<time> <part-of-day>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_a_time)),
                PatternItem::Predicate(Box::new(is_a_part_of_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td), Token::Time(pod)] => intersect_td(pod, td).map(Token::Time),
                _ => None,
            }),
        },
        Rule {
            name: "<part-of-day> of <time>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_a_part_of_day)),
                PatternItem::Regex(compile(r"of")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(pod), _, Token::Time(td)] => intersect_td(pod, td).map(Token::Time),
                _ => None,
            }),
        },
    ]
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
    TimeData::new(take_last_of(cyclic.pred.clone(), base.pred.clone()), cyclic.grain)
}

/// The n-th occurrence of a cyclic time at/after `base` (port of predNthAfter,
/// notImmediate=true), e.g. "first monday of last month". Grain from cyclic.
fn pred_nth_after_td(n: i64, cyclic: &TimeData, base: &TimeData) -> TimeData {
    let mut td =
        TimeData::new(take_nth_after(n, true, cyclic.pred.clone(), base.pred.clone()), cyclic.grain);
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
    let mut td = TimeData::new(take_nth_closest(n, td1.pred.clone(), td2.pred.clone()), td1.grain);
    td.holiday = td1.holiday.clone();
    td
}

/// <cycle> after/before <time>, and <ordinal> <cycle> of <time>
/// (ruleCycleAfterBeforeTime, ruleCycleOrdinalOfTime).
/// "the day after tomorrow", "day before yesterday", "first week of october".
fn cycle_after_before_rules() -> Vec<Rule> {
    fn after_before(tokens: &[Token], gi: usize, mi: usize, ti: usize) -> Option<Token> {
        let g = grain_of(tokens.get(gi)?)?;
        let m = match tokens.get(mi)? {
            Token::RegexMatch(m) => m.first()?,
            _ => return None,
        };
        let n = if m.eq_ignore_ascii_case("after") { 1 } else { -1 };
        match tokens.get(ti)? {
            Token::Time(td) => Some(Token::Time(cycle_nth_after_td(false, g, n, td))),
            _ => None,
        }
    }
    vec![
        Rule {
            name: "the <cycle> after|before <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"the")),
                PatternItem::Predicate(Box::new(is_a_grain)),
                PatternItem::Regex(compile(r"(after|before)")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| after_before(tokens, 1, 2, 3)),
        },
        Rule {
            name: "<cycle> after|before <time>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_a_grain)),
                PatternItem::Regex(compile(r"(after|before)")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| after_before(tokens, 0, 1, 2)),
        },
        Rule {
            name: "<ordinal> <cycle> of <time>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Predicate(Box::new(is_a_grain)),
                PatternItem::Regex(compile(r"of|in|from")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [ord, Token::TimeGrain(g), _, Token::Time(td)] => {
                    let n = get_int_value(ord)?;
                    // notImmediate=true (ruleCycleOrdinalOfTime): "first week of
                    // October 2014" skips the week that merely covers Oct 1.
                    Some(Token::Time(cycle_nth_after_td(true, *g, n - 1, td)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "last <day-of-week> of <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"last")),
                PatternItem::Predicate(Box::new(is_a_day_of_week)),
                PatternItem::Regex(compile(r"(of|in)")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(dow), _, Token::Time(td)] => {
                    Some(Token::Time(pred_last_of_td(dow, td)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "last <cycle> of <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"last")),
                PatternItem::Predicate(Box::new(is_a_grain)),
                PatternItem::Regex(compile(r"of|in")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::TimeGrain(g), _, Token::Time(td)] => {
                    Some(Token::Time(cycle_last_of_td(*g, td)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "<ordinal> last <cycle> of <time>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Regex(compile(r"last")),
                PatternItem::Predicate(Box::new(is_a_grain)),
                PatternItem::Regex(compile(r"of|in|from")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [ord, _, Token::TimeGrain(g), _, Token::Time(td)] => {
                    let n = get_int_value(ord)?;
                    let inner = cycle_nth_after_td(true, td.grain, 1, td);
                    Some(Token::Time(cycle_nth_after_td(true, *g, -n, &inner)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "the <ordinal> <cycle> of <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"the")),
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Predicate(Box::new(is_a_grain)),
                PatternItem::Regex(compile(r"of|in|from")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, ord, Token::TimeGrain(g), _, Token::Time(td)] => {
                    let n = get_int_value(ord)?;
                    Some(Token::Time(cycle_nth_after_td(true, *g, n - 1, td)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "the <ordinal> last <cycle> of <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"the")),
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Regex(compile(r"last")),
                PatternItem::Predicate(Box::new(is_a_grain)),
                PatternItem::Regex(compile(r"of|in|from")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, ord, _, Token::TimeGrain(g), _, Token::Time(td)] => {
                    let n = get_int_value(ord)?;
                    let inner = cycle_nth_after_td(true, td.grain, 1, td);
                    Some(Token::Time(cycle_nth_after_td(true, *g, -n, &inner)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "the <cycle> of <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"the")),
                PatternItem::Predicate(Box::new(is_a_grain)),
                PatternItem::Regex(compile(r"of")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::TimeGrain(g), _, Token::Time(td)] => {
                    Some(Token::Time(cycle_nth_after_td(true, *g, 0, td)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "the closest <day> to <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"the\s+closest")),
                PatternItem::Predicate(is_grain_of_time(Grain::Day)),
                PatternItem::Regex(compile(r"to")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(td1), _, Token::Time(td2)] => {
                    Some(Token::Time(pred_nth_closest_td(0, td1, td2)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "the <ordinal> closest <day> to <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"the")),
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Regex(compile(r"closest")),
                PatternItem::Predicate(is_grain_of_time(Grain::Day)),
                PatternItem::Regex(compile(r"to")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, ord, _, Token::Time(td1), _, Token::Time(td2)] => {
                    let n = get_int_value(ord)?;
                    Some(Token::Time(pred_nth_closest_td(n - 1, td1, td2)))
                }
                _ => None,
            }),
        },
    ]
}

/// <ordinal> quarter [<year>], "the <ordinal> quarter", "Q<n>" (ruleQuarter*).
fn quarter_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "<ordinal> quarter".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Predicate(Box::new(is_grain_quarter)),
            ],
            prod: Box::new(|tokens| {
                let n = get_int_value(tokens.first()?)?;
                Some(Token::Time(cycle_nth_after_td(true, Grain::Quarter, n - 1, &cycle_nth_td(Grain::Year, 0))))
            }),
        },
        Rule {
            name: "the <ordinal> quarter".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"the")),
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Predicate(Box::new(is_grain_quarter)),
            ],
            prod: Box::new(|tokens| {
                let n = get_int_value(tokens.get(1)?)?;
                Some(Token::Time(cycle_nth_after_td(true, Grain::Quarter, n - 1, &cycle_nth_td(Grain::Year, 0))))
            }),
        },
        Rule {
            name: "<ordinal> quarter <year>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Predicate(Box::new(is_grain_quarter)),
                PatternItem::Predicate(is_grain_of_time(Grain::Year)),
            ],
            prod: Box::new(|tokens| match tokens {
                [ord, _, Token::Time(td)] => {
                    let n = get_int_value(ord)?;
                    Some(Token::Time(cycle_nth_after_td(false, Grain::Quarter, n - 1, td)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "Q<n>".into(),
            pattern: vec![PatternItem::Regex(compile(r"q([1-4])"))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                let n: i64 = g.first()?.parse().ok()?;
                Some(Token::Time(cycle_nth_after_td(true, Grain::Quarter, n - 1, &cycle_nth_td(Grain::Year, 0))))
            }),
        },
    ]
}
fn is_grain_month_or_coarser(t: &Token) -> bool {
    matches!(t, Token::Time(td) if td.grain >= Grain::Month)
}

/// "<ordinal> <day-of-week> of <month-or-greater>" (ruleNthTimeOfTime).
/// e.g. "third tuesday of september 2014" = 3rd Tuesday in that September.
fn nth_dow_of_time_rules() -> Vec<Rule> {
    vec![
        // "third tuesday after christmas 2014" (ruleNthTimeAfterTime):
        // predNthAfter(n-1, td1, td2).
        Rule {
            name: "nth <time> after <time>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Predicate(Box::new(is_a_time)),
                PatternItem::Regex(compile(r"after")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [ord, Token::Time(a), _, Token::Time(b)] => {
                    Some(Token::Time(pred_nth_after_td(get_int_value(ord)? - 1, a, b)))
                }
                _ => None,
            }),
        },
        // first|second|third|fourth|fifth <day-of-week> of <time> (any time),
        // via predNthAfter — "first monday of last month", "3rd tue of Sep 2014".
        Rule {
            name: "first|second|third|fourth|fifth <day-of-week> of <time>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(|t| {
                    is_ordinal(t) && get_int_value(t).is_some_and(|v| (1..=5).contains(&v))
                })),
                PatternItem::Predicate(Box::new(is_a_day_of_week)),
                PatternItem::Regex(compile(r"(of|in)")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [ord, Token::Time(dow), _, Token::Time(td)] => {
                    Some(Token::Time(pred_nth_after_td(get_int_value(ord)? - 1, dow, td)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "nth <day-of-week> of <month-or-greater>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Predicate(Box::new(is_a_day_of_week)),
                PatternItem::Regex(compile(r"of|in")),
                PatternItem::Predicate(Box::new(is_grain_month_or_coarser)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Ordinal(od), Token::Time(dow), _, Token::Time(td2)] => {
                    let inter = intersect_td(td2, dow)?;
                    Some(Token::Time(pred_nth_td(od.value - 1, false, &inter)))
                }
                _ => None,
            }),
        },
        // Same, consuming a leading "the" (ruleTheNthTimeOfTime).
        Rule {
            name: "the nth <day-of-week> of <month-or-greater>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"the")),
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Predicate(Box::new(is_a_day_of_week)),
                PatternItem::Regex(compile(r"of|in")),
                PatternItem::Predicate(Box::new(is_grain_month_or_coarser)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Ordinal(od), Token::Time(dow), _, Token::Time(td2)] => {
                    let inter = intersect_td(td2, dow)?;
                    Some(Token::Time(pred_nth_td(od.value - 1, false, &inter)))
                }
                _ => None,
            }),
        },
    ]
}

/// this/next/last <time> (ports of ruleThisTime / ruleNextTime / ruleLastTime).
fn this_next_last_time_rules() -> Vec<Rule> {
    fn rule(name: &str, re: &str, n: i64, not_immediate: bool) -> Rule {
        Rule {
            name: name.to_string(),
            pattern: vec![
                PatternItem::Regex(compile(re)),
                PatternItem::Predicate(Box::new(is_ok_with_this_next)),
            ],
            prod: Box::new(move |tokens| match tokens {
                [_, Token::Time(td)] => Some(Token::Time(pred_nth_td(n, not_immediate, td))),
                _ => None,
            }),
        }
    }
    vec![
        rule("this <time>", r"this|current|coming", 0, false),
        rule("next <time>", r"next", 0, true),
        rule("last <time>", r"(this past|last|previous)", -1, false),
    ]
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

/// end/beginning of year & week (ports of ruleEndOfYear/BeginningOfYear,
/// ruleEndOrBeginningOfYear/Week). Bounds oracle-verified.
fn end_beginning_year_week_rules() -> Vec<Rule> {
    fn cy(n: i64) -> TimeData {
        cycle_nth_td(Grain::Year, n)
    }
    fn mo_of(y: &TimeData, m: i64) -> TimeData {
        intersect_td(&month_td(m), y).expect("month-of-year")
    }
    vec![
        Rule {
            name: "by end of year".into(),
            pattern: vec![PatternItem::Regex(compile(r"by (?:the )?(?:eoy|end of (?:the )?year)"))],
            prod: Box::new(|_| {
                interval_td(IntervalType::Open, &now_td(), &mo_of(&cy(1), 1)).map(Token::Time)
            }),
        },
        Rule {
            name: "end of year".into(),
            pattern: vec![PatternItem::Regex(compile(r"(?:(?:at )?the )?(?:eoy|end of (?:the )?year)"))],
            prod: Box::new(|_| {
                interval_td(IntervalType::Closed, &mo_of(&cy(0), 9), &mo_of(&cy(0), 12)).map(Token::Time)
            }),
        },
        Rule {
            name: "beginning of year".into(),
            pattern: vec![PatternItem::Regex(compile(r"(?:(?:at )?the )?(?:boy|beginning of (?:the )?year)"))],
            prod: Box::new(|_| {
                interval_td(IntervalType::Open, &mo_of(&cy(0), 1), &mo_of(&cy(0), 4)).map(Token::Time)
            }),
        },
        Rule {
            name: "at the beginning|end of <year>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(?:at the )?(beginning|end) of")),
                PatternItem::Predicate(is_grain_of_time(Grain::Year)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::RegexMatch(g), Token::Time(td)] => {
                    if g.first()?.eq_ignore_ascii_case("beginning") {
                        interval_td(IntervalType::Open, &intersect_td(&month_td(1), td)?, &intersect_td(&month_td(4), td)?).map(Token::Time)
                    } else {
                        interval_td(IntervalType::Closed, &intersect_td(&month_td(9), td)?, &intersect_td(&month_td(12), td)?).map(Token::Time)
                    }
                }
                _ => None,
            }),
        },
        Rule {
            name: "at the beginning|end of <week>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(?:at the )?(beginning|end) of")),
                PatternItem::Predicate(is_grain_of_time(Grain::Week)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::RegexMatch(g), Token::Time(td)] => {
                    let (sd, ed) = if g.first()?.eq_ignore_ascii_case("beginning") {
                        (1, 3)
                    } else {
                        (5, 7)
                    };
                    interval_td(IntervalType::Closed, &intersect_td(&day_of_week_td(sd), td)?, &intersect_td(&day_of_week_td(ed), td)?).map(Token::Time)
                }
                _ => None,
            }),
        },
    ]
}

/// end-of-month / beginning-of-month (ports of ruleEndOfMonth/ruleBeginningOfMonth).
fn end_beginning_of_month_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "by end of month".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"by (?:the )?(?:eom|end of (?:the )?month)",
            ))],
            prod: Box::new(|_| {
                interval_td(IntervalType::Open, &now_td(), &dom_of_next_month(1)?).map(Token::Time)
            }),
        },
        Rule {
            name: "end of month".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(?:(?:at )?the )?(?:eom|end of (?:the )?month)",
            ))],
            prod: Box::new(|_| {
                interval_td(IntervalType::Open, &dom_of_this_month(21)?, &dom_of_next_month(1)?)
                    .map(Token::Time)
            }),
        },
        Rule {
            name: "beginning of month".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(?:(?:at )?the )?(?:bom|beginning of (?:the )?month)",
            ))],
            prod: Box::new(|_| {
                interval_td(IntervalType::Closed, &dom_of_this_month(1)?, &dom_of_this_month(10)?)
                    .map(Token::Time)
            }),
        },
    ]
}

fn month_num(s: &str) -> Option<i64> {
    let s = s.to_lowercase();
    ["jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec"]
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

/// Numeric date formats (US order: M/D/Y). Ports of the mm/dd(/yyyy), dd/mon/yyyy,
/// mm/yyyy rules.
fn numeric_date_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "yyyy-mm-dd".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(\d{2,4})-(0?[1-9]|1[0-2])-(3[01]|[12]\d|0?[1-9])",
            ))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                year_month_day_td(parse_i(g, 0)?, parse_i(g, 1)?, parse_i(g, 2)?).map(Token::Time)
            }),
        },
        Rule {
            name: "yyyy-mm".into(),
            pattern: vec![PatternItem::Regex(compile(r"(\d{4})\s*[/-]\s*(1[0-2]|0?[1-9])"))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                year_month_td(parse_i(g, 0)?, parse_i(g, 1)?).map(Token::Time)
            }),
        },
        Rule {
            name: "yyyyqq".into(),
            pattern: vec![PatternItem::Regex(compile(r"(\d{2,4})q([1-4])"))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                let q = parse_i(g, 1)?;
                Some(Token::Time(cycle_nth_after_td(
                    true,
                    Grain::Quarter,
                    q - 1,
                    &year_td(parse_i(g, 0)?),
                )))
            }),
        },
        Rule {
            name: "mm/dd/yyyy".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(\d{1,2})[-/.](\d{1,2})[-/.](\d{2,4})",
            ))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                year_month_day_td(parse_i(g, 2)?, parse_i(g, 0)?, parse_i(g, 1)?).map(Token::Time)
            }),
        },
        Rule {
            name: "dd/mon/yyyy".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(\d{1,2})(?:st|nd|rd|th)?[-/.\s](jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*[-/.\s](\d{2,4})",
            ))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                let m = month_num(g.get(1)?)?;
                year_month_day_td(parse_i(g, 2)?, m, parse_i(g, 0)?).map(Token::Time)
            }),
        },
        Rule {
            name: "mm/yyyy".into(),
            pattern: vec![PatternItem::Regex(compile(r"(0?[1-9]|1[0-2])[/-](\d{4})"))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                year_month_td(parse_i(g, 1)?, parse_i(g, 0)?).map(Token::Time)
            }),
        },
        Rule {
            name: "mm/dd".into(),
            pattern: vec![PatternItem::Regex(compile(r"(\d{1,2})\s*[/-]\s*(\d{1,2})"))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                let (m, d) = (parse_i(g, 0)?, parse_i(g, 1)?);
                if !valid_md(m, d) {
                    return None;
                }
                Some(Token::Time(month_day_td(m, d)))
            }),
        },
    ]
}

/// Fixed-date / nth-weekday / last-weekday holidays (port of mkRuleHolidays).
/// Computed/lunar holidays (Easter, Chinese NY, …) need precomputed tables and
/// are out of scope here. dow: Mon=1..Sun=7.
fn holiday_rules() -> Vec<Rule> {
    vec![
        // Fixed dates, year over year (port of rulePeriodicHolidays monthDay entries).
        holiday_rule("Africa Day", r"africa(n (freedom|liberation))? day", || month_day_td(5, 25)),
        holiday_rule("Africa Industrialization Day", r"africa industrialization day", || month_day_td(11, 20)),
        holiday_rule("All Saints' Day", r"all saints' day", || month_day_td(11, 1)),
        holiday_rule("All Souls' Day", r"all souls' day", || month_day_td(11, 2)),
        holiday_rule("April Fools", r"(april|all) fool'?s('? day)?", || month_day_td(4, 1)),
        holiday_rule("Arabic Language Day", r"arabic language day", || month_day_td(12, 18)),
        holiday_rule("Assumption of Mary", r"assumption of mary", || month_day_td(8, 15)),
        holiday_rule("Boxing Day", r"boxing day", || month_day_td(12, 26)),
        holiday_rule("Chinese Language Day", r"chinese language day", || month_day_td(4, 20)),
        holiday_rule("Christmas", r"(xmas|christmas)( day)?", || month_day_td(12, 25)),
        holiday_rule("Christmas Eve", r"(xmas|christmas)( day)?('s)? eve", || month_day_td(12, 24)),
        holiday_rule("Day of Remembrance for all Victims of Chemical Warfare", r"day of remembrance for all victims of chemical warfare", || month_day_td(4, 29)),
        holiday_rule("Day of Remembrance of the Victims of the Rwanda Genocide", r"day of remembrance of the victims of the rwanda genocide", || month_day_td(4, 7)),
        holiday_rule("Day of the Seafarer", r"day of the seafarer", || month_day_td(6, 25)),
        holiday_rule("Earth Day", r"earth day", || month_day_td(4, 22)),
        holiday_rule("English Language Day", r"english language day", || month_day_td(4, 23)),
        holiday_rule("Epiphany", r"Epiphany", || month_day_td(1, 6)),
        holiday_rule("Feast of St Francis of Assisi", r"feast of st\.? francis of assisi", || month_day_td(10, 4)),
        holiday_rule("Feast of the Immaculate Conception", r"feast of the immaculate conception", || month_day_td(12, 8)),
        holiday_rule("Global Day of Parents", r"global day of parents", || month_day_td(6, 1)),
        holiday_rule("Halloween", r"hall?owe?en( day)?", || month_day_td(10, 31)),
        holiday_rule("Human Rights Day", r"human rights? day", || month_day_td(12, 10)),
        holiday_rule("International Albinism Awareness Day", r"international albinism awareness day", || month_day_td(6, 13)),
        holiday_rule("International Anti-Corruption Day", r"international anti(\-|\s)corruption day", || month_day_td(12, 9)),
        holiday_rule("International Asteroid Day", r"international asteroid day", || month_day_td(6, 30)),
        holiday_rule("International Celebrate Bisexuality Day", r"international celebrate bisexuality day", || month_day_td(9, 23)),
        holiday_rule("International Chernobyl Disaster Remembrance Day", r"international chernobyl disaster remembrance day", || month_day_td(4, 26)),
        holiday_rule("International Civil Aviation Day", r"international civil aviation day", || month_day_td(12, 7)),
        holiday_rule("International Customs Day", r"international customs day", || month_day_td(1, 26)),
        holiday_rule("International Day Against Drug Abuse and Illicit Trafficking", r"international day against drug abuse and illicit trafficking", || month_day_td(6, 26)),
        holiday_rule("International Day against Nuclear Tests", r"international day against nuclear tests", || month_day_td(8, 29)),
        holiday_rule("International Day for Biological Diversity", r"international day for biological diversity|world biodiversity day", || month_day_td(5, 22)),
        holiday_rule("International Day for Monuments and Sites", r"international day for monuments and sites", || month_day_td(4, 18)),
        holiday_rule("International Day for Preventing the Exploitation of the Environment in War and Armed Conflict", r"international day for preventing the exploitation of the environment in war and armed conflict", || month_day_td(11, 6)),
        holiday_rule("International Day for South-South Cooperation", r"international day for south(\-|\s)south cooperation", || month_day_td(9, 12)),
        holiday_rule("International Day for Tolerance", r"international day for tolerance", || month_day_td(11, 16)),
        holiday_rule("International Day for the Abolition of Slavery", r"international day for the abolition of slavery", || month_day_td(12, 2)),
        holiday_rule("International Day for the Elimination of Racial Discrimination", r"international day for the elimination of racial discrimination", || month_day_td(3, 21)),
        holiday_rule("International Day for the Elimination of Sexual Violence in Conflict", r"international day for the elimination of sexual violence in conflict", || month_day_td(6, 19)),
        holiday_rule("International Day for the Elimination of Violence against Women", r"international day for the elimination of violence against women", || month_day_td(11, 25)),
        holiday_rule("International Day for the Eradication of Poverty", r"international day for the eradication of poverty", || month_day_td(10, 17)),
        holiday_rule("International Day for the Preservation of the Ozone Layer", r"international day for the preservation of the ozone Layer", || month_day_td(9, 16)),
        holiday_rule("International Day for the Remembrance of the Slave Trade and its Abolition", r"international day for the remembrance of the slave trade and its abolition", || month_day_td(8, 23)),
        holiday_rule("International Day for the Right to the Truth concerning Gross Human Rights Violations and for the Dignity of Victims", r"international day for the right to the truth concerning gross human rights violations and for the dignity of victims", || month_day_td(3, 24)),
        holiday_rule("International Day for the Total Elimination of Nuclear Weapons", r"international day for the total elimination of nuclear weapons", || month_day_td(9, 26)),
        holiday_rule("International Day in Support of Victims of Torture", r"international day in support of victims of torture", || month_day_td(6, 26)),
        holiday_rule("International Day of Charity", r"international day of charity", || month_day_td(9, 5)),
        holiday_rule("International Day of Commemoration in Memory of the Victims of the Holocaust", r"international day of commemoration in memory of the victims of the holocaust", || month_day_td(1, 27)),
        holiday_rule("International Day of Democracy", r"international day of democracy", || month_day_td(9, 15)),
        holiday_rule("International Day of Disabled Persons", r"international day of disabled persons", || month_day_td(12, 3)),
        holiday_rule("International Day of Families", r"international day of families", || month_day_td(5, 15)),
        holiday_rule("International Day of Family Remittances", r"international day of family remittances", || month_day_td(6, 16)),
        holiday_rule("International Day of Forests", r"international day of forests", || month_day_td(3, 21)),
        holiday_rule("International Day of Friendship", r"international day of friendship", || month_day_td(7, 30)),
        holiday_rule("International Day of Happiness", r"international day of happiness", || month_day_td(3, 20)),
        holiday_rule("International Day of Human Space Flight", r"international day of human space flight", || month_day_td(4, 12)),
        holiday_rule("International Day of Innocent Children Victims of Aggression", r"international day of innocent children victims of aggression", || month_day_td(6, 4)),
        holiday_rule("International Day of Non-Violence", r"international day of non(\-|\s)violence", || month_day_td(10, 2)),
        holiday_rule("International Day of Nowruz", r"international day of nowruz", || month_day_td(3, 21)),
        holiday_rule("International Day of Older Persons", r"international day of older persons", || month_day_td(10, 1)),
        holiday_rule("International Day of Peace", r"international day of peace", || month_day_td(9, 21)),
        holiday_rule("International Day of Persons with Disabilities", r"international day of persons with disabilities", || month_day_td(12, 3)),
        holiday_rule("International Day of Remembrance of Slavery Victims and the Transatlantic Slave Trade", r"international day of remembrance of slavery victims and the transatlantic slave trade", || month_day_td(3, 25)),
        holiday_rule("International Day of Rural Women", r"international day of rural women", || month_day_td(10, 15)),
        holiday_rule("International Day of Solidarity with Detained and Missing Staff Members", r"international day of solidarity with detained and missing staff members", || month_day_td(3, 25)),
        holiday_rule("International Day of Solidarity with the Palestinian People", r"international day of solidarity with the palestinian people", || month_day_td(11, 29)),
        holiday_rule("International Day of Sport for Development and Peace", r"international day of sport for development and peace", || month_day_td(4, 6)),
        holiday_rule("International Day of United Nations Peacekeepers", r"international day of united nations peacekeepers", || month_day_td(5, 29)),
        holiday_rule("International Day of Women and Girls in Science", r"international day of women and girls in science", || month_day_td(2, 11)),
        holiday_rule("International Day of Yoga", r"international day of yoga", || month_day_td(6, 21)),
        holiday_rule("International Day of Zero Tolerance for Female Genital Mutilation", r"international day of zero tolerance for female genital mutilation", || month_day_td(2, 6)),
        holiday_rule("International Day of the Girl Child", r"international day of the girl child", || month_day_td(10, 11)),
        holiday_rule("International Day of the Victims of Enforced Disappearances", r"international day of the victims of enforced disappearances", || month_day_td(8, 30)),
        holiday_rule("International Day of the World's Indigenous People", r"international day of the world'?s indigenous people", || month_day_td(8, 9)),
        holiday_rule("International Day to End Impunity for Crimes against Journalists", r"international day to end impunity for crimes against journalists", || month_day_td(11, 2)),
        holiday_rule("International Day to End Obstetric Fistula", r"international day to end obstetric fistula", || month_day_td(5, 23)),
        holiday_rule("International Day for Disaster Reduction", r"iddr|international day for (natural )?disaster reduction", || month_day_td(10, 13)),
        holiday_rule("International Human Solidarity Day", r"international human solidarity day", || month_day_td(12, 20)),
        holiday_rule("International Jazz Day", r"international jazz day", || month_day_td(4, 30)),
        holiday_rule("International Literacy Day", r"international literacy day", || month_day_td(9, 8)),
        holiday_rule("International Men's Day", r"international men'?s day", || month_day_td(11, 19)),
        holiday_rule("International Migrants Day", r"international migrants day", || month_day_td(12, 18)),
        holiday_rule("International Mother Language Day", r"international mother language day", || month_day_td(2, 21)),
        holiday_rule("International Mountain Day", r"international mountain day", || month_day_td(12, 11)),
        holiday_rule("International Nurses Day", r"international nurses day", || month_day_td(5, 12)),
        holiday_rule("International Overdose Awareness Day", r"international overdose awareness day", || month_day_td(8, 31)),
        holiday_rule("International Volunteer Day for Economic and Social Development", r"international volunteer day for economic and social development", || month_day_td(12, 5)),
        holiday_rule("International Widows' Day", r"international widows'? day", || month_day_td(6, 23)),
        holiday_rule("International Women's Day", r"international women'?s day", || month_day_td(3, 8)),
        holiday_rule("International Youth Day", r"international youth day", || month_day_td(8, 12)),
        holiday_rule("May Day", r"may day", || month_day_td(5, 1)),
        holiday_rule("Nelson Mandela Day", r"nelson mandela day", || month_day_td(7, 18)),
        holiday_rule("New Year's Day", r"new year'?s?( day)?", || month_day_td(1, 1)),
        holiday_rule("New Year's Eve", r"new year'?s? eve", || month_day_td(12, 31)),
        holiday_rule("Orthodox Christmas Day", r"orthodox christmas day", || month_day_td(1, 7)),
        holiday_rule("Orthodox New Year", r"orthodox new year", || month_day_td(1, 14)),
        holiday_rule("Public Service Day", r"public service day", || month_day_td(6, 23)),
        holiday_rule("St. George's Day", r"(saint|st\.?) george'?s day|feast of saint george", || month_day_td(4, 23)),
        holiday_rule("St Patrick's Day", r"(saint|st\.?) (patrick|paddy)'?s day", || month_day_td(3, 17)),
        holiday_rule("St. Stephen's Day", r"(saint|st\.?) stephen'?s day", || month_day_td(12, 26)),
        holiday_rule("Time of Remembrance and Reconciliation for Those Who Lost Their Lives during the Second World War", r"time of remembrance and reconciliation for those who lost their lives during the second world war", || month_day_td(5, 8)),
        holiday_rule("United Nations Day", r"united nations day", || month_day_td(10, 24)),
        holiday_rule("United Nations' Mine Awareness Day", r"united nations'? mine awareness day", || month_day_td(4, 4)),
        holiday_rule("United Nations' World Health Day", r"united nations'? world health day", || month_day_td(4, 7)),
        holiday_rule("Universal Children's Day", r"universal children'?s day", || month_day_td(11, 20)),
        holiday_rule("Valentine's Day", r"valentine'?s?( day)?", || month_day_td(2, 14)),
        holiday_rule("World AIDS Day", r"world aids day", || month_day_td(12, 1)),
        holiday_rule("World Autism Awareness Day", r"world autism awareness day", || month_day_td(4, 2)),
        holiday_rule("World Autoimmune Arthritis Day", r"world autoimmune arthritis day", || month_day_td(5, 20)),
        holiday_rule("World Blood Donor Day", r"world blood donor day", || month_day_td(6, 14)),
        holiday_rule("World Book and Copyright Day", r"world book and copyright day", || month_day_td(4, 23)),
        holiday_rule("World Braille Day", r"world braille day", || month_day_td(1, 4)),
        holiday_rule("World Cancer Day", r"world cancer day", || month_day_td(2, 4)),
        holiday_rule("World Cities Day", r"world cities day", || month_day_td(10, 31)),
        holiday_rule("World CP Day", r"world (cerebral palsy| cp) day", || month_day_td(10, 6)),
        holiday_rule("World Day Against Child Labour", r"world day against child labour", || month_day_td(6, 12)),
        holiday_rule("World Day against Trafficking in Persons", r"world day against trafficking in persons", || month_day_td(7, 30)),
        holiday_rule("World Day for Audiovisual Heritage", r"world day for audiovisual heritage", || month_day_td(10, 27)),
        holiday_rule("World Day for Cultural Diversity for Dialogue and Development", r"world day for cultural diversity for dialogue and development", || month_day_td(5, 21)),
        holiday_rule("World Day for Safety and Health at Work", r"world day for safety and health at work", || month_day_td(4, 28)),
        holiday_rule("World Day for the Abolition of Slavery", r"world day for the abolition of slavery", || month_day_td(12, 2)),
        holiday_rule("World Day of Social Justice", r"world day of social justice", || month_day_td(2, 20)),
        holiday_rule("World Day of the Sick", r"world day of the sick", || month_day_td(2, 11)),
        holiday_rule("World Day to Combat Desertification and Drought", r"world day to combat desertification and drought", || month_day_td(6, 17)),
        holiday_rule("World Development Information Day", r"world development information day", || month_day_td(10, 24)),
        holiday_rule("World Diabetes Day", r"world diabetes day", || month_day_td(11, 14)),
        holiday_rule("World Down Syndrome Day", r"world down syndrome day", || month_day_td(3, 21)),
        holiday_rule("World Elder Abuse Awareness Day", r"world elder abuse awareness day", || month_day_td(6, 15)),
        holiday_rule("World Environment Day", r"world environment day", || month_day_td(6, 5)),
        holiday_rule("World Food Day", r"world food day", || month_day_td(10, 16)),
        holiday_rule("World Genocide Commemoration Day", r"world genocide commemoration day", || month_day_td(12, 9)),
        holiday_rule("World Heart Day", r"world heart day", || month_day_td(9, 29)),
        holiday_rule("World Hepatitis Day", r"world hepatitis day", || month_day_td(7, 28)),
        holiday_rule("World Humanitarian Day", r"world humanitarian day", || month_day_td(8, 19)),
        holiday_rule("World Information Society Day", r"world information society day", || month_day_td(5, 17)),
        holiday_rule("World Intellectual Property Day", r"world intellectual property day", || month_day_td(4, 26)),
        holiday_rule("World Malaria Day", r"world malaria day", || month_day_td(4, 25)),
        holiday_rule("World Mental Health Day", r"world mental health day", || month_day_td(10, 10)),
        holiday_rule("World Meteorological Day", r"world meteorological day", || month_day_td(3, 23)),
        holiday_rule("World No Tobacco Day", r"world no tobacco day", || month_day_td(5, 31)),
        holiday_rule("World Oceans Day", r"world oceans day", || month_day_td(6, 8)),
        holiday_rule("World Ovarian Cancer Day", r"world ovarian cancer day", || month_day_td(5, 8)),
        holiday_rule("World Pneumonia Day", r"world pneumonia day", || month_day_td(11, 12)),
        holiday_rule("World Poetry Day", r"world poetry day", || month_day_td(3, 21)),
        holiday_rule("World Population Day", r"world population day", || month_day_td(7, 11)),
        holiday_rule("World Post Day", r"world post day", || month_day_td(10, 9)),
        holiday_rule("World Prematurity Day", r"world prematurity day", || month_day_td(11, 17)),
        holiday_rule("World Press Freedom Day", r"world press freedom day", || month_day_td(5, 3)),
        holiday_rule("World Rabies Day", r"world rabies day", || month_day_td(9, 28)),
        holiday_rule("World Radio Day", r"world radio day", || month_day_td(2, 13)),
        holiday_rule("World Refugee Day", r"world refugee day", || month_day_td(6, 20)),
        holiday_rule("World Science Day for Peace and Development", r"world science day for peace and development", || month_day_td(11, 10)),
        holiday_rule("World Sexual Health Day", r"world sexual health day", || month_day_td(9, 4)),
        holiday_rule("World Soil Day", r"world soil day", || month_day_td(12, 5)),
        holiday_rule("World Stroke Day", r"world stroke day", || month_day_td(10, 29)),
        holiday_rule("World Suicide Prevention Day", r"world suicide prevention day", || month_day_td(9, 10)),
        holiday_rule("World Teachers' Day", r"world teachers'? day", || month_day_td(10, 5)),
        holiday_rule("World Television Day", r"world television day", || month_day_td(11, 21)),
        holiday_rule("World Toilet Day", r"world toilet day", || month_day_td(11, 19)),
        holiday_rule("World Tourism Day", r"world tourism day", || month_day_td(9, 27)),
        holiday_rule("World Tuberculosis Day", r"world tuberculosis day", || month_day_td(3, 24)),
        holiday_rule("World Tuna Day", r"world tuna day", || month_day_td(5, 2)),
        holiday_rule("World Vegan Day", r"world vegan day", || month_day_td(11, 1)),
        holiday_rule("World Vegetarian Day", r"world vegetarian day", || month_day_td(10, 1)),
        holiday_rule("World Water Day", r"world water day", || month_day_td(3, 22)),
        holiday_rule("World Wetlands Day", r"world wetlands day", || month_day_td(2, 2)),
        holiday_rule("World Wildlife Day", r"world wildlife day", || month_day_td(3, 3)),
        holiday_rule("World Youth Skills Day", r"world youth skills day", || month_day_td(7, 15)),
        holiday_rule("Zero Discrimination Day", r"zero discrimination day", || month_day_td(3, 1)),
        // Fixed day/week/month, year over year (nthDOWOfMonth / predLastOf).
        holiday_rule("Commonwealth Day", r"commonwealth day", || nth_dow_of_month_td(2, 1, 3)),
        holiday_rule("Day of Remembrance for Road Traffic Victims", r"(world )?day of remembrance for road traffic victims", || nth_dow_of_month_td(3, 7, 11)),
        holiday_rule("International Day of Cooperatives", r"international day of co\-?operatives", || nth_dow_of_month_td(1, 6, 7)),
        holiday_rule("Martin Luther King's Day", r"(MLK|Martin Luther King('?s)?,?)( Jr\.?| Junior)? day|(civil|idaho human) rights day", || nth_dow_of_month_td(3, 1, 1)),
        holiday_rule("World Habitat Day", r"world habitat day", || nth_dow_of_month_td(1, 1, 10)),
        holiday_rule("World Kidney Day", r"world kidney day", || nth_dow_of_month_td(2, 4, 3)),
        holiday_rule("World Leprosy Day", r"world leprosy day", || last_dow_of_month_td(7, 1)),
        holiday_rule("World Maritime Day", r"world maritime day", || last_dow_of_month_td(4, 9)),
        holiday_rule("World Migratory Bird Day", r"world migratory bird day", || nth_dow_of_month_td(2, 6, 5)),
        holiday_rule("World Philosophy Day", r"world philosophy day", || nth_dow_of_month_td(3, 4, 11)),
        holiday_rule("World Religion Day", r"world religion day", || nth_dow_of_month_td(3, 7, 1)),
        holiday_rule("World Sight Day", r"world sight day", || nth_dow_of_month_td(2, 4, 10)),
        // The day after Thanksgiving (4th Thursday of November + 1 day).
        holiday_rule("Black Friday", r"black frid?day", || {
            cycle_nth_after_td(false, Grain::Day, 1, &nth_dow_of_month_td(4, 4, 11))
        }),
        // Thanksgiving Day is corpus-exercised (EN/US Rules.hs); kept from the
        // prior baseline so its passing cases do not regress.
        holiday_rule("Thanksgiving Day", r"thanks?giving( day)?", || nth_dow_of_month_td(4, 4, 11)),
    ]
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

/// "beginning|end of <named-month>" and "early|mid|late <named-month>"
/// (ports of the <named-month> dom-range variants).
fn named_month_part_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "at the beginning|end of <named-month>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(?:at the )?(beginning|end) of")),
                PatternItem::Predicate(Box::new(is_a_month)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::RegexMatch(g), Token::Time(m)] => {
                    let (sd, ed) = if g.first()?.to_lowercase().contains("beginning") {
                        (1, 10)
                    } else {
                        (21, -1)
                    };
                    month_dom_range(m, sd, ed).map(Token::Time)
                }
                _ => None,
            }),
        },
        Rule {
            name: "part of <named-month>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(early|mid|late)-?( of)?")),
                PatternItem::Predicate(Box::new(is_a_month)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::RegexMatch(g), Token::Time(m)] => {
                    let w = g.first()?.to_lowercase();
                    let (sd, ed) = if w.contains("early") {
                        (1, 10)
                    } else if w.contains("mid") {
                        (11, 20)
                    } else {
                        (21, -1)
                    };
                    month_dom_range(m, sd, ed).map(Token::Time)
                }
                _ => None,
            }),
        },
    ]
}

/// Year with era, and the "about/sharp" precision markers (which just mark the
/// wrapped time non-latent). Ports of ruleYearADBC / ruleTODPrecision /
/// rulePrecisionTOD.
fn precision_and_era_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "<year> (bc|ad)".into(),
            pattern: vec![
                PatternItem::Predicate(is_integer_between(1, 10000)),
                PatternItem::Regex(compile(r"(a\.?d\.?|b\.?c\.?)")),
            ],
            prod: Box::new(|tokens| {
                let y = get_int_value(tokens.first()?)?;
                let ab = match tokens.get(1)? {
                    Token::RegexMatch(g) => g.first()?,
                    _ => return None,
                };
                let y = if ab.to_lowercase().starts_with('b') { -y } else { y };
                Some(Token::Time(TimeData::new(year_pred(y), Grain::Year)))
            }),
        },
        Rule {
            name: "<time-of-day> sharp|exactly".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_a_time_of_day)),
                PatternItem::Regex(compile(r"(sharp|exactly|-?ish|approximately)")),
            ],
            prod: Box::new(|tokens| match tokens.first()? {
                Token::Time(td) => Some(Token::Time(not_latent(td.clone()))),
                _ => None,
            }),
        },
        Rule {
            name: "about|exactly <time-of-day>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(about|around|approximately|exactly)")),
                PatternItem::Predicate(grain_finer_than(Grain::Year)),
            ],
            prod: Box::new(|tokens| match tokens.get(1)? {
                Token::Time(td) => Some(Token::Time(not_latent(td.clone()))),
                _ => None,
            }),
        },
    ]
}

pub fn en_rules() -> Vec<Rule> {
    let mut rules = vec![
        instant("now", Grain::Second, 0, r"(right |just )?now|at\s+the\s+moment|atm"),
        instant("today", Grain::Day, 0, r"todays?|at\s+this\s+time"),
        instant("tomorrow", Grain::Day, 1, r"tmrw?|tomm?or?rows?"),
        instant("yesterday", Grain::Day, -1, r"yesterdays?"),
    ];
    rules.extend(days_of_week());
    rules.extend(months());
    rules.extend(time_of_day_rules());
    rules.extend(past_to_rules());
    rules.extend(numeral_dependent_rules());
    rules.extend(day_of_month_rules());
    rules.extend(cycle_and_relative_rules());
    rules.extend(interval_rules());
    rules.extend(part_of_day_rules());
    rules.extend(duration_rules());
    rules.extend(holiday_rules());
    rules.extend(season_rules());
    rules.extend(numeric_date_rules());
    rules.extend(end_beginning_of_month_rules());
    rules.extend(end_beginning_year_week_rules());
    rules.extend(this_next_last_time_rules());
    rules.extend(nth_dow_of_time_rules());
    rules.extend(quarter_rules());
    rules.extend(cycle_after_before_rules());
    rules.extend(time_pod_rules());
    rules.extend(crate::time::computed::computed_holiday_rules());
    rules.extend(crate::time::computed::computed_holiday_shift_rules());
    rules.extend(crate::time::computed::computed_interval_holiday_rules());
    rules.push(crate::time::computed::earth_hour_rule());
    rules.extend(absorb_rules());
    rules.extend(timezone_rules());
    rules.extend(dom_interval_rules());
    rules.extend(direction_rules());
    rules.extend(precision_and_era_rules());
    rules.extend(named_month_part_rules());
    rules.extend(intersect_rules());
    rules
}

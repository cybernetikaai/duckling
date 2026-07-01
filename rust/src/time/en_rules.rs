//! English Time rules.
//! Phase 1: instants. + days-of-week, months.

use crate::grain::Grain;
use crate::regex::compile;
use crate::time::object::{IntervalDirection, IntervalType};
use crate::time::predicate::{
    Predicate, ampm_predicate, cycle_nth, day_of_month, day_of_week, hour, hour_minute,
    hour_minute_second, in_duration, intersect, month, take_last_of, take_nth, take_nth_after,
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
    }
}

/// Intersect a month/day-of-week time with a day-of-month value (port of intersectDOM).
fn intersect_dom(td: &TimeData, dom_token: &Token) -> Option<TimeData> {
    let n = get_int_value(dom_token)?;
    Some(TimeData {
        pred: intersect(day_of_month(n), td.pred.clone()),
        grain: Grain::Day,
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: td.holiday.clone(),
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

fn is_month_or_year(t: &Token) -> bool {
    matches!(t, Token::Time(td) if matches!(td.form, Some(Form::Month { .. })) || td.grain == Grain::Year)
}

fn hour_td(is12h: bool, n: i64) -> TimeData {
    TimeData {
        pred: hour(is12h, None, n),
        grain: Grain::Hour,
        latent: false,
        not_immediate: false,
        form: Some(Form::TimeOfDay { hours: Some(n as i8), is12h }),
        direction: None,
        holiday: None,
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
    }
}

/// Apply am/pm to a time-of-day by intersecting with a 12h interval.
fn time_of_day_ampm(is_am: bool, td: &TimeData) -> TimeData {
    TimeData {
        pred: intersect(td.pred.clone(), ampm_predicate(is_am)),
        grain: td.grain,
        latent: false,
        not_immediate: false,
        form: Some(Form::TimeOfDay { hours: None, is12h: false }),
        direction: None,
        holiday: td.holiday.clone(),
    }
}

fn tod(pred: Predicate, grain: Grain, hours: Option<i64>, is12h: bool) -> TimeData {
    TimeData {
        pred,
        grain,
        latent: false,
        not_immediate: false,
        form: Some(Form::TimeOfDay { hours: hours.map(|h| h as i8), is12h }),
        direction: None,
        holiday: None,
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
        ("September", 9, r"sept?|september|sep\.?"),
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
            })
        })
        .collect()
}

fn is_an_hour_of_day(t: &Token) -> bool {
    matches!(t, Token::Time(td)
        if matches!(td.form, Some(Form::TimeOfDay { hours: Some(_), .. })) && td.grain > Grain::Minute)
}
fn hour_minute_td(is12h: bool, h: i64, m: i64) -> TimeData {
    tod(hour_minute(is12h, h, m), Grain::Minute, Some(h), is12h)
}
fn minutes_after(n: i64, td: &TimeData) -> Option<TimeData> {
    if let Some(Form::TimeOfDay { hours: Some(h), is12h }) = td.form {
        Some(hour_minute_td(is12h, h as i64, n))
    } else {
        None
    }
}
fn minutes_before(n: i64, td: &TimeData) -> Option<TimeData> {
    if let Some(Form::TimeOfDay { hours: Some(h), is12h }) = td.form {
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
        before_rule("half to <hod>", r"half (to|till|before|of)", 30),
        before_rule("quarter to <hod>", r"(a|one)? ?quarter (to|till|before|of)", 15),
        after_rule("half past <hod>", r"half (after|past)", 30),
        after_rule("quarter past <hod>", r"(a|one)? ?quarter (after|past)", 15),
        // <integer> to|past <hour-of-day>
        Rule {
            name: "<integer> to <hour-of-day>".into(),
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
            name: "<integer> past <hour-of-day>".into(),
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
                Some(Token::Time(tod(hour_minute(is12h, h, m), Grain::Minute, Some(h), is12h)))
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
                Some(Token::Time(tod(hour_minute(false, h, m), Grain::Minute, Some(h), false)))
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
            name: "noon|midnight|EOD".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(noon|midni(ght|te)|(the )?(EOD|end of (the )?day))",
            ))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                let noon = g.first()?.eq_ignore_ascii_case("noon");
                Some(Token::Time(hour_td(false, if noon { 12 } else { 0 })))
            }),
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
    fn cycle_rule(name: &str, re: &str, n: i64) -> Rule {
        Rule {
            name: name.to_string(),
            pattern: vec![
                PatternItem::Regex(compile(re)),
                PatternItem::Predicate(Box::new(is_a_grain)),
            ],
            prod: Box::new(move |tokens| {
                let g = grain_of(tokens.get(1)?)?;
                Some(Token::Time(cycle_nth_td(g, n)))
            }),
        }
    }
    vec![
        cycle_rule("this <cycle>", r"this|current|coming", 0),
        cycle_rule("next <cycle>", r"next|the following", 1),
        cycle_rule("last <cycle>", r"last|past|previous", -1),
        Rule {
            name: "this|next <day-of-week>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(this|next|coming)")),
                PatternItem::Predicate(Box::new(is_a_day_of_week)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::RegexMatch(g), Token::Time(td)] => {
                    if g.first().map(|s| s.eq_ignore_ascii_case("next")).unwrap_or(false) {
                        // the day-of-week falling in next week
                        Some(Token::Time(TimeData {
                            pred: intersect(td.pred.clone(), cycle_nth(Grain::Week, 1)),
                            grain: Grain::Day,
                            latent: false,
                            not_immediate: false,
                            form: td.form,
                            direction: None,
                            holiday: None,
                        }))
                    } else {
                        // this/coming: the upcoming day-of-week (notImmediate already set)
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
    matches!(t, Token::Time(td) if matches!(td.form, Some(Form::PartOfDay)))
}
fn part_of_day(mut td: TimeData) -> TimeData {
    td.form = Some(Form::PartOfDay);
    td
}

/// Intersect two TimeData (finer grain drives the composition).
fn intersect_td(a: &TimeData, b: &TimeData) -> Option<TimeData> {
    if matches!(a.pred, Predicate::Empty) || matches!(b.pred, Predicate::Empty) {
        return None;
    }
    let (fine, coarse) = if a.grain <= b.grain { (a, b) } else { (b, a) };
    Some(TimeData {
        pred: intersect(fine.pred.clone(), coarse.pred.clone()),
        grain: a.grain.min(b.grain),
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: a.holiday.clone().or_else(|| b.holiday.clone()),
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
            name: "until|before <time>".into(),
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
            name: "after|from <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"after|from")),
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

fn interval_rules() -> Vec<Rule> {
    let sep = r"\-|to|th?ru|through|(un)?til(l)?";
    vec![
        Rule {
            name: "<datetime> - <datetime> (interval)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_not_latent)),
                PatternItem::Regex(compile(sep)),
                PatternItem::Predicate(Box::new(is_not_latent)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(a), _, Token::Time(b)] => {
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
                [_, Token::Time(a), _, Token::Time(b)] => {
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
                [_, Token::Time(a), _, Token::Time(b)] => {
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
    ]
}

fn hour_interval(h1: i64, h2: i64) -> Option<TimeData> {
    interval_td(IntervalType::Open, &hour_td(false, h1), &hour_td(false, h2))
}

fn part_of_day_rules() -> Vec<Rule> {
    vec![
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
                Some(Token::Time(part_of_day(mk_latent(hour_interval(h1, h2)?))))
            }),
        },
        Rule {
            name: "early morning".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"early ((in|hours of) the )?morning",
            ))],
            prod: Box::new(|_| Some(Token::Time(part_of_day(mk_latent(hour_interval(0, 9)?))))),
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
                    intersect_td(&today_td(), td).map(|t| Token::Time(part_of_day(t)))
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
                intersect_td(&today_td(), &evening).map(|t| Token::Time(part_of_day(t)))
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

/// Generic intersection of two adjacent times (ports of ruleIntersect /
/// ruleIntersectOf). Composes dates+years, dow+month-day, time-on-day, etc.
fn intersect_rules() -> Vec<Rule> {
    vec![
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
            name: "intersect by ',', 'of', 'from', 's".into(),
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
fn cycle_n_td(grain: Grain, n: i64) -> TimeData {
    TimeData {
        pred: cycle_n(true, grain, n),
        grain,
        latent: false,
        not_immediate: false,
        form: None,
        direction: None,
        holiday: None,
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
    }
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
                    if w == "within" {
                        interval_td(IntervalType::Open, &now_td(), &in_duration_td(v, gr))
                            .map(Token::Time)
                    } else {
                        Some(Token::Time(in_duration_td(v, gr)))
                    }
                }
                _ => None,
            }),
        },
        Rule {
            name: "<duration> from now|hence|ago".into(),
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
            name: "last|past|next|upcoming <duration>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"([lp]ast|next|upcoming|coming)")),
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
        || matches!(td.form, Some(Form::DayOfWeek) | Some(Form::Month { .. }) | Some(Form::PartOfDay) | Some(Form::Season)))
}

fn season_td(sm: i64, sd: i64, em: i64, ed: i64) -> Option<TimeData> {
    let mut td = interval_td(IntervalType::Open, &month_day_td(sm, sd), &month_day_td(em, ed))?;
    td.form = Some(Form::Season);
    Some(td)
}

fn season_rules() -> Vec<Rule> {
    let seasons: [(&str, &str, i64, i64, i64, i64); 4] = [
        ("summer", r"summer", 6, 21, 9, 23),
        ("fall", r"fall|autumn", 9, 23, 12, 21),
        ("winter", r"winter", 12, 21, 3, 20),
        ("spring", r"spring", 3, 20, 6, 21),
    ];
    seasons
        .iter()
        .map(|&(name, re, sm, sd, em, ed)| Rule {
            name: format!("season {name}"),
            pattern: vec![PatternItem::Regex(compile(re))],
            prod: Box::new(move |_| season_td(sm, sd, em, ed).map(Token::Time)),
        })
        .collect()
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
    }
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
    vec![Rule {
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
    }]
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
            name: "beginning|end of <year>".into(),
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
            name: "beginning|end of <week>".into(),
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
                r"(\d{1,2})(?:st|nd|rd|th)?[-/.\s]+(jan|feb|mar|apr|may|jun|jul|aug|sep|oct|nov|dec)[a-z]*[-/.\s]+(\d{2,4})",
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
        // Thanksgiving Day is corpus-exercised (EN/US Rules.hs); kept from the
        // prior baseline so its passing cases do not regress.
        holiday_rule("Thanksgiving Day", r"thanks?giving( day)?", || nth_dow_of_month_td(4, 4, 11)),
    ]
}

pub fn en_rules() -> Vec<Rule> {
    let mut rules = vec![
        instant("now", Grain::Second, 0, r"now|at\s+the\s+moment|atm"),
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
    rules.extend(time_pod_rules());
    rules.extend(crate::time::computed::computed_holiday_rules());
    rules.extend(crate::time::computed::computed_holiday_shift_rules());
    rules.extend(crate::time::computed::computed_interval_holiday_rules());
    rules.extend(absorb_rules());
    rules.extend(direction_rules());
    rules.extend(intersect_rules());
    rules
}

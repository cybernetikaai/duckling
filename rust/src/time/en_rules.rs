//! English Time rules.
//! Phase 1: instants. + days-of-week, months.

use crate::grain::Grain;
use crate::regex::compile;
use crate::time::object::IntervalType;
use crate::time::predicate::{
    Predicate, ampm_predicate, cycle_nth, day_of_month, day_of_week, hour, hour_minute,
    hour_minute_second, intersect, month, time_intervals, year as year_pred,
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
    matches!(t, Token::Numeral(_)) && get_int_value(t).is_some_and(|v| (1..=31).contains(&v))
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
    Box::new(move |t| get_int_value(t).is_some_and(|v| v >= lo && v <= hi))
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
        holiday: None,
    })
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
    rules.extend(numeral_dependent_rules());
    rules.extend(day_of_month_rules());
    rules.extend(cycle_and_relative_rules());
    rules.extend(interval_rules());
    rules.extend(part_of_day_rules());
    rules.extend(intersect_rules());
    rules
}

//! English Time rules.
//! Phase 1: instants. + days-of-week, months.

use crate::grain::Grain;
use crate::regex::compile;
use crate::time::predicate::{
    Predicate, cycle_nth, day_of_week, hour_minute, hour_minute_second, month,
};
use crate::types::{Form, PatternItem, Rule, TimeData, Token};

fn regex_groups(tokens: &[Token]) -> Option<&Vec<String>> {
    match tokens.first() {
        Some(Token::RegexMatch(g)) => Some(g),
        _ => None,
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
    rules
}

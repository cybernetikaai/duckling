//! `dates` rule builders (split from the en Time monolith).

use super::*;

pub(super) fn days_of_week() -> Vec<Rule> {
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

pub(super) fn months() -> Vec<Rule> {
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
                // "May" is latent — it collides with the modal verb "may", so a
                // bare match is dropped by default (Duckling's mkLatent), avoiding
                // false positives like "you may go". Every other month is concrete.
                // Composition ("in May", "May 1st", "next May") de-latents it.
                latent: n == 5,
                not_immediate: false,
                form: Some(Form::Month { month: n as i8 }),
                direction: None,
                holiday: None,
                has_timezone: false,
            })
        })
        .collect()
}

/// Day-of-month + month-day rules (need Ordinal/Numeral). Ports of the
/// ruleDOM* / ruleNamedDOM* / ruleMonthDOM* family.
pub(super) fn day_of_month_rules() -> Vec<Rule> {
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

/// Numeric date formats (US order: M/D/Y). Ports of the mm/dd(/yyyy), dd/mon/yyyy,
/// mm/yyyy rules.
pub(super) fn numeric_date_rules(locale: Locale) -> Vec<Rule> {
    use crate::types::DateConvention::*;
    let conv = locale.date_convention();
    // with-year forms ("3/4/2015"): day-first for GB-style AND the ZA hybrid.
    let year_day_first = matches!(conv, DayFirst | ZaHybrid);
    // no-year forms ("3/4"): day-first for GB-style only (ZA is month-first here).
    let noyear_day_first = matches!(conv, DayFirst);
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
            pattern: vec![PatternItem::Regex(compile(
                r"(\d{4})\s*[/-]\s*(1[0-2]|0?[1-9])",
            ))],
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
            // US: mm/dd/yyyy (month first). GB: dd/mm/yyyy (day first). Same regex;
            // the two leading fields swap roles by locale. "." separator included,
            // covering the "dd.mm.yyyy" GB / "mm.dd.yyyy" US variants too.
            name: if year_day_first {
                "dd/mm/yyyy".into()
            } else {
                "mm/dd/yyyy".into()
            },
            pattern: vec![PatternItem::Regex(compile(
                r"(\d{1,2})[-/.](\d{1,2})[-/.](\d{2,4})",
            ))],
            prod: Box::new(move |tokens| {
                let g = regex_groups(tokens)?;
                let (m_idx, d_idx) = if year_day_first { (1, 0) } else { (0, 1) };
                year_month_day_td(parse_i(g, 2)?, parse_i(g, m_idx)?, parse_i(g, d_idx)?)
                    .map(Token::Time)
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
            // US: mm/dd (month first). GB: dd/mm (day first).
            name: if noyear_day_first {
                "dd/mm".into()
            } else {
                "mm/dd".into()
            },
            pattern: vec![PatternItem::Regex(compile(r"(\d{1,2})\s*[/-]\s*(\d{1,2})"))],
            prod: Box::new(move |tokens| {
                let g = regex_groups(tokens)?;
                let (m, d) = if noyear_day_first {
                    (parse_i(g, 1)?, parse_i(g, 0)?)
                } else {
                    (parse_i(g, 0)?, parse_i(g, 1)?)
                };
                if !valid_md(m, d) {
                    return None;
                }
                Some(Token::Time(month_day_td(m, d)))
            }),
        },
    ]
}

/// end-of-month / beginning-of-month (ports of ruleEndOfMonth/ruleBeginningOfMonth).
pub(super) fn end_beginning_of_month_rules() -> Vec<Rule> {
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
                interval_td(
                    IntervalType::Open,
                    &dom_of_this_month(21)?,
                    &dom_of_next_month(1)?,
                )
                .map(Token::Time)
            }),
        },
        Rule {
            name: "beginning of month".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(?:(?:at )?the )?(?:bom|beginning of (?:the )?month)",
            ))],
            prod: Box::new(|_| {
                interval_td(
                    IntervalType::Closed,
                    &dom_of_this_month(1)?,
                    &dom_of_this_month(10)?,
                )
                .map(Token::Time)
            }),
        },
    ]
}

/// end/beginning of year & week (ports of ruleEndOfYear/BeginningOfYear,
/// ruleEndOrBeginningOfYear/Week). Bounds oracle-verified.
pub(super) fn end_beginning_year_week_rules() -> Vec<Rule> {
    fn cy(n: i64) -> TimeData {
        cycle_nth_td(Grain::Year, n)
    }
    fn mo_of(y: &TimeData, m: i64) -> TimeData {
        intersect_td(&month_td(m), y).expect("month-of-year")
    }
    vec![
        Rule {
            name: "by end of year".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"by (?:the )?(?:eoy|end of (?:the )?year)",
            ))],
            prod: Box::new(|_| {
                interval_td(IntervalType::Open, &now_td(), &mo_of(&cy(1), 1)).map(Token::Time)
            }),
        },
        Rule {
            name: "end of year".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(?:(?:at )?the )?(?:eoy|end of (?:the )?year)",
            ))],
            prod: Box::new(|_| {
                interval_td(IntervalType::Closed, &mo_of(&cy(0), 9), &mo_of(&cy(0), 12))
                    .map(Token::Time)
            }),
        },
        Rule {
            name: "beginning of year".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(?:(?:at )?the )?(?:boy|beginning of (?:the )?year)",
            ))],
            prod: Box::new(|_| {
                interval_td(IntervalType::Open, &mo_of(&cy(0), 1), &mo_of(&cy(0), 4))
                    .map(Token::Time)
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
                        interval_td(
                            IntervalType::Open,
                            &intersect_td(&month_td(1), td)?,
                            &intersect_td(&month_td(4), td)?,
                        )
                        .map(Token::Time)
                    } else {
                        interval_td(
                            IntervalType::Closed,
                            &intersect_td(&month_td(9), td)?,
                            &intersect_td(&month_td(12), td)?,
                        )
                        .map(Token::Time)
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
                    interval_td(
                        IntervalType::Closed,
                        &intersect_td(&day_of_week_td(sd), td)?,
                        &intersect_td(&day_of_week_td(ed), td)?,
                    )
                    .map(Token::Time)
                }
                _ => None,
            }),
        },
    ]
}

/// "beginning|end of <named-month>" and "early|mid|late <named-month>"
/// (ports of the <named-month> dom-range variants).
pub(super) fn named_month_part_rules() -> Vec<Rule> {
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

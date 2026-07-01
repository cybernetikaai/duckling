//! `timeofday` rule builders (split from the en Time monolith).

use super::*;

pub(super) fn time_of_day_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "hh:mm".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"((?:[01]?\d)|(?:2[0-3]))[:.]([0-5]\d)",
            ))],
            prod: Box::new(|tokens| {
                let g = regex_groups(tokens)?;
                let h: i64 = g.first()?.parse().ok()?;
                let m: i64 = g.get(1)?.parse().ok()?;
                let is12h = h != 0 && h < 12;
                Some(Token::Time(tod(
                    hour_minute(is12h, h, m),
                    Grain::Minute,
                    Some(h),
                    Some(m),
                    is12h,
                )))
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
                Some(Token::Time(tod(
                    hour_minute(false, h, m),
                    Grain::Minute,
                    Some(h),
                    Some(m),
                    false,
                )))
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

/// quarter/half/N past|to <hour-of-day> (ruleHODHalf/Quarter, ruleNumeral/Half/
/// Quarter To/After HOD). e.g. "half past 3", "quarter to 3", "20 past 3".
pub(super) fn past_to_rules() -> Vec<Rule> {
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
                        Some(Form::TimeOfDay {
                            hours: Some(h),
                            is12h,
                            ..
                        }) => (h as i64, is12h),
                        _ => return None,
                    };
                    let td = hour_minute_td(is12h, h, get_int_value(num)?);
                    Some(Token::Time(if hod.latent { mk_latent(td) } else { td }))
                }
                _ => None,
            }),
        },
        // "eight oh five", "twelve oh three", "seven oh eight" -> H:0N (spoken
        // "oh"/"zero" for the leading-zero minute). Port of ruleHONumeralAlt;
        // the integer is a single digit 1-9.
        Rule {
            name: "<hour-of-day> zero <integer>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_an_hour_of_day)),
                PatternItem::Regex(compile(r"(zero|o(h|u)?)")),
                PatternItem::Predicate(is_integer_between(1, 9)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(hod), _, num] => {
                    let (h, is12h) = match hod.form {
                        Some(Form::TimeOfDay {
                            hours: Some(h),
                            is12h,
                            ..
                        }) => (h as i64, is12h),
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
        before_rule(
            "half to|till|before <hour-of-day>",
            r"half (to|till|before|of)",
            30,
        ),
        before_rule(
            "quarter to|till|before <hour-of-day>",
            r"(a|one)? ?quarter (to|till|before|of)",
            15,
        ),
        after_rule("half after|past <hour-of-day>", r"half (after|past)", 30),
        after_rule(
            "quarter after|past <hour-of-day>",
            r"(a|one)? ?quarter (after|past)",
            15,
        ),
        // <integer> to|past <hour-of-day>
        Rule {
            name: "<integer> to|till|before <hour-of-day>".into(),
            pattern: vec![
                PatternItem::Predicate(is_integer_between(1, 59)),
                PatternItem::Regex(compile(r"to|till|before|of")),
                PatternItem::Predicate(Box::new(is_an_hour_of_day)),
            ],
            prod: Box::new(|tokens| match tokens {
                [num, _, Token::Time(td)] => {
                    minutes_before(get_int_value(num)?, td).map(Token::Time)
                }
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
                [num, _, Token::Time(td)] => {
                    minutes_after(get_int_value(num)?, td).map(Token::Time)
                }
                _ => None,
            }),
        },
    ]
}

/// Rules that consume Numeral tokens (years, bare hours) and the rules that
/// build on them (am/pm, at-TOD, noon/midnight).
pub(super) fn numeral_dependent_rules() -> Vec<Rule> {
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
                    let is_am = g
                        .get(1)
                        .map(|s| s.eq_ignore_ascii_case("a"))
                        .unwrap_or(false);
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

pub(super) fn part_of_day_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "as soon as possible".into(),
            pattern: vec![PatternItem::Regex(compile(r"asap|as\ssoon\sas\spossible"))],
            prod: Box::new(|_| {
                Some(Token::Time(with_direction(
                    IntervalDirection::After,
                    now_td(),
                )))
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
                td.form = Some(Form::PartOfDay {
                    start_hour: WEEKEND_POD_HOUR,
                });
                Some(Token::Time(td))
            }),
        },
        Rule {
            name: "after lunch/work/school".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"after[\s-]?(lunch|work|school)",
            ))],
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
                Some(Token::Time(part_of_day(
                    h1,
                    mk_latent(hour_interval(h1, h2)?),
                )))
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
                        Some(Form::TimeOfDay {
                            hours: Some(h),
                            is12h: true,
                            ..
                        }) => h as i64,
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

pub(super) fn time_pod_rules() -> Vec<Rule> {
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

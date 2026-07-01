//! `intervals` rule builders (split from the en Time monolith).

use super::*;

pub(super) fn interval_rules() -> Vec<Rule> {
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
                PatternItem::Regex(compile(
                    r"(?:from )?((?:[01]?\d)|(?:2[0-3]))([:.]([0-5]\d))?",
                )),
                PatternItem::Regex(compile(r"\-|to|th?ru|through|(un)?til(l)?")),
                PatternItem::Predicate(Box::new(is_a_time_of_day)),
                PatternItem::Regex(compile(r"(in the )?([ap])(\s|\.)?m?\.?")),
            ],
            prod: Box::new(|tokens| match tokens {
                [
                    Token::RegexMatch(g1),
                    _,
                    Token::Time(td2),
                    Token::RegexMatch(g4),
                ] => {
                    let h: i64 = g1.first()?.parse().ok()?;
                    let m = g1.get(2).and_then(|s| s.parse::<i64>().ok());
                    let is_am = g4
                        .get(1)
                        .map(|s| s.eq_ignore_ascii_case("a"))
                        .unwrap_or(false);
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
                    let signed = if g.first()?.eq_ignore_ascii_case("before") {
                        -v
                    } else {
                        v
                    };
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
        // "2 fridays from today" / "3 tuesdays from tomorrow" (ruleDOWFromTime):
        // the n-th day-of-week strictly after the base time (predNthAfter n-1).
        // Generalizes the "from now" rule above to any base <time>.
        Rule {
            name: "<integer> <day-of-week> from <time>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(|t| get_int_value(t).is_some_and(|v| v >= 1))),
                PatternItem::Predicate(Box::new(is_a_day_of_week)),
                PatternItem::Regex(compile(r"from")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [num, Token::Time(dow), _, Token::Time(base)] => {
                    let n = get_int_value(num)?;
                    Some(Token::Time(TimeData {
                        pred: take_nth_after(n - 1, true, dow.pred.clone(), base.pred.clone()),
                        grain: Grain::Day,
                        latent: false,
                        not_immediate: false,
                        form: None,
                        direction: None,
                        holiday: None,
                        has_timezone: false,
                    }))
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
                PatternItem::Predicate(Box::new(
                    |t| matches!(t, Token::Time(td) if td.grain == Grain::Day || td.grain == Grain::Month),
                )),
                PatternItem::Regex(compile(r"in")),
                PatternItem::Predicate(Box::new(
                    |t| matches!(t, Token::Duration(d) if d.grain > Grain::Hour),
                )),
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
                PatternItem::Predicate(Box::new(
                    |t| matches!(t, Token::Time(td) if td.grain == Grain::Day || td.grain == Grain::Month),
                )),
                PatternItem::Predicate(Box::new(is_a_duration)),
                PatternItem::Regex(compile(r"(from now|hence|ago)")),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Time(td), dur, Token::RegexMatch(g)] => {
                    let (v, gr) = duration_of(dur)?;
                    let signed = if g.first()?.eq_ignore_ascii_case("ago") {
                        -v
                    } else {
                        v
                    };
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
                PatternItem::Predicate(Box::new(
                    |t| matches!(t, Token::Duration(d) if d.grain > Grain::Hour),
                )),
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
                let start = if m == "all" {
                    cycle_nth_td(Grain::Week, 0)
                } else {
                    today_td()
                };
                let period = interval_td(IntervalType::Closed, &start, &end)?;
                Some(Token::Time(if m == "the" {
                    mk_latent(period)
                } else {
                    period
                }))
            }),
        },
    ]
}

/// Day-of-month intervals within a month (ruleIntervalMonthDDDD family):
/// "July 13 to 15", "23rd to 26th Oct", "from 13 to 15 of July".
pub(super) fn dom_interval_rules() -> Vec<Rule> {
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

/// Open-ended intervals: "until/before <time>" (to), "after/from <time>" (from).
pub(super) fn direction_rules() -> Vec<Rule> {
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
                [_, Token::Time(td)] => Some(Token::Time(with_direction(
                    IntervalDirection::Before,
                    td.clone(),
                ))),
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
                [_, Token::Time(td)] => Some(Token::Time(with_direction(
                    IntervalDirection::After,
                    td.clone(),
                ))),
                _ => None,
            }),
        },
    ]
}

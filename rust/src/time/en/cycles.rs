//! `cycles` rule builders (split from the en Time monolith).

use super::*;

/// this/next/last <cycle> and this/next <day-of-week>.
pub(super) fn cycle_and_relative_rules() -> Vec<Rule> {
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
                [_, num, grain] => Some(Token::Time(cycle_nth_td(
                    grain_of(grain)?,
                    get_int_value(num)?,
                ))),
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
                [num, _, grain] => Some(Token::Time(cycle_nth_td(
                    grain_of(grain)?,
                    get_int_value(num)?,
                ))),
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
                    let word = g
                        .first()
                        .map(|s| s.to_ascii_lowercase())
                        .unwrap_or_default();
                    if word == "next" {
                        // Proximity convention (min gap 2 calendar days), NOT
                        // upstream's day-of-week-in-next-week — see
                        // take_next_dow for the rationale and the ruling date.
                        Some(Token::Time(TimeData {
                            pred: take_next_dow(2, td.pred.clone()),
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

/// this/next/last <time> (ports of ruleThisTime / ruleNextTime / ruleLastTime).
pub(super) fn this_next_last_time_rules() -> Vec<Rule> {
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

/// "<ordinal> <day-of-week> of <month-or-greater>" (ruleNthTimeOfTime).
/// e.g. "third tuesday of september 2014" = 3rd Tuesday in that September.
pub(super) fn nth_dow_of_time_rules() -> Vec<Rule> {
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
                [ord, Token::Time(a), _, Token::Time(b)] => Some(Token::Time(pred_nth_after_td(
                    get_int_value(ord)? - 1,
                    a,
                    b,
                ))),
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
                [ord, Token::Time(dow), _, Token::Time(td)] => Some(Token::Time(
                    pred_nth_after_td(get_int_value(ord)? - 1, dow, td),
                )),
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

/// <ordinal> quarter [<year>], "the <ordinal> quarter", "Q<n>" (ruleQuarter*).
pub(super) fn quarter_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "<ordinal> quarter".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Predicate(Box::new(is_grain_quarter)),
            ],
            prod: Box::new(|tokens| {
                let n = get_int_value(tokens.first()?)?;
                Some(Token::Time(cycle_nth_after_td(
                    true,
                    Grain::Quarter,
                    n - 1,
                    &cycle_nth_td(Grain::Year, 0),
                )))
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
                Some(Token::Time(cycle_nth_after_td(
                    true,
                    Grain::Quarter,
                    n - 1,
                    &cycle_nth_td(Grain::Year, 0),
                )))
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
                    Some(Token::Time(cycle_nth_after_td(
                        false,
                        Grain::Quarter,
                        n - 1,
                        td,
                    )))
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
                Some(Token::Time(cycle_nth_after_td(
                    true,
                    Grain::Quarter,
                    n - 1,
                    &cycle_nth_td(Grain::Year, 0),
                )))
            }),
        },
    ]
}

/// <cycle> after/before <time>, and <ordinal> <cycle> of <time>
/// (ruleCycleAfterBeforeTime, ruleCycleOrdinalOfTime).
/// "the day after tomorrow", "day before yesterday", "first week of october".
pub(super) fn cycle_after_before_rules() -> Vec<Rule> {
    fn after_before(tokens: &[Token], gi: usize, mi: usize, ti: usize) -> Option<Token> {
        let g = grain_of(tokens.get(gi)?)?;
        let m = match tokens.get(mi)? {
            Token::RegexMatch(m) => m.first()?,
            _ => return None,
        };
        let n = if m.eq_ignore_ascii_case("after") {
            1
        } else {
            -1
        };
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
        // "(the) 3rd week after next monday" (ruleCycleOrdinalAfterTime):
        // cycleNthAfter True grain (n-1) — ordinal 3 = 2 cycles after the base.
        Rule {
            name: "<ordinal> <cycle> after <time>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"the")),
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Predicate(Box::new(is_a_grain)),
                PatternItem::Regex(compile(r"after")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, ord, Token::TimeGrain(g), _, Token::Time(td)] => Some(Token::Time(
                    cycle_nth_after_td(true, *g, get_int_value(ord)? - 1, td),
                )),
                _ => None,
            }),
        },
        Rule {
            name: "<ordinal> <cycle> after <time> (no the)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_ordinal)),
                PatternItem::Predicate(Box::new(is_a_grain)),
                PatternItem::Regex(compile(r"after")),
                PatternItem::Predicate(Box::new(is_a_time)),
            ],
            prod: Box::new(|tokens| match tokens {
                [ord, Token::TimeGrain(g), _, Token::Time(td)] => Some(Token::Time(
                    cycle_nth_after_td(true, *g, get_int_value(ord)? - 1, td),
                )),
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

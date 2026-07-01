//! `modifiers` rule builders (split from the en Time monolith).

use super::*;

/// Relative-duration rules (ports of ruleIntervalForDurations / inDuration etc).
pub(super) fn duration_rules() -> Vec<Rule> {
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
                        "within" => {
                            interval_td(IntervalType::Open, &now_td(), &in_duration_td(v, gr))
                                .map(Token::Time)
                        }
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
                    let signed = if g.first()?.eq_ignore_ascii_case("ago") {
                        -v
                    } else {
                        v
                    };
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

pub(super) fn season_rules() -> Vec<Rule> {
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

/// Absorb connective words so the surrounded time can intersect (ruleAbsorbOnDay,
/// ruleAbsorbOnADOW, ruleAbsorbCommaTOD). e.g. "on Thursday", "Monday,".
pub(super) fn absorb_rules() -> Vec<Rule> {
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

/// "<time-of-day> <timezone>" (ruleTimezone): shift the time into the frame.
pub(super) fn timezone_rules() -> Vec<Rule> {
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
                    interval_timezone(g.first()?, a, b)
                }
                _ => None,
            }),
        },
        // "from 3pm to 5pm PST": leading from/later-than variant (the bare rule
        // above can't match once "from" precedes it). "between … and … TZ" is
        // deliberately excluded — Duckling applies the tz only to the 2nd endpoint
        // there (a quirk), which the plain between rule already reproduces.
        Rule {
            name: "from <datetime> - <datetime> (interval) timezone".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"later than|from")),
                PatternItem::Predicate(Box::new(|t| is_a_time_of_day(t) && has_no_timezone(t))),
                PatternItem::Regex(compile(r"\-|to|th?ru|through|(un)?til(l)?")),
                PatternItem::Predicate(Box::new(|t| is_a_time_of_day(t) && has_no_timezone(t))),
                PatternItem::Regex(compile(&tz_re)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Time(a), _, Token::Time(b), Token::RegexMatch(g)] => {
                    interval_timezone(g.first()?, a, b)
                }
                _ => None,
            }),
        },
    ]
}

/// Generic intersection of two adjacent times (ports of ruleIntersect /
/// ruleIntersectOf). Composes dates+years, dow+month-day, time-on-day, etc.
pub(super) fn intersect_rules() -> Vec<Rule> {
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

/// Year with era, and the "about/sharp" precision markers (which just mark the
/// wrapped time non-latent). Ports of ruleYearADBC / ruleTODPrecision /
/// rulePrecisionTOD.
pub(super) fn precision_and_era_rules() -> Vec<Rule> {
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
                let y = if ab.to_lowercase().starts_with('b') {
                    -y
                } else {
                    y
                };
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

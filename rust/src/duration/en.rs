//! English (`en`) Duration rules — counts of a grain plus the English fraction
//! and half-grain idioms ("quarter of an hour", "half an hour", "2.5 hours",
//! "an hour and a half"). Port of Duckling/Duration/EN/Rules.hs. The value type
//! (`DurationData`) and Semigroup math (`dur`/`merge`/`in_seconds`/…) are
//! language-agnostic — see `super`.

use super::*;

fn is_a_duration(t: &Token) -> bool {
    matches!(t, Token::Duration(_))
}
fn as_duration(t: &Token) -> Option<&DurationData> {
    match t {
        Token::Duration(d) => Some(d),
        _ => None,
    }
}

fn is_a_grain(t: &Token) -> bool {
    matches!(t, Token::TimeGrain(_))
}
fn is_natural(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if crate::numeral::int_value(n).is_some_and(|v| v >= 0))
}
fn is_natural_between(lo: i64, hi: i64) -> impl Fn(&Token) -> bool {
    move |t: &Token| matches!(t, Token::Numeral(n) if crate::numeral::int_value(n).is_some_and(|v| v >= lo && v <= hi))
}
fn nat(t: &Token) -> Option<i64> {
    match t {
        Token::Numeral(n) => crate::numeral::int_value(n),
        _ => None,
    }
}
fn groups(t: &Token) -> Option<&Vec<String>> {
    match t {
        Token::RegexMatch(g) => Some(g),
        _ => None,
    }
}

pub fn duration_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "<integer> <unit-of-duration>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_natural)),
                PatternItem::Predicate(Box::new(is_a_grain)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Numeral(n), Token::TimeGrain(g)] => {
                    Some(dur(*g, crate::numeral::int_value(n)?))
                }
                _ => None,
            }),
        },
        Rule {
            name: "a <unit-of-duration>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"an?")),
                PatternItem::Predicate(Box::new(is_a_grain)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::TimeGrain(g)] => Some(dur(*g, 1)),
                _ => None,
            }),
        },
        Rule {
            name: "quarter of an hour".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(1/4\s?h(our)?|(a\s)?quarter of an hour)",
            ))],
            prod: Box::new(|_| Some(dur(Grain::Minute, 15))),
        },
        Rule {
            name: "half an hour (abbrev).".into(),
            pattern: vec![PatternItem::Regex(compile(r"1/2\s?h"))],
            prod: Box::new(|_| Some(dur(Grain::Minute, 30))),
        },
        Rule {
            name: "three-quarters of an hour".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(3/4\s?h(our)?|three(\s|-)quarters of an hour)",
            ))],
            prod: Box::new(|_| Some(dur(Grain::Minute, 45))),
        },
        Rule {
            name: "fortnight".into(),
            pattern: vec![PatternItem::Regex(compile(r"(a|one)? fortnight"))],
            prod: Box::new(|_| Some(dur(Grain::Day, 14))),
        },
        Rule {
            name: "<integer> + '\"".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_natural)),
                PatternItem::Regex(compile("(['\"])")),
            ],
            prod: Box::new(|tokens| {
                let v = nat(tokens.first()?)?;
                match groups(tokens.get(1)?)?.first()?.as_str() {
                    "'" => Some(dur(Grain::Minute, v)),
                    "\"" => Some(dur(Grain::Second, v)),
                    _ => None,
                }
            }),
        },
        Rule {
            name: "<integer> more <unit-of-duration>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_natural)),
                PatternItem::Regex(compile(r"more|additional|extra|less|fewer")),
                PatternItem::Predicate(Box::new(is_a_grain)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Numeral(n), _, Token::TimeGrain(g)] => {
                    Some(dur(*g, crate::numeral::int_value(n)?))
                }
                _ => None,
            }),
        },
        Rule {
            name: "number.number hours".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(\d+)\.(\d+)")),
                PatternItem::Predicate(Box::new(|t| matches!(t, Token::TimeGrain(Grain::Hour)))),
            ],
            prod: Box::new(|tokens| {
                let g = groups(tokens.first()?)?;
                let h: i64 = g.first()?.parse().ok()?;
                let frac = g.get(1)?;
                let n: i64 = frac.parse().ok()?;
                let d: i64 = 10i64.pow(frac.len() as u32);
                Some(dur(Grain::Minute, 60 * h + n * 60 / d))
            }),
        },
        Rule {
            name: "<integer> and an half hour".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_natural)),
                PatternItem::Regex(compile(r"and (an? )?half hours?")),
            ],
            prod: Box::new(|tokens| Some(dur(Grain::Minute, 30 + 60 * nat(tokens.first()?)?))),
        },
        Rule {
            name: "<integer> and a half minutes".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_natural)),
                PatternItem::Regex(compile(r"and (an? )?half min(ute)?s?")),
            ],
            prod: Box::new(|tokens| Some(dur(Grain::Second, 30 + 60 * nat(tokens.first()?)?))),
        },
        Rule {
            name: "half a <time-grain>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(1/2|half)( an?)?")),
                PatternItem::Predicate(Box::new(is_a_grain)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::TimeGrain(g)] => n_plus_one_half(*g, 0),
                _ => None,
            }),
        },
        Rule {
            name: "a <unit-of-duration> and a half".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"an?|one")),
                PatternItem::Predicate(Box::new(is_a_grain)),
                PatternItem::Regex(compile(r"and (a )?half")),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::TimeGrain(g), _] => n_plus_one_half(*g, 1),
                _ => None,
            }),
        },
        Rule {
            name: "<integer> hour and <integer>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_natural)),
                PatternItem::Regex(compile(r"hours?( and)?")),
                PatternItem::Predicate(Box::new(is_natural_between(1, 60))),
            ],
            prod: Box::new(|tokens| {
                let h = nat(tokens.first()?)?;
                let m = nat(tokens.get(2)?)?;
                Some(dur(Grain::Minute, m + 60 * h))
            }),
        },
        Rule {
            name: "about|exactly <duration>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(about|around|approximately|exactly)")),
                PatternItem::Predicate(Box::new(|t| matches!(t, Token::Duration(_)))),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Duration(d)] => Some(Token::Duration(d.clone())),
                _ => None,
            }),
        },
        // ruleCompositeDuration: "<int> <coarse grain> <finer duration>" — e.g.
        // "2 years 3 months" -> 27 months (no connector).
        Rule {
            name: "composite <duration>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_natural)),
                PatternItem::Predicate(Box::new(is_a_grain)),
                PatternItem::Predicate(Box::new(is_a_duration)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Numeral(n), Token::TimeGrain(g), Token::Duration(dd)] if *g > dd.grain => {
                    Some(Token::Duration(merge(
                        &DurationData {
                            value: nat_num(n)?,
                            grain: *g,
                        },
                        dd,
                    )))
                }
                _ => None,
            }),
        },
        // ruleCompositeDurationCommasAnd: "<int> <coarse grain> ,|and <finer duration>"
        // — "2 years and 3 months", "2 years, 3 months".
        Rule {
            name: "composite <duration> (with ,/and)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_natural)),
                PatternItem::Predicate(Box::new(is_a_grain)),
                PatternItem::Regex(compile(r",|and")),
                PatternItem::Predicate(Box::new(is_a_duration)),
            ],
            prod: Box::new(|tokens| match tokens {
                [
                    Token::Numeral(n),
                    Token::TimeGrain(g),
                    _,
                    Token::Duration(dd),
                ] if *g > dd.grain => Some(Token::Duration(merge(
                    &DurationData {
                        value: nat_num(n)?,
                        grain: *g,
                    },
                    dd,
                ))),
                _ => None,
            }),
        },
        // ruleCompositeDurationAnd: "<coarse duration> ,|and <finer duration>" —
        // "an hour and 45 minutes", "a minute and 30 seconds".
        Rule {
            name: "composite <duration> and <duration>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_a_duration)),
                PatternItem::Regex(compile(r",|and")),
                PatternItem::Predicate(Box::new(is_a_duration)),
            ],
            prod: Box::new(|tokens| {
                let a = as_duration(tokens.first()?)?;
                let b = as_duration(tokens.get(2)?)?;
                if a.grain > b.grain {
                    Some(Token::Duration(merge(a, b)))
                } else {
                    None
                }
            }),
        },
        // ruleDurationNumeralAndQuarterHour: "one and a quarter hour" -> 75 min,
        // "one and three quarters hour" -> 105 min.
        Rule {
            name: "<Integer> and <Integer> quarter of hour".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_natural)),
                PatternItem::Regex(compile(
                    r"and (a |an |one |two |three )?quarters?( of)?( an)?",
                )),
                PatternItem::Predicate(Box::new(|t| matches!(t, Token::TimeGrain(Grain::Hour)))),
            ],
            prod: Box::new(|tokens| {
                let h = nat(tokens.first()?)?;
                let q = match groups(tokens.get(1)?)?
                    .first()
                    .map(|s| s.trim().to_lowercase())
                    .as_deref()
                {
                    Some("two") => 2,
                    Some("three") => 3,
                    _ => 1,
                };
                Some(dur(Grain::Minute, 15 * q + 60 * h))
            }),
        },
        // ruleDurationDotNumeralMinutes: "15.5 minutes" -> 930 seconds.
        Rule {
            name: "number.number minutes".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(\d+)\.(\d+)")),
                PatternItem::Predicate(Box::new(|t| matches!(t, Token::TimeGrain(Grain::Minute)))),
            ],
            prod: Box::new(|tokens| {
                let g = groups(tokens.first()?)?;
                let m: i64 = g.first()?.parse().ok()?;
                let frac = g.get(1)?;
                let s: i64 = frac.parse().ok()?;
                let d: i64 = 10i64.pow(frac.len() as u32);
                Some(dur(Grain::Second, 60 * m + s * 60 / d))
            }),
        },
    ]
}

fn nat_num(n: &crate::numeral::NumeralData) -> Option<i64> {
    crate::numeral::int_value(n)
}

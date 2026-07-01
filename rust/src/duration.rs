//! Duration dimension — counts of a time grain, plus the English fraction and
//! half-grain idioms ("quarter of an hour", "half an hour", "2.5 hours",
//! "an hour and a half"). Ports of Duckling/Duration/EN/Rules.hs.

use crate::grain::Grain;
use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

#[derive(Clone, Debug)]
pub struct DurationData {
    pub value: i64,
    pub grain: Grain,
}

fn dur(grain: Grain, value: i64) -> Token {
    Token::Duration(DurationData { value, grain })
}

/// n grains + half a grain, expressed in the next finer grain (port of
/// nPlusOneHalf): half an hour -> 30 min, an hour and a half -> 90 min.
fn n_plus_one_half(grain: Grain, n: i64) -> Option<Token> {
    Some(match grain {
        Grain::Minute => dur(Grain::Second, 30 + 60 * n),
        Grain::Hour => dur(Grain::Minute, 30 + 60 * n),
        Grain::Day => dur(Grain::Hour, 12 + 24 * n),
        Grain::Month => dur(Grain::Day, 15 + 30 * n),
        Grain::Year => dur(Grain::Month, 6 + 12 * n),
        _ => return None,
    })
}

fn is_a_grain(t: &Token) -> bool {
    matches!(t, Token::TimeGrain(_))
}
fn is_natural(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if crate::numeral::int_value(n).is_some_and(|v| v >= 0))
}
fn is_natural_between(lo: i64, hi: i64) -> impl Fn(&Token) -> bool {
    move |t: &Token| {
        matches!(t, Token::Numeral(n) if crate::numeral::int_value(n).is_some_and(|v| v >= lo && v <= hi))
    }
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
                let h: i64 = g.get(1)?.parse().ok()?;
                let frac = g.get(2)?;
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
    ]
}

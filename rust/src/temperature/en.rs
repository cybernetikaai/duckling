//! English (`en`) Temperature rules — port of Duckling/Temperature/EN/Rules.hs
//! plus the shared `ruleNumeralAsTemp` lift. Runs in Temperature's own rule set
//! (numerals + these), never the Time set.

use super::{TempUnit, TemperatureData};
use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

fn value_only(v: f64) -> TemperatureData {
    TemperatureData {
        unit: None,
        value: Some(v),
        min: None,
        max: None,
    }
}

/// value present, no min/max, and (no unit, or Degree when `allow_degree`).
fn is_value_only(allow_degree: bool) -> impl Fn(&Token) -> bool {
    move |t: &Token| {
        matches!(t, Token::Temperature(td)
            if td.value.is_some() && td.min.is_none() && td.max.is_none()
               && (td.unit.is_none() || (allow_degree && td.unit == Some(TempUnit::Degree))))
    }
}
fn is_simple(t: &Token) -> bool {
    matches!(t, Token::Temperature(td) if td.value.is_some())
}
/// Duckling `unitsAreCompatible`: an unset first unit matches anything.
fn compatible(u1: Option<TempUnit>, u2: TempUnit) -> bool {
    u1.is_none_or(|u| u == u2)
}

fn with_unit_rule(name: &'static str, re: &str, allow_degree: bool, u: TempUnit) -> Rule {
    Rule {
        name: name.into(),
        pattern: vec![
            PatternItem::Predicate(Box::new(is_value_only(allow_degree))),
            // Optional leading hyphen (beyond-Duckling): "70-degree" -> "-degree".
            PatternItem::Regex(compile(&format!("-?{re}"))),
        ],
        prod: Box::new(move |tokens| match tokens.first()? {
            Token::Temperature(td) => Some(Token::Temperature(TemperatureData {
                unit: Some(u),
                ..td.clone()
            })),
            _ => None,
        }),
    }
}

/// The two interval rules share this: from < to, compatible units, unit = 2nd's.
fn interval(a: &TemperatureData, b: &TemperatureData) -> Option<Token> {
    let (from, to) = (a.value?, b.value?);
    let u2 = b.unit?;
    (from < to && compatible(a.unit, u2)).then(|| {
        Token::Temperature(TemperatureData {
            unit: Some(u2),
            value: None,
            min: Some(from),
            max: Some(to),
        })
    })
}

pub fn temperature_rules() -> Vec<Rule> {
    vec![
        // ruleNumeralAsTemp (shared): a bare numeral is a latent temperature.
        Rule {
            name: "number as temp".into(),
            pattern: vec![PatternItem::Predicate(Box::new(|t| {
                matches!(t, Token::Numeral(_))
            }))],
            prod: Box::new(|tokens| match tokens.first()? {
                Token::Numeral(n) => Some(Token::Temperature(value_only(n.value))),
                _ => None,
            }),
        },
        // "<latent> degrees" -> Degree (only a unit-less value).
        with_unit_rule(
            "<latent temp> degrees",
            r"(deg(ree?)?s?\.?)|°",
            false,
            TempUnit::Degree,
        ),
        // "<temp> C/F" -> Celsius/Fahrenheit (unit-less or already Degree).
        with_unit_rule(
            "<temp> Celsius",
            r"c(el[cs]?(ius)?)?\.?",
            true,
            TempUnit::Celsius,
        ),
        with_unit_rule(
            "<temp> Fahrenheit",
            r"f(ah?rh?eh?n(h?eit)?)?\.?",
            true,
            TempUnit::Fahrenheit,
        ),
        // "<temp> below zero" -> negate; default unit Degree.
        Rule {
            name: "<temp> below zero".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_value_only(true))),
                PatternItem::Regex(compile(r"below zero")),
            ],
            prod: Box::new(|tokens| match tokens.first()? {
                Token::Temperature(td) => {
                    let v = td.value?;
                    Some(Token::Temperature(TemperatureData {
                        unit: td.unit.or(Some(TempUnit::Degree)),
                        value: Some(-v),
                        min: None,
                        max: None,
                    }))
                }
                _ => None,
            }),
        },
        // "between|from <temp> and|to <temp>" -> interval.
        Rule {
            name: "between|from <temp> and|to <temp>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"between|from")),
                PatternItem::Predicate(Box::new(is_simple)),
                PatternItem::Regex(compile(r"to|and")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Temperature(a), _, Token::Temperature(b)] => interval(a, b),
                _ => None,
            }),
        },
        // "<temp> - <temp>" -> interval.
        Rule {
            name: "<temp> - <temp>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_simple)),
                PatternItem::Regex(compile(r"-")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Temperature(a), _, Token::Temperature(b)] => interval(a, b),
                _ => None,
            }),
        },
        // "over/above/at least/more than <temp>" -> min only.
        Rule {
            name: "over/above/at least/more than <temp>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"over|above|at least|more than")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match tokens.get(1)? {
                Token::Temperature(td) => Some(Token::Temperature(TemperatureData {
                    unit: Some(td.unit?),
                    value: None,
                    min: td.value,
                    max: None,
                })),
                _ => None,
            }),
        },
        // "under/less/lower/no more than <temp>" -> max only.
        Rule {
            name: "under/less/lower/no more than <temp>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"under|(less|lower|not? more) than")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match tokens.get(1)? {
                Token::Temperature(td) => Some(Token::Temperature(TemperatureData {
                    unit: Some(td.unit?),
                    value: None,
                    min: None,
                    max: td.value,
                })),
                _ => None,
            }),
        },
    ]
}

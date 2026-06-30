//! Minimal Duration dimension — "<n> <unit>" and "a <unit>".

use crate::grain::Grain;
use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

#[derive(Clone, Debug)]
pub struct DurationData {
    pub value: i64,
    pub grain: Grain,
}

fn is_a_grain(t: &Token) -> bool {
    matches!(t, Token::TimeGrain(_))
}
fn is_natural(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if crate::numeral::int_value(n).is_some_and(|v| v >= 0))
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
                    let v = crate::numeral::int_value(n)?;
                    Some(Token::Duration(DurationData { value: v, grain: *g }))
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
                [_, Token::TimeGrain(g)] => {
                    Some(Token::Duration(DurationData { value: 1, grain: *g }))
                }
                _ => None,
            }),
        },
    ]
}

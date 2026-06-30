//! Minimal Numeral dimension — just what EN Time needs so far.
//! Phase: integer (numeric). Written numbers / composition added when a Time
//! rule (durations, spelled times) requires them.

use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

#[derive(Clone, Debug)]
pub struct NumeralData {
    pub value: f64,
}

/// Integer value if the numeral is a whole number.
pub fn int_value(n: &NumeralData) -> Option<i64> {
    if n.value.fract() == 0.0 {
        Some(n.value as i64)
    } else {
        None
    }
}

pub fn numeral_rules() -> Vec<Rule> {
    vec![Rule {
        name: "integer (numeric)".into(),
        pattern: vec![PatternItem::Regex(compile(r"(\d+)"))],
        prod: Box::new(|tokens| {
            if let Some(Token::RegexMatch(g)) = tokens.first() {
                let v: f64 = g.first()?.parse().ok()?;
                Some(Token::Numeral(NumeralData { value: v }))
            } else {
                None
            }
        }),
    }]
}

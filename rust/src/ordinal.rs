//! Minimal Ordinal dimension — digits (1st, 19th, 31st) + small words.

use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

#[derive(Clone, Debug)]
pub struct OrdinalData {
    pub value: i64,
}

pub fn ordinal_rules() -> Vec<Rule> {
    let mut rules = vec![Rule {
        name: "ordinal (digits)".into(),
        pattern: vec![PatternItem::Regex(compile(r"0*(\d+) ?(?:st|nd|rd|th)"))],
        prod: Box::new(|tokens| {
            if let Some(Token::RegexMatch(g)) = tokens.first() {
                let v: i64 = g.first()?.parse().ok()?;
                Some(Token::Ordinal(OrdinalData { value: v }))
            } else {
                None
            }
        }),
    }];
    let words: [(&str, i64); 10] = [
        ("first", 1),
        ("second", 2),
        ("third", 3),
        ("fourth", 4),
        ("fifth", 5),
        ("sixth", 6),
        ("seventh", 7),
        ("eighth", 8),
        ("ninth", 9),
        ("tenth", 10),
    ];
    for (w, v) in words {
        rules.push(Rule {
            name: format!("ordinal ({w})"),
            pattern: vec![PatternItem::Regex(compile(w))],
            prod: Box::new(move |_| Some(Token::Ordinal(OrdinalData { value: v }))),
        });
    }
    rules
}

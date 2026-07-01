//! Ordinal dimension — port of `Duckling/Ordinal/EN/Rules.hs`: digit ordinals
//! (1st, 19th, 31st), base words (first..twentieth, thirtieth..ninetieth), and
//! composite words (twenty-first..ninety-ninth, incl. spaced "twenty fifth").

use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

#[derive(Clone, Debug)]
pub struct OrdinalData {
    pub value: i64,
}

/// `ordinalsMap` (first..ninetieth). Also the units table for composites.
fn base_ordinal(w: &str) -> Option<i64> {
    Some(match w {
        "first" => 1,
        "second" => 2,
        "third" => 3,
        "fourth" => 4,
        "fifth" => 5,
        "sixth" => 6,
        "seventh" => 7,
        "eighth" => 8,
        "ninth" => 9,
        "tenth" => 10,
        "eleventh" => 11,
        "twelfth" => 12,
        "thirteenth" => 13,
        "fourteenth" => 14,
        "fifteenth" => 15,
        "sixteenth" => 16,
        "seventeenth" => 17,
        "eighteenth" => 18,
        "nineteenth" => 19,
        "twentieth" => 20,
        "thirtieth" => 30,
        "fortieth" => 40,
        "fiftieth" => 50,
        "sixtieth" => 60,
        "seventieth" => 70,
        "eightieth" => 80,
        "ninetieth" => 90,
        _ => return None,
    })
}

/// `cardinalsMap` (tens) for the composite rule.
fn tens(w: &str) -> Option<i64> {
    Some(match w {
        "twenty" => 20,
        "thirty" => 30,
        "forty" => 40,
        "fifty" => 50,
        "sixty" => 60,
        "seventy" => 70,
        "eighty" => 80,
        "ninety" => 90,
        _ => return None,
    })
}

pub fn ordinal_rules() -> Vec<Rule> {
    vec![
        // ruleOrdinals
        Rule {
            name: "ordinals (first..twentieth,thirtieth,...)".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(first|second|third|fourth|fifth|sixth|seventh|eighth|ninth|tenth|eleventh|twelfth|thirteenth|fourteenth|fifteenth|sixteenth|seventeenth|eighteenth|nineteenth|twentieth|thirtieth|fortieth|fiftieth|sixtieth|seventieth|eightieth|ninetieth)",
            ))],
            prod: Box::new(|tokens| {
                if let Some(Token::RegexMatch(g)) = tokens.first() {
                    let v = base_ordinal(&g.first()?.to_lowercase())?;
                    Some(Token::Ordinal(OrdinalData { value: v }))
                } else {
                    None
                }
            }),
        },
        // ruleCompositeOrdinals — tens + units ("twenty-first", "twenty fifth", "thirtythird")
        Rule {
            name:
                "ordinals (composite, e.g. eighty-seven, forty—seventh, twenty ninth, thirtythird)"
                    .into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(twenty|thirty|forty|fifty|sixty|seventy|eighty|ninety)[\s\-\—]?(first|second|third|fourth|fifth|sixth|seventh|eighth|ninth)",
            ))],
            prod: Box::new(|tokens| {
                if let Some(Token::RegexMatch(g)) = tokens.first() {
                    let t = tens(&g.first()?.to_lowercase())?;
                    let u = base_ordinal(&g.get(1)?.to_lowercase())?;
                    Some(Token::Ordinal(OrdinalData { value: t + u }))
                } else {
                    None
                }
            }),
        },
        // ruleOrdinalDigits
        Rule {
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
        },
    ]
}

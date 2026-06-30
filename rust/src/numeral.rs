//! Minimal Numeral dimension — just what EN Time needs so far.
//! Phase: integer (numeric). Written numbers / composition added when a Time
//! rule (durations, spelled times) requires them.

use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

#[derive(Clone, Debug)]
pub struct NumeralData {
    pub value: f64,
    /// false for informal numerals (couple/few/dozen/single/pair) which Duckling
    /// marks notOkForAnyTime — they can't be a time-of-day/year/day-of-month.
    pub ok_for_time: bool,
}

/// Integer value if the numeral is a whole number.
pub fn int_value(n: &NumeralData) -> Option<i64> {
    if n.value.fract() == 0.0 {
        Some(n.value as i64)
    } else {
        None
    }
}

pub fn ok_for_time(n: &NumeralData) -> bool {
    n.ok_for_time
}

const INFORMAL: &[&str] = &["single", "couple", "pair", "few", "dozen"];

const UNITS: &[(&str, i64)] = &[
    ("zero", 0), ("one", 1), ("single", 1), ("two", 2), ("couple", 2), ("pair", 2),
    ("three", 3), ("few", 3), ("four", 4), ("five", 5), ("six", 6), ("seven", 7),
    ("eight", 8), ("nine", 9), ("ten", 10), ("eleven", 11), ("twelve", 12),
    ("dozen", 12), ("thirteen", 13), ("fourteen", 14), ("fifteen", 15),
    ("sixteen", 16), ("seventeen", 17), ("eighteen", 18), ("nineteen", 19),
];
const TENS: &[(&str, i64)] = &[
    ("twenty", 20), ("thirty", 30), ("forty", 40), ("fourty", 40), ("fifty", 50),
    ("sixty", 60), ("seventy", 70), ("eighty", 80), ("ninety", 90),
];

fn from_table(table: &[(&str, i64)], w: &str) -> Option<i64> {
    let w = w.to_lowercase();
    table.iter().find(|(k, _)| *k == w).map(|&(_, v)| v)
}

fn numeral(v: i64) -> Option<Token> {
    Some(Token::Numeral(NumeralData { value: v as f64, ok_for_time: true }))
}

pub fn numeral_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "integer (numeric)".into(),
            pattern: vec![PatternItem::Regex(compile(r"(\d+)"))],
            prod: Box::new(|tokens| {
                if let Some(Token::RegexMatch(g)) = tokens.first() {
                    let v: f64 = g.first()?.parse().ok()?;
                    Some(Token::Numeral(NumeralData { value: v, ok_for_time: true }))
                } else {
                    None
                }
            }),
        },
        // 0..19 (+ informal couple/pair/few/dozen/single). Longest-first.
        Rule {
            name: "integer (0..19)".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(nineteen|eighteen|seventeen|sixteen|fifteen|fourteen|thirteen|twelve|eleven|ten|nine|eight|seven|six|five|four|three|two|one|zero|single|couples?|pair|few|dozens?)",
            ))],
            prod: Box::new(|tokens| {
                let g = match tokens.first() {
                    Some(Token::RegexMatch(g)) => g,
                    _ => return None,
                };
                let w = g.first()?.trim_end_matches('s');
                let v = from_table(UNITS, w)?;
                Some(Token::Numeral(NumeralData {
                    value: v as f64,
                    ok_for_time: !INFORMAL.contains(&w),
                }))
            }),
        },
        // 20..90
        Rule {
            name: "integer (20..90)".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(twenty|thirty|fou?rty|fifty|sixty|seventy|eighty|ninety)",
            ))],
            prod: Box::new(|tokens| {
                let g = match tokens.first() {
                    Some(Token::RegexMatch(g)) => g,
                    _ => return None,
                };
                numeral(from_table(TENS, g.first()?)?)
            }),
        },
        // 21..99 composite, e.g. "twenty-three", "twenty three"
        Rule {
            name: "integer ([2-9][1-9])".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(twenty|thirty|fou?rty|fifty|sixty|seventy|eighty|ninety)[\s\-]?(one|two|three|four|five|six|seven|eight|nine)",
            ))],
            prod: Box::new(|tokens| {
                let g = match tokens.first() {
                    Some(Token::RegexMatch(g)) => g,
                    _ => return None,
                };
                let tens = from_table(TENS, g.first()?)?;
                let unit = from_table(UNITS, g.get(1)?)?;
                numeral(tens + unit)
            }),
        },
    ]
}

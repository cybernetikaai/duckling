//! English (`en`) Numeral rules — integers, written numbers, informal
//! quantifiers ("a couple", "a dozen"), and composition. Produces
//! `super::NumeralData` (the language-agnostic value type).

use super::NumeralData;
use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

const INFORMAL: &[&str] = &["single", "couple", "pair", "few"];

const UNITS: &[(&str, i64)] = &[
    ("zero", 0),
    ("naught", 0),
    ("nought", 0),
    ("nil", 0),
    ("none", 0),
    ("zilch", 0),
    ("one", 1),
    ("single", 1),
    ("two", 2),
    ("couple", 2),
    ("pair", 2),
    ("three", 3),
    ("few", 3),
    ("four", 4),
    ("five", 5),
    ("six", 6),
    ("seven", 7),
    ("eight", 8),
    ("nine", 9),
    ("ten", 10),
    ("eleven", 11),
    ("twelve", 12),
    ("thirteen", 13),
    ("fourteen", 14),
    ("fifteen", 15),
    ("sixteen", 16),
    ("seventeen", 17),
    ("eighteen", 18),
    ("nineteen", 19),
];
const TENS: &[(&str, i64)] = &[
    ("twenty", 20),
    ("thirty", 30),
    ("forty", 40),
    ("fourty", 40),
    ("fifty", 50),
    ("sixty", 60),
    ("seventy", 70),
    ("eighty", 80),
    ("ninety", 90),
];

fn from_table(table: &[(&str, i64)], w: &str) -> Option<i64> {
    let w = w.to_lowercase();
    table.iter().find(|(k, _)| *k == w).map(|&(_, v)| v)
}

fn numeral(v: i64) -> Option<Token> {
    Some(Token::Numeral(NumeralData::new(v as f64, true)))
}

fn is_numeral(t: &Token) -> bool {
    matches!(t, Token::Numeral(_))
}
/// A numeral without a (powers-of-ten) grain — the fractional operand of a
/// spelled-out decimal (`not . hasGrain` in Duckling).
fn is_numeral_no_grain(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if n.grain.is_none_or(|g| g <= 1))
}
/// `77 -> 0.77`: divide by the smallest power of ten strictly greater than x
/// (port of `decimalsToDouble`).
fn decimals_to_double(x: f64) -> f64 {
    let mut m = 1.0;
    for _ in 0..10 {
        if x - m < 0.0 {
            return x / m;
        }
        m *= 10.0;
    }
    0.0
}
/// Parse a decimal string; a leading "." gets a "0" prepended (`parseDouble`).
fn parse_double(s: &str) -> Option<f64> {
    let s = if s.starts_with('.') {
        format!("0{s}")
    } else {
        s.to_string()
    };
    s.parse::<f64>().ok()
}

fn is_positive(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if n.value >= 0.0)
}
fn is_multipliable(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if n.multipliable)
}
fn has_grain(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if n.grain.is_some_and(|g| g > 1))
}
fn is_int_positive(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if n.value > 0.0 && n.value.fract() == 0.0)
}
/// Power-of-ten exponent for a magnitude word (port of powersOfTensMap).
fn power_of_ten(w: &str) -> Option<i64> {
    Some(match w {
        "hundred" => 2,
        "thousand" => 3,
        "million" => 6,
        "billion" => 9,
        "trillion" => 12,
        _ if w.starts_with('l') => 5,
        _ if w.starts_with("cr") || w.starts_with("kr") || w == "koti" => 7,
        _ => return None,
    })
}

pub fn numeral_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "integer (numeric)".into(),
            pattern: vec![PatternItem::Regex(compile(r"(\d+)"))],
            prod: Box::new(|tokens| {
                if let Some(Token::RegexMatch(g)) = tokens.first() {
                    let v: f64 = g.first()?.parse().ok()?;
                    Some(Token::Numeral(NumeralData::new(v, true)))
                } else {
                    None
                }
            }),
        },
        // 0..19 (+ informal couple/pair/few/dozen/single). Longest-first.
        Rule {
            name: "integer (0..19)".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(nineteen|eighteen|seventeen|sixteen|fifteen|fourteen|thirteen|twelve|eleven|ten|nine|eight|seven|six|five|four|three|two|one|zero|naught|nought|nil|none|zilch|single|(a )?(pair|couple)s?( of)?|(a )?few)",
            ))],
            prod: Box::new(|tokens| {
                let g = match tokens.first() {
                    Some(Token::RegexMatch(g)) => g,
                    _ => return None,
                };
                // Normalize informal wrappers: "a couple of" -> "couple",
                // "a pair" -> "pair", "a few" -> "few", "couples" -> "couple".
                let raw = g.first()?.to_lowercase();
                let w = raw.strip_prefix("a ").unwrap_or(&raw);
                let w = w.strip_suffix(" of").unwrap_or(w);
                let w = w.trim_end_matches('s');
                let v = from_table(UNITS, w)?;
                Some(Token::Numeral(NumeralData::new(
                    v as f64,
                    !INFORMAL.contains(&w),
                )))
            }),
        },
        // "a dozen", "two dozen", "two hundred dozens" (ruleDozen): 12,
        // multipliable (so "two dozen" -> 2*12), not ok for time.
        Rule {
            name: "a dozen of".into(),
            pattern: vec![PatternItem::Regex(compile(r"(a )?dozens?( of)?"))],
            prod: Box::new(|_| {
                Some(Token::Numeral(NumeralData {
                    value: 12.0,
                    ok_for_time: false,
                    grain: None,
                    multipliable: true,
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
        // "one twenty two" -> 122 (ruleSkipHundreds2). NOTE: the two-word
        // ruleSkipHundreds1 ("nine thirty" -> 930) is intentionally NOT ported —
        // its shape collides with time-of-day composition ("half an hour after
        // nine thirty") in this crate's shared rule set (Duckling avoids this by
        // parsing each dimension separately). The 3-word skip2 has no such clash.
        Rule {
            name: "one twenty two".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(one|two|three|four|five|six|seven|eight|nine)")),
                PatternItem::Regex(compile(
                    r"(twenty|thirty|fou?rty|fifty|sixty|seventy|eighty|ninety)",
                )),
                PatternItem::Regex(compile(r"(one|two|three|four|five|six|seven|eight|nine)")),
            ],
            prod: Box::new(|tokens| match tokens {
                [
                    Token::RegexMatch(a),
                    Token::RegexMatch(b),
                    Token::RegexMatch(c),
                ] => {
                    let h = from_table(UNITS, a.first()?)?;
                    let tens = from_table(TENS, b.first()?)?;
                    let rest = from_table(UNITS, c.first()?)?;
                    numeral(h * 100 + tens + rest)
                }
                _ => None,
            }),
        },
        // "hundred"/"thousand"/... -> 10^grain, multipliable (rulePowersOfTen).
        Rule {
            name: "powers of tens".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(hundred|thousand|l(ac|(a?kh)?)|million|((k|c)r(ore)?|koti)|billion|trillion)s?",
            ))],
            prod: Box::new(|tokens| {
                let w = match tokens.first() {
                    Some(Token::RegexMatch(g)) => g.first()?.to_lowercase(),
                    _ => return None,
                };
                let grain = power_of_ten(&w)?;
                Some(Token::Numeral(NumeralData {
                    value: 10f64.powi(grain as i32),
                    ok_for_time: true,
                    grain: Some(grain),
                    multipliable: true,
                }))
            }),
        },
        // "two thousand" -> 2000 (ruleMultiply): positive x multipliable.
        Rule {
            name: "compose by multiplication".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_positive)),
                PatternItem::Predicate(Box::new(is_multipliable)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Numeral(a), Token::Numeral(b)] => match b.grain {
                    None => Some(Token::Numeral(NumeralData::new(a.value * b.value, true))),
                    Some(g) if b.value > a.value => Some(Token::Numeral(NumeralData {
                        value: a.value * b.value,
                        ok_for_time: true,
                        grain: Some(g),
                        multipliable: false,
                    })),
                    _ => None,
                },
                _ => None,
            }),
        },
        // "two thousand ten" -> 2010 (ruleSum): grained + smaller non-multipliable.
        Rule {
            name: "intersect 2 numbers".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(|t| has_grain(t) && is_positive(t))),
                PatternItem::Predicate(Box::new(|t| !is_multipliable(t) && is_positive(t))),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Numeral(a), Token::Numeral(b)] => {
                    let g = a.grain?;
                    (10f64.powi(g as i32) > b.value)
                        .then(|| Token::Numeral(NumeralData::new(a.value + b.value, true)))
                }
                _ => None,
            }),
        },
        Rule {
            name: "intersect 2 numbers (with and)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(|t| has_grain(t) && is_positive(t))),
                PatternItem::Regex(compile(r"and")),
                PatternItem::Predicate(Box::new(|t| !is_multipliable(t) && is_positive(t))),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Numeral(a), _, Token::Numeral(b)] => {
                    let g = a.grain?;
                    (10f64.powi(g as i32) > b.value)
                        .then(|| Token::Numeral(NumeralData::new(a.value + b.value, true)))
                }
                _ => None,
            }),
        },
        // "-504", "minus 1,200,000" (ruleNegative): (-|minus|negative) <positive>.
        Rule {
            name: "negative numbers".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"(-|minus|negative)(?!\s*-)")),
                PatternItem::Predicate(Box::new(is_positive)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Numeral(n)] => Some(Token::Numeral(NumeralData::new(-n.value, true))),
                _ => None,
            }),
        },
        // "forty-five (45)", "45 (forty five)" (ruleLegalParentheses): a spelled
        // and a digit form of the same positive integer confirm each other.
        Rule {
            name: "<integer> '('<integer>')'".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_int_positive)),
                PatternItem::Regex(compile(r"\(")),
                PatternItem::Predicate(Box::new(is_int_positive)),
                PatternItem::Regex(compile(r"\)")),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Numeral(a), _, Token::Numeral(b), _]
                    if a.value as i64 == b.value as i64 =>
                {
                    Some(Token::Numeral(NumeralData::new(a.value, true)))
                }
                _ => None,
            }),
        },
        // "1.1", ".77" (ruleDecimals).
        Rule {
            name: "decimal number".into(),
            pattern: vec![PatternItem::Regex(compile(r"(\d*\.\d+)"))],
            prod: Box::new(|tokens| {
                let g = match tokens.first() {
                    Some(Token::RegexMatch(g)) => g,
                    _ => return None,
                };
                Some(Token::Numeral(NumeralData::new(
                    parse_double(g.first()?)?,
                    true,
                )))
            }),
        },
        // "1 point 1" -> 1.1 (ruleDotSpelledOut).
        Rule {
            name: "one point 2".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_numeral)),
                PatternItem::Regex(compile(r"point|dot")),
                PatternItem::Predicate(Box::new(is_numeral_no_grain)),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Numeral(a), _, Token::Numeral(b)] => Some(Token::Numeral(
                    NumeralData::new(a.value + decimals_to_double(b.value), true),
                )),
                _ => None,
            }),
        },
        // "point 77" -> 0.77 (ruleLeadingDotSpelledOut).
        Rule {
            name: "point 77".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"point|dot")),
                PatternItem::Predicate(Box::new(is_numeral_no_grain)),
            ],
            prod: Box::new(|tokens| match tokens {
                [_, Token::Numeral(b)] => Some(Token::Numeral(NumeralData::new(
                    decimals_to_double(b.value),
                    true,
                ))),
                _ => None,
            }),
        },
        // "100,000", "3,000,000" (ruleCommas): strip commas, parse.
        Rule {
            name: "comma-separated numbers".into(),
            pattern: vec![PatternItem::Regex(compile(r"(\d+(,\d\d\d)+(\.\d+)?)"))],
            prod: Box::new(|tokens| {
                let g = match tokens.first() {
                    Some(Token::RegexMatch(g)) => g,
                    _ => return None,
                };
                Some(Token::Numeral(NumeralData::new(
                    parse_double(&g.first()?.replace(',', ""))?,
                    true,
                )))
            }),
        },
        // "1/5" -> 0.2 (ruleFractions; Duckling puts this in the shared Numeral
        // rules). notOkForAnyTime — a fraction is never a date/hour/year.
        Rule {
            name: "fractional number".into(),
            pattern: vec![PatternItem::Regex(compile(r"(\d+)/(\d+)"))],
            prod: Box::new(|tokens| {
                let g = match tokens.first() {
                    Some(Token::RegexMatch(g)) => g,
                    _ => return None,
                };
                let n: f64 = g.first()?.parse().ok()?;
                let d: f64 = g.get(1)?.parse().ok()?;
                (d != 0.0).then(|| Token::Numeral(NumeralData::new(n / d, false)))
            }),
        },
        // "100K", "1.2M" (ruleSuffixes): <numeral> (k|m|g) at a word boundary.
        Rule {
            name: "suffixes (K,M,G)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_numeral)),
                PatternItem::Regex(compile(r"(k|m|g)(?=[\W$€¢£]|$)")),
            ],
            prod: Box::new(|tokens| match tokens {
                [Token::Numeral(n), Token::RegexMatch(g)] => {
                    let mult = match g.first()?.to_lowercase().as_str() {
                        "k" => 1e3,
                        "m" => 1e6,
                        "g" => 1e9,
                        _ => return None,
                    };
                    Some(Token::Numeral(NumeralData::new(n.value * mult, true)))
                }
                _ => None,
            }),
        },
    ]
}

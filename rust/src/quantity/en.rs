//! English (`en`) Quantity rules — port of Duckling/Quantity/EN/Rules.hs (units
//! with mg/kg scaling, "a <unit>", <quantity> of <product>, precision, intervals,
//! and the latent bare-number rule). Runs in Quantity's own rule set
//! (numerals + these), never the Time set.

use super::{QuantityData, Unit};
use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

/// Duckling `opsMap`/`getValue`: a gram spelling may scale the value (milli /
/// 1000, kilo * 1000). Any other unit spelling leaves the value unchanged.
/// Division/multiplication (not a 0.001 factor) mirrors the Haskell exactly.
fn apply_scale(m: &str, v: f64) -> f64 {
    match m {
        "milligram" | "milligrams" | "mg" | "mgs" | "m.g" | "m.gs" | "m.g." | "m.g.s" => v / 1000.0,
        "kilogram" | "kilograms" | "kg" | "kgs" | "k.g" | "k.gs" | "k.g." | "k.g.s" => v * 1000.0,
        _ => v,
    }
}

// ----- predicates (Duckling/Quantity/Helpers.hs) -----

fn is_positive(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if n.value >= 0.0)
}
/// `isSimpleQuantity`: a Quantity with both a unit and a value.
fn is_simple(t: &Token) -> bool {
    matches!(t, Token::Quantity(q) if q.unit.is_some() && q.value.is_some())
}
fn is_quantity(t: &Token) -> bool {
    matches!(t, Token::Quantity(_))
}

// ----- value constructors (Duckling/Quantity/Helpers.hs) -----

fn quantity(u: Unit, v: f64) -> QuantityData {
    QuantityData {
        unit: Some(u),
        value: Some(v),
        product: None,
        min: None,
        max: None,
        latent: false,
    }
}
fn value_only_latent(v: f64) -> QuantityData {
    QuantityData {
        unit: None,
        value: Some(v),
        product: None,
        min: None,
        max: None,
        latent: true,
    }
}
fn with_interval(from: f64, to: f64, u: Unit) -> Token {
    Token::Quantity(QuantityData {
        unit: Some(u),
        value: None,
        product: None,
        min: Some(from),
        max: Some(to),
        latent: false,
    })
}
fn open_min(from: f64, u: Unit) -> Token {
    Token::Quantity(QuantityData {
        unit: Some(u),
        value: None,
        product: None,
        min: Some(from),
        max: None,
        latent: false,
    })
}
fn open_max(to: f64, u: Unit) -> Token {
    Token::Quantity(QuantityData {
        unit: Some(u),
        value: None,
        product: None,
        min: None,
        max: Some(to),
        latent: false,
    })
}

/// The unit-word regexes: `(name, regex, unit)`. Group 1 (the outer capture) is
/// the matched spelling, fed to `apply_scale` for the gram mg/kg case.
fn unit_specs() -> [(&'static str, &'static str, Unit); 4] {
    [
        ("<quantity> cups", r"(cups?)", Unit::Cup),
        (
            "<quantity> grams",
            r"(((m(illi)?[.]?)|(k(ilo)?)[.]?)?g(ram)?s?[.]?)[.]?",
            Unit::Gram,
        ),
        ("<quantity> lb", r"((lb|pound)s?)", Unit::Pound),
        ("<quantity> oz", r"((ounces?)|oz)", Unit::Ounce),
    ]
}

/// "<positive numeral> <unit>" -> a simple quantity (value scaled for mg/kg).
fn numeral_quantity_rule(name: &'static str, re: &str, u: Unit) -> Rule {
    Rule {
        name: name.into(),
        pattern: vec![
            PatternItem::Predicate(Box::new(is_positive)),
            PatternItem::Regex(compile(re)),
        ],
        prod: Box::new(move |tokens| match (tokens.first()?, tokens.get(1)?) {
            (Token::Numeral(n), Token::RegexMatch(g)) => {
                let v = apply_scale(&g.first()?.to_lowercase(), n.value);
                Some(Token::Quantity(quantity(u, v)))
            }
            _ => None,
        }),
    }
}

/// "a/an <unit>" -> a simple quantity of 1 (scaled for mg/kg).
fn a_quantity_rule(name: &'static str, re: &str, u: Unit) -> Rule {
    let full = format!("an? {re}");
    Rule {
        name: name.into(),
        pattern: vec![PatternItem::Regex(compile(&full))],
        prod: Box::new(move |tokens| match tokens.first()? {
            Token::RegexMatch(g) => {
                let v = apply_scale(&g.first()?.to_lowercase(), 1.0);
                Some(Token::Quantity(quantity(u, v)))
            }
            _ => None,
        }),
    }
}

pub fn quantity_rules() -> Vec<Rule> {
    let mut rules: Vec<Rule> = vec![
        // "<quantity> of product": attach a product word to any quantity.
        Rule {
            name: "<quantity> of product".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_quantity)),
                PatternItem::Regex(compile(r"of (\w+)")),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(1)?) {
                (Token::Quantity(q), Token::RegexMatch(g)) => Some(Token::Quantity(QuantityData {
                    product: Some(g.first()?.to_lowercase()),
                    ..q.clone()
                })),
                _ => None,
            }),
        },
        // "over/above/... <quantity>": min only (operand carries no product).
        Rule {
            name: "over/above/exceeding/beyond/at least/more than <quantity>".into(),
            pattern: vec![
                PatternItem::Regex(compile(
                    r"over|above|exceeding|beyond|at least|(more|larger|bigger|heavier) than",
                )),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match tokens.get(1)? {
                Token::Quantity(q) if q.product.is_none() => Some(open_min(q.value?, q.unit?)),
                _ => None,
            }),
        },
        // "under/below/... <quantity>": max only.
        Rule {
            name: "under/below/less/lower/at most/no more than <quantity>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"under|below|at most|(less|lower|not? more) than")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match tokens.get(1)? {
                Token::Quantity(q) if q.product.is_none() => Some(open_max(q.value?, q.unit?)),
                _ => None,
            }),
        },
        // "between|from <numeral> and|to <quantity>": interval, from < to.
        Rule {
            name: "between|from <numeral> and|to <quantity>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"between|from")),
                PatternItem::Predicate(Box::new(is_positive)),
                PatternItem::Regex(compile(r"to|and")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.get(1)?, tokens.get(3)?) {
                (Token::Numeral(n), Token::Quantity(q)) if q.product.is_none() => {
                    let (from, to) = (n.value, q.value?);
                    let u = q.unit?;
                    (from < to).then(|| with_interval(from, to, u))
                }
                _ => None,
            }),
        },
        // "between|from <quantity> to|and <quantity>": interval, same unit.
        Rule {
            name: "between|from <quantity> to|and <quantity>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"between|from")),
                PatternItem::Predicate(Box::new(is_simple)),
                PatternItem::Regex(compile(r"and|to")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.get(1)?, tokens.get(3)?) {
                (Token::Quantity(a), Token::Quantity(b))
                    if a.product.is_none() && b.product.is_none() =>
                {
                    let (from, to) = (a.value?, b.value?);
                    let (u1, u2) = (a.unit?, b.unit?);
                    (from < to && u1 == u2).then(|| with_interval(from, to, u1))
                }
                _ => None,
            }),
        },
        // "<numeral> - <quantity>": interval, from < to.
        Rule {
            name: "<numeral> - <quantity>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_positive)),
                PatternItem::Regex(compile(r"\-")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(2)?) {
                (Token::Numeral(n), Token::Quantity(q)) if q.product.is_none() => {
                    let (from, to) = (n.value, q.value?);
                    let u = q.unit?;
                    (from < to).then(|| with_interval(from, to, u))
                }
                _ => None,
            }),
        },
        // "<quantity> - <quantity>": interval, same unit.
        Rule {
            name: "<quantity> - <quantity>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_simple)),
                PatternItem::Regex(compile(r"\-")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(2)?) {
                (Token::Quantity(a), Token::Quantity(b))
                    if a.product.is_none() && b.product.is_none() =>
                {
                    let (from, to) = (a.value?, b.value?);
                    let (u1, u2) = (a.unit?, b.unit?);
                    (from < to && u1 == u2).then(|| with_interval(from, to, u1))
                }
                _ => None,
            }),
        },
        // "about|exactly <quantity>": precision markers pass the quantity through.
        Rule {
            name: "about|exactly <quantity>".into(),
            pattern: vec![
                PatternItem::Regex(compile(
                    r"~|exactly|precisely|about|approx(\.|imately)?|close to|near( to)?|around|almost",
                )),
                PatternItem::Predicate(Box::new(is_quantity)),
            ],
            prod: Box::new(|tokens| tokens.get(1).cloned()),
        },
        // "<positive numeral> (latent)": a bare number is a latent unnamed quantity.
        Rule {
            name: "<quantity> (latent)".into(),
            pattern: vec![PatternItem::Predicate(Box::new(is_positive))],
            prod: Box::new(|tokens| match tokens.first()? {
                Token::Numeral(n) => Some(Token::Quantity(value_only_latent(n.value))),
                _ => None,
            }),
        },
    ];

    // unit words: "<numeral> <unit>" and "a <unit>".
    for (name, re, u) in unit_specs() {
        rules.push(numeral_quantity_rule(name, re, u));
    }
    for (name, re, u) in unit_specs() {
        rules.push(a_quantity_rule(name, re, u));
    }

    rules
}

//! English (`en`) Volume rules — port of Duckling/Volume/Rules.hs (shared
//! numeral-composition rules) plus Duckling/Volume/EN/Rules.hs (unit words,
//! fractions, precision, intervals). Runs in Volume's own rule set
//! (numerals + these), never the Time set.

use super::{Unit, VolumeData};
use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

// ----- predicates (Duckling/Volume/Helpers.hs) -----

/// Duckling `isPositive`: a Numeral with value >= 0.
fn is_positive(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if n.value >= 0.0)
}
/// `isSimpleVolume`: a Volume with a value and no min/max (unit not required).
fn is_simple_volume(t: &Token) -> bool {
    matches!(t, Token::Volume(v) if v.value.is_some() && v.min.is_none() && v.max.is_none())
}
/// `isUnitOnly`: a Volume with a unit but no value/min/max.
fn is_unit_only(t: &Token) -> bool {
    matches!(t, Token::Volume(v)
        if v.value.is_none() && v.unit.is_some() && v.min.is_none() && v.max.is_none())
}

// ----- value constructors (Duckling/Volume/Helpers.hs) -----

fn value_only(v: f64) -> VolumeData {
    VolumeData {
        value: Some(v),
        unit: None,
        min: None,
        max: None,
    }
}
fn unit_only(u: Unit) -> VolumeData {
    VolumeData {
        value: None,
        unit: Some(u),
        min: None,
        max: None,
    }
}
fn volume(u: Unit, v: f64) -> VolumeData {
    VolumeData {
        value: Some(v),
        unit: Some(u),
        min: None,
        max: None,
    }
}
fn with_interval(from: f64, to: f64, u: Unit) -> Token {
    Token::Volume(VolumeData {
        value: None,
        unit: Some(u),
        min: Some(from),
        max: Some(to),
    })
}

fn unit_rule(name: &'static str, re: &str, u: Unit) -> Rule {
    Rule {
        name: name.into(),
        pattern: vec![PatternItem::Regex(compile(re))],
        prod: Box::new(move |_| Some(Token::Volume(unit_only(u)))),
    }
}

/// "<fraction-word> <unit-only>" -> a simple volume of that fraction.
fn fraction_rule(name: &'static str, re: &str, f: f64) -> Rule {
    Rule {
        name: name.into(),
        pattern: vec![
            PatternItem::Regex(compile(re)),
            PatternItem::Predicate(Box::new(is_unit_only)),
        ],
        prod: Box::new(move |tokens| match tokens.get(1)? {
            Token::Volume(v) => Some(Token::Volume(volume(v.unit?, f))),
            _ => None,
        }),
    }
}

pub fn volume_rules() -> Vec<Rule> {
    let mut rules: Vec<Rule> = vec![
        // --- shared numeral-composition rules (Duckling/Volume/Rules.hs) ---
        // "number as volume": a bare positive numeral is a latent (unit-less) volume.
        Rule {
            name: "number as volume".into(),
            pattern: vec![PatternItem::Predicate(Box::new(is_positive))],
            prod: Box::new(|tokens| match tokens.first()? {
                Token::Numeral(n) => Some(Token::Volume(value_only(n.value))),
                _ => None,
            }),
        },
        // "<number> <volume>": numeral + unit-only -> simple volume.
        Rule {
            name: "<number> <volume>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_positive)),
                PatternItem::Predicate(Box::new(is_unit_only)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(1)?) {
                (Token::Numeral(n), Token::Volume(v)) => {
                    Some(Token::Volume(volume(v.unit?, n.value)))
                }
                _ => None,
            }),
        },
        // "<number>-<volume>" hyphenated (beyond-Duckling): "2-liter". A dedicated
        // rule (not a `-?` on the unit regex) so it can't disturb the hyphen-eating
        // fraction words ("half-litre"); `isPositive` never matches "half".
        Rule {
            name: "<number>-<volume>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_positive)),
                PatternItem::Regex(compile(r"-")),
                PatternItem::Predicate(Box::new(is_unit_only)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(2)?) {
                (Token::Numeral(n), Token::Volume(v)) => {
                    Some(Token::Volume(volume(v.unit?, n.value)))
                }
                _ => None,
            }),
        },
        // "<numeral> - <volume>": interval, from < to; unit from the volume.
        Rule {
            name: "<numeral> - <volume>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_positive)),
                PatternItem::Regex(compile(r"\-")),
                PatternItem::Predicate(Box::new(is_simple_volume)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(2)?) {
                (Token::Numeral(n), Token::Volume(v)) => {
                    let (from, to) = (n.value, v.value?);
                    let u = v.unit?;
                    (from < to).then(|| with_interval(from, to, u))
                }
                _ => None,
            }),
        },
        // "<volume> - <volume>": interval, from < to, same unit.
        Rule {
            name: "<volume> - <volume>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_simple_volume)),
                PatternItem::Regex(compile(r"\-")),
                PatternItem::Predicate(Box::new(is_simple_volume)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(2)?) {
                (Token::Volume(a), Token::Volume(b)) => {
                    let (from, to) = (a.value?, b.value?);
                    let (u1, u2) = (a.unit?, b.unit?);
                    (from < to && u1 == u2).then(|| with_interval(from, to, u1))
                }
                _ => None,
            }),
        },
        // --- EN rules (Duckling/Volume/EN/Rules.hs) ---
        // "about <volume>": precision markers pass the volume through unchanged.
        Rule {
            name: "about <volume>".into(),
            pattern: vec![
                PatternItem::Regex(compile(
                    r"~|exactly|precisely|about|approx(\.|imately)?|close to|near( to)?|around|almost",
                )),
                PatternItem::Predicate(Box::new(|t| matches!(t, Token::Volume(_)))),
            ],
            prod: Box::new(|tokens| tokens.get(1).cloned()),
        },
        // "between|from <numeral> and|to <volume>": interval, from < to.
        Rule {
            name: "between|from <numeral> and|to <volume>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"between|from")),
                PatternItem::Predicate(Box::new(is_positive)),
                PatternItem::Regex(compile(r"to|and")),
                PatternItem::Predicate(Box::new(is_simple_volume)),
            ],
            prod: Box::new(|tokens| match (tokens.get(1)?, tokens.get(3)?) {
                (Token::Numeral(n), Token::Volume(v)) => {
                    let (from, to) = (n.value, v.value?);
                    let u = v.unit?;
                    (from < to).then(|| with_interval(from, to, u))
                }
                _ => None,
            }),
        },
        // "between|from <volume> to|and <volume>": interval, from < to, same unit.
        Rule {
            name: "between|from <volume> to|and <volume>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"between|from")),
                PatternItem::Predicate(Box::new(is_simple_volume)),
                PatternItem::Regex(compile(r"to|and")),
                PatternItem::Predicate(Box::new(is_simple_volume)),
            ],
            prod: Box::new(|tokens| match (tokens.get(1)?, tokens.get(3)?) {
                (Token::Volume(a), Token::Volume(b)) => {
                    let (from, to) = (a.value?, b.value?);
                    let (u1, u2) = (a.unit?, b.unit?);
                    (from < to && u1 == u2).then(|| with_interval(from, to, u1))
                }
                _ => None,
            }),
        },
        // "at most <volume>": max only.
        Rule {
            name: "at most <volume>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"under|below|at most|(less|lower|not? more) than")),
                PatternItem::Predicate(Box::new(is_simple_volume)),
            ],
            prod: Box::new(|tokens| match tokens.get(1)? {
                Token::Volume(v) => Some(Token::Volume(VolumeData {
                    value: None,
                    unit: Some(v.unit?),
                    min: None,
                    max: v.value,
                })),
                _ => None,
            }),
        },
        // "more than <volume>": min only.
        Rule {
            name: "more than <volume>".into(),
            pattern: vec![
                PatternItem::Regex(compile(
                    r"over|above|exceeding|beyond|at least|(more|larger|bigger|heavier) than",
                )),
                PatternItem::Predicate(Box::new(is_simple_volume)),
            ],
            prod: Box::new(|tokens| match tokens.get(1)? {
                Token::Volume(v) => Some(Token::Volume(VolumeData {
                    value: None,
                    unit: Some(v.unit?),
                    min: v.value,
                    max: None,
                })),
                _ => None,
            }),
        },
    ];

    // unit words -> unit-only volume.
    rules.push(unit_rule(
        "<latent vol> ml",
        r"m(l(s?)|illilit(er|re)s?)",
        Unit::Millilitre,
    ));
    rules.push(unit_rule(
        "<vol> hectoliters",
        r"hectolit(er|re)s?",
        Unit::Hectolitre,
    ));
    rules.push(unit_rule("<vol> liters", r"l(it(er|re)s?)?", Unit::Litre));
    rules.push(unit_rule(
        "<latent vol> gallon",
        r"gal((l?ons?)|s)?",
        Unit::Gallon,
    ));

    // fraction words + unit-only -> simple fractional volume.
    let frac = r"(-|(( of)?( a(n?))?))?";
    rules.push(fraction_rule("one", r"an? ", 1.0));
    rules.push(fraction_rule("half", &format!("half{frac}"), 1.0 / 2.0));
    rules.push(fraction_rule("third", &format!("third{frac}"), 1.0 / 3.0));
    rules.push(fraction_rule(
        "fourth",
        &format!("(quarter|fourth){frac}"),
        1.0 / 4.0,
    ));
    rules.push(fraction_rule("fifth", &format!("fifth{frac}"), 1.0 / 5.0));
    rules.push(fraction_rule("tenth", &format!("tenth{frac}"), 1.0 / 10.0));

    rules
}

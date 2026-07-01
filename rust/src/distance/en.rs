//! English (`en`) Distance rules — port of Duckling/Distance/Rules.hs (shared
//! numeral lift), Duckling/Distance/EN/Rules.hs (units, intervals, precision,
//! composites) and Duckling/DistanceUnits/Types.hs (the metric/imperial
//! conversion semigroup used to fold composites like "7 feet 10 inches").
//! Runs in Distance's own rule set (numerals + these), never the Time set.

use super::{DistanceData, Unit};
use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

// ----- unit conversion (Duckling/DistanceUnits/Types.hs) -----
//
// `Sys` lists metric units ascending then imperial units ascending, so derived
// `Ord` gives Duckling's ordering: all metric < all imperial, and the smaller
// unit within a system sorts first. `min(u1, u2)` therefore picks the finer /
// metric-preferred unit exactly as the Haskell `SystemUnit` `Ord` does.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Sys {
    Millimetre,
    Centimetre,
    Metre,
    Kilometre,
    Inch,
    Foot,
    Yard,
    Mile,
}

/// A distance unit that may still be the ambiguous "m" (Mile or Metre).
enum Def {
    Definite(Sys),
    Ambiguous, // only "m" is ambiguous
}

fn to_deferrable(u: Unit) -> Def {
    match u {
        Unit::M => Def::Ambiguous,
        Unit::Millimetre => Def::Definite(Sys::Millimetre),
        Unit::Centimetre => Def::Definite(Sys::Centimetre),
        Unit::Metre => Def::Definite(Sys::Metre),
        Unit::Kilometre => Def::Definite(Sys::Kilometre),
        Unit::Inch => Def::Definite(Sys::Inch),
        Unit::Foot => Def::Definite(Sys::Foot),
        Unit::Yard => Def::Definite(Sys::Yard),
        Unit::Mile => Def::Definite(Sys::Mile),
    }
}

fn sys_to_unit(s: Sys) -> Unit {
    match s {
        Sys::Millimetre => Unit::Millimetre,
        Sys::Centimetre => Unit::Centimetre,
        Sys::Metre => Unit::Metre,
        Sys::Kilometre => Unit::Kilometre,
        Sys::Inch => Unit::Inch,
        Sys::Foot => Unit::Foot,
        Sys::Yard => Unit::Yard,
        Sys::Mile => Unit::Mile,
    }
}

fn is_metric(s: Sys) -> bool {
    matches!(
        s,
        Sys::Millimetre | Sys::Centimetre | Sys::Metre | Sys::Kilometre
    )
}

/// Value expressed in metres (the SI unit). Inch factor 0.0254 is exact.
fn in_si(s: Sys, v: f64) -> f64 {
    const M_PER_IN: f64 = 0.0254;
    match s {
        Sys::Millimetre => v / 1000.0,
        Sys::Centimetre => v / 100.0,
        Sys::Metre => v,
        Sys::Kilometre => v * 1000.0,
        Sys::Inch => v * M_PER_IN,
        Sys::Foot => v * 12.0 * M_PER_IN,
        Sys::Yard => v * 36.0 * M_PER_IN,
        Sys::Mile => v * 63360.0 * M_PER_IN,
    }
}

/// Convert `v` from `start` units into `target` units.
fn scale_units(target: Sys, start: Sys, v: f64) -> f64 {
    if start == target {
        v
    } else {
        in_si(start, v) / in_si(target, 1.0)
    }
}

/// The ambiguous "m" resolved in the context of a definite unit's system.
fn resolve_ambiguous(du: Sys) -> Sys {
    if is_metric(du) { Sys::Metre } else { Sys::Mile }
}

/// Fold two (value, unit) distances into one, disambiguating "m" against a
/// definite neighbour (Duckling's `distanceSum` via the `ContextualDistance`
/// semigroup). `None` only for two *different* ambiguous units, impossible today.
fn distance_sum(v1: f64, u1: Unit, v2: f64, u2: Unit) -> Option<DistanceData> {
    let (value, unit) = match (to_deferrable(u1), to_deferrable(u2)) {
        (Def::Definite(s1), Def::Definite(s2)) => {
            let u = s1.min(s2);
            (
                scale_units(u, s1, v1) + scale_units(u, s2, v2),
                sys_to_unit(u),
            )
        }
        (Def::Ambiguous, Def::Ambiguous) => (v1 + v2, Unit::M),
        (Def::Ambiguous, Def::Definite(du)) => reconcile(v1, du, v2),
        (Def::Definite(du), Def::Ambiguous) => reconcile(v2, du, v1),
    };
    Some(DistanceData {
        unit: Some(unit),
        value: Some(value),
        min: None,
        max: None,
    })
}

/// `reconcileAmbiguousWithDefinite`: `av` is the ambiguous value, `dv` the
/// definite value in unit `du`.
fn reconcile(av: f64, du: Sys, dv: f64) -> (f64, Unit) {
    let resolved = resolve_ambiguous(du);
    let preferred = du.min(resolved);
    let v = scale_units(preferred, resolved, av) + scale_units(preferred, du, dv);
    (v, sys_to_unit(preferred))
}

// ----- predicates (Duckling/Distance/Helpers.hs) -----

fn is_numeral(t: &Token) -> bool {
    matches!(t, Token::Numeral(_))
}
fn is_positive(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if n.value >= 0.0)
}
/// `isSimpleDistance`: a Distance with both a value and a unit.
fn is_simple(t: &Token) -> bool {
    matches!(t, Token::Distance(d) if d.value.is_some() && d.unit.is_some())
}
fn is_distance(t: &Token) -> bool {
    matches!(t, Token::Distance(_))
}

// ----- value constructors (Duckling/Distance/Helpers.hs) -----

fn distance_val(v: f64) -> DistanceData {
    DistanceData {
        unit: None,
        value: Some(v),
        min: None,
        max: None,
    }
}
fn with_interval(from: f64, to: f64, u: Unit) -> Token {
    Token::Distance(DistanceData {
        unit: Some(u),
        value: None,
        min: Some(from),
        max: Some(to),
    })
}
fn open_min(from: f64, u: Unit) -> Token {
    Token::Distance(DistanceData {
        unit: Some(u),
        value: None,
        min: Some(from),
        max: None,
    })
}
fn open_max(to: f64, u: Unit) -> Token {
    Token::Distance(DistanceData {
        unit: Some(u),
        value: None,
        min: None,
        max: Some(to),
    })
}

/// "<distance> <unit-regex>" -> attach the unit (Duckling `ruleDistances`).
fn unit_rule(name: &'static str, re: &str, u: Unit) -> Rule {
    Rule {
        name: name.into(),
        pattern: vec![
            PatternItem::Predicate(Box::new(is_distance)),
            PatternItem::Regex(compile(re)),
        ],
        prod: Box::new(move |tokens| match tokens.first()? {
            Token::Distance(dd) => Some(Token::Distance(DistanceData {
                unit: Some(u),
                ..dd.clone()
            })),
            _ => None,
        }),
    }
}

/// Shared body of the two composite rules: distinct units, both positive.
fn composite(a: &DistanceData, b: &DistanceData) -> Option<Token> {
    let (v1, u1) = (a.value?, a.unit?);
    let (v2, u2) = (b.value?, b.unit?);
    (u1 != u2 && v1 > 0.0 && v2 > 0.0)
        .then(|| distance_sum(v1, u1, v2, u2).map(Token::Distance))
        .flatten()
}

pub fn distance_rules() -> Vec<Rule> {
    let mut rules: Vec<Rule> = vec![
        // --- shared numeral lift (Duckling/Distance/Rules.hs) ---
        // "number as distance": any numeral is a latent (unit-less) distance.
        Rule {
            name: "number as distance".into(),
            pattern: vec![PatternItem::Predicate(Box::new(is_numeral))],
            prod: Box::new(|tokens| match tokens.first()? {
                Token::Numeral(n) => Some(Token::Distance(distance_val(n.value))),
                _ => None,
            }),
        },
        // --- EN rules (Duckling/Distance/EN/Rules.hs) ---
        // "between|from <numeral> to|and <dist>": interval, from < to.
        Rule {
            name: "between|from <numeral> to|and <dist>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"between|from")),
                PatternItem::Predicate(Box::new(is_positive)),
                PatternItem::Regex(compile(r"to|and")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.get(1)?, tokens.get(3)?) {
                (Token::Numeral(n), Token::Distance(d)) => {
                    let (from, to) = (n.value, d.value?);
                    let u = d.unit?;
                    (from < to).then(|| with_interval(from, to, u))
                }
                _ => None,
            }),
        },
        // "between|from <dist> to|and <dist>": interval, from < to, same unit.
        Rule {
            name: "between|from <dist> to|and <dist>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"between|from")),
                PatternItem::Predicate(Box::new(is_simple)),
                PatternItem::Regex(compile(r"to|and")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.get(1)?, tokens.get(3)?) {
                (Token::Distance(a), Token::Distance(b)) => {
                    let (from, to) = (a.value?, b.value?);
                    let (u1, u2) = (a.unit?, b.unit?);
                    (from < to && u1 == u2).then(|| with_interval(from, to, u1))
                }
                _ => None,
            }),
        },
        // "under/less/lower/no more than <dist>": max only.
        Rule {
            name: "under/less/lower/no more than <dist>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"under|(less|lower|not? more) than")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match tokens.get(1)? {
                Token::Distance(d) => Some(open_max(d.value?, d.unit?)),
                _ => None,
            }),
        },
        // "over/above/at least/more than <dist>": min only.
        Rule {
            name: "over/above/at least/more than <dist>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"over|above|at least|more than")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match tokens.get(1)? {
                Token::Distance(d) => Some(open_min(d.value?, d.unit?)),
                _ => None,
            }),
        },
        // "<numeral> - <dist>": interval, from < to; unit from the distance.
        Rule {
            name: "<numeral> - <dist>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_positive)),
                PatternItem::Regex(compile(r"-")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(2)?) {
                (Token::Numeral(n), Token::Distance(d)) => {
                    let (from, to) = (n.value, d.value?);
                    let u = d.unit?;
                    (from < to).then(|| with_interval(from, to, u))
                }
                _ => None,
            }),
        },
        // "<dist> - <dist>": interval, from < to, same unit.
        Rule {
            name: "<dist> - <dist>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_simple)),
                PatternItem::Regex(compile(r"-")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(2)?) {
                (Token::Distance(a), Token::Distance(b)) => {
                    let (from, to) = (a.value?, b.value?);
                    let (u1, u2) = (a.unit?, b.unit?);
                    (from < to && u1 == u2).then(|| with_interval(from, to, u1))
                }
                _ => None,
            }),
        },
        // "about|exactly <dist>": precision markers pass the distance through.
        Rule {
            name: "about|exactly <dist>".into(),
            pattern: vec![
                PatternItem::Regex(compile(
                    r"exactly|precisely|about|approx(\.|imately)?|close to| near( to)?|around|almost",
                )),
                PatternItem::Predicate(Box::new(is_distance)),
            ],
            prod: Box::new(|tokens| tokens.get(1).cloned()),
        },
        // "composite <distance> (with ,/and)": fold two distinct-unit distances.
        Rule {
            name: "composite <distance> (with ,/and)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_simple)),
                PatternItem::Regex(compile(r",|and")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(2)?) {
                (Token::Distance(a), Token::Distance(b)) => composite(a, b),
                _ => None,
            }),
        },
        // "composite <distance>": adjacent distinct-unit distances (e.g. 5'9").
        Rule {
            name: "composite <distance>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_simple)),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(1)?) {
                (Token::Distance(a), Token::Distance(b)) => composite(a, b),
                _ => None,
            }),
        },
    ];

    // unit words -> attach unit to the preceding distance.
    // Imperial:
    rules.push(unit_rule("miles", r"mi(le(s)?)?", Unit::Mile));
    rules.push(unit_rule("yard", r"y(ar)?ds?", Unit::Yard));
    rules.push(unit_rule("feet", r"('|f(oo|ee)?ts?)", Unit::Foot));
    rules.push(unit_rule("inch", r#"("|''|in(ch(es)?)?)"#, Unit::Inch));
    // Metric:
    rules.push(unit_rule("km", r"k(ilo)?m?(et(er|re))?s?", Unit::Kilometre));
    rules.push(unit_rule("meters", r"met(er|re)s?", Unit::Metre));
    rules.push(unit_rule(
        "centimeters",
        r"cm|centimet(er|re)s?",
        Unit::Centimetre,
    ));
    rules.push(unit_rule(
        "millimeters",
        r"mm|millimet(er|re)s?",
        Unit::Millimetre,
    ));
    // Ambiguous:
    rules.push(unit_rule("m (miles or meters)", r"m", Unit::M));

    rules
}

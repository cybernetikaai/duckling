//! Duration dimension. `DurationData` (the resolved value), the grain→seconds
//! conversion, and the Semigroup that combines durations at the finer grain are
//! all language-agnostic; the words/regexes that produce a duration are
//! per-language. To add a language, add a sibling `duration/<lang>.rs`.
//!
//! External imports are re-exported `pub(super)` so language submodules reach
//! them (and these helpers) via a single `use super::*`.

pub mod en;

pub(super) use crate::grain::Grain;
pub(super) use crate::regex::compile;
pub(super) use crate::types::{PatternItem, Rule, Token};

#[derive(Clone, Debug)]
pub struct DurationData {
    pub value: i64,
    pub grain: Grain,
}

pub(super) fn dur(grain: Grain, value: i64) -> Token {
    Token::Duration(DurationData { value, grain })
}

/// Grain → seconds multiplier (port of TimeGrain.inSeconds); Month = 30 days,
/// Year = 365 days, Quarter = 3 months.
pub fn in_seconds(g: Grain, v: i64) -> i64 {
    let per = match g {
        Grain::NoGrain | Grain::Second => 1,
        Grain::Minute => 60,
        Grain::Hour => 3600,
        Grain::Day => 86400,
        Grain::Week => 604800,
        Grain::Month => 2_592_000,
        Grain::Quarter => 7_776_000,
        Grain::Year => 31_536_000,
    };
    per * v
}

/// Convert a duration to grain `g`, rounding to nearest (port of `withGrain`).
fn with_grain_value(g: Grain, d: &DurationData) -> i64 {
    if g == d.grain {
        d.value
    } else {
        (in_seconds(d.grain, d.value) as f64 / in_seconds(g, 1) as f64).round() as i64
    }
}

/// DurationData Semigroup `<>`: combine at the finer of the two grains (port of
/// the `instance Semigroup DurationData`). "2 years" <> "3 months" = 27 months.
pub(super) fn merge(a: &DurationData, b: &DurationData) -> DurationData {
    let g = a.grain.min(b.grain);
    DurationData {
        value: with_grain_value(g, a) + with_grain_value(g, b),
        grain: g,
    }
}

/// n grains + half a grain, expressed in the next finer grain (port of
/// nPlusOneHalf): half an hour -> 30 min, an hour and a half -> 90 min.
pub(super) fn n_plus_one_half(grain: Grain, n: i64) -> Option<Token> {
    Some(match grain {
        Grain::Minute => dur(Grain::Second, 30 + 60 * n),
        Grain::Hour => dur(Grain::Minute, 30 + 60 * n),
        Grain::Day => dur(Grain::Hour, 12 + 24 * n),
        Grain::Month => dur(Grain::Day, 15 + 30 * n),
        Grain::Year => dur(Grain::Month, 6 + 12 * n),
        _ => return None,
    })
}

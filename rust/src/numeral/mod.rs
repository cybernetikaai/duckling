//! Numeral dimension. `NumeralData` (the resolved value) and the whole-number /
//! ok-for-time accessors are language-agnostic; the words/regexes that produce a
//! numeral are per-language. To add a language, add a sibling `numeral/<lang>.rs`.

pub mod en;

#[derive(Clone, Debug)]
pub struct NumeralData {
    pub value: f64,
    /// false for informal numerals (couple/few/dozen/single/pair) which Duckling
    /// marks notOkForAnyTime — they can't be a time-of-day/year/day-of-month.
    pub ok_for_time: bool,
    /// Power-of-ten exponent for "hundred"/"thousand"/... (Duckling's grain).
    pub grain: Option<i64>,
    /// True for powers of ten (multiplicands like "thousand").
    pub multipliable: bool,
}

impl NumeralData {
    pub fn new(value: f64, ok_for_time: bool) -> Self {
        NumeralData {
            value,
            ok_for_time,
            grain: None,
            multipliable: false,
        }
    }
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

//! Distance dimension (port of Duckling/Distance/Types.hs). `DistanceData`/`Unit`
//! are the value types; the English unit + interval + composite rules and the
//! shared numeral→distance lift live in `en`. `Unit::M` is the *ambiguous*
//! mile-or-metre unit (rendered "m"), disambiguated only when composed with a
//! definite unit. To add a language, add `distance/<lang>.rs`.

pub mod en;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Centimetre,
    Foot,
    Inch,
    Kilometre,
    /// Ambiguous between Mile and Metre; kept until context resolves it.
    M,
    Metre,
    Mile,
    Millimetre,
    Yard,
}

impl Unit {
    /// JSON rendering — lowercase of the Haskell constructor (`toJSON = toLower . show`).
    pub fn as_str(self) -> &'static str {
        match self {
            Unit::Centimetre => "centimetre",
            Unit::Foot => "foot",
            Unit::Inch => "inch",
            Unit::Kilometre => "kilometre",
            Unit::M => "m",
            Unit::Metre => "metre",
            Unit::Mile => "mile",
            Unit::Millimetre => "millimetre",
            Unit::Yard => "yard",
        }
    }
}

#[derive(Clone, Debug)]
pub struct DistanceData {
    pub unit: Option<Unit>,
    pub value: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

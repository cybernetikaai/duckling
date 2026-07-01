//! Volume dimension (port of Duckling/Volume/Types.hs). `VolumeData`/`Unit` are
//! the value types; the English unit + interval rules plus the shared
//! numeral→volume lifts live in `en`. To add a language, add `volume/<lang>.rs`.

pub mod en;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Gallon,
    Hectolitre,
    Litre,
    Centilitre,
    Millilitre,
}

impl Unit {
    /// JSON rendering — lowercase of the Haskell constructor (`toJSON = toLower . show`).
    pub fn as_str(self) -> &'static str {
        match self {
            Unit::Gallon => "gallon",
            Unit::Hectolitre => "hectolitre",
            Unit::Litre => "litre",
            Unit::Centilitre => "centilitre",
            Unit::Millilitre => "millilitre",
        }
    }
}

#[derive(Clone, Debug)]
pub struct VolumeData {
    pub value: Option<f64>,
    pub unit: Option<Unit>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

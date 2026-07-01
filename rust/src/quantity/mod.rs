//! Quantity dimension (port of Duckling/Quantity/Types.hs). `QuantityData`/`Unit`
//! are the value types; the English unit + product + interval rules and the
//! shared numeral lift live in `en`. Only the units the EN rules produce are
//! modelled — Cup, Gram, Ounce, Pound, plus Unnamed (for a value with no unit).
//! Duckling's full `Unit` also has Bowl/Dish/Pint/Quart/Tablespoon/Teaspoon and
//! a `Custom Text`; those are used by other locales/features and can be added
//! alongside a language that produces them. To add a language, add
//! `quantity/<lang>.rs`.

pub mod en;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Unit {
    Cup,
    Gram,
    Ounce,
    Pound,
    /// A value with no named unit (Duckling's `Unnamed`).
    Unnamed,
}

impl Unit {
    /// JSON rendering — lowercase of the Haskell constructor (`toJSON = toLower . show`).
    pub fn as_str(self) -> &'static str {
        match self {
            Unit::Cup => "cup",
            Unit::Gram => "gram",
            Unit::Ounce => "ounce",
            Unit::Pound => "pound",
            Unit::Unnamed => "unnamed",
        }
    }
}

#[derive(Clone, Debug)]
pub struct QuantityData {
    pub unit: Option<Unit>,
    pub value: Option<f64>,
    pub product: Option<String>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    /// When true, dropped unless the caller opts into latent parses.
    pub latent: bool,
}

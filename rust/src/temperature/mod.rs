//! Temperature dimension. `TemperatureData`/`TempUnit` are the value types; the
//! English unit + interval rules (and the shared numeral→latent-temp lift) live
//! in `en`. To add a language, add `temperature/<lang>.rs`.

pub mod en;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TempUnit {
    Degree,
    Celsius,
    Fahrenheit,
}

impl TempUnit {
    pub fn as_str(self) -> &'static str {
        match self {
            TempUnit::Degree => "degree",
            TempUnit::Celsius => "celsius",
            TempUnit::Fahrenheit => "fahrenheit",
        }
    }
}

#[derive(Clone, Debug)]
pub struct TemperatureData {
    pub unit: Option<TempUnit>,
    pub value: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
}

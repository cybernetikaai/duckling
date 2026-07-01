//! Email dimension. `EmailData` is the resolved value; the spoken "at"/"dot"
//! forms make the rule English-specific. To add a language, add `email/<lang>.rs`.

pub mod en;

#[derive(Clone, Debug)]
pub struct EmailData {
    pub value: String,
}

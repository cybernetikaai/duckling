//! Ordinal dimension. `OrdinalData` (the resolved value) is language-agnostic;
//! the words/regexes that produce it are per-language. To add a language, add a
//! sibling `ordinal/<lang>.rs`.

pub mod en;

#[derive(Clone, Debug)]
pub struct OrdinalData {
    pub value: i64,
}

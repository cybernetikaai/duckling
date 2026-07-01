//! TimeGrain dimension. The `Grain` type is language-agnostic (see `crate::grain`);
//! only the words that name a grain are language-specific, so the rules live per
//! language. To add a language, add a sibling `timegrain/<lang>.rs`.

pub mod en;

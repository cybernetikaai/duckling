//! Rust port of Duckling's English Time parsing (behavior-compatible).
//!
//! Strategy: the test corpus (transcribed to `fixtures/en_time_corpus.json`)
//! is the oracle; every rule is driven red->green against it. See
//! `docs/superpowers/plans/2026-06-30-duckling-rust-en-time.md`.

pub mod grain;
pub mod document;
pub mod regex;
pub mod types;
pub mod engine;
pub mod json;
pub mod resolve;
pub mod time;

pub use resolve::{Entity, ResolveContext};

/// Parse `input` against the EN Time rules and return resolved entities.
///
/// Stub until Phase 1: returns empty so the corpus harness compiles and the
/// positive fixtures fail loudly (the intended red baseline).
pub fn parse(_input: &str, _ctx: &ResolveContext) -> Vec<Entity> {
    Vec::new()
}

//! Rust port of Duckling's English Time parsing (behavior-compatible).
//!
//! Strategy: the test corpus (transcribed to `fixtures/en_time_corpus.json`)
//! is the oracle; every rule is driven red->green against it. See
//! `docs/superpowers/plans/2026-06-30-duckling-rust-en-time.md`.

pub mod grain;
pub mod document;
pub mod duration;
pub mod regex;
pub mod types;
pub mod engine;
pub mod json;
pub mod numeral;
pub mod ordinal;
pub mod resolve;
pub mod time;
pub mod timegrain;

pub use resolve::{Entity, ResolveContext};

use document::Document;
use types::{Rule, Token};

thread_local! {
    // Compile the rule set (regexes) once per thread, not once per parse.
    // All dimensions share one rule set; the engine produces Numeral/Time/...
    // tokens and Time rules consume the others via predicate pattern items.
    static RULES: Vec<Rule> = {
        let mut r = numeral::numeral_rules();
        r.extend(ordinal::ordinal_rules());
        r.extend(timegrain::timegrain_rules());
        r.extend(duration::duration_rules());
        r.extend(time::en_rules::en_rules());
        r
    };
}

/// Parse `input` against the EN Time rules and return resolved entities.
pub fn parse(input: &str, ctx: &ResolveContext) -> Vec<Entity> {
    let doc = Document::new(input);
    RULES.with(|rules| {
        let nodes = engine::parse_string(rules, &doc);
        nodes
            .iter()
            .filter_map(|n| match &n.token {
                Token::Time(td) => resolve::resolve_time(td, ctx).map(|value| Entity {
                    dim: "time".to_string(),
                    body: doc.substring(n.range.0, n.range.1),
                    start: n.range.0,
                    end: n.range.1,
                    value,
                    latent: td.latent,
                }),
                _ => None,
            })
            .collect()
    })
}

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
pub mod ranking;
pub mod resolve;
pub mod time;
pub mod timegrain;

pub use resolve::{Entity, ResolveContext};

use document::Document;
use types::{Node, Rule, Token};

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
    static CLASSIFIERS: ranking::Classifiers = ranking::classifiers();
}

/// Parse `input` against the EN Time rules and return resolved entities,
/// ranked (competing parses collapsed to the winner).
pub fn parse(input: &str, ctx: &ResolveContext) -> Vec<Entity> {
    let doc = Document::new(input);
    RULES.with(|rules| {
        let nodes = engine::parse_string(rules, &doc);
        let scored: Vec<(Node, Entity)> = nodes
            .into_iter()
            .filter_map(|n| {
                let td = match &n.token {
                    Token::Time(td) => td.clone(),
                    _ => return None,
                };
                let value = resolve::resolve_time(&td, ctx)?;
                let e = Entity {
                    dim: "time".to_string(),
                    body: doc.substring(n.range.0, n.range.1),
                    start: n.range.0,
                    end: n.range.1,
                    value,
                    latent: td.latent,
                };
                Some((n, e))
            })
            .collect();
        CLASSIFIERS.with(|cl| ranking::rank(cl, scored))
    })
}

/// Debug: every Time candidate (unranked) as "rule | range | score | value".
pub fn parse_all_debug(input: &str, ctx: &ResolveContext) -> Vec<String> {
    let doc = Document::new(input);
    RULES.with(|rules| {
        let nodes = engine::parse_string(rules, &doc);
        CLASSIFIERS.with(|cl| {
            let mut out = Vec::new();
            for n in &nodes {
                let td = match &n.token {
                    Token::Time(td) => td.clone(),
                    _ => continue,
                };
                let value = match resolve::resolve_time(&td, ctx) {
                    Some(v) => v,
                    None => continue,
                };
                let sc = ranking::score(cl, n);
                out.push(format!(
                    "{:<44} [{:>2},{:>2}] score={:>10.4}  {}",
                    n.rule.clone().unwrap_or_default(),
                    n.range.0,
                    n.range.1,
                    sc,
                    serde_json::to_string(&value).unwrap_or_default()
                ));
            }
            out
        })
    })
}

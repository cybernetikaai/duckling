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

pub use types::Locale;

fn build_rules(locale: Locale) -> Vec<Rule> {
    let mut r = numeral::numeral_rules();
    r.extend(ordinal::ordinal_rules());
    r.extend(timegrain::timegrain_rules());
    r.extend(duration::duration_rules());
    r.extend(time::en_rules::en_rules(locale));
    r
}

thread_local! {
    // Compile the rule set (regexes) once per (thread, locale), not once per parse.
    // All dimensions share one rule set; the engine produces Numeral/Time/... tokens
    // and Time rules consume the others via predicate pattern items. Locale variants
    // differ only in numeric-date field order (and, later, regional holidays); each
    // is compiled lazily on first use and cached.
    static RULES: std::cell::RefCell<std::collections::HashMap<Locale, std::rc::Rc<Vec<Rule>>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    static CLASSIFIERS: ranking::Classifiers = ranking::classifiers();
}

fn rules_for(locale: Locale) -> std::rc::Rc<Vec<Rule>> {
    RULES.with(|c| {
        c.borrow_mut()
            .entry(locale)
            .or_insert_with(|| std::rc::Rc::new(build_rules(locale)))
            .clone()
    })
}

fn resolve_entities(rules: &[Rule], doc: &Document, ctx: &ResolveContext) -> Vec<Entity> {
    let nodes = engine::parse_string(rules, doc);
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
}

/// Parse `input` against the EN (US) Time rules and return resolved entities,
/// ranked (competing parses collapsed to the winner).
pub fn parse(input: &str, ctx: &ResolveContext) -> Vec<Entity> {
    parse_locale(input, ctx, Locale::EnUs)
}

/// Parse in a specific English locale. The only behavioral difference is numeric
/// date field order — US "3/4"→March 4, GB "3/4"→April 3 (and GB accepts "13/12").
pub fn parse_locale(input: &str, ctx: &ResolveContext, locale: Locale) -> Vec<Entity> {
    let doc = Document::new(input);
    resolve_entities(&rules_for(locale), &doc, ctx)
}

/// Debug: every Time candidate (unranked) as "rule | range | score | value".
pub fn parse_all_debug(input: &str, ctx: &ResolveContext) -> Vec<String> {
    let doc = Document::new(input);
    let rules = rules_for(Locale::EnUs);
    {
        let nodes = engine::parse_string(&rules, &doc);
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
    }
}

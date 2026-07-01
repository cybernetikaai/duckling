//! Bottom-up saturating matcher.
//!
//! Clean reimplementation of Duckling's chart parser: repeatedly match every
//! rule's full pattern against the regex hits + the produced-token stash until
//! no new token appears. Behavioral fidelity is enforced by the corpus, not by
//! mirroring Haskell's incremental matcher.
//!
//! Termination/dedup: each production is keyed by (rule, start, end). For a
//! given span a rule's matched text is fixed, so its token is deterministic;
//! this avoids needing structural equality on `Token`. (Revisited in Phase 3
//! when predicate combinations can yield distinct tokens at one span.)

use std::collections::{HashMap, HashSet};

use crate::document::Document;
use crate::regex::{Re, RegexHit};
use crate::types::{Node, PatternItem, Range, Rule, Token};

/// Regex hits are invariant across a parse (they depend only on the document),
/// but the saturating loop and the recursive route expansion would otherwise
/// re-scan every regex many times. Precompute each distinct regex's hits once,
/// keyed by the `Re`'s address (rules are compiled once per thread, so the `Re`
/// instances — and their addresses — are stable for the whole parse).
type RegexCache = HashMap<usize, Vec<RegexHit>>;

fn re_key(re: &Re) -> usize {
    re as *const Re as usize
}

pub fn parse_string(rules: &[Rule], doc: &Document) -> Vec<Node> {
    let mut cache: RegexCache = HashMap::new();
    for rule in rules {
        for item in &rule.pattern {
            if let PatternItem::Regex(re) = item {
                cache.entry(re_key(re)).or_insert_with(|| re.all_hits(doc));
            }
        }
    }
    // A regex-only rule (no predicate item) matches only fixed regex hits, so it
    // produces everything it ever will in the first round; skip it thereafter.
    let has_predicate: Vec<bool> = rules
        .iter()
        .map(|r| r.pattern.iter().any(|it| matches!(it, PatternItem::Predicate(_))))
        .collect();

    let mut stash: Vec<Node> = Vec::new();
    let mut emitted: HashSet<(String, usize, usize)> = HashSet::new();
    let mut first = true;
    loop {
        // Collect this round's new nodes separately so match_pattern can read a
        // frozen `&stash` without us cloning the whole (growing) stash each round.
        let mut new_nodes: Vec<Node> = Vec::new();
        for (rule, &has_pred) in rules.iter().zip(&has_predicate) {
            if !first && !has_pred {
                continue;
            }
            for route in match_pattern(&rule.pattern, doc, &stash, None, &cache) {
                if route.is_empty() {
                    continue;
                }
                let tokens: Vec<Token> = route.iter().map(|n| n.token.clone()).collect();
                if let Some(tok) = (rule.prod)(&tokens) {
                    let start = route.first().unwrap().range.0;
                    let end = route.last().unwrap().range.1;
                    if emitted.insert((rule.name.clone(), start, end)) {
                        new_nodes.push(Node {
                            range: Range(start, end),
                            token: tok,
                            rule: Some(rule.name.clone()),
                            children: route,
                        });
                    }
                }
            }
        }
        first = false;
        if new_nodes.is_empty() {
            break;
        }
        stash.extend(new_nodes);
    }
    stash
}

/// All routes matching `items`. `from = None` for the first item (match
/// anywhere); subsequent items must be adjacent to the previous item's end.
fn match_pattern(
    items: &[PatternItem],
    doc: &Document,
    stash: &[Node],
    from: Option<usize>,
    cache: &RegexCache,
) -> Vec<Vec<Node>> {
    let Some((head, tail)) = items.split_first() else {
        return vec![Vec::new()];
    };
    let mut routes = Vec::new();
    for hn in match_item(head, doc, stash, from, cache) {
        let end = hn.range.1;
        for mut rest in match_pattern(tail, doc, stash, Some(end), cache) {
            let mut route = Vec::with_capacity(rest.len() + 1);
            route.push(hn.clone());
            route.append(&mut rest);
            routes.push(route);
        }
    }
    routes
}

fn match_item(
    item: &PatternItem,
    doc: &Document,
    stash: &[Node],
    from: Option<usize>,
    cache: &RegexCache,
) -> Vec<Node> {
    match item {
        PatternItem::Regex(re) => cache[&re_key(re)]
            .iter()
            .filter(|h| match from {
                None => true,
                Some(p) => doc.is_adjacent(p, h.start),
            })
            .map(|h| Node {
                range: Range(h.start, h.end),
                token: Token::RegexMatch(h.groups.clone()),
                rule: None,
                children: Vec::new(),
            })
            .collect(),
        PatternItem::Predicate(f) => stash
            .iter()
            .filter(|n| match from {
                None => true,
                Some(p) => doc.is_adjacent(p, n.range.0),
            })
            .filter(|n| f(&n.token))
            .cloned()
            .collect(),
    }
}

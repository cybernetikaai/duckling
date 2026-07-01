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

use std::collections::HashSet;

use crate::document::Document;
use crate::types::{Node, PatternItem, Range, Rule, Token};

pub fn parse_string(rules: &[Rule], doc: &Document) -> Vec<Node> {
    let mut stash: Vec<Node> = Vec::new();
    let mut emitted: HashSet<(String, usize, usize)> = HashSet::new();
    loop {
        let snapshot = stash.clone();
        let mut added = false;
        for rule in rules {
            for route in match_pattern(&rule.pattern, doc, &snapshot, None) {
                if route.is_empty() {
                    continue;
                }
                let tokens: Vec<Token> = route.iter().map(|n| n.token.clone()).collect();
                if let Some(tok) = (rule.prod)(&tokens) {
                    let start = route.first().unwrap().range.0;
                    let end = route.last().unwrap().range.1;
                    if emitted.insert((rule.name.clone(), start, end)) {
                        stash.push(Node {
                            range: Range(start, end),
                            token: tok,
                            rule: Some(rule.name.clone()),
                            children: route,
                        });
                        added = true;
                    }
                }
            }
        }
        if !added {
            break;
        }
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
) -> Vec<Vec<Node>> {
    let Some((head, tail)) = items.split_first() else {
        return vec![Vec::new()];
    };
    let mut routes = Vec::new();
    for hn in match_item(head, doc, stash, from) {
        let end = hn.range.1;
        for mut rest in match_pattern(tail, doc, stash, Some(end)) {
            let mut route = Vec::with_capacity(rest.len() + 1);
            route.push(hn.clone());
            route.append(&mut rest);
            routes.push(route);
        }
    }
    routes
}

fn match_item(item: &PatternItem, doc: &Document, stash: &[Node], from: Option<usize>) -> Vec<Node> {
    match item {
        PatternItem::Regex(re) => re
            .all_hits(doc)
            .into_iter()
            .filter(|h| match from {
                None => true,
                Some(p) => doc.is_adjacent(p, h.start),
            })
            .map(|h| Node {
                range: Range(h.start, h.end),
                token: Token::RegexMatch(h.groups),
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

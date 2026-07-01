//! Naive-Bayes ranking (port of Duckling/Ranking/{Rank,Types,Extraction}.hs).
//! Collapses competing parses to the winner — the `unique`-mode correctness bar.

use std::cmp::Ordering;
use std::collections::HashMap;

use crate::grain::grain_str;
use crate::types::{Node, Token};

#[derive(Clone)]
pub struct ClassData {
    pub prior: f64,
    pub unseen: f64,
    pub likelihoods: HashMap<String, f64>,
    pub n: i64,
}

#[derive(Clone)]
pub struct Classifier {
    pub ok: ClassData,
    pub ko: ClassData,
}

pub type Classifiers = HashMap<String, Classifier>;

/// Features: (1) concatenated child rule names, (2) concatenated child grains
/// (Time/Duration/TimeGrain). Port of extractFeatures.
fn extract_features(node: &Node) -> HashMap<String, i64> {
    let feat_rules: String = node.children.iter().filter_map(|c| c.rule.clone()).collect();
    let grains: Vec<&str> = node
        .children
        .iter()
        .filter_map(|c| match &c.token {
            Token::Time(td) => Some(grain_str(td.grain)),
            Token::Duration(d) => Some(grain_str(d.grain)),
            Token::TimeGrain(g) => Some(grain_str(*g)),
            _ => None,
        })
        .collect();
    let mut m = HashMap::new();
    m.insert(feat_rules, 1);
    if !grains.is_empty() {
        m.insert(grains.concat(), 1);
    }
    m
}

fn ll(feats: &HashMap<String, i64>, cd: &ClassData) -> f64 {
    cd.prior
        + feats
            .iter()
            .map(|(f, &x)| x as f64 * cd.likelihoods.get(f).copied().unwrap_or(cd.unseen))
            .sum::<f64>()
}

/// Recursive log-likelihood of the parse tree (posLL of this node + children).
pub fn score(cl: &Classifiers, node: &Node) -> f64 {
    match &node.rule {
        Some(r) => match cl.get(r) {
            Some(c) => {
                ll(&extract_features(node), &c.ok)
                    + node.children.iter().map(|ch| score(cl, ch)).sum::<f64>()
            }
            None => 0.0,
        },
        None => 0.0,
    }
}

/// Candidate partial order (Ranking/Types.hs), specialized to a single
/// dimension (we only emit Time): a wider range dominates a contained one;
/// equal ranges compare by score; disjoint/overlapping ranges are incomparable.
fn cmp_cand(a: &(usize, usize, f64), b: &(usize, usize, f64)) -> Ordering {
    let starts = a.0.cmp(&b.0);
    let ends = a.1.cmp(&b.1);
    match starts {
        Ordering::Equal => match ends {
            Ordering::Equal => a.2.partial_cmp(&b.2).unwrap_or(Ordering::Equal),
            z => z,
        },
        Ordering::Less => {
            if ends == Ordering::Less {
                Ordering::Equal
            } else {
                Ordering::Greater
            }
        }
        Ordering::Greater => {
            if ends == Ordering::Greater {
                Ordering::Equal
            } else {
                Ordering::Less
            }
        }
    }
}

/// Keep only the winners (candidates not dominated by any other). `items` pairs
/// each candidate Node with its payload (the resolved Entity).
pub fn rank<T: Clone>(cl: &Classifiers, items: Vec<(Node, T)>) -> Vec<T> {
    let cands: Vec<(usize, usize, f64)> =
        items.iter().map(|(n, _)| (n.range.0, n.range.1, score(cl, n))).collect();
    (0..cands.len())
        .filter(|&i| {
            (0..cands.len())
                .all(|j| i == j || cmp_cand(&cands[i], &cands[j]) != Ordering::Less)
        })
        .map(|i| items[i].1.clone())
        .collect()
}

/// The EN classifier model. Stub until the transcribed model lands; an empty
/// model scores everything 0, so `rank` only drops range-dominated candidates
/// (contains-mode is unchanged; unique-mode needs the real weights).
pub fn classifiers() -> Classifiers {
    HashMap::new()
}

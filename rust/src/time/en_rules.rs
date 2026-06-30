//! English Time rules. Phase 1: the instant rules (now/today/tomorrow/yesterday).

use crate::grain::Grain;
use crate::regex::compile;
use crate::time::predicate::cycle_nth;
use crate::types::{PatternItem, Rule, TimeData, Token};

/// A rule whose regex matches an instant phrase and produces `cycle_nth(g, n)`.
fn instant(name: &str, g: Grain, n: i64, re: &str) -> Rule {
    Rule {
        name: name.to_string(),
        pattern: vec![PatternItem::Regex(compile(re))],
        prod: Box::new(move |_tokens| Some(Token::Time(TimeData::new(cycle_nth(g, n), g)))),
    }
}

pub fn en_rules() -> Vec<Rule> {
    vec![
        instant("now", Grain::Second, 0, r"now|at\s+the\s+moment|atm"),
        instant("today", Grain::Day, 0, r"todays?|at\s+this\s+time"),
        instant("tomorrow", Grain::Day, 1, r"tmrw?|tomm?or?rows?"),
        instant("yesterday", Grain::Day, -1, r"yesterdays?"),
    ]
}

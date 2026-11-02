//! A day range intersected with a clock part ("Mon-Fri 9am-5pm") names many
//! windows, so it is reported as its two parts rather than its first window.

use crate::grain::Grain;
use crate::types::{Node, Token};

const DAY_RANGE_RULES: &[&str] = &[
    "<datetime> - <datetime> (interval)",
    "<datetime>/<datetime> (interval)",
    "from <datetime> - <datetime> (interval)",
    "between <time> and <time>",
    "<month> dd-dd (interval)",
    "dd-dd <month> (interval)",
    "dd-dd of <month> (interval)",
    "from <month> dd-dd (interval)",
    "from the <day-of-month> (ordinal or number) to the <day-of-month> (ordinal or number) <named-month> (interval)",
    "from the <day-of-month> (ordinal or number) to the <day-of-month> (ordinal or number) of <named-month> (interval)",
];

const INTERSECT_RULES: &[&str] = &["intersect", "intersect by \",\", \"of\", \"from\", \"'s\""];

const WRAPPER_RULES: &[&str] = &["about|exactly <time-of-day>"];

fn rule_in(n: &Node, rules: &[&str]) -> bool {
    n.rule.as_deref().is_some_and(|r| rules.contains(&r))
}

fn grain(n: &Node) -> Option<Grain> {
    match &n.token {
        Token::Time(td) => Some(td.grain),
        _ => None,
    }
}

fn time_children(n: &Node) -> Vec<&Node> {
    n.children.iter().filter(|c| grain(c).is_some()).collect()
}

/// An explicit range or list of days, alone or narrowed by another date part
/// ("next week Mon-Fri").
fn is_day_range(n: &Node) -> bool {
    grain(n) == Some(Grain::Day)
        && (rule_in(n, DAY_RANGE_RULES)
            || (rule_in(n, INTERSECT_RULES) && time_children(n).into_iter().any(is_day_range)))
}

/// A time of day or clock range that names no day of its own.
fn is_clock(n: &Node) -> bool {
    fn names_no_day(n: &Node) -> bool {
        grain(n).is_none_or(|g| g < Grain::Day) && n.children.iter().all(names_no_day)
    }
    grain(n).is_some_and(|g| g < Grain::Day) && names_no_day(n)
}

/// The day-range part and the clock part of `n`, in source order, when `n`
/// intersects the two.
pub fn split(n: &Node) -> Option<[&Node; 2]> {
    let parts = time_children(n);
    if rule_in(n, WRAPPER_RULES) && parts.len() == 1 {
        return split(parts[0]);
    }
    if !rule_in(n, INTERSECT_RULES) || parts.len() != 2 {
        return None;
    }
    let (a, b) = (parts[0], parts[1]);
    ((is_day_range(a) && is_clock(b)) || (is_clock(a) && is_day_range(b))).then_some([a, b])
}

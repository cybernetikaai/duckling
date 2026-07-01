//! Minimal TimeGrain dimension — grain words ("week", "month", ...) -> Grain.

use crate::grain::Grain;
use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

pub fn timegrain_rules() -> Vec<Rule> {
    // Regexes and rule names ported from Duckling/TimeGrain/EN/Rules.hs (the
    // names feed ranking features like "day (grain)tomorrow"). Abbreviations
    // matter: "yr" (year), "hr"/"h" (hour), "m"/"min" (minute).
    let grains: [(&str, Grain, &str); 8] = [
        ("second (grain) ", Grain::Second, r"sec(ond)?s?"),
        ("minute (grain)", Grain::Minute, r"m(in(ute)?s?)?"),
        ("hour (grain)", Grain::Hour, r"h(((ou)?rs?)|r)?"),
        ("day (grain)", Grain::Day, r"days?"),
        ("week (grain)", Grain::Week, r"weeks?"),
        ("month (grain)", Grain::Month, r"months?"),
        ("quarter (grain)", Grain::Quarter, r"(quarter|qtr)s?"),
        ("year (grain)", Grain::Year, r"y(ea)?rs?"),
    ];
    grains
        .iter()
        .map(|&(name, g, re)| Rule {
            name: name.to_string(),
            pattern: vec![PatternItem::Regex(compile(re))],
            prod: Box::new(move |_| Some(Token::TimeGrain(g))),
        })
        .collect()
}

//! Minimal TimeGrain dimension — grain words ("week", "month", ...) -> Grain.

use crate::grain::Grain;
use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

pub fn timegrain_rules() -> Vec<Rule> {
    let grains: [(&str, Grain, &str); 8] = [
        ("second", Grain::Second, r"sec(ond)?s?"),
        ("minute", Grain::Minute, r"min(ute)?s?"),
        ("hour", Grain::Hour, r"hours?|hrs?"),
        ("day", Grain::Day, r"days?"),
        ("week", Grain::Week, r"weeks?"),
        ("month", Grain::Month, r"months?"),
        ("quarter", Grain::Quarter, r"quarters?|qtrs?"),
        ("year", Grain::Year, r"years?"),
    ];
    grains
        .iter()
        .map(|&(name, g, re)| Rule {
            name: format!("grain ({name})"),
            pattern: vec![PatternItem::Regex(compile(re))],
            prod: Box::new(move |_| Some(Token::TimeGrain(g))),
        })
        .collect()
}

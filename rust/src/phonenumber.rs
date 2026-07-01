//! PhoneNumber dimension (language-agnostic) — port of
//! `Duckling/PhoneNumber/Rules.hs`. One regex captures optional area code,
//! the number body, and optional extension; the resolved value is the
//! normalized string "(+<code>) <digits> ext <ext>".

use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

#[derive(Clone, Debug)]
pub struct PhoneNumberData {
    pub prefix: Option<i64>,
    pub number: String,
    pub extension: Option<i64>,
}

fn cleanup(s: &str) -> String {
    s.chars()
        .filter(|c| !matches!(c, '.' | ' ' | '-' | '\t' | '(' | ')'))
        .collect()
}

pub fn phonenumber_rules() -> Vec<Rule> {
    // Hyphens escaped inside char classes; the {0,20}/{1,20} caps bound
    // backtracking (verbatim from Duckling's comment).
    let re = concat!(
        r"(?:\(?\+(\d{1,2})\)?[\s\-\.]*)?",
        r"((?=[\-\d()\s\.]{6,16}(?:\s*e?xt?\.?\s*(?:\d{1,20}))?(?:[^\d]+|$))",
        r"(?:[\d(]{1,20}(?:[\-)\s\.]*\d{1,20}){0,20}){1,20})",
        r"(?:\s*e?xt?\.?\s*(\d{1,20}))?",
    );
    vec![Rule {
        name: "phone number".into(),
        pattern: vec![PatternItem::Regex(compile(re))],
        prod: Box::new(|tokens| {
            let g = match tokens.first() {
                Some(Token::RegexMatch(g)) => g,
                _ => return None,
            };
            let parse = |i: usize| {
                g.get(i)
                    .filter(|s| !s.is_empty())
                    .and_then(|s| s.parse::<i64>().ok())
            };
            Some(Token::Phone(PhoneNumberData {
                prefix: parse(0),
                number: cleanup(g.get(1)?),
                extension: parse(2),
            }))
        }),
    }]
}

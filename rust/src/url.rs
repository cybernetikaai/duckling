//! Url dimension (language-agnostic) — port of `Duckling/Url/Rules.hs`.
//! Three rules (general URL, localhost, protocol+host). The value is the whole
//! match; `domain` is the host, lowercased.

use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

#[derive(Clone, Debug)]
pub struct UrlData {
    pub value: String,
    pub domain: String,
}

fn mk(value: &str, domain: &str) -> Token {
    Token::Url(UrlData {
        value: value.to_string(),
        domain: domain.to_lowercase(),
    })
}

fn groups(t: &Token) -> Option<&Vec<String>> {
    match t {
        Token::RegexMatch(g) => Some(g),
        _ => None,
    }
}

pub fn url_rules() -> Vec<Rule> {
    vec![
        Rule {
            name: "url".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"((([a-zA-Z]+)://)?(w{2,3}[0-9]*\.)?(([\w_-]+\.)+[a-z]{2,4})(:(\d+))?(/[^?\s#]*)?(\?[^\s#]+)?(#[\-,*=&a-z0-9]+)?)",
            ))],
            prod: Box::new(|tokens| {
                let g = groups(tokens.first()?)?;
                Some(mk(g.first()?, g.get(4)?))
            }),
        },
        Rule {
            name: "localhost".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"((([a-zA-Z]+)://)?localhost(:(\d+))?(/[^?\s#]*)?(\?[^\s#]+)?)",
            ))],
            prod: Box::new(|tokens| {
                let g = groups(tokens.first()?)?;
                Some(mk(g.first()?, "localhost"))
            }),
        },
        Rule {
            name: "local url".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(([a-zA-Z]+)://([\w_-]+)(:(\d+))?(/[^?\s#]*)?(\?[^\s#]+)?)",
            ))],
            prod: Box::new(|tokens| {
                let g = groups(tokens.first()?)?;
                Some(mk(g.first()?, g.get(2)?))
            }),
        },
    ]
}

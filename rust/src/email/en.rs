//! English (`en`) Email rules — port of `Duckling/Email/EN/Rules.hs`.
//! One regex; the local and domain groups have their spoken " dot " turned back
//! into "." and are joined with "@".

use super::EmailData;
use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

pub fn email_rules() -> Vec<Rule> {
    vec![Rule {
        name: "email spelled out".into(),
        pattern: vec![PatternItem::Regex(compile(
            r"([\w_+-]+(?:(?: dot |\.)[\w_+-]+){0,10})(?: at |@)([a-zA-Z]+(?:(?:\.| dot )[\w_-]+){1,10})",
        ))],
        prod: Box::new(|tokens| {
            let g = match tokens.first() {
                Some(Token::RegexMatch(g)) => g,
                _ => return None,
            };
            let local = g.first()?.replace(" dot ", ".");
            let domain = g.get(1)?.replace(" dot ", ".");
            Some(Token::Email(EmailData {
                value: format!("{local}@{domain}"),
            }))
        }),
    }]
}

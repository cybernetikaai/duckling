//! English (`en`) Email rules — port of `Duckling/Email/EN/Rules.hs`.
//! One regex; the local and domain groups have their spoken " dot " turned back
//! into "." and are joined with "@".

use super::EmailData;
use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

pub fn email_rules() -> Vec<Rule> {
    vec![Rule {
        name: "email spelled out".into(),
        // The trailing `(?!...)` guards against a literal `local@domain` email
        // being mistaken for a spoken-form one: a dotted local part (e.g.
        // "lina.muller@teamfoxy.ai") is itself shaped like a valid
        // "<local> at <domain>" match ("me at lina.muller"), stopping right
        // before the real "@". Reject any match whose "domain" is directly
        // followed (once you keep reading contiguous word/dot/hyphen chars)
        // by a literal "@" — that's the sign the true "@" of a real email is
        // just past what we grabbed as the domain, at any backtrack depth.
        pattern: vec![PatternItem::Regex(compile(
            r"([\w_+-]+(?:(?: dot |\.)[\w_+-]+){0,10})(?: at |@)([a-zA-Z]+(?:(?:\.| dot )[\w_-]+){1,10})(?![\w.-]*@)",
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

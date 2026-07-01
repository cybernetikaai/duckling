//! CreditCardNumber dimension (language-agnostic) — port of
//! `Duckling/CreditCardNumber`. Per-issuer regexes (with/without dashes) plus a
//! catch-all "other" (negative-lookahead against the issuers), then a Luhn +
//! length check. Emits dim "credit-card-number" {value, issuer}.

use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

#[derive(Clone, Debug)]
pub struct CreditCardData {
    pub number: String,
    pub issuer: &'static str,
}

const VISA: &str = r"(4[0-9]{15}|4[0-9]{3}-[0-9]{4}-[0-9]{4}-[0-9]{4})";
const AMEX: &str = r"(3[47][0-9]{13}|3[47][0-9]{2}-[0-9]{6}-[0-9]{5})";
const DISCOVER: &str =
    r"(6(?:011|[45][0-9]{2})[0-9]{12}|6(?:011|[45][0-9]{2})-[0-9]{4}-[0-9]{4}-[0-9]{4})";
const MASTERCARD: &str = r"(5[1-5][0-9]{14}|5[1-5][0-9]{2}-[0-9]{4}-[0-9]{4}-[0-9]{4})";
const DINERCLUB: &str =
    r"(3(?:0[0-5]|[68][0-9])[0-9]{11}|3(?:0[0-5]|[68][0-9])[0-9]-[0-9]{6}-[0-9]{4})";

fn other_regex() -> String {
    format!("((?!{VISA})(?!{AMEX})(?!{DISCOVER})(?!{MASTERCARD})(?!{DINERCLUB})\\d{{8,19}})")
}

/// Luhn checksum + length (8..=19), on a digits-only string (dashes stripped).
fn is_valid(digits: &str) -> bool {
    let len = digits.len();
    if !(8..=19).contains(&len) || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let mut sum: i64 = 0;
    let mut e: u32 = 0; // from the right; every other digit is doubled
    for b in digits.bytes().rev() {
        let doubled = ((b - b'0') as i64) << e;
        sum += if doubled > 9 { doubled - 9 } else { doubled };
        e = 1 - e;
    }
    sum % 10 == 0
}

fn cc_rule(name: &'static str, re: String, issuer: &'static str) -> Rule {
    Rule {
        name: name.into(),
        pattern: vec![PatternItem::Regex(compile(&re))],
        prod: Box::new(move |tokens| {
            let g = match tokens.first() {
                Some(Token::RegexMatch(g)) => g,
                _ => return None,
            };
            let number: String = g.first()?.chars().filter(char::is_ascii_digit).collect();
            is_valid(&number).then_some(Token::CreditCard(CreditCardData { number, issuer }))
        }),
    }
}

pub fn creditcard_rules() -> Vec<Rule> {
    vec![
        cc_rule("visa credit card number", VISA.to_string(), "visa"),
        cc_rule("amex card number", AMEX.to_string(), "amex"),
        cc_rule("discover card number", DISCOVER.to_string(), "discover"),
        cc_rule(
            "mastercard card number",
            MASTERCARD.to_string(),
            "mastercard",
        ),
        cc_rule("diner club card number", DINERCLUB.to_string(), "dinerclub"),
        cc_rule("credit card number", other_regex(), "other"),
    ]
}

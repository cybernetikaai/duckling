//! English (`en`) AmountOfMoney rules — port of Duckling/AmountOfMoney/Rules.hs
//! (shared: currency lookup, <amount> <unit>, latent) and
//! Duckling/AmountOfMoney/EN/Rules.hs (symbols/words, cents composition,
//! intervals, precision). Runs in AmountOfMoney's own rule set (numerals +
//! these), never the Time set.

use super::{AmountOfMoneyData, Currency};
use crate::regex::compile;
use crate::types::{PatternItem, Rule, Token};

/// Duckling's `currencies` map: a matched currency spelling (lowercased) -> Currency.
fn currency_from(m: &str) -> Option<Currency> {
    use Currency::*;
    Some(match m {
        "aed" => Aed,
        "aud" => Aud,
        "bgn" => Bgn,
        "brl" => Brl,
        "byn" => Byn,
        "cad" => Cad,
        "\u{a2}" | "c" => Cent,
        "chf" => Chf,
        "cny" | "rmb" | "yuan" => Cny,
        "czk" => Czk,
        "$" => Dollar,
        "dinar" | "dinars" => Dinar,
        "dkk" => Dkk,
        "dollar" | "dollars" => Dollar,
        "egp" => Egp,
        "\u{20ac}" | "eur" | "euro" | "euros" | "eurs" | "\u{20ac}ur" | "\u{20ac}uro"
        | "\u{20ac}uros" | "\u{20ac}urs" => Eur,
        "gbp" => Gbp,
        "gel" | "lari" | "\u{20be}" => Gel,
        "hkd" => Hkd,
        "hrk" => Hrk,
        "idr" => Idr,
        "ils" | "\u{20aa}" | "nis" | "shekel" | "shekels" => Ils,
        "inr" | "rs" | "rs." | "rupee" | "rupees" => Inr,
        "iqd" => Iqd,
        "jmd" => Jmd,
        "jod" => Jod,
        "\u{a5}" | "jpy" | "yen" => Jpy,
        "krw" => Krw,
        "kwd" => Kwd,
        "lbp" => Lbp,
        "mad" => Mad,
        "mnt" | "\u{20ae}" | "tugrik" | "tugriks" => Mnt,
        "myr" | "rm" => Myr,
        "nok" => Nok,
        "nzd" => Nzd,
        "pkr" => Pkr,
        "pln" => Pln,
        "\u{a3}" => Pound,
        "pt" | "pta" | "ptas" | "pts" => Pts,
        "qar" => Qar,
        "\u{20bd}" | "rub" => Rub,
        "rial" | "rials" => Rial,
        "riyal" | "riyals" => Riyal,
        "ron" => Ron,
        "sar" => Sar,
        "sek" => Sek,
        "sgd" => Sgd,
        "thb" => Thb,
        "ttd" => Ttd,
        "\u{20b4}" | "uah" => Uah,
        "usd" | "us$" => Usd,
        "vnd" => Vnd,
        "zar" => Zar,
        "tl" | "lira" | "\u{20ba}" => Try,
        _ => return None,
    })
}

// ----- predicates (Duckling/AmountOfMoney/Helpers.hs) -----

fn is_integer(v: f64) -> bool {
    v.fract() == 0.0
}
fn is_positive(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if n.value >= 0.0)
}
fn is_natural(t: &Token) -> bool {
    matches!(t, Token::Numeral(n) if is_integer(n.value) && n.value >= 0.0)
}
fn is_cents(t: &Token) -> bool {
    matches!(t, Token::AmountOfMoney(a) if a.value.is_some() && a.currency == Currency::Cent)
}
/// Not cents, and an integer value (so cents can be appended).
fn is_without_cents(t: &Token) -> bool {
    match t {
        Token::AmountOfMoney(a) if a.currency == Currency::Cent => false,
        Token::AmountOfMoney(a) => a.value.is_some_and(is_integer),
        _ => false,
    }
}
fn is_money_with_value(t: &Token) -> bool {
    matches!(t, Token::AmountOfMoney(a) if a.value.is_some() || a.min.is_some() || a.max.is_some())
}
fn is_currency_only(t: &Token) -> bool {
    matches!(t, Token::AmountOfMoney(a) if a.value.is_none() && a.min.is_none() && a.max.is_none())
}
fn is_dollar_coin(t: &Token) -> bool {
    use Currency::*;
    matches!(t, Token::AmountOfMoney(a)
        if a.value.is_some_and(|d| d == 0.05 || d == 0.1 || d == 0.25)
           && matches!(a.currency, Dollar | Aud | Cad | Jmd | Nzd | Sgd | Ttd | Usd))
}
fn is_simple(t: &Token) -> bool {
    matches!(t, Token::AmountOfMoney(a) if a.value.is_some() && a.min.is_none() && a.max.is_none())
}

// ----- value constructors (Duckling/AmountOfMoney/Helpers.hs) -----

fn currency_only(c: Currency) -> AmountOfMoneyData {
    AmountOfMoneyData {
        value: None,
        currency: c,
        min: None,
        max: None,
        latent: false,
    }
}
fn value_only(v: f64) -> AmountOfMoneyData {
    AmountOfMoneyData {
        value: Some(v),
        currency: Currency::Unnamed,
        min: None,
        max: None,
        latent: false,
    }
}
/// Append `x` as cents: if there is already a value, add x/100 (keep currency);
/// otherwise the amount *is* `x` cents.
fn with_cents(x: f64, fd: &AmountOfMoneyData) -> AmountOfMoneyData {
    match fd.value {
        Some(val) => AmountOfMoneyData {
            value: Some(val + x / 100.0),
            ..fd.clone()
        },
        None => AmountOfMoneyData {
            value: Some(x),
            currency: Currency::Cent,
            min: None,
            max: None,
            latent: false,
        },
    }
}
fn with_interval(from: f64, to: f64, c: Currency) -> Token {
    Token::AmountOfMoney(AmountOfMoneyData {
        value: None,
        currency: c,
        min: Some(from),
        max: Some(to),
        latent: false,
    })
}
fn open_min(from: f64, c: Currency) -> Token {
    Token::AmountOfMoney(AmountOfMoneyData {
        value: None,
        currency: c,
        min: Some(from),
        max: None,
        latent: false,
    })
}
fn open_max(to: f64, c: Currency) -> Token {
    Token::AmountOfMoney(AmountOfMoneyData {
        value: None,
        currency: c,
        min: None,
        max: Some(to),
        latent: false,
    })
}

/// A simple regex-word -> currency-only rule (pounds, dirham, ringgit, …).
fn word_currency_rule(name: &'static str, re: &str, c: Currency) -> Rule {
    Rule {
        name: name.into(),
        pattern: vec![PatternItem::Regex(compile(re))],
        prod: Box::new(move |_| Some(Token::AmountOfMoney(currency_only(c)))),
    }
}

const PRECISION: &str =
    r"exactly|precisely|about|approx(\.|imately)?|close to|near( to)?|around|almost";

pub fn rules() -> Vec<Rule> {
    let mut rules: Vec<Rule> = vec![
        // ===== shared (Duckling/AmountOfMoney/Rules.hs) =====
        // "currencies": a symbol/code word -> currency-only.
        Rule {
            name: "currencies".into(),
            pattern: vec![PatternItem::Regex(compile(
                r"(aed|aud|bgn|brl|byn|\x{a2}|cad|chf|cny|c|\$|dinars?|dkk|dollars?|egp|(e|\x{20ac})uro?s?|\x{20ac}|gbp|gel|\x{20be}|hkd|hrk|idr|ils|\x{20aa}|inr|iqd|jmd|jod|\x{a5}|jpy|lari|krw|kwd|lbp|mad|\x{20ae}|mnt|tugriks?|myr|rm|nis|nok|nzd|\x{a3}|pkr|pln|pta?s?|qar|\x{20bd}|rs\.?|riy?als?|ron|rub|rupees?|sar|sek|sgb|shekels?|thb|ttd|\x{20b4}|uah|us(d|\$)|vnd|yen|yuan|zar|tl|lira|\x{20ba})",
            ))],
            prod: Box::new(|tokens| match tokens.first()? {
                Token::RegexMatch(g) => Some(Token::AmountOfMoney(currency_only(currency_from(
                    &g.first()?.to_lowercase(),
                )?))),
                _ => None,
            }),
        },
        // "<amount> <unit>": positive numeral then currency-only. "10 dollars".
        Rule {
            name: "<amount> <unit>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_positive)),
                PatternItem::Predicate(Box::new(is_currency_only)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(1)?) {
                (Token::Numeral(n), Token::AmountOfMoney(a)) => {
                    Some(Token::AmountOfMoney(AmountOfMoneyData {
                        value: Some(n.value),
                        ..currency_only(a.currency)
                    }))
                }
                _ => None,
            }),
        },
        // "<amount> (latent)": a bare positive numeral is a latent unnamed amount.
        Rule {
            name: "<amount> (latent)".into(),
            pattern: vec![PatternItem::Predicate(Box::new(is_positive))],
            prod: Box::new(|tokens| match tokens.first()? {
                Token::Numeral(n) => Some(Token::AmountOfMoney(AmountOfMoneyData {
                    latent: true,
                    ..value_only(n.value)
                })),
                _ => None,
            }),
        },
        // ===== EN (Duckling/AmountOfMoney/EN/Rules.hs) =====
        // "<unit> <amount>": currency-only then positive numeral. "$10", "EUR 20".
        Rule {
            name: "<unit> <amount>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_currency_only)),
                PatternItem::Predicate(Box::new(is_positive)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(1)?) {
                (Token::AmountOfMoney(a), Token::Numeral(n)) => {
                    Some(Token::AmountOfMoney(AmountOfMoneyData {
                        value: Some(n.value),
                        ..currency_only(a.currency)
                    }))
                }
                _ => None,
            }),
        },
        // "a <currency>": "a dollar" -> value 1 of that currency.
        Rule {
            name: "a <currency>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"an?")),
                PatternItem::Predicate(Box::new(is_currency_only)),
            ],
            prod: Box::new(|tokens| match tokens.get(1)? {
                Token::AmountOfMoney(a) => Some(Token::AmountOfMoney(AmountOfMoneyData {
                    value: Some(1.0),
                    ..a.clone()
                })),
                _ => None,
            }),
        },
        // "a <amount-of-money>": absorb a leading article. "a US$4.7 billion".
        Rule {
            name: "a <amount-of-money>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"an?")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| tokens.get(1).cloned()),
        },
        // "a <dollar coin>".
        Rule {
            name: "a <dollar coin>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"an?")),
                PatternItem::Predicate(Box::new(is_dollar_coin)),
            ],
            prod: Box::new(|tokens| tokens.get(1).cloned()),
        },
        // "X <dollar coins>": natural count of coins.
        Rule {
            name: "X <dollar coins>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_natural)),
                PatternItem::Predicate(Box::new(is_dollar_coin)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(1)?) {
                (Token::Numeral(n), Token::AmountOfMoney(a)) => {
                    Some(Token::AmountOfMoney(AmountOfMoneyData {
                        value: Some(n.value * a.value?),
                        ..currency_only(a.currency)
                    }))
                }
                _ => None,
            }),
        },
        // "intersect": <no-cents amount> <natural> -> append as cents. "$20 43".
        Rule {
            name: "intersect".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_without_cents)),
                PatternItem::Predicate(Box::new(is_natural)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(1)?) {
                (Token::AmountOfMoney(a), Token::Numeral(n)) => {
                    Some(Token::AmountOfMoney(with_cents(n.value, a)))
                }
                _ => None,
            }),
        },
        // "intersect (and number)": "twenty dollar and 43".
        Rule {
            name: "intersect (and number)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_without_cents)),
                PatternItem::Regex(compile(r"and")),
                PatternItem::Predicate(Box::new(is_natural)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(2)?) {
                (Token::AmountOfMoney(a), Token::Numeral(n)) => {
                    Some(Token::AmountOfMoney(with_cents(n.value, a)))
                }
                _ => None,
            }),
        },
        // "intersect (and X cents)": "$20 and 43c".
        Rule {
            name: "intersect (and X cents)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_without_cents)),
                PatternItem::Regex(compile(r"and")),
                PatternItem::Predicate(Box::new(is_cents)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(2)?) {
                (Token::AmountOfMoney(a), Token::AmountOfMoney(c)) => {
                    Some(Token::AmountOfMoney(with_cents(c.value?, a)))
                }
                _ => None,
            }),
        },
        // "intersect (X cents)": "20 dollars 43 cents".
        Rule {
            name: "intersect (X cents)".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_without_cents)),
                PatternItem::Predicate(Box::new(is_cents)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(1)?) {
                (Token::AmountOfMoney(a), Token::AmountOfMoney(c)) => {
                    Some(Token::AmountOfMoney(with_cents(c.value?, a)))
                }
                _ => None,
            }),
        },
        // "about|exactly <amount>": precision markers pass the amount through.
        Rule {
            name: "about|exactly <amount-of-money>".into(),
            pattern: vec![
                PatternItem::Regex(compile(PRECISION)),
                PatternItem::Predicate(Box::new(is_money_with_value)),
            ],
            prod: Box::new(|tokens| tokens.get(1).cloned()),
        },
        // "between|from <numeral> to|and <amount>": interval, from < to.
        Rule {
            name: "between|from <numeral> to|and <amount-of-money>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"between|from")),
                PatternItem::Predicate(Box::new(is_positive)),
                PatternItem::Regex(compile(r"to|and")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.get(1)?, tokens.get(3)?) {
                (Token::Numeral(n), Token::AmountOfMoney(a)) => {
                    let (from, to) = (n.value, a.value?);
                    let c = a.currency;
                    (from < to).then(|| with_interval(from, to, c))
                }
                _ => None,
            }),
        },
        // "between|from <amount> to|and <amount>": interval, same currency.
        Rule {
            name: "between|from <amount-of-money> to|and <amount-of-money>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"between|from")),
                PatternItem::Predicate(Box::new(is_simple)),
                PatternItem::Regex(compile(r"to|and")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.get(1)?, tokens.get(3)?) {
                (Token::AmountOfMoney(a), Token::AmountOfMoney(b)) => {
                    let (from, to) = (a.value?, b.value?);
                    (from < to && a.currency == b.currency)
                        .then(|| with_interval(from, to, a.currency))
                }
                _ => None,
            }),
        },
        // "under/less/lower/no more than <amount>": max only.
        Rule {
            name: "under/less/lower/no more than <amount-of-money>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"under|at most|(less|lower|not? more) than")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match tokens.get(1)? {
                Token::AmountOfMoney(a) => Some(open_max(a.value?, a.currency)),
                _ => None,
            }),
        },
        // "over/above/at least/more than <amount>": min only.
        Rule {
            name: "over/above/at least/more than <amount-of-money>".into(),
            pattern: vec![
                PatternItem::Regex(compile(r"over|above|at least|(more|not? less) than")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match tokens.get(1)? {
                Token::AmountOfMoney(a) => Some(open_min(a.value?, a.currency)),
                _ => None,
            }),
        },
        // "<numeral> - <amount>": interval, from < to.
        Rule {
            name: "<numeral> - <amount-of-money>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_natural)),
                PatternItem::Regex(compile(r"-")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(2)?) {
                (Token::Numeral(n), Token::AmountOfMoney(a)) => {
                    let (from, to) = (n.value, a.value?);
                    let c = a.currency;
                    (from < to).then(|| with_interval(from, to, c))
                }
                _ => None,
            }),
        },
        // "<amount> - <amount>": interval, same currency.
        Rule {
            name: "<amount-of-money> - <amount-of-money>".into(),
            pattern: vec![
                PatternItem::Predicate(Box::new(is_simple)),
                PatternItem::Regex(compile(r"-")),
                PatternItem::Predicate(Box::new(is_simple)),
            ],
            prod: Box::new(|tokens| match (tokens.first()?, tokens.get(2)?) {
                (Token::AmountOfMoney(a), Token::AmountOfMoney(b)) => {
                    let (from, to) = (a.value?, b.value?);
                    (from < to && a.currency == b.currency)
                        .then(|| with_interval(from, to, a.currency))
                }
                _ => None,
            }),
        },
        // "(egyptian|lebanese) pounds" -> EGP / LBP.
        Rule {
            name: "other pounds".into(),
            pattern: vec![PatternItem::Regex(compile(r"(egyptian|lebanese) ?pounds?"))],
            prod: Box::new(|tokens| match tokens.first()? {
                Token::RegexMatch(g) => match g.first()?.to_lowercase().as_str() {
                    "egyptian" => Some(Token::AmountOfMoney(currency_only(Currency::Egp))),
                    "lebanese" => Some(Token::AmountOfMoney(currency_only(Currency::Lbp))),
                    _ => None,
                },
                _ => None,
            }),
        },
        // "(qatari|saudi) riyals" -> QAR / SAR.
        Rule {
            name: "riyals".into(),
            pattern: vec![PatternItem::Regex(compile(r"(qatari|saudi) ?riyals?"))],
            prod: Box::new(|tokens| match tokens.first()? {
                Token::RegexMatch(g) => match g.first()?.to_lowercase().as_str() {
                    "qatari" => Some(Token::AmountOfMoney(currency_only(Currency::Qar))),
                    "saudi" => Some(Token::AmountOfMoney(currency_only(Currency::Sar))),
                    _ => None,
                },
                _ => None,
            }),
        },
        // "(kuwaiti) dinars" -> KWD.
        Rule {
            name: "dinars".into(),
            pattern: vec![PatternItem::Regex(compile(r"(kuwaiti) ?dinars?"))],
            prod: Box::new(|tokens| match tokens.first()? {
                Token::RegexMatch(g) => match g.first()?.to_lowercase().as_str() {
                    "kuwaiti" => Some(Token::AmountOfMoney(currency_only(Currency::Kwd))),
                    _ => None,
                },
                _ => None,
            }),
        },
        // "L.E" abbreviation -> EGP.
        word_currency_rule("livre égyptienne", r"[lL].?[eE].?", Currency::Egp),
        // "geneh" (arabizi) -> EGP.
        word_currency_rule(
            "geneh",
            r"[Gg][eiy]*n[eiy]*h(at)?( m[aiey]?sr[eiy]+a?)?",
            Currency::Egp,
        ),
        word_currency_rule("£", r"pounds?", Currency::Pound),
        word_currency_rule("dirham", r"dirhams?", Currency::Aed),
        word_currency_rule("ringgit", r"(malaysian? )?ringgits?", Currency::Myr),
        word_currency_rule("hryvnia", r"hryvnia", Currency::Uah),
        word_currency_rule("cent", r"cents?|penn(y|ies)|pence|sens?", Currency::Cent),
        word_currency_rule("kopiyka", r"kopiy(ok|kas?)", Currency::Cent),
        word_currency_rule("bucks", r"bucks?", Currency::Unnamed),
    ];

    rules.shrink_to_fit();
    rules
}

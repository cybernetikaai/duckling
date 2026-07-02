//! Rust port of Duckling's English Time parsing (behavior-compatible).
//!
//! Strategy: the test corpus (transcribed to `fixtures/en_time_corpus.json`)
//! is the oracle; every rule is driven red->green against it. See
//! `docs/RUST_PORT_PROGRESS.md` for the milestone log and design notes.

// The port contains zero `unsafe`; forbid it so that stays true.
#![forbid(unsafe_code)]

pub mod amountofmoney;
pub mod creditcard;
pub mod distance;
pub mod document;
pub mod duration;
pub mod email;
pub mod engine;
pub mod grain;
pub mod json;
pub mod numeral;
pub mod ordinal;
pub mod phonenumber;
pub mod quantity;
pub mod ranking;
pub mod regex;
pub mod resolve;
pub mod temperature;
pub mod time;
pub mod timegrain;
pub mod types;
pub mod url;
pub mod volume;

pub use resolve::{Entity, ResolveContext, to_grain_precision, value_at_grain};

use document::Document;
use types::{Node, Rule, Token};

pub use types::Locale;

fn build_rules(locale: Locale) -> Vec<Rule> {
    let mut r = numeral::en::numeral_rules();
    r.extend(ordinal::en::ordinal_rules());
    r.extend(timegrain::en::timegrain_rules());
    r.extend(duration::en::duration_rules());
    r.extend(time::en::en_rules(locale));
    r
}

thread_local! {
    // Compile the rule set (regexes) once per (thread, locale), not once per parse.
    // All dimensions share one rule set; the engine produces Numeral/Time/... tokens
    // and Time rules consume the others via predicate pattern items. Locale variants
    // differ only in numeric-date field order (and, later, regional holidays); each
    // is compiled lazily on first use and cached.
    static RULES: std::cell::RefCell<std::collections::HashMap<Locale, std::rc::Rc<Vec<Rule>>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
    static CLASSIFIERS: ranking::Classifiers = ranking::classifiers();
    // Rule sets for the standalone regex dimensions (email/url/…), compiled once
    // per thread and keyed by dimension name. Kept out of the Time rule set so
    // they never perturb the Time ranker.
    static DIM_RULES: std::cell::RefCell<std::collections::HashMap<&'static str, std::rc::Rc<Vec<Rule>>>> =
        std::cell::RefCell::new(std::collections::HashMap::new());
}

fn rules_for(locale: Locale) -> std::rc::Rc<Vec<Rule>> {
    RULES.with(|c| {
        c.borrow_mut()
            .entry(locale)
            .or_insert_with(|| std::rc::Rc::new(build_rules(locale)))
            .clone()
    })
}

fn dim_rules(name: &'static str, build: fn() -> Vec<Rule>) -> std::rc::Rc<Vec<Rule>> {
    DIM_RULES.with(|c| {
        c.borrow_mut()
            .entry(name)
            .or_insert_with(|| std::rc::Rc::new(build()))
            .clone()
    })
}

/// Run `rules` over `input`, resolve each produced token to a `(dim, value)` via
/// `extract` (returning None for tokens of other dimensions), then rank by range
/// domination and drop identical (range, value) duplicates. The shared emitter
/// for the standalone regex dimensions.
fn emit_entities(
    rules: &[Rule],
    input: &str,
    extract: impl Fn(&Token) -> Option<(&'static str, serde_json::Value)>,
) -> Vec<Entity> {
    let doc = Document::new(input);
    let nodes = engine::parse_string(rules, &doc);
    let scored: Vec<(Node, Entity)> = nodes
        .into_iter()
        .filter_map(|n| {
            let (dim, value) = extract(&n.token)?;
            let e = Entity {
                dim: dim.to_string(),
                body: doc.substring(n.range.0, n.range.1),
                start: n.range.0,
                end: n.range.1,
                value,
                latent: false,
            };
            Some((n, e))
        })
        .collect();
    let ranked = CLASSIFIERS.with(|cl| ranking::rank(cl, scored));
    let mut seen = std::collections::HashSet::new();
    ranked
        .into_iter()
        .filter(|e| seen.insert((e.start, e.end, e.value.to_string())))
        .collect()
}

fn resolve_entities(rules: &[Rule], doc: &Document, ctx: &ResolveContext) -> Vec<Entity> {
    let nodes = engine::parse_string(rules, doc);
    let scored: Vec<(Node, Entity)> = nodes
        .into_iter()
        .filter_map(|n| {
            let td = match &n.token {
                Token::Time(td) => td.clone(),
                _ => return None,
            };
            let value = resolve::resolve_time(&td, ctx)?;
            let e = Entity {
                dim: "time".to_string(),
                body: doc.substring(n.range.0, n.range.1),
                start: n.range.0,
                end: n.range.1,
                value,
                latent: td.latent,
            };
            Some((n, e))
        })
        .collect();
    CLASSIFIERS.with(|cl| ranking::rank(cl, scored))
}

/// Parse `input` against the EN (US) Time rules and return resolved entities,
/// ranked (competing parses collapsed to the winner).
pub fn parse(input: &str, ctx: &ResolveContext) -> Vec<Entity> {
    parse_locale(input, ctx, Locale::EnUs)
}

/// Parse in a specific English locale. The only behavioral difference is numeric
/// date field order — US "3/4"→March 4, GB "3/4"→April 3 (and GB accepts "13/12").
pub fn parse_locale(input: &str, ctx: &ResolveContext, locale: Locale) -> Vec<Entity> {
    let doc = Document::new(input);
    resolve_entities(&rules_for(locale), &doc, ctx)
}

/// Parse `input` for **both** Time and Duration and return the ranked entities —
/// the `dims:["time","duration"]` surface. Time and Duration compete in one pool
/// by range domination (dimension-agnostic, exactly as Duckling): the widest
/// match at each position wins, disjoint matches all surface. So "in 2 hours" →
/// one Time entity (the contained "2 hours" Duration is dominated), while
/// "set a timer for 20 minutes and wake me at 7am" → a Duration and a Time
/// (disjoint). `parse` (Time-only) is unchanged, so the Time corpus is untouched.
pub fn parse_time_and_duration(input: &str, ctx: &ResolveContext) -> Vec<Entity> {
    let doc = Document::new(input);
    let rules = rules_for(Locale::EnUs);
    let nodes = engine::parse_string(&rules, &doc);
    let scored: Vec<(Node, Entity)> = nodes
        .into_iter()
        .filter_map(|n| {
            let e = match &n.token {
                Token::Time(td) => Entity {
                    dim: "time".to_string(),
                    body: doc.substring(n.range.0, n.range.1),
                    start: n.range.0,
                    end: n.range.1,
                    value: resolve::resolve_time(td, ctx)?,
                    latent: td.latent,
                },
                Token::Duration(dd) => Entity {
                    dim: "duration".to_string(),
                    body: doc.substring(n.range.0, n.range.1),
                    start: n.range.0,
                    end: n.range.1,
                    value: resolve::duration_value(dd),
                    latent: false,
                },
                _ => return None,
            };
            Some((n, e))
        })
        .collect();
    let ranked = CLASSIFIERS.with(|cl| ranking::rank(cl, scored));
    let mut seen = std::collections::HashSet::new();
    ranked
        .into_iter()
        .filter(|e| seen.insert((e.start, e.end, e.dim.clone(), e.value.to_string())))
        .collect()
}

/// Parse `input` across **every** dimension and return the surviving entities —
/// the unrestricted "all dimensions" surface for extracting all structured data
/// from an utterance (e.g. "pay $20 for 2 lbs of coffee at 3pm" → an
/// amount-of-money, a quantity, and a time). Time/Duration are ranked together
/// by the classifier (via [`parse_time_and_duration`]); every other dimension is
/// resolved in its own isolated rule set. All results are then merged by
/// **cross-dimension range domination**: an entity whose span is strictly
/// contained in another's is dropped (so the bare numeral inside "$20" or the
/// "3" inside "3pm" disappears), while disjoint entities all survive. Genuinely
/// ambiguous equal spans across dimensions (e.g. "10 c" as cents vs. Celsius) are
/// both surfaced — the caller picks. `parse` (Time-only) is untouched.
pub fn parse_all(input: &str, ctx: &ResolveContext) -> Vec<Entity> {
    let mut all = parse_time_and_duration(input, ctx);
    all.extend(parse_numeral(input));
    all.extend(parse_ordinal(input));
    all.extend(parse_temperature(input));
    all.extend(parse_volume(input));
    all.extend(parse_distance(input));
    all.extend(parse_quantity(input));
    all.extend(parse_amountofmoney(input));
    all.extend(parse_email(input));
    all.extend(parse_url(input));
    all.extend(parse_creditcard(input));
    all.extend(parse_phonenumber(input));

    // Cross-dimension range domination: drop any entity whose span is strictly
    // contained within another entity's span. Equal / partially-overlapping
    // spans are both kept.
    let spans: Vec<(usize, usize)> = all.iter().map(|e| (e.start, e.end)).collect();
    let mut kept: Vec<Entity> = all
        .iter()
        .enumerate()
        .filter(|(i, _)| {
            let (si, ei) = spans[*i];
            !spans
                .iter()
                .enumerate()
                .any(|(j, &(sj, ej))| *i != j && sj <= si && ei <= ej && (ej - sj) > (ei - si))
        })
        .map(|(_, e)| e.clone())
        .collect();

    // Drop exact duplicates, then order by position for a stable, readable result.
    let mut seen = std::collections::HashSet::new();
    kept.retain(|e| seen.insert((e.start, e.end, e.dim.clone(), e.value.to_string())));
    kept.sort_by_key(|e| (e.start, e.end));
    kept
}

/// Parse `input` and return resolved **Duration** entities (dim `"duration"`),
/// ranked (range-dominated). Durations are context-free — "half an hour",
/// "2 years and 3 months", "an hour and 45 minutes" — so no `ResolveContext` is
/// needed. This is the `dims:["duration"]` surface, separate from `parse`
/// (Time), so it never perturbs the Time ranker.
pub fn parse_duration(input: &str) -> Vec<Entity> {
    let doc = Document::new(input);
    let rules = rules_for(Locale::EnUs);
    let nodes = engine::parse_string(&rules, &doc);
    let scored: Vec<(Node, Entity)> = nodes
        .into_iter()
        .filter_map(|n| {
            let dd = match &n.token {
                Token::Duration(dd) => dd.clone(),
                _ => return None,
            };
            let e = Entity {
                dim: "duration".to_string(),
                body: doc.substring(n.range.0, n.range.1),
                start: n.range.0,
                end: n.range.1,
                value: resolve::duration_value(&dd),
                latent: false,
            };
            Some((n, e))
        })
        .collect();
    let ranked = CLASSIFIERS.with(|cl| ranking::rank(cl, scored));
    // Several composite rules can produce the same span+value (e.g. "2 years and
    // 3 months" via both the ",/and" and the <duration>-and-<duration> rules);
    // collapse identical (range, value) entities.
    let mut seen = std::collections::HashSet::new();
    ranked
        .into_iter()
        .filter(|e| seen.insert((e.start, e.end, e.value.to_string())))
        .collect()
}

/// Parse `input` and return resolved **Ordinal** entities (dim `"ordinal"`,
/// `{type:"value", value:<int>}`), ranked by range domination — the
/// `dims:["ordinal"]` surface. Context-free. The ordinal rules (first..ninetieth,
/// composites like "twenty-fifth", digit forms) are the ones the Time path uses;
/// this just exposes them as standalone entities.
pub fn parse_ordinal(input: &str) -> Vec<Entity> {
    let doc = Document::new(input);
    let rules = rules_for(Locale::EnUs);
    let nodes = engine::parse_string(&rules, &doc);
    let scored: Vec<(Node, Entity)> = nodes
        .into_iter()
        .filter_map(|n| {
            let od = match &n.token {
                Token::Ordinal(od) => od.clone(),
                _ => return None,
            };
            let e = Entity {
                dim: "ordinal".to_string(),
                body: doc.substring(n.range.0, n.range.1),
                start: n.range.0,
                end: n.range.1,
                value: resolve::ordinal_value(&od),
                latent: false,
            };
            Some((n, e))
        })
        .collect();
    let ranked = CLASSIFIERS.with(|cl| ranking::rank(cl, scored));
    let mut seen = std::collections::HashSet::new();
    ranked
        .into_iter()
        .filter(|e| seen.insert((e.start, e.end, e.value.to_string())))
        .collect()
}

/// Parse `input` and return resolved **Numeral** entities (dim `"number"`,
/// `{type:"value", value:<number>}`), ranked by range domination — the
/// `dims:["number"]` surface. Context-free. Covers the forms the Time/Duration
/// path needs (integers, written numbers, informal quantifiers, decimals,
/// composition); magnitude suffixes (K/M/G/lakh) and some fractions are not yet
/// ported — see docs/REMAINING_DIMENSIONS.md.
pub fn parse_numeral(input: &str) -> Vec<Entity> {
    let doc = Document::new(input);
    let rules = rules_for(Locale::EnUs);
    let nodes = engine::parse_string(&rules, &doc);
    let scored: Vec<(Node, Entity)> = nodes
        .into_iter()
        .filter_map(|n| {
            let nd = match &n.token {
                Token::Numeral(nd) => nd.clone(),
                _ => return None,
            };
            let e = Entity {
                dim: "number".to_string(),
                body: doc.substring(n.range.0, n.range.1),
                start: n.range.0,
                end: n.range.1,
                value: resolve::numeral_value(&nd),
                latent: false,
            };
            Some((n, e))
        })
        .collect();
    let ranked = CLASSIFIERS.with(|cl| ranking::rank(cl, scored));
    let mut seen = std::collections::HashSet::new();
    ranked
        .into_iter()
        .filter(|e| seen.insert((e.start, e.end, e.value.to_string())))
        .collect()
}

/// Parse `input` and return resolved **Email** entities (dim `"email"`,
/// `{type:"value", value:"a@b.com"}`) — the `dims:["email"]` surface. Handles
/// both literal (`a@b.com`) and spelled-out (`a at b dot com`) forms.
pub fn parse_email(input: &str) -> Vec<Entity> {
    let rules = dim_rules("email", email::en::email_rules);
    emit_entities(&rules, input, |t| match t {
        Token::Email(e) => Some(("email", resolve::email_value(e))),
        _ => None,
    })
}

/// Parse `input` and return resolved **Url** entities (dim `"url"`,
/// `{value, domain, type:"value"}`) — the `dims:["url"]` surface. Language-agnostic.
pub fn parse_url(input: &str) -> Vec<Entity> {
    let rules = dim_rules("url", url::url_rules);
    emit_entities(&rules, input, |t| match t {
        Token::Url(u) => Some(("url", resolve::url_value(u))),
        _ => None,
    })
}

/// Parse `input` and return resolved **CreditCardNumber** entities (dim
/// `"credit-card-number"`, `{value, issuer}`) — the `dims:["credit-card-number"]`
/// surface. Language-agnostic; validated with the Luhn checksum.
pub fn parse_creditcard(input: &str) -> Vec<Entity> {
    let rules = dim_rules("credit-card-number", creditcard::creditcard_rules);
    emit_entities(&rules, input, |t| match t {
        Token::CreditCard(c) => Some(("credit-card-number", resolve::creditcard_value(c))),
        _ => None,
    })
}

/// Parse `input` and return resolved **PhoneNumber** entities (dim
/// `"phone-number"`, `{value, type:"value"}`) — the `dims:["phone-number"]`
/// surface. Language-agnostic; value is the normalized number string.
pub fn parse_phonenumber(input: &str) -> Vec<Entity> {
    let rules = dim_rules("phone-number", phonenumber::phonenumber_rules);
    emit_entities(&rules, input, |t| match t {
        Token::Phone(p) => Some(("phone-number", resolve::phonenumber_value(p))),
        _ => None,
    })
}

/// Parse `input` and return resolved **Temperature** entities (dim
/// `"temperature"`). Runs its own rule set (numerals + temperature rules), so it
/// never touches the Time ranker.
pub fn parse_temperature(input: &str) -> Vec<Entity> {
    let rules = dim_rules("temperature", || {
        let mut r = numeral::en::numeral_rules();
        r.extend(temperature::en::temperature_rules());
        r
    });
    emit_entities(&rules, input, |t| match t {
        Token::Temperature(td) => resolve::temperature_value(td).map(|v| ("temperature", v)),
        _ => None,
    })
}

/// Parse volumes ("2 liters", "between 100 and 1000 l", "at least 4 ml"). Runs
/// in Volume's own rule set (numerals + volume rules), so it never touches the
/// Time ranker.
pub fn parse_volume(input: &str) -> Vec<Entity> {
    let rules = dim_rules("volume", || {
        let mut r = numeral::en::numeral_rules();
        r.extend(volume::en::volume_rules());
        r
    });
    emit_entities(&rules, input, |t| match t {
        Token::Volume(vd) => resolve::volume_value(vd).map(|v| ("volume", v)),
        _ => None,
    })
}

/// Parse distances ("3 km", "7 feet 10 inches", "between 3 and 5 km", "over 5\"").
/// Runs in Distance's own rule set (numerals + distance rules), so it never
/// touches the Time ranker.
pub fn parse_distance(input: &str) -> Vec<Entity> {
    let rules = dim_rules("distance", || {
        let mut r = numeral::en::numeral_rules();
        r.extend(distance::en::distance_rules());
        r
    });
    emit_entities(&rules, input, |t| match t {
        Token::Distance(dd) => resolve::distance_value(dd).map(|v| ("distance", v)),
        _ => None,
    })
}

/// Parse quantities ("2 cups of sugar", "500g", "between 100 and 1000 grams").
/// `with_latent` controls whether a bare number resolves as an `unnamed`
/// quantity (Duckling's `withLatent` option). Runs in Quantity's own rule set,
/// so it never touches the Time ranker.
pub fn parse_quantity_opts(input: &str, with_latent: bool) -> Vec<Entity> {
    let rules = dim_rules("quantity", || {
        let mut r = numeral::en::numeral_rules();
        r.extend(quantity::en::quantity_rules());
        r
    });
    emit_entities(&rules, input, move |t| match t {
        Token::Quantity(qd) => resolve::quantity_value(qd, with_latent).map(|v| ("quantity", v)),
        _ => None,
    })
}

/// Parse quantities, dropping latent bare-number quantities (Duckling default).
pub fn parse_quantity(input: &str) -> Vec<Entity> {
    parse_quantity_opts(input, false)
}

/// Parse amounts of money ("$10", "20 euros", "$20 and 43c", "between 10 and 20
/// dollars", "over $1.42"). `with_latent` controls whether a bare number resolves
/// as an `unknown`-currency amount. Runs in AmountOfMoney's own rule set, so it
/// never touches the Time ranker.
pub fn parse_amountofmoney_opts(input: &str, with_latent: bool) -> Vec<Entity> {
    let rules = dim_rules("amountofmoney", || {
        let mut r = numeral::en::numeral_rules();
        r.extend(amountofmoney::en::rules());
        r
    });
    emit_entities(&rules, input, move |t| match t {
        Token::AmountOfMoney(a) => {
            resolve::amountofmoney_value(a, with_latent).map(|v| ("amount-of-money", v))
        }
        _ => None,
    })
}

/// Parse amounts of money, dropping latent bare-number amounts (Duckling default).
pub fn parse_amountofmoney(input: &str) -> Vec<Entity> {
    parse_amountofmoney_opts(input, false)
}

/// Debug: every Time candidate (unranked) as "rule | range | score | value".
pub fn parse_all_debug(input: &str, ctx: &ResolveContext) -> Vec<String> {
    let doc = Document::new(input);
    let rules = rules_for(Locale::EnUs);
    {
        let nodes = engine::parse_string(&rules, &doc);
        CLASSIFIERS.with(|cl| {
            let mut out = Vec::new();
            for n in &nodes {
                let td = match &n.token {
                    Token::Time(td) => td.clone(),
                    _ => continue,
                };
                let value = match resolve::resolve_time(&td, ctx) {
                    Some(v) => v,
                    None => continue,
                };
                let sc = ranking::score(cl, n);
                out.push(format!(
                    "{:<44} [{:>2},{:>2}] score={:>10.4}  {}",
                    n.rule.clone().unwrap_or_default(),
                    n.range.0,
                    n.range.1,
                    sc,
                    serde_json::to_string(&value).unwrap_or_default()
                ));
            }
            out
        })
    }
}

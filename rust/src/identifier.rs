//! Identifier dimension — not in upstream Duckling.
//!
//! Upstream has no notion of an identifier: every digit-bearing token is read
//! as a quantity, a time, or one of the regex contact dims. So a version
//! `4.19.3` comes out as a phone number, `127.0.0.1` as phone `127001`,
//! `COVID-19` as the number `-19`, `INV-2024-00417` as number `-2024` plus
//! phone `2024-00417`, and "from 1.2.3 to 1.4.0" as the time "3 to 1".
//!
//! This pass finds identifier-shaped tokens first and gives each a
//! *provisional* kind (`ipv4`, `version`, `uuid`, `code`, …). It is a shape
//! classifier, not a validator of record: the caller (e.g. an NER model) is
//! expected to confirm or override the kind from context. Its main job in
//! [`crate::parse_all`] is to claim the whole token so the other dims cannot
//! scramble it — see [`crate::parse_all`] for how the claim is enforced.
//!
//! A token is a maximal run of ASCII alphanumerics joined by single `-`, `.`,
//! `_` or `:` (or `::`, for IPv6), optionally led by `#`. It must contain a
//! digit. Plain quantities stay quantities: `5km`, `3pm`, `2.5k`, `21-mile`,
//! `1,234.5`, `10:30` and bare integers are not identifiers.

use fancy_regex::Regex;
use std::sync::LazyLock;

/// One identifier-shaped token, char-indexed like every other entity.
#[derive(Clone, Debug, PartialEq)]
pub struct IdentifierHit {
    pub start: usize,
    pub end: usize,
    pub body: String,
    pub kind: &'static str,
}

fn re(p: &str) -> Regex {
    Regex::new(p).expect("valid identifier regex")
}

static TOKEN: LazyLock<Regex> =
    LazyLock::new(|| re(r"#?[A-Za-z0-9]+(?:(?:::?|[-._])[A-Za-z0-9]+)*"));

static UUID: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$"));
static MAC: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^[0-9a-f]{2}([:-])[0-9a-f]{2}(?:\1[0-9a-f]{2}){4}$"));
static IPV4: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(\d{1,3})\.(\d{1,3})\.(\d{1,3})\.(\d{1,3})(?::(\d{1,5}))?$"));
static IPV6: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^[0-9a-f]{0,4}(?::{1,2}[0-9a-f]{0,4}){2,7}$"));
// SemVer 2.0 (no leading zeros in the core), plus a "v" prefix.
static SEMVER: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)^v?(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)\.(?:0|[1-9]\d*)(?:-[0-9a-z]+(?:[.-][0-9a-z]+)*)?$")
});
static V_VERSION: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^v\d+(?:\.\d+)*$"));
static DOTTED: LazyLock<Regex> = LazyLock::new(|| re(r"^\d+(?:\.\d+){2,}$"));
// A US phone written with dots, the one dotted shape that is genuinely a phone.
static DOTTED_PHONE: LazyLock<Regex> = LazyLock::new(|| re(r"^\d{3}\.\d{3}\.\d{4}$"));
static HEX_LITERAL: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^0x[0-9a-f]+$"));
// Hashes / commit ids: 7+ hex chars mixing letters and digits.
static HEX_RUN: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^(?=.*[a-f])(?=.*\d)[0-9a-f]{7,}$"));
// Badges and ZIPs ("00731", "02134"). 7+ digits with a leading zero is phone
// territory ("06354640807"); long account numbers need a cue word instead.
static LEADING_ZERO: LazyLock<Regex> = LazyLock::new(|| re(r"^0\d{4,5}$"));
// A quantity with a unit/suffix glued on: 5km, 3pm, 1st, 2.5k, 24hrs, 4B, 3C.
static NUM_SUFFIX: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^\d[\d.,]*[a-z]+$"));
// A bare number, possibly decimal: one segment of a hyphen/colon compound.
static NUM: LazyLock<Regex> = LazyLock::new(|| re(r"^\d+(?:\.\d+)?$"));
// 3x5, 4x4, 1920x1080: a multiplication or dimension, not a code.
static TIMES: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^\d+(?:\.\d+)?x\d+(?:\.\d+)?$"));
// A phone extension: "x57478", "x-6249". The phone's, not a record's.
static EXTENSION: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^x-?\d{3,6}$"));
// A Moody's-style credit grade: "Baa1", "Aa3". A grade, not a record.
static GRADE: LazyLock<Regex> =
    LazyLock::new(|| re(r"^(?:Aaa|Aa[1-3]|Baa[1-3]|Ba[1-3]|Caa[1-3])$"));

/// Words that open a compound with a number without making it a code:
/// "mid-2024", "pre-1990", "top-10", "under-18".
const PREFIX_WORDS: &[&str] = &[
    "mid", "pre", "post", "early", "late", "end", "start", "circa", "ca", "top", "under", "over",
    "about", "around", "approx", "up", "to", "from", "since", "until", "by", "per", "and", "or",
    "the", "a", "an", "non", "anti", "sub", "ex", "gmt", "utc",
];

/// Month names: "07-Feb" is a date, not a code.
const MONTHS: &[&str] = &[
    "jan",
    "feb",
    "mar",
    "apr",
    "may",
    "jun",
    "jul",
    "aug",
    "sep",
    "sept",
    "oct",
    "nov",
    "dec",
    "january",
    "february",
    "march",
    "april",
    "june",
    "july",
    "august",
    "september",
    "october",
    "november",
    "december",
];

/// Words after which a dotted number is a version, not a quantity.
const VERSION_CUES: &[&str] = &["version", "ver", "release", "build", "firmware", "patch"];
static DOTTED_VERSION: LazyLock<Regex> = LazyLock::new(|| re(r"^\d+(?:\.\d+)+$"));

/// Words that announce an identifier: the next number-ish token is a
/// reference, not a quantity ("order 112-4471983-2236150", "SSN 078-05-1120").
const CUE_WORDS: &[&str] = &[
    "order",
    "invoice",
    "inv",
    "account",
    "acct",
    "ticket",
    "case",
    "ref",
    "reference",
    "tracking",
    "confirmation",
    "conf",
    "booking",
    "reservation",
    "policy",
    "claim",
    "member",
    "membership",
    "customer",
    "client",
    "transaction",
    "txn",
    "po",
    "sku",
    "upc",
    "ean",
    "serial",
    "sn",
    "part",
    "model",
    "lot",
    "batch",
    "build",
    "version",
    "ver",
    "release",
    "commit",
    "sha",
    "hash",
    "id",
    "mrn",
    "ssn",
    "ein",
    "tin",
    "vin",
    "isbn",
    "issn",
    "iban",
    "routing",
    "aba",
    "swift",
    "badge",
    "employee",
    "patient",
    "item",
    "product",
    "license",
    "licence",
    "permit",
    "docket",
    "file",
    "record",
    "receipt",
    "voucher",
    "coupon",
    "promo",
    "zip",
    "postcode",
    "postal",
    "deal",
    "contract",
    "notice",
    "issue",
    "bug",
    "incident",
    "request",
    "req",
    "job",
    "shipment",
    "container",
    "awb",
    "waybill",
    "passport",
    "registration",
    "reg",
];

/// Filler allowed between a cue word and its identifier: "order no. 1234",
/// "Order Number: 1234", "account #: 0012", "MRN is 4471-2290".
static CUE_TAIL: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?<![a-z])([a-z]+)(?:\s*(?:no\.?|num(?:ber)?\.?|#|id))?(?:\s+(?:is|was))?\s*[:#=]?\s*$",
    )
});

fn has_digit(s: &str) -> bool {
    s.bytes().any(|b| b.is_ascii_digit())
}

fn has_letter(s: &str) -> bool {
    s.bytes().any(|b| b.is_ascii_alphabetic())
}

fn m(r: &Regex, s: &str) -> bool {
    r.is_match(s).unwrap_or(false)
}

fn valid_ipv4(t: &str) -> bool {
    let Ok(Some(c)) = IPV4.captures(t) else {
        return false;
    };
    (1..=4).all(|i| {
        let o = &c[i];
        (o == "0" || !o.starts_with('0')) && o.parse::<u16>().is_ok_and(|v| v <= 255)
    })
}

fn valid_ipv6(t: &str) -> bool {
    if !m(&IPV6, t) {
        return false;
    }
    let double = t.matches("::").count();
    let groups = t.split(':').filter(|g| !g.is_empty()).count();
    // "::" compresses zero groups (once); without it all 8 must be spelled out.
    // Requiring a hex letter or "::" keeps clock times ("10:30:45") out.
    (double == 1 && groups <= 7 || double == 0 && groups == 8)
        && (double == 1 || t.bytes().any(|b| b.is_ascii_alphabetic()))
}

/// The segments of a compound, split on its joiners.
fn segments(t: &str) -> Vec<&str> {
    t.split(['-', '.', '_']).filter(|s| !s.is_empty()).collect()
}

/// A digit-led compound whose remaining segments are numbers, numbers with a
/// unit suffix, or lowercase words: "21-mile", "37-year-old", "3-6pm",
/// "9am-5pm", "10:30", "1958-1962". These read as quantities or ranges.
fn quantity_compound(t: &str) -> bool {
    let segs = segments(t);
    let numberish = |s: &str| m(&NUM, s) || m(&NUM_SUFFIX, s);
    let first = segs.first().copied().unwrap_or("");
    numberish(first)
        && segs[1..]
            .iter()
            .all(|s| numberish(s) || s.bytes().all(|b| b.is_ascii_lowercase()))
}

/// Letters-first compound ("INV-2024", "COVID-19", "I-95") unless the letter
/// segment is an ordinary prefix word ("mid-2024", "top-10").
fn code_compound(t: &str) -> bool {
    let segs = segments(t);
    let Some(first) = segs.first() else {
        return false;
    };
    let led_by_letters = first
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic());
    if led_by_letters {
        return !PREFIX_WORDS.contains(&first.to_ascii_lowercase().as_str());
    }
    // Digit-led with an uppercase segment: "7700-112-B", "2024-CV-00891", "10-K".
    segs[1..].iter().any(|s| {
        s.bytes().any(|b| b.is_ascii_uppercase())
            && !MONTHS.contains(&s.to_ascii_lowercase().as_str())
    })
}

/// The provisional kind of a bare token (no cue word), or None when it reads
/// as a quantity, a time, or plain prose.
fn shape_kind(t: &str) -> Option<&'static str> {
    if !has_digit(t) {
        return None;
    }
    if m(&UUID, t) {
        return Some("uuid");
    }
    if m(&MAC, t) {
        return Some("mac-address");
    }
    if valid_ipv4(t) {
        return Some("ipv4");
    }
    if t.contains(':') && valid_ipv6(t) {
        return Some("ipv6");
    }
    if m(&DOTTED_PHONE, t) || m(&EXTENSION, t) || m(&GRADE, t) {
        return None;
    }
    if m(&SEMVER, t) || m(&V_VERSION, t) {
        return Some("version");
    }
    if m(&DOTTED, t) {
        return Some("dotted-number");
    }
    if m(&HEX_LITERAL, t) || m(&HEX_RUN, t) {
        return Some("hex");
    }
    if m(&LEADING_ZERO, t) {
        return Some("code");
    }
    if t.contains(':') {
        return None; // the caller re-scans the ':'-separated parts
    }
    if !has_letter(t) {
        return None; // digits and joiners only: numbers, ranges, clocks, phones
    }
    if t.contains(['-', '_', '.']) {
        if quantity_compound(t) {
            return None;
        }
        return code_compound(t).then_some("code");
    }
    if m(&NUM_SUFFIX, t) || m(&TIMES, t) {
        return None;
    }
    Some("code") // B22, EB572, 1Z999AA1…, 23H2, deal227922
}

/// The word right before `start_byte`, past cue filler ("no.", "#", ":"),
/// lowercased.
fn cue_word(text: &str, start_byte: usize) -> Option<String> {
    let mut from = start_byte.saturating_sub(64);
    while !text.is_char_boundary(from) {
        from += 1;
    }
    match CUE_TAIL.captures(&text[from..start_byte]) {
        Ok(Some(c)) => Some(c[1].to_ascii_lowercase()),
        _ => None,
    }
}

/// Whether a cue word ("order", "SSN", …) sits right before `start_byte`.
fn cued(text: &str, start_byte: usize) -> bool {
    cue_word(text, start_byte).is_some_and(|w| CUE_WORDS.contains(&w.as_str()))
}

/// A dotted number after a version word is a version: "Version 2.5",
/// "release 10.1". A bare "2.5" stays a number.
fn cued_version(text: &str, start_byte: usize, t: &str) -> bool {
    m(&DOTTED_VERSION, t)
        && cue_word(text, start_byte).is_some_and(|w| VERSION_CUES.contains(&w.as_str()))
}

/// A number-ish token after a cue word is a reference when it is long or
/// grouped enough not to be a count: "order 112-4471983-2236150" yes,
/// "order 3 pizzas" no.
fn cued_reference(t: &str) -> bool {
    let digits = t.bytes().filter(u8::is_ascii_digit).count();
    !has_letter(t) && (digits >= 5 || (digits >= 4 && t.contains(['-', '.'])))
}

/// The kind of the token `t` found at `start_byte`, or None.
fn classify(text: &str, start_byte: usize, t: &str, hashed: bool) -> Option<&'static str> {
    if !hashed && cued_version(text, start_byte, t) {
        return Some("version");
    }
    shape_kind(t).or_else(|| {
        let tagged = hashed && t.len() >= 3 && has_digit(t);
        let cued = !hashed && cued(text, start_byte) && cued_reference(t);
        (tagged || cued).then_some("reference")
    })
}

/// Find identifier-shaped tokens in `text`, char-indexed.
pub fn find_identifiers(text: &str) -> Vec<IdentifierHit> {
    let char_idx = |byte: usize| text[..byte].chars().count();
    let mut spans: Vec<(usize, usize, &'static str)> = Vec::new();
    for hit in TOKEN.find_iter(text).flatten() {
        let (sb, eb) = (hit.start(), hit.end());
        let raw = hit.as_str();
        // A '#' glued to a word ("C#7", "k#H02262") is not a reference
        // marker: the token starts after it.
        let glued = raw.starts_with('#')
            && text[..sb]
                .chars()
                .next_back()
                .is_some_and(char::is_alphanumeric);
        let (sb, raw) = if glued {
            (sb + 1, &raw[1..])
        } else {
            (sb, raw)
        };
        let hashed = raw.starts_with('#');
        // Either side of an '@' is part of an email address or a handle.
        if text[..sb].ends_with('@') || text[eb..].starts_with('@') {
            continue;
        }
        let t = if hashed { &raw[1..] } else { raw };
        if let Some(kind) = classify(text, sb, t, hashed) {
            spans.push((sb, eb, kind));
        } else if t.contains(':') {
            // ':' only joins MAC/IPv6/ip:port; otherwise each part stands
            // alone ("ref:INV-2024-00417", "hey:42").
            let base = sb + usize::from(hashed);
            let mut off = 0;
            for part in t.split(':') {
                let ps = base + off;
                off += part.len() + 1;
                if part.is_empty() {
                    continue;
                }
                if let Some(kind) = classify(text, ps, part, false) {
                    spans.push((ps, ps + part.len(), kind));
                }
            }
        }
    }
    spans
        .into_iter()
        .map(|(sb, eb, kind)| IdentifierHit {
            start: char_idx(sb),
            end: char_idx(eb),
            body: text[sb..eb].to_string(),
            kind,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<(String, &'static str)> {
        find_identifiers(text)
            .into_iter()
            .map(|h| (h.body, h.kind))
            .collect()
    }

    fn one(text: &str) -> Option<(String, &'static str)> {
        let mut k = kinds(text);
        assert!(k.len() <= 1, "{text:?} -> {k:?}");
        k.pop()
    }

    fn id(body: &str, kind: &'static str) -> Option<(String, &'static str)> {
        Some((body.to_string(), kind))
    }

    #[test]
    fn versions() {
        assert_eq!(one("version 4.19.3 ships"), id("4.19.3", "version"));
        assert_eq!(one("upgrade to 1.11.0"), id("1.11.0", "version"));
        assert_eq!(one("1.0.0-rc.1 is out"), id("1.0.0-rc.1", "version"));
        assert_eq!(one("v2 api"), id("v2", "version"));
        assert_eq!(one("on v1.2"), id("v1.2", "version"));
        assert_eq!(one("Version 2.5 ships Friday"), id("2.5", "version"));
        assert_eq!(one("release 10.1"), id("10.1", "version"));
        assert_eq!(one("it costs 2.5 dollars"), None);
    }

    #[test]
    fn ip_addresses() {
        assert_eq!(one("server on 127.0.0.1"), id("127.0.0.1", "ipv4"));
        assert_eq!(one("box is 10.0.0.12."), id("10.0.0.12", "ipv4"));
        assert_eq!(one("bind 0.0.0.0:8080"), id("0.0.0.0:8080", "ipv4"));
        assert_eq!(one("ping 2001:db8::1"), id("2001:db8::1", "ipv6"));
        // out-of-range octet: still dotted, but not an IP
        assert_eq!(one("see 300.1.2.3"), id("300.1.2.3", "dotted-number"));
    }

    #[test]
    fn dotted_numbers_that_are_not_versions() {
        assert_eq!(one("on 01.29.01 we met"), id("01.29.01", "dotted-number"));
        assert_eq!(one("total 1.234.567"), id("1.234.567", "version"));
        // a dotted US phone stays a phone
        assert_eq!(one("call 202.555.0121"), None);
    }

    #[test]
    fn structured_ids() {
        assert_eq!(
            one("uuid 550e8400-e29b-41d4-a716-446655440000"),
            id("550e8400-e29b-41d4-a716-446655440000", "uuid")
        );
        assert_eq!(
            one("MAC 00:1A:2B:3C:4D:5E"),
            id("00:1A:2B:3C:4D:5E", "mac-address")
        );
        assert_eq!(one("commit 4438b3d fixed it"), id("4438b3d", "hex"));
        assert_eq!(one("error 0x80070005"), id("0x80070005", "hex"));
    }

    #[test]
    fn codes() {
        assert_eq!(
            one("invoice INV-2024-00417 is due"),
            id("INV-2024-00417", "code")
        );
        assert_eq!(one("ticket JIRA-4821"), id("JIRA-4821", "code"));
        assert_eq!(one("COVID-19 cases"), id("COVID-19", "code"));
        assert_eq!(one("I-95 north"), id("I-95", "code"));
        assert_eq!(one("gate B22"), id("B22", "code"));
        assert_eq!(one("order #A1234-5678"), id("#A1234-5678", "code"));
        assert_eq!(one("part no. 7700-112-B"), id("7700-112-B", "code"));
        assert_eq!(one("case 2024-CV-00891"), id("2024-CV-00891", "code"));
        assert_eq!(
            one("VIN 1HGCM82633A004352"),
            id("1HGCM82633A004352", "code")
        );
        assert_eq!(one("Windows 11 23H2"), id("23H2", "code"));
        assert_eq!(one("badge 00731"), id("00731", "code"));
        assert_eq!(one("ref:INV-2024-00417"), id("INV-2024-00417", "code"));
        assert_eq!(one("k#H02262"), id("H02262", "code"));
        assert_eq!(one("k# A02353"), id("A02353", "code"));
        assert_eq!(one("deal227922 and"), id("deal227922", "code"));
        assert_eq!(one("deal 227926 settles"), id("227926", "reference"));
    }

    #[test]
    fn cued_references() {
        assert_eq!(
            one("order 112-4471983-2236150 shipped"),
            id("112-4471983-2236150", "reference")
        );
        assert_eq!(one("SSN 078-05-1120"), id("078-05-1120", "reference"));
        assert_eq!(one("Order Number: 99812345"), id("99812345", "reference"));
        assert_eq!(one("account no. 12345678"), id("12345678", "reference"));
        assert_eq!(
            one("the patient's MRN is 4471-2290"),
            id("4471-2290", "reference")
        );
        assert_eq!(one("see PR #1342"), id("#1342", "reference"));
        // a count after a cue word is still a count
        assert_eq!(one("order 3 pizzas"), None);
        assert_eq!(one("order 2000 units"), None);
    }

    #[test]
    fn quantities_and_times_are_not_identifiers() {
        for t in [
            "5km away",
            "at 3pm",
            "the 1st",
            "2.5k views",
            "24hrs",
            "3C outside",
            "4B people",
            "a 21-mile run",
            "37-year-old",
            "3-6pm",
            "9am-5pm",
            "10:30 meeting",
            "1958-1962",
            "1,234,567.89",
            "2.5",
            "port 8080",
            "flight 1549",
            "mid-2024",
            "top-10",
            "3x5 cards",
            "call 650-701-8887",
            "we're #1",
            "C#7 chord",
            "2024-01-15",
            "1/2",
            "john2@x2.com",
            "@user42",
            "hey:42",
            "at 650.650.6500",
            "0033",
        ] {
            assert_eq!(kinds(t), vec![], "{t:?}");
        }
    }
}

//! Identifiers claim their whole token in `parse_all`.
//!
//! Upstream Duckling has no identifier concept, so every digit-bearing token is
//! read as a quantity, a time, or a contact dim. Versions and IPv4 addresses
//! came back as phone numbers ("4.19.3" → phone "4193"), codes were carved into
//! negative numbers ("COVID-19" → -19), and "from 1.2.3 to 1.4.0" read as the
//! time "3 to 1". Downstream (cybernetikaai/ner#24) the phone reading of a
//! version crashed the NER post-pass and cost the text all its entities.
//!
//! The identifier dim finds those tokens first with a provisional kind, and
//! `parse_all` drops whatever the other dims made of them, unless a trusted
//! reading covers the whole token ("Q3" is a quarter, "at 0800" a time,
//! "USD3.14" money, "(+33) 1.46.64.79.98" a phone).

fn ctx() -> duckling::ResolveContext {
    let zone = jiff::tz::TimeZone::fixed(jiff::tz::Offset::constant(-4));
    let reference = jiff::civil::date(2026, 10, 15)
        .at(9, 0, 0, 0)
        .to_zoned(zone.clone())
        .unwrap()
        .timestamp();
    duckling::ResolveContext {
        reference,
        zone,
        with_latent: false,
    }
}

/// `(dim, body)` per entity, with the identifier kind appended as `dim[kind]`.
fn all(input: &str) -> Vec<(String, String)> {
    duckling::parse_all(input, &ctx())
        .into_iter()
        .map(|e| {
            let dim = match e.value["kind"].as_str() {
                Some(kind) if e.dim == "identifier" => format!("identifier[{kind}]"),
                _ => e.dim.clone(),
            };
            (dim, e.body)
        })
        .collect()
}

fn pairs(expected: &[(&str, &str)]) -> Vec<(String, String)> {
    expected
        .iter()
        .map(|(d, b)| (d.to_string(), b.to_string()))
        .collect()
}

#[track_caller]
fn assert_all(input: &str, expected: &[(&str, &str)]) {
    assert_eq!(all(input), pairs(expected), "{input:?}");
}

#[test]
fn a_version_is_not_a_phone_and_the_time_beside_it_survives() {
    assert_all(
        "version 4.19.3 ships Tuesday at 3pm",
        &[
            ("identifier[version]", "4.19.3"),
            ("time", "Tuesday at 3pm"),
        ],
    );
}

#[test]
fn version_range_is_not_a_time_range() {
    // Before: time "from 1", time "3 to 1", phone "1.2.3", numbers "1.3", ".0".
    assert_all(
        "Pricing went from 1.2.3 to 1.4.0, let's talk Monday",
        &[
            ("identifier[version]", "1.2.3"),
            ("identifier[version]", "1.4.0"),
            ("time", "Monday"),
        ],
    );
}

#[test]
fn ip_addresses() {
    assert_all("server on 127.0.0.1", &[("identifier[ipv4]", "127.0.0.1")]);
    assert_all(
        "the staging box is 192.168.1.10:8080",
        &[("identifier[ipv4]", "192.168.1.10:8080")],
    );
}

#[test]
fn codes_are_not_carved_into_numbers() {
    assert_all("COVID-19 cases", &[("identifier[code]", "COVID-19")]);
    assert_all("I-95 northbound", &[("identifier[code]", "I-95")]);
    assert_all(
        "invoice INV-2024-00417 is overdue",
        &[("identifier[code]", "INV-2024-00417")],
    );
    assert_all(
        "MAC 00:1A:2B:3C:4D:5E",
        &[("identifier[mac-address]", "00:1A:2B:3C:4D:5E")],
    );
}

#[test]
fn cue_words_make_digit_runs_references() {
    assert_all(
        "order 112-4471983-2236150 shipped",
        &[("identifier[reference]", "112-4471983-2236150")],
    );
    assert_all(
        "SSN 078-05-1120",
        &[("identifier[reference]", "078-05-1120")],
    );
    // a count after a cue word stays a count
    assert_all("order 3 pizzas", &[("number", "3")]);
}

#[test]
fn trusted_readings_that_cover_the_token_win() {
    assert_all("results for Q3 are in", &[("time", "Q3")]);
    assert_all("meet at 0800", &[("time", "at 0800")]);
    assert_all("a 1h30m meeting", &[("duration", "1h30m")]);
    assert_all("pay USD3.14", &[("amount-of-money", "USD3.14")]);
    assert_all(
        "visit www.site2.com/v1.2.3",
        &[("url", "www.site2.com/v1.2.3")],
    );
}

#[test]
fn phones_keep_their_country_code_and_dotted_us_form() {
    assert_all(
        "call (+33) 1.46.64.79.98",
        &[("phone-number", "(+33) 1.46.64.79.98")],
    );
    assert_all("call 202.555.0121", &[("phone-number", "202.555.0121")]);
    assert_all("call 650-701-8887", &[("phone-number", "650-701-8887")]);
}

#[test]
fn quantities_are_untouched() {
    assert_all("a 21-mile run", &[("distance", "21-mile")]);
    assert_all("a 5km walk", &[("distance", "5km")]);
    assert_all(
        "50% of 1,234 users",
        &[("number", "50"), ("number", "1,234")],
    );
}

#[test]
fn identifier_dim_alone() {
    let ids: Vec<(String, String)> = duckling::parse_identifier("upgrade 1.11.0 on 10.0.0.12")
        .into_iter()
        .map(|e| (e.value["kind"].as_str().unwrap().to_string(), e.body))
        .collect();
    assert_eq!(ids, pairs(&[("version", "1.11.0"), ("ipv4", "10.0.0.12")]));
}

#[test]
fn dates_offsets_extensions_and_grades_are_not_identifiers() {
    // Shapes from the customlong/custom300 gold that read as codes but
    // aren't records (ner gold/CONVENTIONS.md ruling 16).
    for t in [
        "Canada defeated Italy , 07-Feb , while",
        "1:30 PM-2:30 PM (GMT-06:00) Central Time",
        "Robin x57478",
        "Call Jim Pratico (x-6249) or",
        "contact Cathy Phillips (X-36898) and",
        "cut the rating from Baa1 to Baa2.",
    ] {
        assert!(
            !all(t).iter().any(|(d, _)| d.starts_with("identifier")),
            "{t:?} -> {:?}",
            all(t)
        );
    }
}

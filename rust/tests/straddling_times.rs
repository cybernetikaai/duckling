//! In `parse_all`, a time or duration can't begin partway through a number,
//! phone, or identifier.
//!
//! Duckling reads a bare four-digit clock ("0930 tomorrow" is 09:30 tomorrow),
//! and a match may start at any digit run that isn't split. So the last group
//! of a phone number was a clock: "reach me at 555-0121 tomorrow" came back as
//! phone "555-0121" plus the time "0121 tomorrow", 01:21 tomorrow. Ranking
//! keeps the widest time, so plain "tomorrow" was thrown away. It fired for
//! any phone whose last four digits read as 00:00–23:59, about a quarter of
//! them, and the same happened to a decimal ("1.5 tomorrow" → 05:00) and to an
//! identifier ("version 2.1130 tomorrow" lost "tomorrow").
//!
//! The straddling candidates are now dropped before ranking, so the real date
//! survives. A clock that starts at its own number ("meet 0930 tomorrow") is
//! untouched, and so is a date read across a slash-"fraction" ("13/12/2013").

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

fn all(input: &str) -> Vec<(String, String)> {
    duckling::parse_all(input, &ctx())
        .into_iter()
        .map(|e| (e.dim, e.body))
        .collect()
}

#[track_caller]
fn assert_all(input: &str, expected: &[(&str, &str)]) {
    let expected: Vec<(String, String)> = expected
        .iter()
        .map(|(d, b)| (d.to_string(), b.to_string()))
        .collect();
    assert_eq!(all(input), expected, "{input:?}");
}

#[test]
fn a_phones_last_group_is_not_a_clock() {
    assert_all(
        "you can reach me at 555-0121 tomorrow",
        &[("phone-number", "555-0121"), ("time", "tomorrow")],
    );
    assert_all(
        "my cell is 617-555-1830 Friday afternoon",
        &[
            ("phone-number", "617-555-1830"),
            ("time", "Friday afternoon"),
        ],
    );
    assert_all(
        "call (202) 555 0121 tomorrow",
        &[("phone-number", "(202) 555 0121"), ("time", "tomorrow")],
    );
    assert_all(
        "call +1 202.555.0121 tomorrow",
        &[("phone-number", "+1 202.555.0121"), ("time", "tomorrow")],
    );
}

#[test]
fn a_decimals_fraction_is_not_an_hour() {
    assert_all(
        "the 1.5 tomorrow",
        &[("number", "1.5"), ("time", "tomorrow")],
    );
}

#[test]
fn an_identifiers_tail_is_not_a_clock() {
    assert_all(
        "version 2.1130 tomorrow",
        &[("identifier", "2.1130"), ("time", "tomorrow")],
    );
}

#[test]
fn clocks_and_dates_that_start_at_their_own_number_stay() {
    assert_all("meet 0930 tomorrow", &[("time", "0930 tomorrow")]);
    assert_all("it costs 12.30 tomorrow", &[("time", "12.30 tomorrow")]);
    assert_all("set a timer for 1.5 hours", &[("duration", "1.5 hours")]);
    // en_US can't read day-first "13/12/2013"; the month/year tail it does
    // read survives, as before.
    assert_all("13/12/2013", &[("number", "13/12"), ("time", "12/2013")]);
}

#[test]
fn a_dash_before_a_number_does_not_shut_out_its_duration() {
    // "- 30" reads as the number -30, but the duration starts at its digits,
    // not partway through it (v0.4.0 kept "30 days"; customlong gold).
    assert_all(
        "Inbox - 30 days Sent Mail",
        &[("number", "- 30"), ("duration", "30 days")],
    );
}

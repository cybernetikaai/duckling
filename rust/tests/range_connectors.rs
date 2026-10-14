//! Range connectors: an en dash joins a range exactly as a hyphen does, an em
//! dash does not, and "between <hh> and <time-of-day> am|pm" shares the
//! trailing am/pm.
//!
//! Typed and pasted text carries "2–4pm" (en dash) as often as "2-4pm". Before
//! this, every connector accepted only the ASCII hyphen, so "2–4pm" came back as
//! the bare "4pm", "Tue–Thu" as two separate days, and "October 20–22" lost the
//! 22nd. Downstream (cybernetikaai/foxy's NER sidecar) a booking request of
//! "2–4pm" then composed into the whole afternoon. Separately, "between 2 and
//! 4pm" resolved its leading hour to 2am, because only the dash/"to" spelling
//! shared the meridiem.
//!
//! The em dash is prose punctuation — a break or an aside — not a range mark.
//! Read as a connector it joined the two sides of a sentence break into one
//! range ("a Thursday — Monday is wrong" became Thursday to Monday), so it stays
//! a separator, as upstream has it.
//!
//! Reference: Tue 2026-10-13 23:23 at a fixed -04:00 zone, the send instant of
//! the booking reply that surfaced the en dash split.

use serde_json::Value;

fn ctx() -> duckling::ResolveContext {
    let zone = jiff::tz::TimeZone::fixed(jiff::tz::Offset::constant(-4));
    let reference = jiff::civil::date(2026, 10, 13)
        .at(23, 23, 1, 0)
        .to_zoned(zone.clone())
        .unwrap()
        .timestamp();
    duckling::ResolveContext {
        reference,
        zone,
        with_latent: false,
    }
}

/// The single full-span reading of `input`, whole (the candidate spread
/// dropped, as `week_of.rs` does). Full-span, so a connector the grammar does
/// not accept shows up as a missing reading rather than a shorter one.
fn reading(input: &str) -> Value {
    let n = input.chars().count();
    let got: Vec<Value> = duckling::parse(input, &ctx())
        .into_iter()
        .filter(|e| e.dim == "time" && e.start == 0 && e.end == n)
        .map(|e| {
            let mut v = e.value;
            if let Value::Object(ref mut o) = v {
                o.remove("values");
            }
            v
        })
        .collect();
    assert!(!got.is_empty(), "{input:?}: no full-span time reading");
    assert!(
        got.iter().all(|v| v == &got[0]),
        "{input:?}: ambiguous full-span readings: {got:?}"
    );
    got[0].clone()
}

/// `(from, to)` of a full-span interval reading.
fn span(input: &str) -> (String, String) {
    let v = reading(input);
    assert_eq!(v["type"], "interval", "{input:?}: not an interval: {v}");
    let end = |k: &str| v[k]["value"].as_str().expect(k).to_string();
    (end("from"), end("to"))
}

/// The en-dash spelling reads exactly as the hyphen does, and the hyphen
/// reading is the stated one.
fn assert_dash_forms(hyphen: &str, from: &str, to: &str) {
    assert_eq!(span(hyphen), (from.into(), to.into()), "{hyphen:?}");
    let input = hyphen.replacen('-', "–", 1);
    assert_eq!(reading(&input), reading(hyphen), "{input:?} vs {hyphen:?}");
}

/// The surface of every time reading of `input`, sorted.
fn time_bodies(input: &str) -> Vec<String> {
    let mut got: Vec<String> = duckling::parse(input, &ctx())
        .into_iter()
        .filter(|e| e.dim == "time")
        .map(|e| e.body)
        .collect();
    got.sort();
    got
}

// --- an en dash is a range connector ---------------------------------------

#[test]
fn a_clock_range_sharing_its_meridiem() {
    assert_dash_forms(
        "2-4pm",
        "2026-10-14T14:00:00.000-04:00",
        "2026-10-14T17:00:00.000-04:00",
    );
    assert_dash_forms(
        "2 - 4pm",
        "2026-10-14T14:00:00.000-04:00",
        "2026-10-14T17:00:00.000-04:00",
    );
}

#[test]
fn a_clock_range_with_both_ends_clocked() {
    assert_dash_forms(
        "2pm-4pm",
        "2026-10-14T14:00:00.000-04:00",
        "2026-10-14T17:00:00.000-04:00",
    );
    assert_dash_forms(
        "9am - 5pm",
        "2026-10-14T09:00:00.000-04:00",
        "2026-10-14T18:00:00.000-04:00",
    );
}

#[test]
fn a_weekday_range() {
    assert_dash_forms(
        "Tue-Thu",
        "2026-10-13T00:00:00.000-04:00",
        "2026-10-16T00:00:00.000-04:00",
    );
}

#[test]
fn a_day_of_month_range() {
    assert_dash_forms(
        "October 20-22",
        "2026-10-20T00:00:00.000-04:00",
        "2026-10-23T00:00:00.000-04:00",
    );
}

#[test]
fn a_clock_range_with_a_trailing_timezone() {
    // 9:30 CST is 11:30 at -04:00.
    assert_dash_forms(
        "9:30-11:00 CST",
        "2026-10-14T11:30:00.000-04:00",
        "2026-10-14T13:01:00.000-04:00",
    );
}

#[test]
fn a_year_range() {
    let v = reading("1960–1961");
    assert_eq!(v, reading("1960-1961"));
    assert_eq!(v["from"]["value"], "1960-01-01T00:00:00.000-04:00");
}

// --- an em dash is punctuation, not a range connector ---------------------

#[test]
fn an_em_dash_sentence_break_does_not_join_its_sides() {
    // Each side keeps its own reading, as with any other punctuation.
    assert_eq!(
        time_bodies("June 11, 2026 is a Thursday — Monday is wrong."),
        ["June 11, 2026", "Monday", "Thursday"]
    );
    assert_eq!(
        time_bodies("Tuesday at 3pm — Thursday at 4pm"),
        ["Thursday at 4pm", "Tuesday at 3pm"]
    );
}

#[test]
fn an_em_dash_aside_does_not_join_its_sides() {
    // The same instant written two ways is two readings, not a 2pm-6pm range.
    assert_eq!(
        time_bodies("the proposed time (2:00 PM EDT — 18:00Z in the host's zone)"),
        ["18:00", "2:00 PM EDT"]
    );
}

#[test]
fn an_unspaced_em_dash_is_not_a_connector_either() {
    assert_eq!(time_bodies("2pm—4pm"), ["2pm", "4pm"]);
}

// --- "between … and" shares the trailing meridiem --------------------------

#[test]
fn between_a_bare_hour_and_a_clocked_time() {
    assert_eq!(
        span("between 2 and 4pm"),
        (
            "2026-10-14T14:00:00.000-04:00".into(),
            "2026-10-14T17:00:00.000-04:00".into()
        )
    );
    assert_eq!(
        span("between 2:30 and 4pm"),
        (
            "2026-10-14T14:30:00.000-04:00".into(),
            "2026-10-14T16:01:00.000-04:00".into()
        )
    );
}

#[test]
fn between_with_no_trailing_meridiem_is_unchanged() {
    // Nothing to share: both ends stay bare hours, nearest-upcoming as before.
    assert_eq!(
        span("between 9 and 11"),
        (
            "2026-10-13T21:00:00.000-04:00".into(),
            "2026-10-14T00:00:00.000-04:00".into()
        )
    );
    assert_eq!(
        span("between 9am and 11am"),
        (
            "2026-10-14T09:00:00.000-04:00".into(),
            "2026-10-14T12:00:00.000-04:00".into()
        )
    );
}

#[test]
fn and_without_between_is_not_a_range() {
    // "2 and 4pm" lists two times; only "between" makes "and" a connector.
    assert_eq!(time_bodies("2 and 4pm"), ["4pm"]);
}

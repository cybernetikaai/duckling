//! A day range by a clock range ("Mon-Fri 9am-5pm") is two readings, and a
//! range that ends on a clock time ends at that time (cybernetikaai/foxy#6952).
//!
//! Reference: Tue 2026-10-13 23:23 at a fixed -04:00 zone.

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

fn time_entities(input: &str) -> Vec<duckling::Entity> {
    let mut got: Vec<duckling::Entity> = duckling::parse(input, &ctx())
        .into_iter()
        .filter(|e| e.dim == "time")
        .collect();
    got.sort_by_key(|e| (e.start, e.end));
    got
}

fn bodies(input: &str) -> Vec<String> {
    let mut b: Vec<String> = time_entities(input).into_iter().map(|e| e.body).collect();
    b.dedup();
    b
}

/// `(from, to)` of the interval reading whose surface is `body`.
fn span_of(input: &str, body: &str) -> (String, String) {
    let e = time_entities(input)
        .into_iter()
        .find(|e| e.body == body)
        .unwrap_or_else(|| panic!("{input:?}: no reading with body {body:?}"));
    let v: &Value = &e.value;
    assert_eq!(
        v["type"], "interval",
        "{input:?}/{body:?}: not an interval: {v}"
    );
    let end = |k: &str| v[k]["value"].as_str().expect(k).to_string();
    (end("from"), end("to"))
}

fn span(input: &str) -> (String, String) {
    span_of(input, input)
}

#[test]
fn day_range_and_clock_range_are_separate_readings() {
    for (input, days, clock) in [
        ("next week Mon-Fri 9am-5pm", "next week Mon-Fri", "9am-5pm"),
        ("next week Mon-Fri, 9am-5pm", "next week Mon-Fri", "9am-5pm"),
        ("Tue-Thu 2-4pm", "Tue-Thu", "2-4pm"),
        ("Tue–Thu 2–4pm", "Tue–Thu", "2–4pm"),
        ("Tue/Wed 2-4pm", "Tue/Wed", "2-4pm"),
        (
            "Oct 20 - Oct 22 from 2 to 4pm",
            "Oct 20 - Oct 22",
            "from 2 to 4pm",
        ),
        (
            "Monday to Friday, 9am to 3pm",
            "Monday to Friday",
            "9am to 3pm",
        ),
    ] {
        assert_eq!(
            bodies(input),
            vec![days.to_string(), clock.to_string()],
            "{input:?}"
        );
    }
}

#[test]
fn clock_first_then_day_range_also_splits() {
    assert_eq!(
        bodies("9am to 3pm, Monday to Friday"),
        vec!["9am to 3pm".to_string(), "Monday to Friday".to_string()]
    );
}

#[test]
fn split_parts_resolve_as_they_would_alone() {
    assert_eq!(
        span_of("next week Mon-Fri 9am-5pm", "next week Mon-Fri"),
        span("next week Mon-Fri")
    );
    assert_eq!(
        span_of("next week Mon-Fri 9am-5pm", "9am-5pm"),
        span("9am-5pm")
    );
}

#[test]
fn a_wrapped_day_range_by_clock_still_splits() {
    assert_eq!(
        bodies("about Tue-Thu next week 2-4pm"),
        vec!["Tue-Thu next week".to_string(), "2-4pm".to_string()]
    );
}

#[test]
fn one_window_stays_one_reading() {
    for input in [
        "next Tuesday 2-4pm",
        "tomorrow at 3pm",
        "next week at 3pm",
        "Mon 9am - Fri 5pm",
        "Oct 20 9am - Oct 22 5pm",
    ] {
        assert_eq!(bodies(input), vec![input.to_string()], "{input:?}");
    }
}

#[test]
fn parse_all_splits_too() {
    let input = "next week Mon-Fri 9am-5pm";
    let mut got: Vec<String> = duckling::parse_all(input, &ctx())
        .into_iter()
        .filter(|e| e.dim == "time")
        .map(|e| e.body)
        .collect();
    got.dedup();
    assert_eq!(
        got,
        vec!["next week Mon-Fri".to_string(), "9am-5pm".to_string()]
    );
}

#[test]
fn a_range_ending_on_a_clock_time_ends_at_that_time() {
    for (input, from, to) in [
        (
            "2-4pm",
            "2026-10-14T14:00:00.000-04:00",
            "2026-10-14T16:00:00.000-04:00",
        ),
        (
            "9am-5pm",
            "2026-10-14T09:00:00.000-04:00",
            "2026-10-14T17:00:00.000-04:00",
        ),
        (
            "9am to noon",
            "2026-10-14T09:00:00.000-04:00",
            "2026-10-14T12:00:00.000-04:00",
        ),
        (
            "9:30am-5:30pm",
            "2026-10-14T09:30:00.000-04:00",
            "2026-10-14T17:30:00.000-04:00",
        ),
        (
            "Mon 9am - Fri 5pm",
            "2026-10-12T09:00:00.000-04:00",
            "2026-10-16T17:00:00.000-04:00",
        ),
    ] {
        assert_eq!(span(input), (from.to_string(), to.to_string()), "{input:?}");
    }
}

#[test]
fn a_range_ending_on_a_day_includes_that_day() {
    for (input, from, to) in [
        (
            "Aug 8 - Aug 12",
            "2027-08-08T00:00:00.000-04:00",
            "2027-08-13T00:00:00.000-04:00",
        ),
        (
            "next week Mon-Fri",
            "2026-10-19T00:00:00.000-04:00",
            "2026-10-24T00:00:00.000-04:00",
        ),
        (
            "Mon 9am - Fri",
            "2026-10-12T09:00:00.000-04:00",
            "2026-10-17T00:00:00.000-04:00",
        ),
    ] {
        assert_eq!(span(input), (from.to_string(), to.to_string()), "{input:?}");
    }
}

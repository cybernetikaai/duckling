//! "the <cycle> of <time>" names the cycle that CONTAINS the time.
//!
//! "the week of October 20th" is the week the 20th falls in. That week starts
//! on Monday the 19th — BEFORE the base — and `notImmediate` drops exactly the
//! occurrence that starts before its base, so this rule returned the FOLLOWING
//! week for every input whose date was not already a Monday, and was right only
//! by accident when it was.
//!
//! Downstream this is a booking anchor (`bv3:windowStart` in cybernetikaai/foxy),
//! so a week-late start silently offers the caller the wrong week and never
//! refuses. Reference: 2026-09-21 at a fixed +10:00 zone, matching
//! `intra_period.rs`.

use serde_json::Value;

fn ctx() -> duckling::ResolveContext {
    let zone = jiff::tz::TimeZone::fixed(jiff::tz::Offset::constant(10));
    let reference = jiff::civil::date(2026, 9, 21)
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

/// The single full-span reading, as `(date, grain)`. Full-span because a
/// dropped qualifier leaves a shorter reading behind ("October 20th" out of
/// "the week of October 20th"), and requiring the whole input makes that visible.
fn point(input: &str) -> (String, String) {
    let n = input.chars().count();
    let got: Vec<Value> = duckling::parse(input, &ctx())
        .into_iter()
        .filter(|e| e.dim == "time" && e.start == 0 && e.end == n)
        .map(|e| e.value)
        .collect();
    assert!(!got.is_empty(), "{input:?}: no full-span time reading");
    let first = &got[0];
    assert!(
        got.iter().all(|v| v["value"] == first["value"]),
        "{input:?}: ambiguous full-span readings: {got:?}"
    );
    (
        first["value"].as_str().expect("value")[..10].to_string(),
        first["grain"].as_str().expect("grain").to_string(),
    )
}

// --- the week containing the stated day -------------------------------------
// October 2026: the 19th is a Monday, so its week is Mon 19 - Sun 25.

#[test]
fn a_midweek_day_gives_the_week_it_falls_in() {
    // The 20th is a Tuesday. Before the fix this answered 2026-10-26, a week late.
    assert_eq!(
        point("the week of October 20th"),
        ("2026-10-19".into(), "week".into())
    );
}

#[test]
fn a_monday_gives_its_own_week() {
    // The case that was accidentally right: the containing week starts exactly
    // at the base, so nothing was dropped.
    assert_eq!(
        point("the week of October 19th"),
        ("2026-10-19".into(), "week".into())
    );
}

#[test]
fn the_last_day_of_a_week_still_gives_that_week() {
    // The 25th is a Sunday — the far end of the same week, not the next one.
    assert_eq!(
        point("the week of October 25th"),
        ("2026-10-19".into(), "week".into())
    );
}

#[test]
fn the_next_monday_gives_the_next_week() {
    assert_eq!(
        point("the week of October 26th"),
        ("2026-10-26".into(), "week".into())
    );
}

#[test]
fn a_week_that_starts_in_the_previous_month() {
    // December 1st 2026 is a Tuesday, so its week starts Monday November 30th.
    // The containing week is not required to sit inside the stated month.
    assert_eq!(
        point("the week of December 1st"),
        ("2026-11-30".into(), "week".into())
    );
}

// --- what must NOT change ---------------------------------------------------

#[test]
fn the_ordinal_form_still_means_the_first_week_wholly_inside() {
    // "the <ordinal> <cycle> of <time>" keeps notImmediate=true on purpose:
    // "the first week of October" is the first week wholly inside October, not
    // the week October's 1st falls in.
    assert_eq!(
        point("the first week of October"),
        ("2026-10-05".into(), "week".into())
    );
}

#[test]
fn coarser_bases_are_unaffected() {
    // A base already aligned to the cycle start was never dropped, so these
    // resolve identically before and after.
    assert_eq!(point("the month of October").0, "2026-10-01");
    assert_eq!(
        point("the day of October 20th"),
        ("2026-10-20".into(), "day".into())
    );
}

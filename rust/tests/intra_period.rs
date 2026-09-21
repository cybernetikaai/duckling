//! Intra-period qualifiers — the spec for cybernetikaai/ner#17.
//!
//! These are the surfaces the NER service receives from callers picking a
//! booking window, and that this parser does not compose yet. Each test states
//! the span a caller MEANS, not the span we happen to return today, so a red
//! test here is a rule that has not been written rather than a behaviour to
//! preserve.
//!
//! Reference: 2026-09-21 at a fixed +10:00 zone. Fixed rather than a real
//! Australia/Sydney, deliberately: Sydney's DST transition falls inside
//! October, and these assertions are about rule composition, not offset
//! selection — `tz_stress` in the corpus suite owns that.

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

fn strip_values(mut v: Value) -> Value {
    if let Value::Object(ref mut o) = v {
        o.remove("values");
    }
    v
}

/// Every reading that spans the WHOLE input. A qualifier that gets dropped
/// leaves a shorter reading behind ("October" out of "first half of October"),
/// so requiring the full span is what makes the drop visible.
fn readings(input: &str) -> Vec<Value> {
    let n = input.chars().count();
    let ctx = ctx();
    duckling::parse(input, &ctx)
        .into_iter()
        .filter(|e| e.dim == "time" && e.start == 0 && e.end == n)
        .map(|e| strip_values(e.value))
        .collect()
}

/// The single full-span reading. Redundant rule paths that resolve to the same
/// value count as one; genuinely differing values are an ambiguity the caller
/// cannot act on, so they fail here.
fn one(input: &str) -> Value {
    let got = readings(input);
    assert!(!got.is_empty(), "{input:?}: no full-span time reading");
    let first = got[0].clone();
    assert!(
        got.iter().all(|v| *v == first),
        "{input:?}: ambiguous full-span readings: {got:?}"
    );
    first
}

fn date_of(v: &Value) -> String {
    v.as_str().expect("resolved value is a string")[..10].to_string()
}

/// `(from, to)` as plain dates. `to` is None for an open-ended window, which is
/// represented by ABSENCE of the key — never a null and never a fabricated end.
fn interval(input: &str) -> (String, Option<String>) {
    let v = one(input);
    assert_eq!(v["type"], "interval", "{input:?} is not an interval: {v}");
    (
        date_of(&v["from"]["value"]),
        v.get("to").map(|t| date_of(&t["value"])),
    )
}

/// `(date, grain)` of a point reading.
fn point(input: &str) -> (String, String) {
    let v = one(input);
    assert_eq!(v["type"], "value", "{input:?} is not a point: {v}");
    (
        date_of(&v["value"]),
        v["grain"].as_str().expect("grain").to_string(),
    )
}

// --- halves of a month ------------------------------------------------------
// Year-invariant spans: Oct 1-15 and Oct 16-31 in EVERY year. Today the
// qualifier is dropped whole and only the bare month survives, because the only
// "half" rules in the grammar are hour-of-day ("half past three").

#[test]
fn first_half_of_a_month() {
    assert_eq!(
        interval("first half of October"),
        ("2026-10-01".into(), Some("2026-10-16".into()))
    );
}

#[test]
fn second_half_of_a_month() {
    assert_eq!(
        interval("second half of October"),
        ("2026-10-16".into(), Some("2026-11-01".into()))
    );
}

#[test]
fn the_article_form_of_a_half() {
    assert_eq!(
        interval("the first half of October"),
        ("2026-10-01".into(), Some("2026-10-16".into()))
    );
}

// --- last N weeks of a month ------------------------------------------------

#[test]
fn last_two_weeks_of_a_month() {
    // The last two Mon-Sun weeks the month ends on: Oct 19 through Nov 2.
    assert_eq!(
        interval("last two weeks of October"),
        ("2026-10-19".into(), Some("2026-11-02".into()))
    );
}

#[test]
fn last_n_weeks_of_a_month_never_resolves_into_the_past() {
    // Today this answers Sep 7-21: "of October" is dropped entirely and "last
    // two weeks" resolves BACKWARDS from the reference. Stated separately from
    // the span above because a window that closed before the caller spoke is a
    // different and worse failure than a window off by a week.
    let (from, _) = interval("last two weeks of October");
    assert!(
        from.as_str() >= "2026-09-21",
        "resolved behind the reference: {from}"
    );
}

// --- which week is "the last week" ------------------------------------------

#[test]
fn the_last_week_of_a_month_is_the_week_the_month_ends_on() {
    // October 2026 ends Sat Oct 31, so the week of Mon Oct 26 carries six of
    // the month's days and is what a caller means. duckling answers Oct 19
    // today — the last week lying WHOLLY inside October — which also
    // contradicts its own "late October" = [Oct 21, Nov 1).
    assert_eq!(
        point("the last week of October"),
        ("2026-10-26".into(), "week".into())
    );
}

#[test]
fn a_week_that_merely_clips_the_month_is_not_its_last_week() {
    // September 2014 ends Tue Sep 30, so the Sep 29 week holds only two
    // September days and the answer stays Sep 22. Both the old "wholly inside"
    // reading and the new "the week it ends on" reading agree here — which is
    // why the corpus's pinned value for this input survives the change.
    assert_eq!(
        point("last week of september 2014"),
        ("2014-09-22".into(), "week".into())
    );
}

// --- a trailing open end ----------------------------------------------------

#[test]
fn trailing_onwards_opens_the_window() {
    assert_eq!(
        interval("October 15th onwards"),
        ("2026-10-15".into(), None)
    );
}

#[test]
fn leading_after_already_opens_the_window() {
    // The structure is supported; only the trailing surface form is missing.
    // Asserted so the pair above reads as an asymmetry, not a mystery.
    assert_eq!(interval("after October 15th"), ("2026-10-15".into(), None));
}

// --- regression guard -------------------------------------------------------

#[test]
fn late_month_is_unchanged() {
    // The nearest neighbour to everything above, and the value the halves are
    // matched against for consistency.
    assert_eq!(
        interval("late October"),
        ("2026-10-21".into(), Some("2026-11-01".into()))
    );
}

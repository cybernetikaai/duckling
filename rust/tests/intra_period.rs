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
    ctx_in(2026)
}

fn ctx_in(year: i16) -> duckling::ResolveContext {
    let zone = jiff::tz::TimeZone::fixed(jiff::tz::Offset::constant(10));
    let reference = jiff::civil::date(year, 9, 21)
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
fn readings_in(input: &str, ctx: duckling::ResolveContext) -> Vec<Value> {
    let n = input.chars().count();
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
    one_in(input, ctx())
}

fn one_in(input: &str, ctx: duckling::ResolveContext) -> Value {
    let got = readings_in(input, ctx);
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
    interval_in(input, ctx())
}

fn interval_in(input: &str, ctx: duckling::ResolveContext) -> (String, Option<String>) {
    let v = one_in(input, ctx);
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
    // 14 days counted back from the end of the month, so the plural cannot
    // disagree with the singular about where the span ends.
    assert_eq!(
        interval("last two weeks of October"),
        ("2026-10-18".into(), Some("2026-11-01".into()))
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

// --- what "the last week of <month>" means ---------------------------------
// The month's final seven days, ending exactly where the month ends. NOT a
// Mon-Sun week: `take_last_days_of` records why the week-aligned readings were
// rejected. There is no universal convention for this phrase, so these tests
// pin the three properties that made this reading the usable one.

#[test]
fn the_last_week_of_a_month_is_its_final_seven_days() {
    assert_eq!(
        interval("the last week of October"),
        ("2026-10-25".into(), Some("2026-11-01".into()))
    );
}

#[test]
fn the_last_week_ends_where_the_month_ends_even_when_it_ends_mid_week() {
    // November 2026 ends on a MONDAY, which is where every week-aligned
    // reading has to choose badly: retreat to the last fully-contained week
    // and Nov 30 -- the month's own last day -- falls outside "the last week
    // of November", or keep the week containing it and run six days into
    // December. Counting seven days back from the month's end does neither,
    // and agrees with the grammar's "late November" = [Nov 21, Dec 1) about
    // where November stops.
    assert_eq!(
        interval("the last week of November"),
        ("2026-11-24".into(), Some("2026-12-01".into()))
    );
}

#[test]
fn the_last_week_is_the_same_dates_in_every_year() {
    // The property that earns this definition its keep. The span depends only
    // on the month's LENGTH, so October's last week is Oct 25-31 whatever the
    // year -- which lets a consumer that refuses to assert an unstated year
    // still carry the month and day. A week-aligned span moves with the
    // weekday and cannot be stated at all without an anchor.
    let a = interval_in("the last week of October", ctx_in(2026));
    let b = interval_in("the last week of October", ctx_in(2029));
    assert_eq!(a.0[4..], b.0[4..], "start moved between years: {a:?} {b:?}");
    assert_eq!(&a.0[4..], "-10-25");
    assert_eq!(
        a.1.as_deref().map(|s| &s[4..]),
        b.1.as_deref().map(|s| &s[4..]),
        "end moved between years: {a:?} {b:?}"
    );
}

#[test]
fn only_week_changed_other_grains_keep_the_containment_reading() {
    // The dispatch is scoped to Week. "the last day of October" is Oct 31
    // under either reading and stays a day-grained POINT rather than becoming
    // a span -- this is also a corpus-pinned input, so it is checked twice.
    assert_eq!(
        point("last day of october 2015"),
        ("2015-10-31".into(), "day".into())
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

//! Resolution context, output entity, and TimeData resolution.

use serde::Serialize;

use crate::duration::{DurationData, in_seconds};
use crate::grain::{Grain, add, grain_str};
use crate::json::{interval_value, open_interval_value, simple_value};
use crate::time::object::{IntervalDirection, TimeObject, time_intersect};
use crate::time::predicate::TimeContext;
use crate::types::TimeData;

/// Reference instant plus the zone it is interpreted in.
///
/// The output offset for each resolved value is derived per-instant from
/// `zone` — never hard-coded. The Duckling test context is the special case
/// where `zone` is a fixed -02:00 offset with no transitions; production uses
/// a real IANA zone. To "coerce into the user's target zone", set `zone` to
/// that zone here at parse time — do not convert the resolved instant after.
pub struct ResolveContext {
    /// The "now" as a true UTC instant.
    pub reference: jiff::Timestamp,
    /// The zone relative expressions resolve in and output offsets come from.
    pub zone: jiff::tz::TimeZone,
    /// When false, latent parses (e.g. a bare "7" as an hour) are dropped.
    pub with_latent: bool,
}

/// A resolved entity in the public API format.
#[derive(Debug, Clone, Serialize)]
pub struct Entity {
    pub dim: String,
    pub body: String,
    pub start: usize,
    pub end: usize,
    pub value: serde_json::Value,
    pub latent: bool,
}

/// Resolve an Ordinal to Duckling's JSON: `{type:"value", value:<int>}`.
pub fn ordinal_value(o: &crate::ordinal::OrdinalData) -> serde_json::Value {
    serde_json::json!({"type": "value", "value": o.value})
}

/// Resolve a Duration to Duckling's JSON: `{value, unit, <unit>: value, type,
/// normalized: {value: <seconds>, unit: "second"}}` (port of the DurationData
/// ToJSON instance). The `<unit>` key is dynamic — e.g. `"minute": 30`.
pub fn duration_value(d: &DurationData) -> serde_json::Value {
    let unit = grain_str(d.grain);
    let mut o = serde_json::Map::new();
    o.insert("type".to_string(), serde_json::json!("value"));
    o.insert("value".to_string(), serde_json::json!(d.value));
    o.insert("unit".to_string(), serde_json::json!(unit));
    o.insert(unit.to_string(), serde_json::json!(d.value));
    o.insert(
        "normalized".to_string(),
        serde_json::json!({"value": in_seconds(d.grain, d.value), "unit": "second"}),
    );
    serde_json::Value::Object(o)
}

/// Resolve a TimeData against the context, returning its value JSON
/// (the corpus harness strips the "values" alternatives array, so we emit only
/// the primary value here). Picks the first future occurrence, else the first
/// past one. notImmediate / intervals / directions arrive in later phases.
/// The offset a wall-clock time carries in `zone`. For a DST gap or fold the
/// local time is invalid/ambiguous; Duckling keeps the pre-transition offset
/// (e.g. 2:30am on a spring-forward day stays at the earlier -05:00, reported
/// against the unchanged wall clock), so pick `before` rather than letting the
/// default `Compatible` disambiguation shift the instant across the gap.
fn zone_offset(dt: jiff::civil::DateTime, zone: &jiff::tz::TimeZone) -> jiff::tz::Offset {
    use jiff::tz::AmbiguousOffset::*;
    match zone.to_ambiguous_timestamp(dt).offset() {
        Unambiguous { offset } => offset,
        Gap { before, .. } | Fold { before, .. } => before,
    }
}

pub fn resolve_time(td: &TimeData, ctx: &ResolveContext) -> Option<serde_json::Value> {
    if td.latent && !ctx.with_latent {
        return None;
    }
    let ref_zoned = ctx.reference.to_zoned(ctx.zone.clone());
    let ref_dt = ref_zoned.datetime();
    let ref_offset_minutes = (ref_zoned.offset().seconds() / 60) as i64;
    let ref_time = TimeObject { start: ref_dt, grain: Grain::Second, end: None };
    let tc = TimeContext {
        ref_time,
        min_time: TimeObject { start: add(ref_dt, Grain::Year, -2000), grain: Grain::Second, end: None },
        max_time: TimeObject { start: add(ref_dt, Grain::Year, 2000), grain: Grain::Second, end: None },
        ref_offset_minutes,
    };
    // --- Primary resolution (`chosen`) — the validated single-occurrence logic. ---
    let (mut past, mut future) = td.pred.run(ref_time, &tc);
    let chosen = match future.next() {
        None => past.next()?,
        Some(ahead) => {
            let ahead_covers = time_intersect(ahead, ref_time).is_some();
            if td.not_immediate && ahead_covers {
                // notImmediate: the first future occurrence covers "now"; use the next.
                future.next().unwrap_or(ahead)
            } else if !ahead_covers {
                // Ongoing multi-day holiday (Ramadan/Hanukkah/Lent asked during it):
                // return the covering interval rather than skipping a year. Seasons/
                // weekend surface it as `ahead`, so this only fires for predicates
                // that don't. (end.is_some() → a real interval, not a day/hour point.)
                match past.next() {
                    Some(behind)
                        if behind.end.is_some() && time_intersect(behind, ref_time).is_some() =>
                    {
                        behind
                    }
                    _ => ahead,
                }
            } else {
                ahead
            }
        }
    };

    // --- `values` alternatives (Duckling's array): a *separate* enumeration from a
    // fresh run, so it never perturbs `chosen`. Take up to 3 occurrences forward
    // from the current one: the occurrence covering "now" (if any) then the future
    // ones. Recurring predicates yield 3; single-occurrence / past-direction ones
    // ("next week", "today", "last monday") yield 1. Note values[0] can differ from
    // `chosen` — "tuesday" on a Tuesday resolves (notImmediate) to next Tuesday, but
    // its alternatives list today first: [today, +1wk, +2wk].
    let occs: Vec<TimeObject> = {
        let (mut vpast, mut vfut) = td.pred.run(ref_time, &tc);
        let first_past = vpast.next();
        let mut v: Vec<TimeObject> = Vec::new();
        if let Some(c) = first_past.filter(|b| time_intersect(*b, ref_time).is_some()) {
            v.push(c);
        }
        v.extend(vfut.by_ref().take(3usize.saturating_sub(v.len())));
        if v.is_empty() {
            if let Some(b) = first_past {
                v.push(b);
            }
        }
        v
    };

    // Build the JSON for one occurrence (value / interval / open-interval), with
    // per-instant DST-correct offsets. No holidayBeta or nested values here —
    // those live only on the top-level object.
    let occ_json = |occ: &TimeObject| -> serde_json::Value {
        let off = zone_offset(occ.start, &ctx.zone);
        if let Some(dir) = td.direction {
            open_interval_value(occ.start, off, occ.grain, matches!(dir, IntervalDirection::After))
        } else {
            match occ.end {
                Some(end) => {
                    interval_value(occ.start, off, end, zone_offset(end, &ctx.zone), occ.grain)
                }
                None => simple_value(occ.start, off, occ.grain),
            }
        }
    };
    let values: Vec<serde_json::Value> = occs.iter().map(&occ_json).collect();

    let mut value = occ_json(&chosen);
    if let serde_json::Value::Object(o) = &mut value {
        if let Some(h) = &td.holiday {
            o.insert("holidayBeta".to_string(), serde_json::Value::String(h.clone()));
        }
        o.insert("values".to_string(), serde_json::Value::Array(values));
    }
    Some(value)
}

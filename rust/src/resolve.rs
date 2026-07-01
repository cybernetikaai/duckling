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

/// Resolve a Numeral to Duckling's JSON: `{type:"value", value:<number>}`.
/// Whole numbers emit as integers (Duckling renders `20`, not `20.0`).
pub fn numeral_value(n: &crate::numeral::NumeralData) -> serde_json::Value {
    if n.value.fract() == 0.0 {
        serde_json::json!({"type": "value", "value": n.value as i64})
    } else {
        serde_json::json!({"type": "value", "value": n.value})
    }
}

/// Resolve an Email to Duckling's JSON: `{type:"value", value:"a@b.com"}`.
pub fn email_value(e: &crate::email::EmailData) -> serde_json::Value {
    serde_json::json!({"type": "value", "value": e.value})
}

/// Resolve a Url to Duckling's JSON: `{value, domain, type:"value"}`.
pub fn url_value(u: &crate::url::UrlData) -> serde_json::Value {
    serde_json::json!({"value": u.value, "domain": u.domain, "type": "value"})
}

/// Resolve a PhoneNumber to Duckling's JSON: `{value, type:"value"}` — the
/// normalized "(+<code>) <digits> ext <ext>" string.
pub fn phonenumber_value(p: &crate::phonenumber::PhoneNumberData) -> serde_json::Value {
    let prefix = p.prefix.map(|c| format!("(+{c}) ")).unwrap_or_default();
    let ext = p.extension.map(|e| format!(" ext {e}")).unwrap_or_default();
    serde_json::json!({"value": format!("{prefix}{}{ext}", p.number), "type": "value"})
}

/// A number as JSON: integer when whole (Duckling renders `37`, not `37.0`).
fn num(v: f64) -> serde_json::Value {
    if v.fract() == 0.0 {
        serde_json::json!(v as i64)
    } else {
        serde_json::json!(v)
    }
}

/// Resolve a Temperature (port of the TemperatureData Resolve instance). None
/// when there is no unit (a latent value-only temperature is never emitted).
pub fn temperature_value(t: &crate::temperature::TemperatureData) -> Option<serde_json::Value> {
    let u = t.unit?.as_str();
    Some(match (t.value, t.min, t.max) {
        (Some(v), _, _) => serde_json::json!({"value": num(v), "type": "value", "unit": u}),
        (None, Some(from), Some(to)) => serde_json::json!({
            "from": {"value": num(from), "unit": u}, "to": {"value": num(to), "unit": u}, "type": "interval"}),
        (None, Some(from), None) => serde_json::json!({
            "from": {"value": num(from), "unit": u}, "type": "interval"}),
        (None, None, Some(to)) => serde_json::json!({
            "to": {"value": num(to), "unit": u}, "type": "interval"}),
        _ => return None,
    })
}

/// Resolve a Volume (port of the VolumeData Resolve instance). None when there
/// is no unit (a latent value-only or unit-only volume is never emitted).
pub fn volume_value(v: &crate::volume::VolumeData) -> Option<serde_json::Value> {
    let u = v.unit?.as_str();
    Some(match (v.value, v.min, v.max) {
        (Some(val), _, _) => serde_json::json!({"value": num(val), "unit": u, "type": "value"}),
        (None, Some(from), Some(to)) => serde_json::json!({
            "type": "interval", "from": {"value": num(from), "unit": u}, "to": {"value": num(to), "unit": u}}),
        (None, Some(from), None) => serde_json::json!({
            "type": "interval", "from": {"value": num(from), "unit": u}}),
        (None, None, Some(to)) => serde_json::json!({
            "type": "interval", "to": {"value": num(to), "unit": u}}),
        _ => return None,
    })
}

/// Resolve a CreditCardNumber to Duckling's JSON: `{value, issuer}` (no `type`).
pub fn creditcard_value(c: &crate::creditcard::CreditCardData) -> serde_json::Value {
    serde_json::json!({"value": c.number, "issuer": c.issuer})
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
    let ref_time = TimeObject {
        start: ref_dt,
        grain: Grain::Second,
        end: None,
    };
    let tc = TimeContext {
        ref_time,
        min_time: TimeObject {
            start: add(ref_dt, Grain::Year, -2000),
            grain: Grain::Second,
            end: None,
        },
        max_time: TimeObject {
            start: add(ref_dt, Grain::Year, 2000),
            grain: Grain::Second,
            end: None,
        },
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
        if v.is_empty()
            && let Some(b) = first_past
        {
            v.push(b);
        }
        v
    };

    // Build the JSON for one occurrence (value / interval / open-interval), with
    // per-instant DST-correct offsets. No holidayBeta or nested values here —
    // those live only on the top-level object.
    let occ_json = |occ: &TimeObject| -> serde_json::Value {
        let off = zone_offset(occ.start, &ctx.zone);
        if let Some(dir) = td.direction {
            open_interval_value(
                occ.start,
                off,
                occ.grain,
                matches!(dir, IntervalDirection::After),
            )
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
            o.insert(
                "holidayBeta".to_string(),
                serde_json::Value::String(h.clone()),
            );
        }
        o.insert("values".to_string(), serde_json::Value::Array(values));
    }
    Some(value)
}

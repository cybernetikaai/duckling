//! Resolution context, output entity, and TimeData resolution.

use serde::Serialize;

use crate::grain::{Grain, add};
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
    let (mut past, mut future) = td.pred.run(ref_time, &tc);
    let chosen = match future.next() {
        None => past.next()?,
        Some(ahead) => {
            // notImmediate: if the first future occurrence covers "now", use the next.
            if td.not_immediate && time_intersect(ahead, ref_time).is_some() {
                future.next().unwrap_or(ahead)
            } else {
                ahead
            }
        }
    };
    // Offset for this resolved local instant, from the zone (DST-correct).
    let off = zone_offset(chosen.start, &ctx.zone);
    if let Some(dir) = td.direction {
        return Some(open_interval_value(
            chosen.start,
            off,
            chosen.grain,
            matches!(dir, IntervalDirection::After),
        ));
    }
    let mut value = match chosen.end {
        Some(end) => {
            let off_end = zone_offset(end, &ctx.zone);
            interval_value(chosen.start, off, end, off_end, chosen.grain)
        }
        None => simple_value(chosen.start, off, chosen.grain),
    };
    if let Some(h) = &td.holiday {
        if let serde_json::Value::Object(o) = &mut value {
            o.insert("holidayBeta".to_string(), serde_json::Value::String(h.clone()));
        }
    }
    Some(value)
}

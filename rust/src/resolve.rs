//! Resolution context, output entity, and TimeData resolution.

use serde::Serialize;

use crate::grain::{Grain, add};
use crate::json::{interval_value, simple_value};
use crate::time::object::{TimeObject, time_intersect};
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
pub fn resolve_time(td: &TimeData, ctx: &ResolveContext) -> Option<serde_json::Value> {
    if td.latent && !ctx.with_latent {
        return None;
    }
    let ref_dt = ctx.reference.to_zoned(ctx.zone.clone()).datetime();
    let ref_time = TimeObject { start: ref_dt, grain: Grain::Second, end: None };
    let tc = TimeContext {
        ref_time,
        min_time: TimeObject { start: add(ref_dt, Grain::Year, -2000), grain: Grain::Second, end: None },
        max_time: TimeObject { start: add(ref_dt, Grain::Year, 2000), grain: Grain::Second, end: None },
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
    let off = chosen.start.to_zoned(ctx.zone.clone()).ok()?.offset();
    match chosen.end {
        Some(end) => {
            let off_end = end.to_zoned(ctx.zone.clone()).ok()?.offset();
            Some(interval_value(chosen.start, off, end, off_end, chosen.grain))
        }
        None => Some(simple_value(chosen.start, off, chosen.grain)),
    }
}

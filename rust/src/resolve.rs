//! Resolution context and output entity.

use serde::Serialize;

/// Reference instant plus the zone it is interpreted in.
///
/// The output offset for each resolved value is derived per-instant from
/// `zone` — never hard-coded. The Duckling test context is the special case
/// where `zone` is a fixed -02:00 offset with no transitions; production uses
/// a real IANA zone (e.g. `America/New_York`). To "coerce into the user's
/// target zone", set `zone` to that zone here at parse time — do not convert
/// the resolved instant afterward (grain-bearing/interval results would be
/// silently corrupted).
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

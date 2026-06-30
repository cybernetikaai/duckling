//! RFC3339 + Time value JSON shaping (port of Duckling/Time/Types.hs:730-197).
//!
//! Matches Duckling's exact shape: milliseconds padded to 3 digits, offset
//! `±HH:MM`. The offset is supplied per-resolved-instant (DST-correct).

use crate::grain::{Grain, grain_str};
use jiff::civil::DateTime;
use jiff::tz::Offset;
use serde_json::{Value, json};

pub fn rfc3339(dt: DateTime, off: Offset) -> String {
    let ms = dt.subsec_nanosecond() / 1_000_000;
    let total = off.seconds();
    let sign = if total < 0 { '-' } else { '+' };
    let a = total.abs();
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}{}{:02}:{:02}",
        dt.year(),
        dt.month(),
        dt.day(),
        dt.hour(),
        dt.minute(),
        dt.second(),
        ms,
        sign,
        a / 3600,
        (a % 3600) / 60
    )
}

pub fn simple_value(dt: DateTime, off: Offset, g: Grain) -> Value {
    json!({ "type": "value", "value": rfc3339(dt, off), "grain": grain_str(g) })
}

pub fn interval_value(
    start: DateTime,
    off_start: Offset,
    end: DateTime,
    off_end: Offset,
    g: Grain,
) -> Value {
    json!({
        "type": "interval",
        "from": { "value": rfc3339(start, off_start), "grain": grain_str(g) },
        "to": { "value": rfc3339(end, off_end), "grain": grain_str(g) },
    })
}

//! Integration tests for the `duckling` CLI binary. `cargo test` builds the bin
//! and exposes its path via `CARGO_BIN_EXE_duckling`; we run it as a subprocess
//! and assert on stdout / exit status. Regression coverage for arg parsing,
//! dimension dispatch, stdin, and error paths — the library itself is covered by
//! the corpus tests.

use serde_json::Value;
use std::io::Write;
use std::process::{Command, Stdio};

/// Run the CLI with `args` (and optional stdin); return (stdout, exit_code).
fn run(args: &[&str], stdin: Option<&str>) -> (String, i32) {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_duckling"));
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());
    if stdin.is_some() {
        cmd.stdin(Stdio::piped());
    }
    let mut child = cmd.spawn().expect("spawn duckling");
    if let Some(s) = stdin {
        child.stdin.take().unwrap().write_all(s.as_bytes()).unwrap();
    }
    let out = child.wait_with_output().expect("wait");
    (
        String::from_utf8_lossy(&out.stdout).into_owned(),
        out.status.code().unwrap_or(-1),
    )
}

fn parse(args: &[&str]) -> Vec<Value> {
    let (stdout, code) = run(args, None);
    assert_eq!(code, 0, "expected exit 0 for {args:?}, stdout={stdout}");
    serde_json::from_str(&stdout).unwrap_or_else(|e| panic!("bad JSON for {args:?}: {e}\n{stdout}"))
}

#[test]
fn time_with_ref_and_tz() {
    // "in 2 hours" from 2013-02-12T04:30Z, coerced into America/New_York.
    // --raw keeps the full RFC3339 instant (default truncates to grain).
    let v = parse(&[
        "--raw",
        "--tz",
        "America/New_York",
        "--ref",
        "2013-02-12T04:30:00Z",
        "in 2 hours",
    ]);
    assert_eq!(v[0]["dim"], "time");
    assert_eq!(v[0]["value"]["value"], "2013-02-12T01:30:00.000-05:00");
}

#[test]
fn grain_precision_default_and_raw() {
    // Default: a day-grain time truncates to a bare date (no midnight, no offset).
    let day = parse(&["--ref", "2026-07-02T12:00:00Z", "tomorrow"]);
    assert_eq!(day[0]["value"]["grain"], "day");
    assert_eq!(day[0]["value"]["value"], "2026-07-03");
    // Default: a sub-day (hour) grain keeps the offset.
    let hour = parse(&[
        "--tz",
        "America/New_York",
        "--ref",
        "2026-07-02T12:00:00-04:00",
        "tomorrow at 5pm",
    ]);
    assert_eq!(hour[0]["value"]["value"], "2026-07-03T17:00-04:00");
    // --raw restores the full timestamp.
    let raw = parse(&["--raw", "--ref", "2026-07-02T12:00:00Z", "tomorrow"]);
    assert_eq!(raw[0]["value"]["value"], "2026-07-03T00:00:00.000+00:00");
}

#[test]
fn dims_all_surfaces_both() {
    let v = parse(&[
        "--dims",
        "all",
        "set a timer for 20 minutes and wake me at 7am",
    ]);
    let dims: Vec<&str> = v.iter().filter_map(|e| e["dim"].as_str()).collect();
    assert!(dims.contains(&"duration"), "got {dims:?}");
    assert!(dims.contains(&"time"), "got {dims:?}");
}

#[test]
fn each_dimension_dispatches() {
    assert_eq!(
        parse(&["--dims", "duration", "an hour and a half"])[0]["value"]["value"],
        90
    );
    assert_eq!(
        parse(&["--dims", "ordinal", "twenty-fifth"])[0]["value"]["value"],
        25
    );
    assert_eq!(
        parse(&["--dims", "number", "twenty three"])[0]["value"]["value"],
        23
    );
    assert_eq!(
        parse(&["--dims", "email", "a dot b at x dot com"])[0]["value"]["value"],
        "a.b@x.com"
    );
    assert_eq!(
        parse(&["--dims", "url", "foo.com"])[0]["value"]["domain"],
        "foo.com"
    );
    assert_eq!(
        parse(&["--dims", "credit-card-number", "4111111111111111"])[0]["value"]["issuer"],
        "visa"
    );
    assert_eq!(
        parse(&["--dims", "phone-number", "call +1 (650) 123-4567"])[0]["value"]["value"],
        "(+1) 6501234567"
    );
    assert_eq!(
        parse(&["--dims", "temperature", "70 degrees"])[0]["value"]["unit"],
        "degree"
    );
    assert_eq!(
        parse(&["--dims", "volume", "2 liters"])[0]["value"]["unit"],
        "litre"
    );
    assert_eq!(
        parse(&["--dims", "distance", "3 km"])[0]["value"]["unit"],
        "kilometre"
    );
    assert_eq!(
        parse(&["--dims", "quantity", "3 cups of sugar"])[0]["value"]["product"],
        "sugar"
    );
    assert_eq!(
        parse(&["--dims", "amount-of-money", "$10"])[0]["value"]["unit"],
        "$"
    );
}

#[test]
fn locale_affects_numeric_dates() {
    let us = parse(&["--locale", "en_US", "--ref", "2013-02-12T00:00:00Z", "3/4"]);
    let gb = parse(&["--locale", "en_GB", "--ref", "2013-02-12T00:00:00Z", "3/4"]);
    // US: month/day (March 4); GB: day/month (April 3).
    assert!(
        us[0]["value"]["value"]
            .as_str()
            .unwrap()
            .starts_with("2013-03-04")
    );
    assert!(
        gb[0]["value"]["value"]
            .as_str()
            .unwrap()
            .starts_with("2013-04-03")
    );
}

#[test]
fn reads_stdin() {
    let (stdout, code) = run(&["--dims", "duration"], Some("half an hour"));
    assert_eq!(code, 0);
    let v: Vec<Value> = serde_json::from_str(&stdout).unwrap();
    assert_eq!(v[0]["value"]["value"], 30);
}

#[test]
fn help_exits_zero() {
    let (stdout, code) = run(&["--help"], None);
    assert_eq!(code, 0);
    assert!(stdout.contains("duckling"));
}

#[test]
fn unknown_dims_errors() {
    let (_stdout, code) = run(&["--dims", "bogus", "tomorrow"], None);
    assert_eq!(code, 2, "bad --dims should exit 2");
}

#[test]
fn bad_timezone_errors() {
    let (_stdout, code) = run(&["--tz", "Not/AZone", "tomorrow"], None);
    assert_eq!(code, 2, "bad --tz should exit 2");
}

#[test]
fn batch_mode_ndjson() {
    // One input per line -> one JSON array per line, order preserved; empty
    // input -> empty array.
    let (stdout, code) = run(
        &["--dims", "time", "--ref", "2013-02-12T04:30:00Z", "--batch"],
        Some("tomorrow at 5pm\nno time in this line\nnext friday\n"),
    );
    assert_eq!(code, 0);
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3, "one output line per input line: {stdout}");
    let l0: Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(l0[0]["dim"], "time");
    let l1: Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(l1.as_array().unwrap().len(), 0, "no time -> []");
    let l2: Value = serde_json::from_str(lines[2]).unwrap();
    assert_eq!(l2[0]["dim"], "time");
}

#[test]
fn latent_flag_surfaces_bare_year() {
    // A bare year is latent: dropped by default, surfaced with --latent.
    let (def, _) = run(
        &["--dims", "time", "--ref", "2013-02-12T04:30:00Z", "2001"],
        None,
    );
    assert_eq!(
        serde_json::from_str::<Value>(&def)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
    let (lat, _) = run(
        &[
            "--dims",
            "time",
            "--ref",
            "2013-02-12T04:30:00Z",
            "--latent",
            "2001",
        ],
        None,
    );
    let v: Value = serde_json::from_str(&lat).unwrap();
    assert!(
        v[0]["value"]["value"]
            .as_str()
            .unwrap_or("")
            .starts_with("2001"),
        "expected a 2001 time with --latent, got {lat}"
    );
}

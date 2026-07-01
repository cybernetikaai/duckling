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
    let v = parse(&[
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

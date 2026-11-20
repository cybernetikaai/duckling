use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};

fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_duckling")
}

/// One warm process, two lines with different tz/ref: answers must differ
/// accordingly and each must arrive before the next line is written (flush).
#[test]
fn batch_json_resolves_per_line_context_and_flushes() {
    let mut child = Command::new(bin())
        .args(["--dims", "time", "--batch-json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn duckling");
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();

    writeln!(
        stdin,
        r#"{{"text":"tomorrow at 5pm","ref":"2013-02-12T04:30:00Z","tz":"America/New_York"}}"#
    )
    .unwrap();
    let l1 = lines.next().unwrap().unwrap(); // blocks: only passes if flushed per line
    assert!(
        l1.contains("2013-02-12T17:00"),
        "NY: ref is 2013-02-11T23:30 local, tomorrow = Feb 12: {l1}"
    );

    writeln!(
        stdin,
        r#"{{"text":"tomorrow at 5pm","ref":"2013-02-12T04:30:00Z","tz":"Asia/Tokyo"}}"#
    )
    .unwrap();
    let l2 = lines.next().unwrap().unwrap();
    assert!(
        l2.contains("2013-02-13T17:00"),
        "Tokyo tomorrow (already Feb 12 local): {l2}"
    );
    assert_ne!(l1, l2, "different tz must change resolution");

    drop(stdin);
    child.wait().unwrap();
}

/// Invalid JSON line -> empty array line, process keeps serving.
#[test]
fn batch_json_bad_line_yields_empty_array_and_continues() {
    let mut child = Command::new(bin())
        .args(["--dims", "time", "--batch-json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    writeln!(stdin, "not json").unwrap();
    assert_eq!(lines.next().unwrap().unwrap(), "[]");
    writeln!(
        stdin,
        r#"{{"text":"tomorrow","ref":"2013-02-12T04:30:00Z","tz":"UTC"}}"#
    )
    .unwrap();
    assert!(lines.next().unwrap().unwrap().contains("2013-02-13"));
    drop(stdin);
    child.wait().unwrap();
}

/// Per-line `locale`: day-first locales read 03/02 as 3 Feb, interleaved with
/// month-first lines in one process; hyphen spelling, absent and unknown
/// locales fall back as documented, one reply per line.
#[test]
fn batch_json_per_line_locale() {
    // `all` is what ner drives; it must honour the locale too.
    for dims in ["time", "all"] {
        batch_json_per_line_locale_for(dims);
    }
}

fn batch_json_per_line_locale_for(dims: &str) {
    let mut child = Command::new(bin())
        .args(["--dims", dims, "--batch-json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    for (locale, want) in [
        (r#","locale":"en_AU""#, "1985-02-03"),
        ("", "1985-03-02"),
        (r#","locale":"en-GB""#, "1985-02-03"),
        (r#","locale":"en_US""#, "1985-03-02"),
        (r#","locale":"xx_YY""#, "1985-03-02"),
        (r#","locale":"en-gb""#, "1985-02-03"),
        (r#","locale":"EN_AU""#, "1985-02-03"),
        (r#","locale":" en-AU-x-foo ""#, "1985-02-03"),
        (r#","locale":"en_GB.UTF-8""#, "1985-02-03"),
        (r#","locale":"en""#, "1985-03-02"),
        (r#","locale":null"#, "1985-03-02"),
        (r#","locale":5"#, "1985-03-02"),
    ] {
        writeln!(stdin, r#"{{"text":"03/02/1985"{locale}}}"#).unwrap();
        let got = lines.next().unwrap().unwrap();
        assert!(
            got.contains(want),
            "{dims} {locale:?}: want {want}, got {got}"
        );
    }
    drop(stdin);
    child.wait().unwrap();
}

/// Fallback chain: line `locale` > process `--locale` > en_US.
#[test]
fn batch_json_line_locale_falls_back_to_process_locale() {
    let mut child = Command::new(bin())
        .args(["--dims", "all", "--batch-json", "--locale", "en_GB"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    for (locale, want) in [
        ("", "1985-02-03"),                     // missing: process en_GB
        (r#","locale":"en-US""#, "1985-03-02"), // line wins
        (r#","locale":"xx_YY""#, "1985-02-03"), // unknown: process en_GB
    ] {
        writeln!(stdin, r#"{{"text":"03/02/1985"{locale}}}"#).unwrap();
        let got = lines.next().unwrap().unwrap();
        assert!(got.contains(want), "{locale:?}: want {want}, got {got}");
    }
    drop(stdin);
    child.wait().unwrap();
}

/// Space-separated numeric dates (spoken DOBs) follow the locale's field order;
/// phone numbers and number lists stay out of the time dimension.
#[test]
fn batch_json_spaced_numeric_dates() {
    let mut child = Command::new(bin())
        .args(["--dims", "all", "--batch-json"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    let mut lines = BufReader::new(child.stdout.take().unwrap()).lines();
    for (text, locale, want) in [
        ("26 10 1982", "en-AU", Some("1982-10-26")),
        ("16 2 1985", "en-AU", Some("1985-02-16")),
        ("3 4 1985", "en-US", Some("1985-03-04")),
        ("3 4 1985", "en-GB", Some("1985-04-03")),
        ("born 26 10 1982 thanks", "en-GB", Some("1982-10-26")),
        ("12 31 1985", "en-US", Some("1985-12-31")),
        ("26 10 1982", "en-US", None), // month 26: not a US date
        ("12 31 1985", "en-AU", None), // month 31: not an AU date
        ("00 12 1985", "en-AU", None), // day 0
        ("call 04 12 2034 5678", "en-AU", None), // inside a longer digit run
        ("0412 345 678", "en-AU", None),
        ("rooms 3 4 and 12", "en-AU", None),
        ("3 4 5", "en-AU", None), // no 19xx/20xx year
    ] {
        writeln!(stdin, r#"{{"text":"{text}","locale":"{locale}"}}"#).unwrap();
        let got = lines.next().unwrap().unwrap();
        let entities: serde_json::Value = serde_json::from_str(&got).unwrap();
        let time: Vec<&str> = entities
            .as_array()
            .unwrap()
            .iter()
            .filter(|e| e["dim"] == "time")
            .filter_map(|e| e["value"]["value"].as_str())
            .collect();
        match want {
            Some(day) => assert!(
                time.contains(&day),
                "{text:?} {locale}: want {day}, got {got}"
            ),
            None => assert!(
                time.is_empty(),
                "{text:?} {locale}: want no date, got {got}"
            ),
        }
    }
    drop(stdin);
    child.wait().unwrap();
}

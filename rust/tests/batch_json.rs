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

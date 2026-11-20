//! A long unbroken `[\w_+-]` run (a tracking-link token, say) once made the
//! email regex exceed fancy-regex's backtrack limit; fancy-regex 0.13 then
//! retried the same position forever, wedging the process.

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

fn emails_within(input: String, secs: u64) -> Vec<String> {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        let got = duckling::parse_email(&input)
            .into_iter()
            .filter_map(|e| e.value["value"].as_str().map(str::to_string))
            .collect::<Vec<_>>();
        let _ = tx.send(got);
    });
    rx.recv_timeout(Duration::from_secs(secs))
        .expect("parse_email did not return: backtrack-limit hang")
}

#[test]
fn a_long_word_run_does_not_hang() {
    for unit in ["a", "7", "-", "_", "aB3_-x9Q"] {
        assert!(
            emails_within(unit.repeat(5000), 10).is_empty(),
            "unit {unit:?}"
        );
    }
}

#[test]
fn an_email_after_a_long_token_is_still_found() {
    let token = "aB3_-x9Q".repeat(120);
    let input = format!("Click https://t.example.com/c/{token} then email bob@example.com");
    assert_eq!(emails_within(input, 10), vec!["bob@example.com"]);
}

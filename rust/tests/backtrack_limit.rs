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

fn ctx() -> duckling::ResolveContext {
    let zone = jiff::tz::TimeZone::fixed(jiff::tz::Offset::constant(-4));
    let reference = jiff::civil::date(2026, 10, 13)
        .at(9, 0, 0, 0)
        .to_zoned(zone.clone())
        .unwrap()
        .timestamp();
    duckling::ResolveContext {
        reference,
        zone,
        with_latent: false,
        locale: Default::default(),
    }
}

#[test]
fn a_quarantined_link_is_one_url_over_the_whole_run_and_its_neighbours_keep_their_offsets() {
    let link = format!("https://t.example.com/c/{}", "aB3_-x9Q".repeat(60));
    let input = format!("mail bob@example.com {link} Monday at 3pm");
    let spans = |got: Vec<duckling::Entity>| {
        got.into_iter()
            .map(|e| (e.body, e.start, e.end))
            .collect::<Vec<_>>()
    };
    let (link_at, link_end) = (21, 21 + link.len());
    let time_at = input.chars().count() - "Monday at 3pm".len();
    assert_eq!(
        spans(duckling::parse_email(&input)),
        vec![("bob@example.com".to_string(), 5, 20)]
    );
    let urls = spans(duckling::parse_url(&input));
    assert!(
        urls.contains(&(link.clone(), link_at, link_end)),
        "the run is one url hint: {urls:?}"
    );
    let times = spans(duckling::parse(&input, &ctx()));
    assert!(
        times
            .iter()
            .all(|(_, start, end)| *end <= link_at || *start >= link_end),
        "a time overlaps the quarantined run: {times:?}"
    );
    assert!(times.contains(&("Monday at 3pm".to_string(), time_at, input.chars().count())));
}

#[test]
fn a_link_whose_host_passes_the_sample_is_still_one_url_over_the_run() {
    let link = format!(
        "https://login.bank.com.{}.evil.com/c/{}",
        "a".repeat(150),
        "aB3_-x9Q".repeat(60)
    );
    let urls = duckling::parse_url(&format!("see {link} today"));
    assert!(
        urls.iter()
            .any(|e| (e.start, e.end, e.body.as_str()) == (4, 4 + link.len(), link.as_str())),
        "{:?}",
        urls.iter().map(|e| (e.start, e.end)).collect::<Vec<_>>()
    );
}

#[test]
fn any_dimension_can_hint_a_quarantined_run() {
    let digits = format!("3.{}", "1415926535".repeat(50));
    let input = format!("pi is {digits} roughly");
    let numbers = duckling::parse_numeral(&input);
    assert!(
        numbers
            .iter()
            .any(|e| (e.start, e.end) == (6, 6 + digits.len())),
        "{:?}",
        numbers.iter().map(|e| (e.start, e.end)).collect::<Vec<_>>()
    );
}

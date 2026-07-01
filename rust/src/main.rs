//! Command-line interface for the duckling time parser.
//!
//! Parses English natural-language time / duration / ordinal expressions and
//! prints the resolved entities as JSON (Duckling's entity shape). The `--tz`
//! flag sets the target IANA zone that relative expressions resolve in and that
//! output offsets are derived from — this is how you "coerce into a target
//! timezone": set it here at parse time rather than converting the result after.
//!
//! Usage:
//!   duckling [OPTIONS] "text to parse"
//!   echo "text" | duckling [OPTIONS]
//!
//! Options:
//!   --dims <time|duration|ordinal|all>  dimension(s) to extract (default: time)
//!   --ref  <RFC3339>                    reference "now" instant (default: system now)
//!   --tz   <IANA zone>                  target timezone, e.g. America/New_York (default: UTC)
//!   --locale <en_US|en_GB|en_CA|en_AU|en_NZ|en_IN|en_IE|en_ZA|en_PH|en_BZ|en_JM|en_TT>
//!                                        English locale (affects numeric date order; default: en_US)
//!   -h, --help                          print this help

use duckling::{parse_all, parse_duration, parse_locale, parse_ordinal, Locale, ResolveContext};

const HELP: &str = "\
duckling — English time / duration / ordinal parser

USAGE:
    duckling [OPTIONS] \"text to parse\"
    echo \"text\" | duckling [OPTIONS]

OPTIONS:
    --dims <D>      time | duration | ordinal | all       (default: time)
    --ref <TS>      reference instant, RFC 3339            (default: system now)
                    e.g. 2013-02-12T04:30:00-02:00
    --tz <ZONE>     target IANA timezone                   (default: UTC)
                    e.g. America/New_York, Europe/London
    --locale <L>    en_US en_GB en_CA en_AU en_NZ en_IN
                    en_IE en_ZA en_PH en_BZ en_JM en_TT    (default: en_US)
    -h, --help      print this help

OUTPUT:
    A JSON array of entities: {dim, body, start, end, value, latent}.

EXAMPLES:
    duckling \"tomorrow at 5pm\"
    duckling --tz America/New_York --ref 2013-02-12T04:30:00Z \"in 2 hours\"
    duckling --dims all \"set a timer for 20 minutes and wake me at 7am\"
    duckling --dims duration \"an hour and a half\"
    duckling --locale en_GB \"13/12/2013\"";

fn locale_from(name: &str) -> Option<Locale> {
    Some(match name {
        "en_US" => Locale::EnUs,
        "en_GB" => Locale::EnGb,
        "en_CA" => Locale::EnCa,
        "en_AU" => Locale::EnAu,
        "en_NZ" => Locale::EnNz,
        "en_IN" => Locale::EnIn,
        "en_IE" => Locale::EnIe,
        "en_ZA" => Locale::EnZa,
        "en_PH" => Locale::EnPh,
        "en_BZ" => Locale::EnBz,
        "en_JM" => Locale::EnJm,
        "en_TT" => Locale::EnTt,
        _ => return None,
    })
}

fn fail(msg: &str) -> ! {
    eprintln!("error: {msg}");
    eprintln!("try `duckling --help`");
    std::process::exit(2);
}

fn main() {
    let mut dims = String::from("time");
    let mut ref_str: Option<String> = None;
    let mut tz_str = String::from("UTC");
    let mut locale_str = String::from("en_US");
    let mut words: Vec<String> = Vec::new();

    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                return;
            }
            "--dims" => dims = it.next().unwrap_or_else(|| fail("--dims needs a value")),
            "--ref" => ref_str = Some(it.next().unwrap_or_else(|| fail("--ref needs a value"))),
            "--tz" => tz_str = it.next().unwrap_or_else(|| fail("--tz needs a value")),
            "--locale" => locale_str = it.next().unwrap_or_else(|| fail("--locale needs a value")),
            other => words.push(other.to_string()),
        }
    }

    // Text from positional args, else from stdin.
    let text = if words.is_empty() {
        use std::io::Read;
        let mut s = String::new();
        std::io::stdin().read_to_string(&mut s).ok();
        s.trim().to_string()
    } else {
        words.join(" ")
    };
    if text.is_empty() {
        println!("{HELP}");
        std::process::exit(2);
    }

    let reference: jiff::Timestamp = match &ref_str {
        Some(s) => s.parse().unwrap_or_else(|e| fail(&format!("bad --ref {s:?}: {e}"))),
        None => jiff::Timestamp::now(),
    };
    let zone = jiff::tz::TimeZone::get(&tz_str)
        .unwrap_or_else(|e| fail(&format!("bad --tz {tz_str:?}: {e}")));
    let locale = locale_from(&locale_str).unwrap_or_else(|| fail(&format!("unknown --locale {locale_str:?}")));
    let ctx = ResolveContext { reference, zone, with_latent: false };

    let entities = match dims.as_str() {
        "time" => parse_locale(&text, &ctx, locale),
        "duration" => parse_duration(&text),
        "ordinal" => parse_ordinal(&text),
        "all" => parse_all(&text, &ctx),
        other => fail(&format!("unknown --dims {other:?} (want time|duration|ordinal|all)")),
    };

    match serde_json::to_string_pretty(&entities) {
        Ok(json) => println!("{json}"),
        Err(e) => fail(&format!("serialization: {e}")),
    }
}

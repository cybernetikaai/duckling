#![forbid(unsafe_code)]
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
//!   --dims <D>  dimension: time | duration | ordinal | number | email | url |
//!               credit-card-number | phone-number | temperature | volume | distance | quantity |
//!               amount-of-money | all   (default: time)
//!   --ref  <RFC3339>                    reference "now" instant (default: system now)
//!   --tz   <IANA zone>                  target timezone, e.g. America/New_York (default: UTC)
//!   --locale <en_US|en_GB|en_CA|en_AU|en_NZ|en_IN|en_IE|en_ZA|en_PH|en_BZ|en_JM|en_TT>
//!                                        English locale (affects numeric date order; default: en_US)
//!   -h, --help                          print this help

use duckling::{
    Locale, ResolveContext, parse_all, parse_creditcard, parse_distance, parse_duration,
    parse_email, parse_locale, parse_numeral, parse_ordinal, parse_phonenumber, parse_temperature,
    parse_url, parse_volume,
};

const HELP: &str = "\
duckling — English parser: time, duration, ordinal, number, email, url, credit
card, phone number

USAGE:
    duckling [OPTIONS] \"text to parse\"
    echo \"text\" | duckling [OPTIONS]

OPTIONS:
    --dims <D>      time | duration | ordinal | number | email | url |
                    credit-card-number | phone-number | temperature | volume | distance | quantity |
                    amount-of-money | all   (default: time)
    --ref <TS>      reference instant, RFC 3339            (default: system now)
                    e.g. 2013-02-12T04:30:00-02:00
    --tz <ZONE>     target IANA timezone                   (default: UTC)
                    e.g. America/New_York, Europe/London
    --locale <L>    en_US en_GB en_CA en_AU en_NZ en_IN
                    en_IE en_ZA en_PH en_BZ en_JM en_TT    (default: en_US)
    --latent        surface latent parses (e.g. a bare year \"2001\" as a time)
    --batch         read one input per stdin line; print one compact JSON array
                    per line (NDJSON), for bulk corpora
    -h, --help      print this help

OUTPUT:
    A JSON array of entities: {dim, body, start, end, value, latent}.
    (--batch: one such array per line, in input order.)

EXAMPLES:
    duckling \"tomorrow at 5pm\"
    duckling --tz America/New_York --ref 2013-02-12T04:30:00Z \"in 2 hours\"
    duckling --dims all \"set a timer for 20 minutes and wake me at 7am\"
    duckling --dims duration \"an hour and a half\"
    duckling --dims email \"ping me at a dot b at x dot com\"
    duckling --dims phone-number \"call +1 (650) 123-4567\"
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

/// Run one dimension over one text (the `--dims` dispatch). `with_latent` comes
/// from `ctx.with_latent`; it also toggles the latent variants of the dimensions
/// that have one (quantity, amount-of-money).
fn parse_dims(
    dims: &str,
    text: &str,
    ctx: &ResolveContext,
    locale: Locale,
) -> Vec<duckling::Entity> {
    use duckling::{parse_amountofmoney_opts, parse_quantity_opts};
    let lat = ctx.with_latent;
    match dims {
        "time" => parse_locale(text, ctx, locale),
        "duration" => parse_duration(text),
        "ordinal" => parse_ordinal(text),
        "number" => parse_numeral(text),
        "email" => parse_email(text),
        "url" => parse_url(text),
        "credit-card-number" => parse_creditcard(text),
        "phone-number" => parse_phonenumber(text),
        "temperature" => parse_temperature(text),
        "volume" => parse_volume(text),
        "distance" => parse_distance(text),
        "quantity" => parse_quantity_opts(text, lat),
        "amount-of-money" => parse_amountofmoney_opts(text, lat),
        "all" => parse_all(text, ctx),
        other => fail(&format!(
            "unknown --dims {other:?} (want time|duration|ordinal|number|email|url|\
             credit-card-number|phone-number|temperature|volume|distance|quantity|\
             amount-of-money|all)"
        )),
    }
}

fn main() {
    let mut dims = String::from("time");
    let mut ref_str: Option<String> = None;
    let mut tz_str = String::from("UTC");
    let mut locale_str = String::from("en_US");
    let mut batch = false;
    let mut with_latent = false;
    let mut words: Vec<String> = Vec::new();

    let mut it = std::env::args().skip(1);
    while let Some(a) = it.next() {
        match a.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                return;
            }
            "--batch" => batch = true,
            "--latent" => with_latent = true,
            "--dims" => dims = it.next().unwrap_or_else(|| fail("--dims needs a value")),
            "--ref" => ref_str = Some(it.next().unwrap_or_else(|| fail("--ref needs a value"))),
            "--tz" => tz_str = it.next().unwrap_or_else(|| fail("--tz needs a value")),
            "--locale" => locale_str = it.next().unwrap_or_else(|| fail("--locale needs a value")),
            other => words.push(other.to_string()),
        }
    }

    let reference: jiff::Timestamp = match &ref_str {
        Some(s) => s
            .parse()
            .unwrap_or_else(|e| fail(&format!("bad --ref {s:?}: {e}"))),
        None => jiff::Timestamp::now(),
    };
    let zone = jiff::tz::TimeZone::get(&tz_str)
        .unwrap_or_else(|e| fail(&format!("bad --tz {tz_str:?}: {e}")));
    let locale = locale_from(&locale_str)
        .unwrap_or_else(|| fail(&format!("unknown --locale {locale_str:?}")));
    let ctx = ResolveContext {
        reference,
        zone,
        with_latent,
    };

    // Batch mode: one input text per stdin line -> one compact JSON array per
    // line (NDJSON), preserving order. For bulk corpora / evaluation harnesses.
    if batch {
        use std::io::{BufRead, Write};
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut out = std::io::BufWriter::new(stdout.lock());
        for line in stdin.lock().lines() {
            let line = line.unwrap_or_default();
            let entities = parse_dims(&dims, line.trim(), &ctx, locale);
            let json = serde_json::to_string(&entities).unwrap_or_else(|_| "[]".to_string());
            writeln!(out, "{json}").ok();
        }
        return;
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

    let entities = parse_dims(&dims, &text, &ctx, locale);

    match serde_json::to_string_pretty(&entities) {
        Ok(json) => println!("{json}"),
        Err(e) => fail(&format!("serialization: {e}")),
    }
}

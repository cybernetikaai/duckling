use duckling::{parse_all_debug, ResolveContext};
fn ctx() -> ResolveContext {
    let zone = jiff::tz::TimeZone::fixed(jiff::tz::Offset::constant(-2));
    let reference = jiff::civil::date(2013,2,12).at(4,30,0,0).to_zoned(zone.clone()).unwrap().timestamp();
    ResolveContext { reference, zone, with_latent: false }
}
fn dump(inp:&str){ eprintln!("### {inp}"); for l in parse_all_debug(inp,&ctx()){eprintln!("  {l}");} }
#[test]
fn d(){ for i in ["at 3am","upcoming week","upcoming 2 weeks","last weekend of October"]{ dump(i);} }

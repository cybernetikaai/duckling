//! Token / Dimension / Node / Rule / Pattern types.
//!
//! Duckling's existential GADT `Token` collapses to a Rust enum. More arms
//! (Numeral, Ordinal, Duration, TimeGrain) are added as dependencies land.

use crate::duration::DurationData;
use crate::email::EmailData;
use crate::grain::Grain;
use crate::numeral::NumeralData;
use crate::ordinal::OrdinalData;
use crate::regex::Re;
use crate::time::object::IntervalDirection;
use crate::time::predicate::Predicate;

/// English locale variant. Base EN is region-neutral; regions differ in numeric-
/// date field order — US reads "3/4" as month/day (March 4), GB as day/month
/// (April 3) — and in region-specific holidays. Everything else (named-month
/// dates, ISO, times) is shared. Duckling ships these English regions.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash, Default)]
pub enum Locale {
    #[default]
    EnUs,
    EnGb,
    EnCa,
    EnAu,
    EnNz,
    EnIn,
    EnIe,
    EnZa,
    EnPh,
    EnBz,
    EnJm,
    EnTt,
}

/// How a region orders numeric dates. Three patterns cover all English regions:
/// month-first (US/CA/PH), day-first (GB/AU/NZ/IN/IE/BZ/JM/TT), and ZA's hybrid —
/// no-year forms month-first ("3/4"→Mar 4) but with-year forms day-first
/// ("3/4/2015"→Apr 3), per Duckling/Time/EN/ZA/Rules.hs.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DateConvention {
    MonthFirst,
    DayFirst,
    ZaHybrid,
}

impl Locale {
    pub fn date_convention(self) -> DateConvention {
        use Locale::*;
        match self {
            EnUs | EnCa | EnPh => DateConvention::MonthFirst,
            EnZa => DateConvention::ZaHybrid,
            EnGb | EnAu | EnNz | EnIn | EnIe | EnBz | EnJm | EnTt => DateConvention::DayFirst,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    DayOfWeek,
    TimeOfDay {
        hours: Option<i8>,
        minutes: Option<i8>,
        is12h: bool,
    },
    Month {
        month: i8,
    },
    /// A part of day; `start_hour` is the interval's opening hour, used to
    /// disambiguate am/pm for "<part-of-day> at <time-of-day>".
    PartOfDay {
        start_hour: i8,
    },
    Season,
}

#[derive(Clone)]
pub struct TimeData {
    pub pred: Predicate,
    pub grain: Grain,
    pub latent: bool,
    /// When the first future occurrence covers "now", skip to the next one
    /// (e.g. "tuesday" on a Tuesday means *next* Tuesday). Port of notImmediate.
    pub not_immediate: bool,
    pub form: Option<Form>,
    pub direction: Option<IntervalDirection>,
    pub holiday: Option<String>,
    /// True once a timezone has been applied (port of Duckling's hasTimezone),
    /// so an interval-timezone rule won't double-apply to an already-tz'd end.
    pub has_timezone: bool,
}

impl TimeData {
    pub fn new(pred: Predicate, grain: Grain) -> Self {
        TimeData {
            pred,
            grain,
            latent: false,
            not_immediate: false,
            form: None,
            direction: None,
            holiday: None,
            has_timezone: false,
        }
    }
}

#[derive(Clone)]
pub enum Token {
    RegexMatch(Vec<String>),
    Time(TimeData),
    Numeral(NumeralData),
    Ordinal(OrdinalData),
    TimeGrain(Grain),
    Duration(DurationData),
    Email(EmailData),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Range(pub usize, pub usize);

#[derive(Clone)]
pub struct Node {
    pub range: Range,
    pub token: Token,
    pub rule: Option<String>,
    /// The matched route (child nodes), for ranking feature extraction.
    pub children: Vec<Node>,
}

/// A rule production: turn the matched route's tokens into a new token.
pub type Production = Box<dyn Fn(&[Token]) -> Option<Token>>;

pub enum PatternItem {
    Regex(Re),
    Predicate(Box<dyn Fn(&Token) -> bool>),
}

pub struct Rule {
    pub name: String,
    pub pattern: Vec<PatternItem>,
    pub prod: Production,
}

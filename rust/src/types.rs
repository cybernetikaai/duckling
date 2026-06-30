//! Token / Dimension / Node / Rule / Pattern types.
//!
//! Duckling's existential GADT `Token` collapses to a Rust enum. More arms
//! (Numeral, Ordinal, Duration, TimeGrain) are added as dependencies land.

use crate::grain::Grain;
use crate::regex::Re;
use crate::time::object::IntervalDirection;
use crate::time::predicate::Predicate;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Form {
    DayOfWeek,
    TimeOfDay { hours: Option<i8>, is12h: bool },
    Month { month: i8 },
    PartOfDay,
}

#[derive(Clone)]
pub struct TimeData {
    pub pred: Predicate,
    pub grain: Grain,
    pub latent: bool,
    pub form: Option<Form>,
    pub direction: Option<IntervalDirection>,
    pub holiday: Option<String>,
}

impl TimeData {
    pub fn new(pred: Predicate, grain: Grain) -> Self {
        TimeData { pred, grain, latent: false, form: None, direction: None, holiday: None }
    }
}

#[derive(Clone)]
pub enum Token {
    RegexMatch(Vec<String>),
    Time(TimeData),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Range(pub usize, pub usize);

#[derive(Clone)]
pub struct Node {
    pub range: Range,
    pub token: Token,
    pub rule: Option<String>,
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

//! fancy-regex wrapper: case-insensitive (Haskell uses compCaseless), returns
//! char-indexed matches with capture groups.

use crate::document::Document;
use fancy_regex::Regex;

pub struct Re(Regex);

pub struct RegexHit {
    pub start: usize,
    pub end: usize,
    pub groups: Vec<String>,
}

/// Compile case-insensitively via the inline `(?i)` flag (fancy-regex supports
/// PCRE-style lookaround that the stdlib `regex` crate rejects).
pub fn compile(pat: &str) -> Re {
    Re(Regex::new(&format!("(?i){pat}")).expect("valid regex"))
}

impl Re {
    /// All matches anywhere in the document, char-indexed.
    pub fn all_hits(&self, doc: &Document) -> Vec<RegexHit> {
        let text = doc.text();
        let mut out = Vec::new();
        for cap in self.0.captures_iter(text).flatten() {
            let whole = cap.get(0).unwrap();
            let groups = (1..cap.len())
                .map(|i| cap.get(i).map(|m| m.as_str().to_string()).unwrap_or_default())
                .collect();
            out.push(RegexHit {
                start: doc.char_idx_of_byte(whole.start()),
                end: doc.char_idx_of_byte(whole.end()),
                groups,
            });
        }
        out
    }
}

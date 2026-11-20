//! Input text + adjacency rules for matching.
//!
//! Char-indexed (codepoints), matching Duckling's range semantics. Two tokens
//! are "adjacent" if only separator (whitespace) characters sit between them.

pub struct Document {
    text: String,
    chars: Vec<char>,
    raw: Vec<char>,
    masked_runs: Vec<(usize, usize)>,
}

/// A non-whitespace run this long (a link, an encoded blob) is quarantined:
/// rules see only its first `SAMPLE_CHARS`, and an entity found there is
/// stretched over the whole run as a hint of what it holds.
pub const MAX_RUN_CHARS: usize = 400;
/// Enough for a content hint; too short to strain any regex.
pub const SAMPLE_CHARS: usize = 128;
/// Neither a word char nor whitespace, so word, digit and spacing rules stop at
/// it and composition does not bridge it; each char keeps its offset.
pub const MASK: char = '\u{FFFC}';

fn mask_long_runs(raw: &[char]) -> (Vec<char>, Vec<(usize, usize)>) {
    let mut chars = raw.to_vec();
    let mut runs = Vec::new();
    let mut start = 0;
    for i in 0..=chars.len() {
        if i == chars.len() || chars[i].is_whitespace() {
            if i - start >= MAX_RUN_CHARS {
                chars[start + SAMPLE_CHARS..i].fill(MASK);
                runs.push((start, i));
            }
            start = i + 1;
        }
    }
    (chars, runs)
}

impl Document {
    pub fn new(s: &str) -> Self {
        let raw: Vec<char> = s.chars().collect();
        let (chars, masked_runs) = mask_long_runs(&raw);
        Document {
            text: chars.iter().collect(),
            chars,
            raw,
            masked_runs,
        }
    }

    /// `range` widened over every quarantined run it overlaps.
    pub fn stretch(&self, (mut start, mut end): (usize, usize)) -> (usize, usize) {
        for &(run_start, run_end) in &self.masked_runs {
            if start < run_end && end > run_start {
                start = start.min(run_start);
                end = end.max(run_end);
            }
        }
        (start, end)
    }

    /// The input as given (unmasked) over `(start, end)`.
    pub fn real_text(&self, (start, end): (usize, usize)) -> String {
        self.raw[start..end].iter().collect()
    }

    pub fn len(&self) -> usize {
        self.chars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.chars.is_empty()
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn substring(&self, start: usize, end: usize) -> String {
        self.chars[start..end].iter().collect()
    }

    /// Whether a regex match may begin or end at char boundary `i` — i.e. `i` is
    /// not in the middle of a run of same-class characters. Duckling forbids a
    /// rule match from splitting a maximal run of letters (or of digits): "mon"
    /// must not match inside "monkey" (letter|letter), while "pm" may match after
    /// "3" in "3pm" (digit|letter is a class change, so a boundary exists).
    pub fn is_match_boundary(&self, i: usize) -> bool {
        if i == 0 || i >= self.chars.len() {
            return true;
        }
        let (a, b) = (self.chars[i - 1], self.chars[i]);
        !((a.is_alphabetic() && b.is_alphabetic()) || (a.is_numeric() && b.is_numeric()))
    }

    /// True if the gap between `prev_end` and `next_start` is separators only.
    pub fn is_adjacent(&self, prev_end: usize, next_start: usize) -> bool {
        next_start >= prev_end
            && next_start <= self.chars.len()
            && self.chars[prev_end..next_start]
                .iter()
                .all(|c| c.is_whitespace())
    }

    /// Translate a byte offset (from regex matches on `text`) to a char index.
    pub fn char_idx_of_byte(&self, byte: usize) -> usize {
        self.text[..byte].chars().count()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn adjacency_skips_whitespace() {
        let d = Document::new("on  monday");
        assert!(d.is_adjacent(2, 4)); // "on" + two spaces
    }
    #[test]
    fn adjacency_rejects_letters_between() {
        let d = Document::new("onXmonday");
        assert!(!d.is_adjacent(2, 3)); // 'X' is not a separator
    }
    #[test]
    fn a_run_at_the_cap_keeps_its_sample_and_masks_the_rest() {
        let run = "x".repeat(MAX_RUN_CHARS);
        let d = Document::new(&format!("at {run} 3pm"));
        assert_eq!(d.len(), 3 + MAX_RUN_CHARS + 4);
        assert_eq!(
            d.substring(0, 3 + SAMPLE_CHARS),
            format!("at {}", "x".repeat(SAMPLE_CHARS))
        );
        assert!(
            d.substring(3 + SAMPLE_CHARS, 3 + MAX_RUN_CHARS)
                .chars()
                .all(|c| c == MASK)
        );
        assert_eq!(d.substring(4 + MAX_RUN_CHARS, d.len()), "3pm");
    }
    #[test]
    fn a_run_under_the_cap_is_kept() {
        let run = "é".repeat(MAX_RUN_CHARS - 1);
        assert_eq!(Document::new(&run).text(), run);
    }
}

//! Input text + adjacency rules for matching.
//!
//! Char-indexed (codepoints), matching Duckling's range semantics. Two tokens
//! are "adjacent" if only separator (whitespace) characters sit between them.

pub struct Document {
    text: String,
    chars: Vec<char>,
}

impl Document {
    pub fn new(s: &str) -> Self {
        Document {
            text: s.to_string(),
            chars: s.chars().collect(),
        }
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
}

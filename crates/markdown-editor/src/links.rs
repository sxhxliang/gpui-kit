//! Wiki-links `[[target]]` / `[[target|alias]]`: parsing (pure) and
//! retargeting edits for renames. Pure — no gpui — so it is testable
//! headless.
//!
//! The vault-backed helpers (resolution against the note tree, completion
//! stems, backlink lookup, on-disk relinking) live in the application
//! layer; only the text-level parsing and edit computation are kept here.

use std::ops::Range;

pub struct WikiLink {
    /// Whole `[[…]]` span.
    pub range: Range<usize>,
    /// Trimmed target text inside the brackets.
    pub target: Range<usize>,
    /// Trimmed alias after `|`, when present.
    pub alias: Option<Range<usize>>,
}

/// `[[target]]` and `[[target|alias]]`; no newline inside, target trimmed
/// non-empty. Positions where `skip(offset)` is true (code spans/blocks) are
/// not link starts.
pub(crate) fn parse(text: &str, skip: impl Fn(usize) -> bool) -> Vec<WikiLink> {
    let bytes = text.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i + 1 < bytes.len() {
        if bytes[i] == b'[' && bytes[i + 1] == b'[' && !skip(i) {
            let mut j = i + 2;
            let mut bar = None;
            let mut end = None;
            while j < bytes.len() {
                match bytes[j] {
                    b'\n' => break,
                    b'|' if bar.is_none() => bar = Some(j),
                    b']' if bytes.get(j + 1) == Some(&b']') => {
                        end = Some(j);
                        break;
                    }
                    _ => {}
                }
                j += 1;
            }
            if let Some(close) = end {
                let (t_start, t_end) = trim_range(text, i + 2, bar.unwrap_or(close));
                if t_start < t_end {
                    let alias = bar.and_then(|b| {
                        let (a, z) = trim_range(text, b + 1, close);
                        (a < z).then_some(a..z)
                    });
                    out.push(WikiLink {
                        range: i..close + 2,
                        target: t_start..t_end,
                        alias,
                    });
                }
                i = close + 2;
                continue;
            }
        }
        i += 1;
    }
    out
}

fn trim_range(text: &str, mut a: usize, mut b: usize) -> (usize, usize) {
    let is_ws = |i: usize| text.as_bytes()[i].is_ascii_whitespace();
    while a < b && is_ws(a) {
        a += 1;
    }
    while b > a && is_ws(b - 1) {
        b -= 1;
    }
    (a, b)
}

/// Edits pointing each `[[old]]` / `[[old|alias]]` in `links` at `new`, last
/// first. `old` matches case-insensitively.
pub(crate) fn retarget(
    text: &str,
    links: &[WikiLink],
    old: &str,
    new: &str,
) -> Vec<(Range<usize>, String)> {
    let old = old.trim().to_lowercase();
    links
        .iter()
        .rev()
        .filter(|l| {
            let t = &text[l.target.clone()];
            t.to_lowercase() == old && t != new
        })
        .map(|l| (l.target.clone(), new.to_string()))
        .collect()
}

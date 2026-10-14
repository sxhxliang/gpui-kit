//! Attachments: pure path ↔ markdown helpers for images and files pasted
//! or dropped into a note.
//!
//! `Incoming` describes a pasted/dropped item; the editor surfaces such
//! items through its `Attach` event and the *host* owns storage. The
//! blocking import step from abstract (`import`, which wrote files to disk
//! via abstract's atomic file-write helper) is deliberately not ported here.

use std::ops::Range;
use std::path::{Component, Path, PathBuf};

use crate::md::ImageRef;

/// Folder next to the note that receives pasted/dropped images.
pub const DIR: &str = "attachments";

#[derive(Clone)]
pub enum Incoming {
    /// Raw image bytes from the clipboard; `ext` without the dot.
    Image { bytes: Vec<u8>, ext: &'static str },
    /// A file or folder dropped/pasted from the OS.
    Path(PathBuf),
}

/// `to` relative to the directory `from`, `/`-separated. Falls back to the
/// absolute path when they share no root (different Windows drives).
pub fn relative(from: &Path, to: &Path) -> String {
    let a: Vec<Component> = from.components().collect();
    let b: Vec<Component> = to.components().collect();
    let common = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
    if common == 0 {
        return to.to_string_lossy().replace('\\', "/");
    }
    let mut parts: Vec<String> = vec!["..".into(); a.len() - common];
    parts.extend(
        b[common..]
            .iter()
            .map(|c| c.as_os_str().to_string_lossy().into_owned()),
    );
    parts.join("/")
}

/// Percent-encode what would end or confuse a markdown link destination.
pub fn encode(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for c in path.chars() {
        match c {
            ' ' => out.push_str("%20"),
            '%' => out.push_str("%25"),
            '(' => out.push_str("%28"),
            ')' => out.push_str("%29"),
            '<' => out.push_str("%3C"),
            '>' => out.push_str("%3E"),
            '\\' => out.push('/'),
            c => out.push(c),
        }
    }
    out
}

/// Undo `<…>` wrapping and percent-encoding of a link destination.
pub fn decode(raw: &str) -> String {
    let raw = raw.trim();
    let raw = raw
        .strip_prefix('<')
        .and_then(|r| r.strip_suffix('>'))
        .unwrap_or(raw);
    let b = raw.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        let hex = |c: u8| (c as char).to_digit(16);
        if b[i] == b'%'
            && i + 2 < b.len()
            && let (Some(h), Some(l)) = (hex(b[i + 1]), hex(b[i + 2]))
        {
            out.push((h * 16 + l) as u8);
            i += 3;
            continue;
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| raw.to_string())
}

/// Files an image reference may point at, most likely first. Remote URLs
/// yield nothing (the editor stays offline).
pub fn candidates(raw: &str, embed: bool, note_dir: &Path, root: &Path) -> Vec<PathBuf> {
    let src = decode(raw);
    if src.contains("://") && !src.starts_with("file://") || src.starts_with("data:") {
        return Vec::new();
    }
    if let Some(rest) = src.strip_prefix("file://") {
        let rest = if rest.get(2..3) == Some(":") {
            rest.trim_start_matches('/')
        } else {
            rest
        };
        return vec![PathBuf::from(rest)];
    }
    let p = Path::new(&src);
    if p.is_absolute() {
        return vec![p.to_path_buf()];
    }
    let mut out = vec![note_dir.join(p)];
    if embed {
        out.push(note_dir.join(DIR).join(p));
        out.push(root.join(p));
        out.push(root.join(DIR).join(p));
    }
    out
}

/// Edits, last first, that keep relative `![](…)` sources pointing at the
/// same files once a note moves from `old_dir` to `new_dir`. `![[…]]` embeds
/// also resolve against the space root and are left alone.
pub fn rebase(
    text: &str,
    images: &[ImageRef],
    old_dir: &Path,
    new_dir: &Path,
) -> Vec<(Range<usize>, String)> {
    let mut out = Vec::new();
    for im in images.iter().rev().filter(|im| !im.embed) {
        let src = decode(&text[im.src.clone()]);
        if src.is_empty() || src.contains(':') || Path::new(&src).is_absolute() {
            continue;
        }
        let to = encode(&relative(new_dir, &normalize(&old_dir.join(&src))));
        if to != text[im.src.clone()] {
            out.push((im.src.clone(), to));
        }
    }
    out
}

/// `.` and `..` folded away lexically.
fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rebase_keeps_image_targets() {
        let text = "![a](attachments/a.png) ![b](../b%20c.png) ![w](https://x/y.png) ![[e.png]]";
        let images = crate::md::Analyzer::new().analyze(text).images;
        let mut t = text.to_string();
        for (r, s) in rebase(text, &images, Path::new("/v/n"), Path::new("/v/n/deep")) {
            t.replace_range(r, &s);
        }
        assert_eq!(
            t,
            "![a](../attachments/a.png) ![b](../../b%20c.png) ![w](https://x/y.png) ![[e.png]]"
        );
        assert!(rebase(text, &images, Path::new("/v/n"), Path::new("/v/n")).is_empty());
    }

    #[test]
    fn relative_paths() {
        let r = Path::new("/v");
        assert_eq!(
            relative(r, Path::new("/v/attachments/a.png")),
            "attachments/a.png"
        );
        assert_eq!(
            relative(Path::new("/v/x/y"), Path::new("/v/z/a.png")),
            "../../z/a.png"
        );
    }

    #[test]
    fn encode_decode_round_trip() {
        let s = "my pics/a (1)%.png";
        assert_eq!(encode(s), "my%20pics/a%20%281%29%25.png");
        assert_eq!(decode(&encode(s)), s);
        assert_eq!(decode("<a b.png>"), "a b.png");
        assert_eq!(decode("100%"), "100%");
    }

    #[test]
    fn candidates_for_sources() {
        let (n, r) = (Path::new("/v/notes"), Path::new("/v"));
        assert_eq!(candidates("a%20b.png", false, n, r), [n.join("a b.png")]);
        assert_eq!(
            candidates("https://x/a.png", false, n, r),
            Vec::<PathBuf>::new()
        );
        assert_eq!(
            candidates("file:///tmp/a.png", false, n, r),
            [PathBuf::from("/tmp/a.png")]
        );
        assert_eq!(candidates("s.png", true, n, r).len(), 4);
    }
}

//! Spell checking, host-injected.
//!
//! This module is a seam, not an engine: abstract shipped a Hunspell
//! dictionary-based checker with a personal dictionary persisted next to its
//! settings, but a reusable component must not own dictionaries, files, or
//! threads. Instead the host application provides a [`SpellChecker`]
//! implementation and installs it once with [`set_spell_checker`]; the
//! editor reads everything through the shared [`Engine`].
//!
//! Spell checking is **off until a checker is installed**. With no checker
//! registered (or a checker whose [`SpellChecker::scan_line`] finds nothing)
//! the editor simply paints no spelling wavy underlines — the rest of the
//! editor is unaffected.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::ops::Range;
use std::sync::{Arc, OnceLock, RwLock};

use crate::md::Analysis;

/// Quiet period after the last edit before a dirty line is re-checked.
pub const DEBOUNCE: std::time::Duration = std::time::Duration::from_millis(120);

/// Language selection for spell checking.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SpellLang {
    Auto,
    En,
    PtBr,
    Both,
}

impl SpellLang {
    pub fn parse(s: &str) -> Self {
        match s {
            "en" => Self::En,
            "pt-BR" | "pt" => Self::PtBr,
            "both" => Self::Both,
            _ => Self::Auto,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::En => "en",
            Self::PtBr => "pt-BR",
            Self::Both => "both",
        }
    }

    /// The host decides what `Auto` means; the default just passes the
    /// selection through unchanged.
    pub fn langs(self) -> Langs {
        self
    }
}

/// The language set a checker was asked to check against.
pub type Langs = SpellLang;

/// A spell checker supplied by the host application.
///
/// Implementors own everything spelling-shaped: dictionaries, background
/// loading, the personal word list and its persistence. `epoch` should be
/// bumped whenever the word data changes so line caches invalidate.
pub trait SpellChecker: Send + Sync {
    /// Version of the checker's word data; bumped when it changes.
    fn epoch(&self) -> u32 {
        0
    }

    /// Misspelled word ranges in line `ix`, relative to the line start.
    fn scan_line(&self, a: &Analysis, text: &str, ix: usize, langs: Langs) -> Vec<Range<usize>>;

    /// Up to `limit` suggestions for a misspelled word, best guesses first.
    fn suggest(&self, _word: &str, _langs: Langs, _limit: usize) -> Vec<String> {
        Vec::new()
    }

    /// Add a word to the personal dictionary.
    fn add_user_word(&self, _word: &str) {}

    /// Persist the personal dictionary.
    fn save_user(&self) {}
}

/// The editor-facing handle to the host's [`SpellChecker`].
pub struct Engine {
    checker: Arc<dyn SpellChecker>,
}

impl Engine {
    pub fn new(checker: Arc<dyn SpellChecker>) -> Self {
        Self { checker }
    }

    /// Version of the checker's word data.
    pub fn epoch(&self) -> u32 {
        self.checker.epoch()
    }

    /// Up to `limit` suggestions for a misspelled word, best guesses first.
    pub fn suggest(&self, word: &str, langs: Langs, limit: usize) -> Vec<String> {
        self.checker.suggest(word, langs, limit)
    }

    /// Add a word to the personal dictionary.
    pub fn add_user_word(&self, word: &str) {
        self.checker.add_user_word(word)
    }

    /// Persist the personal dictionary.
    pub fn save_user(&self) {
        self.checker.save_user()
    }

    pub(crate) fn checker(&self) -> &Arc<dyn SpellChecker> {
        &self.checker
    }
}

/// Checker used when none is installed: nothing is ever misspelled.
struct NullChecker;

impl SpellChecker for NullChecker {
    fn scan_line(
        &self,
        _a: &Analysis,
        _text: &str,
        _ix: usize,
        _langs: Langs,
    ) -> Vec<Range<usize>> {
        Vec::new()
    }
}

static REGISTRY: OnceLock<RwLock<Option<Arc<dyn SpellChecker>>>> = OnceLock::new();

/// Install the process-wide spell checker. Call once at startup, before the
/// editor scans its first line; installing a new checker replaces the old.
pub fn set_spell_checker(checker: Arc<dyn SpellChecker>) {
    let slot = REGISTRY.get_or_init(|| RwLock::new(None));
    if let Ok(mut guard) = slot.write() {
        *guard = Some(checker);
    }
}

/// Process-wide engine wrapping the registered checker. When no checker is
/// installed this wraps [`NullChecker`] — scans return nothing and the
/// editor shows no spelling underlines.
pub fn shared() -> Arc<Engine> {
    let checker = REGISTRY
        .get()
        .and_then(|slot| slot.read().ok())
        .and_then(|guard| guard.clone())
        .unwrap_or_else(|| Arc::new(NullChecker) as Arc<dyn SpellChecker>);
    Arc::new(Engine::new(checker))
}

/// Cache key for a line's misspell ranges: the line text plus the
/// configuration that changes results, so edits/config changes invalidate
/// without bookkeeping.
pub fn line_key(line: &str, langs: Langs, epoch: u32) -> u64 {
    let mut h = DefaultHasher::new();
    // Discriminants follow declaration order: Auto=0, En=1, PtBr=2, Both=3.
    (line, langs as u8, epoch).hash(&mut h);
    h.finish()
}

/// Misspelled word ranges in line `ix`, relative to the line start,
/// forwarded to the engine's checker.
pub fn scan_line(
    a: &Analysis,
    text: &str,
    ix: usize,
    engine: &Engine,
    langs: Langs,
) -> Vec<Range<usize>> {
    engine.checker().scan_line(a, text, ix, langs)
}

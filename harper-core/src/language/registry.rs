//! Language registry - central integration point using LanguageModule trait.
//!
//! This module provides all orchestration functions for language support.
//! It is the only place that imports concrete language module implementations.

use std::fmt::Debug;
use std::sync::{Arc, LazyLock};

use super::languages::{Language, LanguageFamily};
use crate::spell::{Dictionary, FstDictionary};
use crate::{
    LintGroup,
    parsers::{Markdown, MarkdownOptions, OrgMode, Parser},
};

use super::module::{LanguageDetector, LanguageModule};
use super::{EnglishModule, GermanModule, PortugueseModule};

// ========== DETECTION ==========

/// All language detectors, sorted by confidence (highest to lowest).
static DETECTORS: LazyLock<Vec<(Box<dyn LanguageDetector>, f64)>> = LazyLock::new(|| {
    vec![
        // German has highest confidence due to unique characters (\u{00df}, \u{00e4}, \u{00f6}, \u{00fc})
        (Box::new(GermanModule::detector()), 0.95),
        // Portuguese has high confidence due to unique characters (\u{00e3}, \u{00f5}, \u{00e7})
        (Box::new(PortugueseModule::detector()), 0.90),
        // English is the fallback with lower confidence
        (Box::new(EnglishModule::detector()), 0.30),
    ]
});

/// Detect the language of the given source text.
pub fn detect_language(source: &str, dict: &FstDictionary, default_language: Language) -> Language {
    use crate::parsers::PlainEnglish;

    let source_chars: Vec<char> = source.chars().collect();
    let tokens = PlainEnglish.parse(&source_chars);

    if tokens.is_empty() {
        return default_language;
    }

    for (detector, _confidence) in DETECTORS.iter() {
        if let Some(language) = detector.detect(&tokens, &source_chars, dict, default_language) {
            return language;
        }
    }
    default_language
}

// ========== PROSE LANGUAGE ==========

/// Prose languages supported by Harper for text parsing.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProseLanguage {
    English,
    German,
    Portuguese,
}

/// Convert a Harper Language to a ProseLanguage.
pub fn prose_language(language: &Language) -> ProseLanguage {
    match language {
        Language::English(_) => ProseLanguage::English,
        Language::German(_) => ProseLanguage::German,
        Language::Portuguese(_) => ProseLanguage::Portuguese,
    }
}

// ========== DICTIONARIES ==========

/// Get the dictionary for a language family.
pub fn dictionary_for_language(family: LanguageFamily) -> Arc<FstDictionary> {
    match family {
        LanguageFamily::English => EnglishModule::dictionary(),
        LanguageFamily::German => GermanModule::dictionary(),
        LanguageFamily::Portuguese => PortugueseModule::dictionary(),
    }
}

/// Get the dictionary for a language.
pub fn dictionary(language: Language) -> Arc<FstDictionary> {
    dictionary_for_language(language.family())
}

// ========== PARSERS ==========

/// Get a parser for the given language ID and language.
pub fn parser_for_prose(
    language_id: &str,
    language: Language,
    markdown_options: MarkdownOptions,
) -> Option<Box<dyn Parser>> {
    match (language_id, prose_language(&language)) {
        // Mail format
        ("mail", ProseLanguage::German) => Some(Box::new(GermanModule::plain_parser())),
        ("mail", ProseLanguage::Portuguese) => Some(Box::new(PortugueseModule::plain_parser())),
        ("mail", ProseLanguage::English) => Some(Box::new(EnglishModule::plain_parser())),

        // Markdown/Quarto format
        ("markdown" | "quarto", ProseLanguage::German) => Some(Box::new(
            Markdown::with_inline_parser(markdown_options, |source| {
                GermanModule::plain_parser().parse(source)
            }),
        )),
        ("markdown" | "quarto", _) => Some(Box::new(Markdown::new(markdown_options))),

        // Org mode format
        ("org", ProseLanguage::German) => Some(Box::new(OrgMode::with_inline_parser(|source| {
            GermanModule::plain_parser().parse(source)
        }))),
        ("org", _) => Some(Box::new(OrgMode::default())),

        // Plain text format
        ("plaintext" | "text", ProseLanguage::German) => {
            Some(Box::new(GermanModule::plain_parser()))
        }
        ("plaintext" | "text", ProseLanguage::Portuguese) => {
            Some(Box::new(PortugueseModule::plain_parser()))
        }
        ("plaintext" | "text", ProseLanguage::English) => {
            Some(Box::new(EnglishModule::plain_parser()))
        }
        _ => None,
    }
}

// ========== LINTERS ==========

/// Add language-specific linters to the lint group.
pub fn add_language_specific_linters(
    out: &mut LintGroup,
    language: Language,
    dictionary: Arc<impl Dictionary + 'static>,
) {
    match language {
        Language::German(_dialect) => {
            let lang_group = GermanModule::rust_lint_group(dictionary);
            out.merge_from(lang_group);
        }
        Language::Portuguese(_dialect) => {
            let lang_group = PortugueseModule::rust_lint_group(dictionary);
            out.merge_from(lang_group);
        }
        Language::English(_dialect) => {
            let lang_group = EnglishModule::rust_lint_group(dictionary);
            out.merge_from(lang_group);
        }
    }
}

// ========== WIR RULES ==========

/// Get the Weir rule lint group for a specific language.
pub fn weir_rules_lint_group(language: Language) -> LintGroup {
    match language {
        Language::German(_) => GermanModule::weir_lint_group(),
        Language::English(_) => EnglishModule::weir_lint_group(),
        Language::Portuguese(_) => PortugueseModule::weir_lint_group(),
    }
}

// ========== CURATED LINT GROUPS ==========

/// Create a new curated lint group for a specific language.
pub fn new_curated_for_language(
    _dictionary: Arc<impl Dictionary + 'static>,
    language: Language,
) -> LintGroup {
    match language {
        Language::English(dialect) => EnglishModule::curated_lint_group(dialect),
        Language::German(dialect) => GermanModule::curated_lint_group(dialect),
        Language::Portuguese(dialect) => PortugueseModule::curated_lint_group(dialect),
    }
}

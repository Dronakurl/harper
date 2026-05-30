//! Language detection support for Harper.

use std::fmt::Debug;

use crate::languages::LanguageFamily;
use crate::parsers::{Parser, PlainEnglish};
use crate::spell::{Dictionary, FstDictionary};
use crate::{Dialect, Document, Token, TokenKind};

/// Check if the contents of the document are likely intended to represent
/// English.
pub fn is_doc_likely_english(doc: &Document, dict: &impl Dictionary) -> bool {
    is_likely_english(doc.get_tokens(), doc.get_source(), dict)
}

/// Check if given tokens are likely intended to represent English.
pub fn is_likely_english(toks: &[Token], source: &[char], dict: &impl Dictionary) -> bool {
    let mut total_words = 0;
    let mut valid_words = 0;
    let mut punctuation = 0;
    let mut unlintable = 0;

    for token in toks {
        match token.kind {
            TokenKind::Word(_) => {
                total_words += 1;

                let word_content = token.get_ch(source);
                if dict.contains_word(word_content) {
                    valid_words += 1;
                }
            }
            TokenKind::Punctuation(_) => punctuation += 1,
            TokenKind::Unlintable => unlintable += 1,
            _ => (),
        }
    }

    if total_words == 0 {
        return false;
    }

    if total_words <= 7 && total_words - valid_words > 0 {
        return false;
    }

    if unlintable > valid_words {
        return false;
    }

    if (punctuation as f32 * 1.25) > valid_words as f32 {
        return false;
    }

    if (valid_words as f64 / total_words as f64) < 0.7 {
        return false;
    }

    true
}

/// Core trait for language detectors.
pub trait LanguageDetector: Debug + Send + Sync {
    fn name(&self) -> &str;
    fn detect(
        &self,
        toks: &[Token],
        source: &[char],
        english_dict: &FstDictionary,
    ) -> Option<LanguageFamily>;
    fn confidence(&self) -> f64;
}

/// Registry of all available language detectors.
pub struct LanguageDetectionRegistry {
    detectors: Vec<Box<dyn LanguageDetector>>,
}

impl LanguageDetectionRegistry {
    #[must_use]
    pub fn new_empty() -> Self {
        Self {
            detectors: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_default_detectors() -> Self {
        let mut registry = Self::new_empty();
        registry.register_detector(Box::new(german::GermanDetector));
        registry.register_detector(Box::new(portuguese::PortugueseDetector));
        registry.register_detector(Box::new(english::EnglishDetector));
        registry
    }

    #[must_use]
    pub fn new() -> Self {
        Self::with_default_detectors()
    }

    pub fn register_detector(&mut self, detector: Box<dyn LanguageDetector>) {
        self.detectors.push(detector);
        self.detectors
            .sort_by(|a, b| b.confidence().partial_cmp(&a.confidence()).unwrap());
    }

    #[must_use]
    pub fn detect_language(&self, source: &str, default_dialect: Dialect) -> Dialect {
        let source_chars: Vec<char> = source.chars().collect();
        // The current shared plain-text lexer is sufficient for the supported
        // Latin-script dialects. If a future language needs a different
        // tokenizer for detection, this is the seam to extend.
        let tokens = PlainEnglish.parse(&source_chars);

        if tokens.is_empty() {
            return default_dialect;
        }

        let english_dict = FstDictionary::curated();

        for detector in &self.detectors {
            if let Some(language) = detector.detect(&tokens, &source_chars, &english_dict) {
                return default_dialect.resolve_detected_language_family(language);
            }
        }

        default_dialect
    }
}

impl Default for LanguageDetectionRegistry {
    fn default() -> Self {
        Self::with_default_detectors()
    }
}

pub mod english;
pub mod german;
pub mod portuguese;

#[cfg(test)]
mod tests {
    use super::{LanguageDetectionRegistry, is_doc_likely_english};
    use crate::Document;
    use crate::spell::FstDictionary;
    use crate::{Dialect, languages::LanguageFamily};

    fn assert_not_english(source: &'static str) {
        let dict = FstDictionary::curated();
        let doc = Document::new_plain_english(source, &dict);
        let is_likely_english = is_doc_likely_english(&doc, &dict);
        dbg!(source);
        assert!(!is_likely_english);
    }

    fn assert_english(source: &'static str) {
        let dict = FstDictionary::curated();
        let doc = Document::new_plain_english(source, &dict);
        let is_likely_english = is_doc_likely_english(&doc, &dict);
        dbg!(source);
        assert!(is_likely_english);
    }

    #[test]
    fn detects_spanish() {
        assert_not_english("Esto es español. Harper no debería marcarlo como inglés.");
    }

    #[test]
    fn detects_french() {
        assert_not_english(
            "C'est du français. Il ne devrait pas être marqué comme anglais par Harper.",
        );
    }

    #[test]
    fn detects_shebang() {
        assert_not_english("#! /bin/bash");
        assert_not_english("#! /usr/bin/fish");
    }

    #[test]
    fn detects_short_english() {
        assert_english("This is English!");
    }

    #[test]
    fn rejects_number_only_text() {
        assert_not_english("12345 67890");
    }

    #[test]
    fn detects_english() {
        assert_english("This is perfectly valid English, evn if it has a cople typos.")
    }

    #[test]
    fn detects_expressive_english() {
        assert_english("Look above! That is real English! So is this: bippity bop!")
    }

    #[test]
    fn detects_python_fib() {
        assert_not_english(
            r"
def fibIter(n):
    if n < 2:
        return n
    fibPrev = 1
    fib = 1
    for _ in range(2, n):
        fibPrev, fib = fib, fib + fibPrev
    return fib
        ",
        );
    }

    #[test]
    fn mixed_french_english_park() {
        assert_not_english("Je voudrais promener au the park a huit heures with ma voisine");
    }

    #[test]
    fn mixed_french_english_drunk() {
        assert_not_english("Je ne suis pas drunk, je suis only ivre by you");
    }

    #[test]
    fn mixed_french_english_dress() {
        assert_not_english(
            "Je buy une robe nouveau chaque Tuesday, mais aujourd'hui, je don't have temps",
        );
    }

    #[test]
    fn english_motto() {
        assert_english("I have a simple motto in life");
    }

    #[test]
    fn empty_registry_falls_back_to_default_dialect() {
        let registry = LanguageDetectionRegistry::new_empty();

        assert_eq!(
            registry.detect_language("Der Hund spielt im Garten.", Dialect::British),
            Dialect::British
        );
    }

    #[test]
    fn detected_language_family_preserves_preferred_dialect_variant() {
        let registry = LanguageDetectionRegistry::with_default_detectors();

        assert_eq!(
            registry.detect_language(
                "The dog plays in the garden. The cat sleeps on the sofa.",
                Dialect::British,
            ),
            Dialect::British
        );
        assert_eq!(
            registry.detect_language(
                "Der Hund spielt im Garten. Die Katze schläft auf dem Sofa.",
                Dialect::GermanSwiss,
            ),
            Dialect::GermanSwiss
        );
        assert_eq!(
            registry.detect_language(
                "O João foi à cidade de São Paulo e comprou pão.",
                Dialect::Portuguese,
            ),
            Dialect::Portuguese
        );
    }

    #[test]
    fn detected_language_family_uses_supported_default_when_preference_is_other_language() {
        assert_eq!(
            Dialect::American.resolve_detected_language_family(LanguageFamily::German),
            Dialect::German
        );
        assert_eq!(
            Dialect::German.resolve_detected_language_family(LanguageFamily::English),
            Dialect::American
        );
    }
}

//! German language detector.
//!
//! Uses characteristic German features:
//! - Special characters (ä, ö, ü, ß)
//! - Common German words and articles
//! - Low English word match rate

use crate::language_detection::LanguageDetector;
use crate::spell::{Dictionary, FstDictionary};
use crate::{Token, TokenKind, languages::LanguageFamily};

/// German language detector with high confidence due to unique characters.
#[derive(Debug)]
pub struct GermanDetector;

impl LanguageDetector for GermanDetector {
    fn name(&self) -> &str {
        "german"
    }

    fn detect(
        &self,
        toks: &[Token],
        source: &[char],
        english_dict: &FstDictionary,
    ) -> Option<LanguageFamily> {
        let mut total_words = 0;
        let mut german_char_count = 0;
        let mut common_german_words = 0;
        let mut english_matches = 0;

        let german_indicators = [
            "der",
            "die",
            "das",
            "den",
            "dem",
            "des",
            "ein",
            "eine",
            "einer",
            "einen",
            "einem",
            "einen",
            "ich",
            "du",
            "er",
            "sie",
            "es",
            "wir",
            "ihr",
            "sie",
            "mich",
            "dich",
            "ihn",
            "sie",
            "es",
            "uns",
            "euch",
            "ist",
            "sind",
            "war",
            "waren",
            "hat",
            "habe",
            "haben",
            "hatte",
            "hatten",
            "werden",
            "wird",
            "wurde",
            "worden",
            "kann",
            "kannst",
            "kann",
            "können",
            "könnte",
            "machen",
            "macht",
            "machte",
            "machen",
            "gehen",
            "geht",
            "ging",
            "gingen",
            "im",
            "am",
            "um",
            "für",
            "durch",
            "während",
            "seit",
            "von",
            "zu",
            "bei",
            "das",
            "dass",
            "dies",
            "diese",
            "dieser",
            "dieses",
            "jener",
            "jene",
            "jenes",
            "nicht",
            "nichts",
            "kein",
            "keine",
            "keiner",
            "keinen",
            "nirgendwo",
            "auch",
            "noch",
            "schon",
            "nur",
            "doch",
            "ja",
            "nein",
            "oder",
            "und",
        ];

        for token in toks {
            match token.kind {
                TokenKind::Word(_) => {
                    total_words += 1;
                    let word_content: String = token.get_ch(source).iter().collect();

                    if word_content.contains('ä')
                        || word_content.contains('ö')
                        || word_content.contains('ü')
                        || word_content.contains('ß')
                    {
                        german_char_count += 1;
                    }

                    let lower_word = word_content.to_lowercase();
                    if german_indicators.contains(&lower_word.as_str()) {
                        common_german_words += 1;
                    }

                    if english_dict.contains_word(token.get_ch(source)) {
                        english_matches += 1;
                    }
                }
                TokenKind::Unlintable => {}
                _ => {}
            }
        }

        if total_words < 5 {
            return None;
        }

        let german_char_ratio = german_char_count as f64 / total_words as f64;
        let german_word_ratio = common_german_words as f64 / total_words as f64;
        let english_match_ratio = if total_words > 0 {
            english_matches as f64 / total_words as f64
        } else {
            0.0
        };

        if german_char_ratio >= 0.01 {
            return Some(LanguageFamily::German);
        }

        if english_match_ratio >= 0.65 {
            return None;
        }

        if german_word_ratio >= 0.20 {
            return Some(LanguageFamily::German);
        }

        if english_match_ratio < 0.4 && german_word_ratio >= 0.08 {
            return Some(LanguageFamily::German);
        }

        None
    }

    fn confidence(&self) -> f64 {
        0.95
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    fn test_detection(text: &str, expected_german: bool) {
        let dict = FstDictionary::curated();
        let doc = Document::new_plain_english_curated(text);
        let detector = GermanDetector;

        let result = detector.detect(doc.get_tokens(), doc.get_source(), &dict);
        assert_eq!(result.is_some(), expected_german, "Failed for: {}", text);
        if expected_german {
            assert_eq!(result.unwrap(), LanguageFamily::German);
        }
    }

    #[test]
    fn detects_german_special_chars() {
        test_detection("Der Hund spielt im Garten mit Äpfeln und Ölkannen.", true);
    }

    #[test]
    fn detects_common_german_words() {
        test_detection(
            "Der Hund ist im Garten. Die Katze schläft auf dem Sofa. Das Auto ist sehr schnell.",
            true,
        );
    }

    #[test]
    fn detects_mixed_german_english() {
        test_detection(
            "Der Hund plays im Garten. die Katze sleeps auf dem Sofa. Das Auto is very schnell.",
            true,
        );
    }

    #[test]
    fn does_not_detect_english() {
        test_detection("The quick brown fox jumps over the lazy dog.", false);
    }

    #[test]
    fn detector_name() {
        let detector = GermanDetector;
        assert_eq!(detector.name(), "german");
    }

    #[test]
    fn detector_confidence() {
        let detector = GermanDetector;
        assert!((detector.confidence() - 0.95).abs() < f64::EPSILON);
    }
}

//! A slot on [`DictWordMetadata`](crate::DictWordMetadata) that a language plugin fills
//! with its own grammatical features.
//!
//! English needs nothing beyond the fields of `DictWordMetadata`. Other languages do:
//! German marks case, gender and number on every form, and another language marks
//! something else. Harper does not try to know all of that. It keeps 128 bits per
//! entry, merges them with a bitwise OR the way it merges its own flags, and leaves
//! their meaning to the plugin that wrote them.
//!
//! Only built with the `language-bits` feature, so English pays nothing for it.

use serde::{Deserialize, Serialize};

/// Grammatical features a language plugin stores on a dictionary entry.
///
/// Harper never reads them. Each plugin decides which bit means what, and a form that
/// several affixes or repeated headwords produce gets the union of their bits, so a
/// plugin should give each reading a form can have a bit of its own.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Hash, Serialize, Deserialize)]
pub struct LanguageBits(pub [u64; 2]);

impl LanguageBits {
    /// No bit is set.
    pub fn is_empty(&self) -> bool {
        self.0 == [0, 0]
    }

    /// The union of both.
    pub fn or(&self, other: &Self) -> Self {
        Self([self.0[0] | other.0[0], self.0[1] | other.0[1]])
    }
}

#[cfg(test)]
mod tests {
    use super::LanguageBits;
    use crate::DictWordMetadata;

    #[test]
    fn merging_metadata_unites_the_bits() {
        let mut a = DictWordMetadata {
            language_bits: LanguageBits([0b01, 0]),
            ..Default::default()
        };
        let b = DictWordMetadata {
            language_bits: LanguageBits([0b10, 1 << 63]),
            ..Default::default()
        };
        a.merge(&b);
        assert_eq!(a.language_bits, LanguageBits([0b11, 1 << 63]));
    }

    #[test]
    fn the_bits_come_from_annotation_metadata_and_are_left_out_when_empty() {
        let metadata: DictWordMetadata =
            serde_json::from_str(r#"{ "language_bits": [5, 0] }"#).unwrap();
        assert_eq!(metadata.language_bits, LanguageBits([5, 0]));

        let json = serde_json::to_string(&DictWordMetadata::default()).unwrap();
        assert!(!json.contains("language_bits"), "{json}");
    }
}

use serde::ser::SerializeStruct;
use serde::{Serialize, Deserialize};
use serde_json::Value;

// Import dialect types from the central dialects module for modularity.
use crate::dialects::dialect_enum::DialectsEnum;
use crate::dialects::dialect_trait::DialectFlags as _;
use crate::dialects::english::{EnglishDialect, EnglishDialectFlags};
use crate::language::german::dialects::{GermanDialect, GermanDialectFlags};
use crate::language::portuguese::dialects::{PortugueseDialect, PortugueseDialectFlags};
use crate::{Document, TokenStringExt};

/// This represents a collection of dialect flags for all supported languages.
/// Each language has its own set of dialect flags.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Hash)]
pub struct DialectFlags {
    // IMPORTANT: These fields must match the LANGUAGES! macro in dict_word_metadata.rs.
    // To add a new language, add a field here and update the LANGUAGES! macro.
    pub english: EnglishDialectFlags,
    pub german: GermanDialectFlags,
    pub portuguese: PortugueseDialectFlags,
}

impl Serialize for DialectFlags {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut scoped = serializer.serialize_struct("DialectFlags", 3)?;
        scoped.serialize_field("english", &self.english)?;
        scoped.serialize_field("german", &self.german)?;
        scoped.serialize_field("portuguese", &self.portuguese)?;
        scoped.end()
    }
}

impl<'de> Deserialize<'de> for DialectFlags {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        // Only accept the new scoped, language-specific dialect flags format.
        let scoped = ScopedDialectFlagsSerde::deserialize(deserializer)?;
        Ok(scoped.into())
    }
}

impl From<ScopedDialectFlagsSerde> for DialectFlags {
    fn from(value: ScopedDialectFlagsSerde) -> Self {
        Self {
            english: value.english,
            german: value.german,
            portuguese: value.portuguese,
        }
    }
}

impl DialectFlags {
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            english: EnglishDialectFlags::empty(),
            german: GermanDialectFlags::empty(),
            portuguese: PortugueseDialectFlags::empty(),
        }
    }

    #[must_use]
    pub fn is_empty(self) -> bool {
        self.english.is_empty() && self.german.is_empty() && self.portuguese.is_empty()
    }

    /// Checks if the provided dialect is enabled.
    /// If no dialect is explicitly enabled, it is assumed that all dialects are enabled.
    #[must_use]
    pub fn is_dialect_enabled(self, dialect: impl Into<DialectsEnum>) -> bool {
        if self.is_empty() {
            return true;
        }

        match dialect.into() {
            DialectsEnum::English(EnglishDialect::American) => {
                !self.english.is_empty()
                    && self.english.is_dialect_enabled(EnglishDialect::American)
            }
            DialectsEnum::English(EnglishDialect::Canadian) => {
                !self.english.is_empty()
                    && self.english.is_dialect_enabled(EnglishDialect::Canadian)
            }
            DialectsEnum::English(EnglishDialect::Australian) => {
                !self.english.is_empty()
                    && self.english.is_dialect_enabled(EnglishDialect::Australian)
            }
            DialectsEnum::English(EnglishDialect::British) => {
                !self.english.is_empty() && self.english.is_dialect_enabled(EnglishDialect::British)
            }
            DialectsEnum::English(EnglishDialect::Indian) => {
                !self.english.is_empty() && self.english.is_dialect_enabled(EnglishDialect::Indian)
            }
            DialectsEnum::German(GermanDialect::Standard) => {
                !self.german.is_empty() && self.german.is_dialect_enabled(GermanDialect::Standard)
            }
            DialectsEnum::German(GermanDialect::Austrian) => {
                !self.german.is_empty() && self.german.is_dialect_enabled(GermanDialect::Austrian)
            }
            DialectsEnum::German(GermanDialect::Swiss) => {
                !self.german.is_empty() && self.german.is_dialect_enabled(GermanDialect::Swiss)
            }
            DialectsEnum::Portuguese(portuguese) => {
                !self.portuguese.is_empty() && self.portuguese.is_dialect_enabled(portuguese)
            }
        }
    }

    /// Checks if the provided dialect is ***explicitly*** enabled.
    ///
    /// Unlike `is_dialect_enabled`, this will return false when no dialects are explicitly
    /// enabled.
    #[must_use]
    pub fn is_dialect_enabled_strict(self, dialect: impl Into<DialectsEnum>) -> bool {
        match dialect.into() {
            DialectsEnum::English(EnglishDialect::American) => self
                .english
                .is_dialect_enabled_strict(EnglishDialect::American),
            DialectsEnum::English(EnglishDialect::Canadian) => self
                .english
                .is_dialect_enabled_strict(EnglishDialect::Canadian),
            DialectsEnum::English(EnglishDialect::Australian) => self
                .english
                .is_dialect_enabled_strict(EnglishDialect::Australian),
            DialectsEnum::English(EnglishDialect::British) => self
                .english
                .is_dialect_enabled_strict(EnglishDialect::British),
            DialectsEnum::English(EnglishDialect::Indian) => self
                .english
                .is_dialect_enabled_strict(EnglishDialect::Indian),
            DialectsEnum::German(GermanDialect::Standard) => self
                .german
                .is_dialect_enabled_strict(GermanDialect::Standard),
            DialectsEnum::German(GermanDialect::Austrian) => self
                .german
                .is_dialect_enabled_strict(GermanDialect::Austrian),
            DialectsEnum::German(GermanDialect::Swiss) => {
                self.german.is_dialect_enabled_strict(GermanDialect::Swiss)
            }
            DialectsEnum::Portuguese(portuguese) => {
                self.portuguese.is_dialect_enabled_strict(portuguese)
            }
        }
    }

    #[must_use]
    pub fn is_english_dialect_enabled(self, dialect: EnglishDialect) -> bool {
        self.english.is_dialect_enabled(dialect)
    }

    #[must_use]
    pub fn is_english_dialect_enabled_strict(self, dialect: EnglishDialect) -> bool {
        self.english.is_dialect_enabled_strict(dialect)
    }

    #[must_use]
    pub fn is_german_dialect_enabled(self, dialect: GermanDialect) -> bool {
        self.german.is_dialect_enabled(dialect)
    }

    #[must_use]
    pub fn is_german_dialect_enabled_strict(self, dialect: GermanDialect) -> bool {
        self.german.is_dialect_enabled_strict(dialect)
    }

    #[must_use]
    pub fn is_portuguese_dialect_enabled(self, dialect: PortugueseDialect) -> bool {
        self.portuguese.is_dialect_enabled(dialect)
    }

    #[must_use]
    pub fn is_portuguese_dialect_enabled_strict(self, dialect: PortugueseDialect) -> bool {
        self.portuguese.is_dialect_enabled_strict(dialect)
    }

    /// Constructs `DialectFlags` from the provided dialect.
    #[must_use]
    pub fn from_dialect(dialect: impl Into<DialectsEnum>) -> Self {
        match dialect.into() {
            DialectsEnum::English(EnglishDialect::American) => Self {
                english: EnglishDialectFlags::from_dialect(EnglishDialect::American),
                ..Self::empty()
            },
            DialectsEnum::English(EnglishDialect::Canadian) => Self {
                english: EnglishDialectFlags::from_dialect(EnglishDialect::Canadian),
                ..Self::empty()
            },
            DialectsEnum::English(EnglishDialect::Australian) => Self {
                english: EnglishDialectFlags::from_dialect(EnglishDialect::Australian),
                ..Self::empty()
            },
            DialectsEnum::English(EnglishDialect::British) => Self {
                english: EnglishDialectFlags::from_dialect(EnglishDialect::British),
                ..Self::empty()
            },
            DialectsEnum::English(EnglishDialect::Indian) => Self {
                english: EnglishDialectFlags::from_dialect(EnglishDialect::Indian),
                ..Self::empty()
            },
            DialectsEnum::German(GermanDialect::Standard) => Self {
                german: GermanDialectFlags::from_dialect(GermanDialect::Standard),
                ..Self::empty()
            },
            DialectsEnum::German(GermanDialect::Austrian) => Self {
                german: GermanDialectFlags::from_dialect(GermanDialect::Austrian),
                ..Self::empty()
            },
            DialectsEnum::German(GermanDialect::Swiss) => Self {
                german: GermanDialectFlags::from_dialect(GermanDialect::Swiss),
                ..Self::empty()
            },
            DialectsEnum::Portuguese(portuguese) => Self {
                portuguese: PortugueseDialectFlags::from_dialect(portuguese),
                ..Self::empty()
            },
        }
    }

    /// Gets the most commonly used dialect(s) in the document.
    ///
    /// If multiple dialects are used equally often, they will all be enabled in the returned
    /// `DialectFlags`. On the other hand, if there is a single dialect that is used the most, it
    /// will be the only one enabled.
    #[must_use]
    pub fn get_most_used_dialects_from_document(document: &crate::Document) -> Self {
        // Initialize counters.
        let mut dialect_counters = [
            (DialectsEnum::English(EnglishDialect::American), 0usize),
            (DialectsEnum::English(EnglishDialect::Canadian), 0usize),
            (DialectsEnum::English(EnglishDialect::Australian), 0usize),
            (DialectsEnum::English(EnglishDialect::British), 0usize),
            (DialectsEnum::English(EnglishDialect::Indian), 0usize),
            (DialectsEnum::German(GermanDialect::Standard), 0usize),
            (DialectsEnum::German(GermanDialect::Austrian), 0usize),
            (DialectsEnum::German(GermanDialect::Swiss), 0usize),
            (
                DialectsEnum::Portuguese(PortugueseDialect::European),
                0usize,
            ),
            (
                DialectsEnum::Portuguese(PortugueseDialect::Brazilian),
                0usize,
            ),
            (DialectsEnum::Portuguese(PortugueseDialect::African), 0usize),
        ];

        // Count word dialects.
        document.iter_words().for_each(|w| {
            if let crate::TokenKind::Word(Some(lexeme_metadata)) = &w.kind {
                // If the token is a word, iterate though the dialects in `dialect_counters` and
                // increment those counters where the word has the respective dialect enabled.
                dialect_counters.iter_mut().for_each(|(dialect, count)| {
                    if lexeme_metadata.dialects.is_dialect_enabled(*dialect) {
                        *count += 1;
                    }
                });
            }
        });

        // Find max counter.
        let max_counter = dialect_counters
            .iter()
            .map(|(_, count)| count)
            .max()
            .unwrap();
        // Get and convert the collection of most used dialects into a `DialectFlags`.
        dialect_counters
            .into_iter()
            .filter(|(_, count)| count == max_counter)
            .fold(DialectFlags::empty(), |acc, dialect| {
                // Fold most used dialects into `DialectFlags` via bitwise or.
                acc | Self::from_dialect(dialect.0)
            })
    }
}

impl std::ops::BitOr for DialectFlags {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            english: self.english | rhs.english,
            german: self.german | rhs.german,
            portuguese: self.portuguese | rhs.portuguese,
        }
    }
}

impl std::ops::BitOrAssign for DialectFlags {
    fn bitor_assign(&mut self, rhs: Self) {
        self.english |= rhs.english;
        self.german |= rhs.german;
        self.portuguese |= rhs.portuguese;
    }
}

impl Default for DialectFlags {
    /// A default value with no dialects explicitly enabled.
    /// Implicitly, this state corresponds to all dialects being enabled.
    fn default() -> Self {
        Self::empty()
    }
}

// Old legacy support (numeric bitmasks and flat strings) has been removed to simplify the data model.
// Use the ScopedDialectFlagsSerde and DialectFlags (language-scoped) for serialization/deserialization.
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Eq, Hash, Default)]
struct ScopedDialectFlagsSerde {
    english: EnglishDialectFlags,
    german: GermanDialectFlags,
    portuguese: PortugueseDialectFlags,
}

impl<'de> Deserialize<'de> for ScopedDialectFlagsSerde {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{Error, Unexpected};

        let value = Value::deserialize(deserializer)?;

        match value {
            Value::Object(map) => {
                let mut english = EnglishDialectFlags::default();
                let mut german = GermanDialectFlags::default();
                let mut portuguese = PortugueseDialectFlags::default();

                for (key, val) in map {
                    match key.as_str() {
                        "english" => match val {
                            Value::String(s) => {
                                english = match s.as_str() {
                                    "AMERICAN" => EnglishDialectFlags::AMERICAN,
                                    "CANADIAN" => EnglishDialectFlags::CANADIAN,
                                    "AUSTRALIAN" => EnglishDialectFlags::AUSTRALIAN,
                                    "BRITISH" => EnglishDialectFlags::BRITISH,
                                    "INDIAN" => EnglishDialectFlags::INDIAN,
                                    _ => {
                                        return Err(Error::custom(format!(
                                            "Unknown English dialect: {s}"
                                        )));
                                    }
                                };
                            }
                            Value::Number(n) => {
                                let num =
                                    n.as_u64().ok_or_else(|| Error::custom("Invalid number"))?
                                        as u8;
                                english = EnglishDialectFlags::from_bits(num)
                                    .ok_or_else(|| Error::custom("Invalid dialect flags"))?;
                            }
                            _ => {
                                return Err(Error::invalid_type(
                                    Unexpected::Other("english"),
                                    &"string or number",
                                ));
                            }
                        },
                        "german" => match val {
                            Value::String(s) => {
                                german = match s.as_str() {
                                    "STANDARD" => GermanDialectFlags::STANDARD,
                                    "AUSTRIAN" => GermanDialectFlags::AUSTRIAN,
                                    "SWISS" => GermanDialectFlags::SWISS,
                                    _ => {
                                        return Err(Error::custom(format!(
                                            "Unknown German dialect: {s}"
                                        )));
                                    }
                                };
                            }
                            Value::Number(n) => {
                                let num =
                                    n.as_u64().ok_or_else(|| Error::custom("Invalid number"))?
                                        as u8;
                                german = GermanDialectFlags::from_bits(num)
                                    .ok_or_else(|| Error::custom("Invalid dialect flags"))?;
                            }
                            _ => {
                                return Err(Error::invalid_type(
                                    Unexpected::Other("german"),
                                    &"string or number",
                                ));
                            }
                        },
                        "portuguese" => match val {
                            Value::String(s) => {
                                portuguese = match s.as_str() {
                                    "EUROPEAN" => PortugueseDialectFlags::EUROPEAN,
                                    "BRAZILIAN" => PortugueseDialectFlags::BRAZILIAN,
                                    "AFRICAN" => PortugueseDialectFlags::AFRICAN,
                                    _ => {
                                        return Err(Error::custom(format!(
                                            "Unknown Portuguese dialect: {s}"
                                        )));
                                    }
                                };
                            }
                            Value::Number(n) => {
                                let num =
                                    n.as_u64().ok_or_else(|| Error::custom("Invalid number"))?
                                        as u8;
                                portuguese = PortugueseDialectFlags::from_bits(num)
                                    .ok_or_else(|| Error::custom("Invalid dialect flags"))?;
                            }
                            _ => {
                                return Err(Error::invalid_type(
                                    Unexpected::Other("portuguese"),
                                    &"string or number",
                                ));
                            }
                        },
                        _ => {
                            return Err(Error::unknown_field(&key, &["english", "german", "portuguese"]));
                        }
                    }
                }

                Ok(ScopedDialectFlagsSerde {
                    english,
                    german,
                    portuguese,
                })
            }
            Value::String(s) => {
                // Legacy format: single string representing one dialect
                match s.as_str() {
                    "AMERICAN" => Ok(ScopedDialectFlagsSerde {
                        english: EnglishDialectFlags::AMERICAN,
                        german: GermanDialectFlags::default(),
                        portuguese: PortugueseDialectFlags::default(),
                    }),
                    "CANADIAN" => Ok(ScopedDialectFlagsSerde {
                        english: EnglishDialectFlags::CANADIAN,
                        german: GermanDialectFlags::default(),
                        portuguese: PortugueseDialectFlags::default(),
                    }),
                    "AUSTRALIAN" => Ok(ScopedDialectFlagsSerde {
                        english: EnglishDialectFlags::AUSTRALIAN,
                        german: GermanDialectFlags::default(),
                        portuguese: PortugueseDialectFlags::default(),
                    }),
                    "BRITISH" => Ok(ScopedDialectFlagsSerde {
                        english: EnglishDialectFlags::BRITISH,
                        german: GermanDialectFlags::default(),
                        portuguese: PortugueseDialectFlags::default(),
                    }),
                    "INDIAN" => Ok(ScopedDialectFlagsSerde {
                        english: EnglishDialectFlags::INDIAN,
                        german: GermanDialectFlags::default(),
                        portuguese: PortugueseDialectFlags::default(),
                    }),
                    "STANDARD" => Ok(ScopedDialectFlagsSerde {
                        english: EnglishDialectFlags::default(),
                        german: GermanDialectFlags::STANDARD,
                        portuguese: PortugueseDialectFlags::default(),
                    }),
                    "AUSTRIAN" => Ok(ScopedDialectFlagsSerde {
                        english: EnglishDialectFlags::default(),
                        german: GermanDialectFlags::AUSTRIAN,
                        portuguese: PortugueseDialectFlags::default(),
                    }),
                    "SWISS" => Ok(ScopedDialectFlagsSerde {
                        english: EnglishDialectFlags::default(),
                        german: GermanDialectFlags::SWISS,
                        portuguese: PortugueseDialectFlags::default(),
                    }),
                    "EUROPEAN" => Ok(ScopedDialectFlagsSerde {
                        english: EnglishDialectFlags::default(),
                        german: GermanDialectFlags::default(),
                        portuguese: PortugueseDialectFlags::EUROPEAN,
                    }),
                    "BRAZILIAN" => Ok(ScopedDialectFlagsSerde {
                        english: EnglishDialectFlags::default(),
                        german: GermanDialectFlags::default(),
                        portuguese: PortugueseDialectFlags::BRAZILIAN,
                    }),
                    "AFRICAN" => Ok(ScopedDialectFlagsSerde {
                        english: EnglishDialectFlags::default(),
                        german: GermanDialectFlags::default(),
                        portuguese: PortugueseDialectFlags::AFRICAN,
                    }),
                    _ => Err(Error::custom(format!(
                        "Unknown dialect string: {s}"
                    ))),
                }
            }
            _ => Err(Error::custom(
                "Expected object or string for dialect flags"
            )),
        }
    }
}
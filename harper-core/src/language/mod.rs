//! Language-specific modules for Harper.
//!
//! This module organizes language-specific functionality using the LanguageModule trait.

pub mod languages;
pub mod module;
pub mod registry;

pub mod english;
pub mod german;
pub mod portuguese;

// Re-export core types
pub use languages::{Language, LanguageFamily};
pub use module::{LanguageDetector, LanguageModule};

// Re-export registry functions
pub use registry::{
    ProseLanguage, add_language_specific_linters, detect_language, dictionary,
    dictionary_for_language, new_curated_for_language, parser_for_prose, prose_language,
    weir_rules_lint_group,
};

// Re-export dialect types for external use
pub use german::dialects::{GermanDialect, GermanDialectFlags};
pub use portuguese::dialects::{PortugueseDialect, PortugueseDialectFlags};

// Re-export dictionary functions for external use
pub use german::spell::{curated_german_dictionary, german_dictionary};
pub use portuguese::spell::{curated_portuguese_dictionary, portuguese_dictionary};

// Re-export parsers for external use
pub use german::parsers::PlainGerman;
pub use portuguese::parsers::PlainPortuguese;

// Re-export concrete module types (for internal use within language folder only)
pub use english::module::EnglishModule;
pub use german::module::GermanModule;
pub use portuguese::module::PortugueseModule;

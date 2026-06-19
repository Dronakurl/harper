//! German language module implementation of LanguageModule trait.

use std::sync::Arc;

use crate::language::german::dialects::GermanDialect;
use crate::language::german::language_detection::GermanDetector;
use crate::language::german::lexing::lex_german_token;
use crate::language::german::linting::{new_curated_german, weir_rules};
use crate::language::german::parsers::PlainGerman;
use crate::language::german::spell::german_dictionary;
use crate::language::languages::Language;
use crate::lexing::FoundToken;
use crate::linting::LintGroup;
use crate::parsers::Parser;
use crate::spell::{Dictionary, FstDictionary};

use crate::language::module::LanguageModule;

/// German language module implementing the LanguageModule trait.
pub struct GermanModule;

impl LanguageModule for GermanModule {
    type Dialect = GermanDialect;
    type Detector = GermanDetector;

    fn default_dialect() -> Self::Dialect {
        GermanDialect::default()
    }

    fn detector() -> Self::Detector {
        GermanDetector
    }

    fn lex_token(source: &[char]) -> FoundToken {
        lex_german_token(source)
    }

    fn plain_parser() -> impl Parser + 'static {
        PlainGerman
    }

    fn dictionary() -> Arc<FstDictionary> {
        german_dictionary()
    }

    fn rust_lint_group(dictionary: Arc<impl Dictionary + 'static>) -> LintGroup {
        use crate::language::registry::add_language_specific_linters;

        let language = Language::German(GermanDialect::default());
        let mut group = LintGroup::empty();
        add_language_specific_linters(&mut group, language, dictionary);
        group
    }

    fn weir_lint_group() -> LintGroup {
        weir_rules::lint_group()
    }

    fn curated_lint_group(dialect: Self::Dialect) -> LintGroup {
        new_curated_german(dialect)
    }
}

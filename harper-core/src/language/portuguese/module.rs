//! Portuguese language module implementation of LanguageModule trait.

use std::sync::Arc;

use crate::language::languages::Language;
use crate::language::portuguese::dialects::PortugueseDialect;
use crate::language::portuguese::language_detection::PortugueseDetector;
use crate::language::portuguese::lexing::lex_portuguese_token;
use crate::language::portuguese::linting::{new_curated_portuguese, weir_rules};
use crate::language::portuguese::parsers::PlainPortuguese;
use crate::language::portuguese::spell::portuguese_dictionary;
use crate::lexing::FoundToken;
use crate::linting::LintGroup;
use crate::parsers::Parser;
use crate::spell::{Dictionary, FstDictionary};

use crate::language::module::LanguageModule;

/// Portuguese language module implementing the LanguageModule trait.
pub struct PortugueseModule;

impl LanguageModule for PortugueseModule {
    type Dialect = PortugueseDialect;
    type Detector = PortugueseDetector;

    fn default_dialect() -> Self::Dialect {
        PortugueseDialect::default()
    }

    fn detector() -> Self::Detector {
        PortugueseDetector
    }

    fn lex_token(source: &[char]) -> FoundToken {
        lex_portuguese_token(source)
    }

    fn plain_parser() -> impl Parser + 'static {
        PlainPortuguese
    }

    fn dictionary() -> Arc<FstDictionary> {
        portuguese_dictionary()
    }

    fn rust_lint_group(dictionary: Arc<impl Dictionary + 'static>) -> LintGroup {
        use crate::language::registry::add_language_specific_linters;

        let language = Language::Portuguese(PortugueseDialect::default());
        let mut group = LintGroup::empty();
        add_language_specific_linters(&mut group, language, dictionary);
        group
    }

    fn weir_lint_group() -> LintGroup {
        weir_rules::lint_group()
    }

    fn curated_lint_group(dialect: Self::Dialect) -> LintGroup {
        new_curated_portuguese(dialect)
    }
}

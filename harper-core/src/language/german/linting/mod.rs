//! German linting rules and checkers.

pub mod german_filler_words;
pub mod german_noun_capitalization;
pub mod german_sentence_capitalization;
pub mod german_spell_check;
pub mod weir_rules;

use std::sync::Arc;

use crate::linting::LintGroup;
use crate::spell::Dictionary;
use crate::language::german::dialects::GermanDialect;
use crate::language::german::spell::curated_german_dictionary;
use crate::languages::Language;

/// Create a new curated lint group for German language.
pub fn new_curated_german(dialect: GermanDialect) -> LintGroup {
    use crate::language::manifest::{add_language_specific_linters, weir_rules_lint_group};
    
    let dictionary = curated_german_dictionary();
    let language = Language::German(dialect);
    
    let mut group = LintGroup::empty();
    group.merge_from(weir_rules_lint_group(language));
    add_language_specific_linters(&mut group, language, dictionary.clone());
    
    group
}

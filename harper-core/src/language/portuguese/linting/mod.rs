//! Portuguese linting rules and checkers.

pub mod portuguese_spell_check;
pub mod weir_rules;

use std::sync::Arc;

use crate::linting::LintGroup;
use crate::spell::Dictionary;
use crate::language::portuguese::dialects::PortugueseDialect;
use crate::language::portuguese::spell::portuguese_dictionary;
use crate::languages::Language;

/// Create a new curated lint group for Portuguese language.
pub fn new_curated_portuguese(dialect: PortugueseDialect) -> LintGroup {
    use crate::language::manifest::{add_language_specific_linters, weir_rules_lint_group};
    
    let dictionary = portuguese_dictionary();
    let language = Language::Portuguese(dialect);
    
    let mut group = LintGroup::empty();
    group.merge_from(weir_rules_lint_group(language));
    add_language_specific_linters(&mut group, language, dictionary.clone());
    
    group
}

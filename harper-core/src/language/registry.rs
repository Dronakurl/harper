use std::sync::Arc;

use crate::language::german::parsers::PlainGerman;
use crate::language::german::spell::german_dictionary;
use crate::language::portuguese::parsers::PlainPortuguese;
use crate::language::portuguese::spell::portuguese_dictionary;
use crate::linting::LintGroup;
use crate::parsers::{Markdown, MarkdownOptions, OrgMode, Parser, PlainEnglish};
use crate::spell::FstDictionary;
use crate::{Dialect, languages::LanguageFamily};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProseLanguage {
    English,
    German,
    Portuguese,
}

pub fn prose_language_for_dialect(dialect: Dialect) -> ProseLanguage {
    if dialect.is_german() {
        ProseLanguage::German
    } else if dialect.is_portuguese() {
        ProseLanguage::Portuguese
    } else {
        ProseLanguage::English
    }
}

pub fn dictionary_for_language(language: LanguageFamily) -> Arc<FstDictionary> {
    match language {
        LanguageFamily::English => FstDictionary::curated(),
        LanguageFamily::German => german_dictionary(),
        LanguageFamily::Portuguese => portuguese_dictionary(),
    }
}

pub fn dictionary_for_dialect(dialect: Dialect) -> Arc<FstDictionary> {
    dictionary_for_language(dialect.language_family())
}

pub fn parser_for_prose(
    language_id: &str,
    dialect: Dialect,
    markdown_options: MarkdownOptions,
) -> Option<Box<dyn Parser>> {
    match (language_id, prose_language_for_dialect(dialect)) {
        ("mail", ProseLanguage::German) => Some(Box::new(PlainGerman)),
        ("mail", ProseLanguage::Portuguese) => Some(Box::new(PlainPortuguese)),
        ("mail", ProseLanguage::English) => Some(Box::new(PlainEnglish)),

        ("markdown" | "quarto", ProseLanguage::German) => {
            Some(Box::new(Markdown::new_german(markdown_options)))
        }
        ("markdown" | "quarto", _) => Some(Box::new(Markdown::new(markdown_options))),

        ("org", ProseLanguage::German) => Some(Box::new(OrgMode::new_german())),
        ("org", _) => Some(Box::new(OrgMode::default())),

        ("plaintext" | "text", ProseLanguage::German) => Some(Box::new(PlainGerman)),
        ("plaintext" | "text", ProseLanguage::Portuguese) => Some(Box::new(PlainPortuguese)),
        ("plaintext" | "text", ProseLanguage::English) => Some(Box::new(PlainEnglish)),
        _ => None,
    }
}

pub fn add_language_specific_linters(out: &mut LintGroup, dialect: Dialect) {
    if dialect.is_german() {
        use crate::linting::german_noun_capitalization::GermanNounCapitalization;
        use crate::linting::german_sentence_capitalization::GermanSentenceCapitalization;
        use crate::linting::german_spell_check::GermanSpellCheck;

        let german_dict = german_dictionary();
        out.add(
            "GermanSpellCheck",
            GermanSpellCheck::new(german_dict.clone()),
        );
        out.config.set_rule_enabled("GermanSpellCheck", true);
        out.add(
            "GermanNounCapitalization",
            GermanNounCapitalization::new(german_dict.clone()),
        );
        out.config
            .set_rule_enabled("GermanNounCapitalization", true);
        out.add(
            "GermanSentenceCapitalization",
            GermanSentenceCapitalization::new(german_dict),
        );
        out.config
            .set_rule_enabled("GermanSentenceCapitalization", true);
    }

    if dialect.is_portuguese() {
        use crate::language::portuguese::linting::portuguese_spell_check::PortugueseSpellCheck;

        let portuguese_dict = portuguese_dictionary();
        out.add(
            "PortugueseSpellCheck",
            PortugueseSpellCheck::new(portuguese_dict),
        );
        out.config.set_rule_enabled("PortugueseSpellCheck", true);
    }
}

pub fn rule_default_enabled(rule_name: &str, dialect: Dialect, default: bool) -> bool {
    match rule_name {
        "FillerWords" => dialect.is_english(),
        _ => default,
    }
}

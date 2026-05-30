use std::sync::Arc;

use crate::language::german::parsers::PlainGerman;
use crate::language::german::spell::german_dictionary;
use crate::language::portuguese::parsers::PlainPortuguese;
use crate::language::portuguese::spell::portuguese_dictionary;
use crate::linting::LintGroup;
use crate::parsers::{Markdown, MarkdownOptions, OrgMode, Parser, PlainEnglish};
use crate::spell::{Dictionary, FstDictionary};
use crate::{Dialect, languages::LanguageFamily};

const LANGUAGE_NEUTRAL_RULES: &[&str] = &[
    "CommaFixes",
    "CorrectNumberSuffix",
    "CurrencyPlacement",
    "Dashes",
    "DotInitialisms",
    "EllipsisLength",
    "ExpandMemoryShorthands",
    "ExpandTimeShorthands",
    "LongSentences",
    "NoFrenchSpaces",
    "NumberSuffixCapitalization",
    "NumericRangeEnDash",
    "QuoteSpacing",
    "RepeatedWords",
    "SentenceCapitalization",
    "Spaces",
    "UnclosedQuotes",
    "UseEllipsisCharacter",
];

const GERMAN_RULES: &[&str] = &[
    "GermanFillerWords",
    "GermanNounCapitalization",
    "GermanSentenceCapitalization",
    "GermanSpellCheck",
];

const PORTUGUESE_RULES: &[&str] = &["PortugueseSpellCheck"];

pub type ProseLanguage = LanguageFamily;

pub fn prose_language_for_dialect(dialect: Dialect) -> ProseLanguage {
    dialect.language_family()
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
    let language = prose_language_for_dialect(dialect);

    match language_id {
        "mail" | "plaintext" | "text" => Some(plain_parser_for_language(language)),
        "markdown" | "quarto" => Some(Box::new(Markdown::new_with_language(
            markdown_options,
            language,
        ))),
        "org" => Some(Box::new(OrgMode::new_with_language(language))),
        _ => None,
    }
}

fn plain_parser_for_language(language: LanguageFamily) -> Box<dyn Parser> {
    match language {
        LanguageFamily::English => Box::new(PlainEnglish),
        LanguageFamily::German => Box::new(PlainGerman),
        LanguageFamily::Portuguese => Box::new(PlainPortuguese),
    }
}

pub fn add_language_specific_linters(
    out: &mut LintGroup,
    dialect: Dialect,
    dictionary: Arc<impl Dictionary + 'static>,
) {
    if dialect.is_german() {
        use crate::linting::german_noun_capitalization::GermanNounCapitalization;
        use crate::linting::german_sentence_capitalization::GermanSentenceCapitalization;
        use crate::linting::german_spell_check::GermanSpellCheck;

        out.add(
            "GermanSpellCheck",
            GermanSpellCheck::new(dictionary.clone()),
        );
        out.config.set_rule_enabled("GermanSpellCheck", true);
        out.add(
            "GermanNounCapitalization",
            GermanNounCapitalization::new(dictionary.clone()),
        );
        out.config
            .set_rule_enabled("GermanNounCapitalization", true);
        out.add(
            "GermanSentenceCapitalization",
            GermanSentenceCapitalization::new(dictionary.clone()),
        );
        out.config
            .set_rule_enabled("GermanSentenceCapitalization", true);
    }

    if dialect.is_portuguese() {
        use crate::language::portuguese::linting::portuguese_spell_check::PortugueseSpellCheck;

        out.add(
            "PortugueseSpellCheck",
            PortugueseSpellCheck::new(dictionary.clone()),
        );
        out.config.set_rule_enabled("PortugueseSpellCheck", true);
    }
}

pub fn rule_default_enabled(rule_name: &str, dialect: Dialect, default: bool) -> bool {
    if !default {
        return false;
    }

    if dialect.is_english() {
        return true;
    }

    if LANGUAGE_NEUTRAL_RULES.contains(&rule_name) {
        return true;
    }

    if dialect.is_german() {
        return GERMAN_RULES.contains(&rule_name);
    }

    if dialect.is_portuguese() {
        return PORTUGUESE_RULES.contains(&rule_name);
    }

    false
}

#[cfg(test)]
mod tests {
    use super::parser_for_prose;
    use crate::parsers::{
        Markdown, MarkdownOptions, OrgMode, PlainEnglish, PlainPortuguese, StrParser,
    };
    use crate::{Dialect, TokenKind};

    fn token_kinds(tokens: Vec<crate::Token>) -> Vec<TokenKind> {
        tokens.into_iter().map(|token| token.kind).collect()
    }

    #[test]
    fn portuguese_markdown_uses_portuguese_parser() {
        let source = "Os anos 1980s mudaram.";
        let parser =
            parser_for_prose("markdown", Dialect::Portuguese, MarkdownOptions::default()).unwrap();

        let registry_tokens = token_kinds(parser.parse_str(source));
        let portuguese_tokens =
            token_kinds(Markdown::new_portuguese(MarkdownOptions::default()).parse_str(source));
        let english_tokens =
            token_kinds(Markdown::new(MarkdownOptions::default()).parse_str(source));

        assert_eq!(registry_tokens, portuguese_tokens);
        assert_ne!(registry_tokens, english_tokens);
    }

    #[test]
    fn portuguese_org_uses_portuguese_parser() {
        let source = "Os anos 1980s mudaram.";
        let parser =
            parser_for_prose("org", Dialect::Portuguese, MarkdownOptions::default()).unwrap();

        let registry_tokens = token_kinds(parser.parse_str(source));
        let portuguese_tokens = token_kinds(OrgMode::new_portuguese().parse_str(source));
        let english_tokens = token_kinds(OrgMode::default().parse_str(source));

        assert_eq!(registry_tokens, portuguese_tokens);
        assert_ne!(registry_tokens, english_tokens);
    }

    #[test]
    fn prose_language_alias_tracks_dialect_language_family() {
        assert_eq!(
            super::prose_language_for_dialect(Dialect::German),
            crate::languages::LanguageFamily::German
        );
        assert_eq!(
            super::prose_language_for_dialect(Dialect::Portuguese),
            crate::languages::LanguageFamily::Portuguese
        );
        assert_eq!(
            super::prose_language_for_dialect(Dialect::American),
            crate::languages::LanguageFamily::English
        );
    }

    #[test]
    fn plaintext_dispatch_remains_plain_english_for_english_dialects() {
        let source = "The 1980s changed everything.";
        let parser =
            parser_for_prose("plaintext", Dialect::American, MarkdownOptions::default()).unwrap();

        let registry_tokens = token_kinds(parser.parse_str(source));
        let english_tokens = token_kinds(PlainEnglish.parse_str(source));

        assert_eq!(registry_tokens, english_tokens);
    }

    #[test]
    fn plaintext_dispatch_uses_plain_portuguese_for_portuguese() {
        let source = "Os anos 1980s mudaram.";
        let parser =
            parser_for_prose("plaintext", Dialect::Portuguese, MarkdownOptions::default()).unwrap();

        let registry_tokens = token_kinds(parser.parse_str(source));
        let portuguese_tokens = token_kinds(PlainPortuguese.parse_str(source));

        assert_eq!(registry_tokens, portuguese_tokens);
    }
}

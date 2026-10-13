use harper_core::Span;
use harper_core::linting::{Lint, LintKind, Suggestion};

use crate::{PluginLint, PluginSuggestion};

/// Guest side: describe a core lint for the wire. `rule` is the name of the rule that produced it.
pub fn lint_to_plugin(lint: &Lint, rule: &str) -> PluginLint {
    PluginLint {
        span: (lint.span.start, lint.span.end),
        rule: rule.to_owned(),
        kind: lint.lint_kind.to_string_key(),
        message: lint.message.clone(),
        suggestions: lint
            .suggestions
            .iter()
            .map(|s| match s {
                Suggestion::ReplaceWith(c) => PluginSuggestion::ReplaceWith(c.iter().collect()),
                Suggestion::InsertAfter(c) => PluginSuggestion::InsertAfter(c.iter().collect()),
                Suggestion::Remove => PluginSuggestion::Remove,
            })
            .collect(),
        priority: lint.priority,
    }
}

/// Host side: turn a wire lint into a core lint so existing consumers need no special cases.
/// Unknown kinds fall back to [`LintKind::Miscellaneous`].
pub fn plugin_to_lint(lint: &PluginLint) -> Lint {
    Lint {
        span: Span::new(lint.span.0, lint.span.1),
        lint_kind: LintKind::from_string_key(&lint.kind).unwrap_or(LintKind::Miscellaneous),
        suggestions: lint
            .suggestions
            .iter()
            .map(|s| match s {
                PluginSuggestion::ReplaceWith(t) => Suggestion::ReplaceWith(t.chars().collect()),
                PluginSuggestion::InsertAfter(t) => Suggestion::InsertAfter(t.chars().collect()),
                PluginSuggestion::Remove => Suggestion::Remove,
            })
            .collect(),
        message: lint.message.clone(),
        priority: lint.priority,
    }
}

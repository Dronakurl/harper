//! A tiny made-up language ("Echo") used to test the whole plugin path without shipping a
//! real second language in this repository. It flags the word "teh" and doubled words.

use std::collections::BTreeMap;

use harper_plugin_api::{
    DialectInfo, LanguagePlugin, LintRequest, PluginConfig, PluginInfo, PluginLint,
    PluginSuggestion, export_plugin,
};

struct Echo {
    user_words: Vec<String>,
}

impl LanguagePlugin for Echo {
    fn info() -> PluginInfo {
        PluginInfo {
            id: "xx".into(),
            name: "Echo".into(),
            plugin_version: env!("CARGO_PKG_VERSION").into(),
            abi: harper_plugin_api::ABI_VERSION,
            harper_core_version: "none".into(),
            dialects: vec![DialectInfo {
                id: "std".into(),
                name: "Standard".into(),
            }],
            default_dialect: Some("std".into()),
        }
    }

    fn new() -> Self {
        Self {
            user_words: Vec::new(),
        }
    }

    fn lint(&mut self, req: LintRequest) -> Vec<PluginLint> {
        let enabled = |rule: &str| {
            req.config
                .as_ref()
                .and_then(|c| c.get(rule).copied().flatten())
                .unwrap_or(true)
        };
        let chars: Vec<char> = req.text.chars().collect();
        let mut lints = Vec::new();
        let mut prev: Option<(usize, usize, String)> = None;
        let mut i = 0;
        while i < chars.len() {
            if !chars[i].is_alphabetic() {
                i += 1;
                continue;
            }
            let start = i;
            while i < chars.len() && chars[i].is_alphabetic() {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            let lower = word.to_lowercase();
            if enabled("Teh") && lower == "teh" && !self.user_words.contains(&word) {
                lints.push(PluginLint {
                    span: (start, i),
                    rule: "Teh".into(),
                    kind: "Spelling".into(),
                    message: "Did you mean “the”?".into(),
                    suggestions: vec![PluginSuggestion::ReplaceWith("the".into())],
                    priority: 63,
                });
            }
            if let Some((ps, _, pw)) = &prev
                && *pw == lower
                && enabled("Repeat")
            {
                lints.push(PluginLint {
                    span: (*ps, i),
                    rule: "Repeat".into(),
                    kind: "Repetition".into(),
                    message: "Repeated word.".into(),
                    suggestions: vec![PluginSuggestion::ReplaceWith(word.clone())],
                    priority: 31,
                });
            }
            prev = Some((start, i, lower));
        }
        lints
    }

    fn lint_descriptions(&self) -> BTreeMap<String, String> {
        BTreeMap::from([
            ("Teh".into(), "Flags “teh”.".into()),
            ("Repeat".into(), "Flags repeated words.".into()),
        ])
    }

    fn default_config(&self) -> PluginConfig {
        BTreeMap::from([("Teh".into(), Some(true)), ("Repeat".into(), Some(true))])
    }

    fn set_user_words(&mut self, words: Vec<String>) {
        self.user_words = words;
    }
}

export_plugin!(Echo);

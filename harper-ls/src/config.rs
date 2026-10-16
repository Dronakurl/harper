use std::path::{Path, PathBuf};

use anyhow::{Result, bail};
use dirs::{config_dir, data_local_dir};
use globset::{Glob, GlobSet};
use harper_core::{Dialect, linting::FlatConfig, parsers::MarkdownOptions};
use resolve_path::PathResolveExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
#[serde(rename_all = "camelCase")]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Information,
    Hint,
}

impl DiagnosticSeverity {
    /// Converts `self` to the equivalent LSP type.
    pub fn to_lsp(self) -> tower_lsp_server::lsp_types::DiagnosticSeverity {
        match self {
            DiagnosticSeverity::Error => tower_lsp_server::lsp_types::DiagnosticSeverity::ERROR,
            DiagnosticSeverity::Warning => tower_lsp_server::lsp_types::DiagnosticSeverity::WARNING,
            DiagnosticSeverity::Information => {
                tower_lsp_server::lsp_types::DiagnosticSeverity::INFORMATION
            }
            DiagnosticSeverity::Hint => tower_lsp_server::lsp_types::DiagnosticSeverity::HINT,
        }
    }
}

/// Configuration for how code actions are displayed.
/// Originally motivated by [#89](https://github.com/automattic/harper/issues/89).
#[derive(Debug, Clone, Default)]
pub struct CodeActionConfig {
    /// Instructs `harper-ls` to place unstable code actions last.
    /// In this case, "unstable" refers to their existence and action.
    ///
    /// For example, we always want to allow users to add "misspelled" elements
    /// to dictionary, regardless of the spelling suggestions.
    pub force_stable: bool,
}

impl CodeActionConfig {
    pub fn from_lsp_config(value: Value) -> Result<Self> {
        let mut base = CodeActionConfig::default();

        let Value::Object(value) = value else {
            bail!("The code action configuration must be an object.");
        };

        if let Some(force_stable_val) = value.get("ForceStable") {
            let Value::Bool(force_stable) = force_stable_val else {
                bail!("ForceStable must be a boolean value.");
            };
            base.force_stable = *force_stable;
        };

        Ok(base)
    }
}

#[derive(Debug, Clone)]
pub struct Config {
    pub user_dict_path: PathBuf,
    pub file_dict_path: PathBuf,
    pub workspace_dict_path: PathBuf,
    pub ignored_lints_path: PathBuf,
    pub stats_path: PathBuf,
    pub lint_config: FlatConfig,
    pub diagnostic_severity: DiagnosticSeverity,
    pub code_action_config: CodeActionConfig,
    pub isolate_english: bool,
    pub markdown_options: MarkdownOptions,
    pub dialect: Dialect,
    /// The language to check: `"en"` (built in), a plugin id such as `"de"` (needs a plugin file
    /// `harper-<language>.wasm` in `plugin_dir`), or `"auto"`, the default, which picks an installed
    /// plugin whose language the text is written in, and English otherwise.
    pub language: String,
    pub plugin_dir: PathBuf,
    /// Maximum length (in bytes) a file can have before it's skipped.
    /// Above this limit, the file will not be linted.
    pub max_file_length: usize,
    pub exclude_patterns: GlobSet,
    /// Delay in milliseconds after typing stops before diagnostics are published.
    /// Set to 0 to publish diagnostics immediately.
    pub diagnostic_delay_ms: u64,
}

impl Config {
    pub fn from_lsp_config(workspace_root: &Path, value: Value) -> Result<Self> {
        let mut base = Config::default();

        let workspace_root = workspace_root.canonicalize()?;
        let workspace_root = workspace_root.as_path();

        let Value::Object(value) = value else {
            bail!("Settings must be an object.");
        };

        let Some(Value::Object(value)) = value.get("harper-ls") else {
            bail!("Settings must contain a \"harper-ls\" key.");
        };

        if let Some(v) = value.get("userDictPath") {
            if !v.is_string() {
                bail!("userDict path must be a string.");
            }

            let path = v.as_str().unwrap();
            if !path.is_empty() {
                base.user_dict_path = path.try_resolve_in(workspace_root)?.to_path_buf();
            }
        }

        if let Some(v) = value.get("fileDictPath") {
            if !v.is_string() {
                bail!("fileDict path must be a string.");
            }

            let path = v.as_str().unwrap();
            if !path.is_empty() {
                base.file_dict_path = path.try_resolve_in(workspace_root)?.to_path_buf();
            }
        }

        if let Some(v) = value.get("workspaceDictPath") {
            if !v.is_string() {
                bail!("workspaceDict path must be a string.");
            }
            let path = v.as_str().unwrap();
            if !path.is_empty() {
                base.workspace_dict_path = path.try_resolve_in(workspace_root)?.to_path_buf();
            }
        } else {
            // Resolve the default path in the project root
            base.workspace_dict_path = base
                .workspace_dict_path
                .try_resolve_in(workspace_root)?
                .to_path_buf();
        }

        if let Some(v) = value.get("ignoredLintsPath") {
            if !v.is_string() {
                bail!("ignoredLintsPath path must be a string.");
            }

            let path = v.as_str().unwrap();
            if !path.is_empty() {
                base.ignored_lints_path = path.try_resolve_in(workspace_root)?.to_path_buf();
            }
        }

        if let Some(v) = value.get("statsPath") {
            if let Value::String(path) = v {
                base.stats_path = path.try_resolve_in(workspace_root)?.to_path_buf();
            } else {
                bail!("statsPath must be a string.");
            }
        }

        if let Some(v) = value.get("linters") {
            base.lint_config = serde_json::from_value(v.clone())?;
        }

        if let Some(v) = value.get("diagnosticSeverity") {
            base.diagnostic_severity = serde_json::from_value(v.clone())?;
        }

        if let Some(v) = value.get("dialect") {
            match v.as_str().and_then(language_from_dialect_name) {
                // An English dialect only picks the dialect, like Harper's own names for them do.
                Some((language, dialect)) => match dialect {
                    Some(dialect) => base.dialect = dialect,
                    None => base.language = language.to_owned(),
                },
                None => base.dialect = serde_json::from_value(v.clone())?,
            }
        }

        if let Some(v) = value.get("language") {
            // `{"German": "Swiss"}` and `{"English": "British"}`: how the multilingual build of
            // harper-ls spelled this setting.
            let object_form = v.as_object().filter(|o| o.len() == 1).and_then(|o| {
                let (family, dialect) = o.iter().next()?;
                let family = family.to_ascii_lowercase();
                if family == "english" {
                    let dialect = dialect.as_str().and_then(language_from_dialect_name);
                    Some(("en", dialect.and_then(|(_, d)| d)))
                } else {
                    language_from_dialect_name(&family).filter(|(l, _)| *l != "en")
                }
            });
            let language = match (object_form, v.as_str()) {
                (Some((language, dialect)), _) => {
                    base.dialect = dialect.unwrap_or(base.dialect);
                    language
                }
                (None, Some(language)) => language,
                (None, None) => bail!("language must be a string."),
            };
            if language.is_empty()
                || !language
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
            {
                bail!("language must be a plugin id such as \"en\" or \"de\".");
            }
            base.language = language.to_owned();
        }

        if let Some(v) = value.get("pluginDirectory") {
            let Some(path) = v.as_str() else {
                bail!("pluginDirectory must be a string.");
            };
            if !path.is_empty() {
                base.plugin_dir = path.try_resolve_in(workspace_root)?.to_path_buf();
            }
        }

        if let Some(v) = value.get("codeActions") {
            base.code_action_config = CodeActionConfig::from_lsp_config(v.clone())?;
        }

        if let Some(v) = value.get("isolateEnglish") {
            if let Value::Bool(v) = v {
                base.isolate_english = *v;
            } else {
                bail!("isolateEnglish path must be a boolean.");
            }
        }

        if let Some(v) = value.get("maxFileLength") {
            base.max_file_length = serde_json::from_value(v.clone())?;
        }

        if let Some(v) = value.get("markdown")
            && let Some(v) = v.get("IgnoreLinkTitle")
        {
            base.markdown_options.ignore_link_title = serde_json::from_value(v.clone())?;
        }

        if let Some(v) = value.get("excludePatterns") {
            let Some(a) = v.as_array() else {
                bail!("excludePatterns must be an array.");
            };

            let patterns: Vec<Value> = a.to_vec();
            if !patterns.is_empty() {
                let mut builder = GlobSet::builder();

                for pattern in patterns {
                    builder.add(Glob::new(pattern.as_str().unwrap())?);
                }

                base.exclude_patterns = builder.build()?;
            }
        }

        if let Some(v) = value.get("diagnosticDelayMs") {
            base.diagnostic_delay_ms = serde_json::from_value(v.clone())?;
        }

        Ok(base)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            user_dict_path: config_dir().unwrap().join("harper-ls/dictionary.txt"),
            file_dict_path: data_local_dir()
                .unwrap()
                .join("harper-ls/file_dictionaries/"),
            workspace_dict_path: ".harper-dictionary.txt".into(),
            ignored_lints_path: data_local_dir().unwrap().join("harper-ls/ignored_lints/"),
            stats_path: data_local_dir().unwrap().join("harper-ls/stats.txt"),
            lint_config: FlatConfig::default(),
            diagnostic_severity: DiagnosticSeverity::Hint,
            code_action_config: CodeActionConfig::default(),
            isolate_english: false,
            markdown_options: MarkdownOptions::default(),
            dialect: Dialect::American,
            language: "auto".to_owned(),
            plugin_dir: config_dir().unwrap().join("harper/plugins"),
            max_file_length: 120_000,
            exclude_patterns: GlobSet::empty(),
            diagnostic_delay_ms: 0,
        }
    }
}

/// The language and English dialect a `dialect` value names. Besides Harper's dialect names
/// (handled by serde), this takes the spellings the multilingual build of harper-ls accepted, so
/// a configuration written for it keeps working: `"uk"`, `"en-GB"`, `"German"`, `"Swiss"`, ...
fn language_from_dialect_name(name: &str) -> Option<(&'static str, Option<Dialect>)> {
    let english = |d| Some(("en", Some(d)));
    match name.to_ascii_lowercase().as_str() {
        "us" | "usa" | "america" | "american" | "en-us" | "en_us" => english(Dialect::American),
        "uk" | "gb" | "british" | "britain" | "en-gb" | "en_gb" => english(Dialect::British),
        "au" | "aus" | "australia" | "australian" | "en-au" | "en_au" => {
            english(Dialect::Australian)
        }
        "in" | "india" | "indian" | "bharat" | "en-in" | "en_in" => english(Dialect::Indian),
        "ca" | "canada" | "canadian" | "en-ca" | "en_ca" => english(Dialect::Canadian),
        "de" | "german" | "deutsch" | "de-de" | "de_de" | "at" | "austria" | "austrian"
        | "de-at" | "de_at" | "ch" | "switzerland" | "swiss" | "de-ch" | "de_ch" => {
            Some(("de", None))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use harper_core::Dialect;
    use serde_json::{Value, json};

    use super::Config;

    /// Settings written for the multilingual build of harper-ls.
    #[test]
    fn accepts_the_language_spellings_of_the_multilingual_build() {
        let parse = |v: Value| Config::from_lsp_config(std::path::Path::new("."), v).unwrap();

        let c = parse(json!({ "harper-ls": { "dialect": "German" } }));
        assert_eq!(c.language, "de");
        let c = parse(json!({ "harper-ls": { "dialect": "Swiss" } }));
        assert_eq!(c.language, "de");
        let c = parse(json!({ "harper-ls": { "dialect": "uk" } }));
        assert_eq!((c.language.as_str(), c.dialect), ("auto", Dialect::British));
        let c = parse(json!({ "harper-ls": { "language": { "German": "Austrian" } } }));
        assert_eq!(c.language, "de");
        let c = parse(json!({ "harper-ls": { "language": { "English": "Canadian" } } }));
        assert_eq!((c.language.as_str(), c.dialect), ("en", Dialect::Canadian));
        let c = parse(json!({ "harper-ls": { "dialect": "British", "language": "de" } }));
        assert_eq!((c.language.as_str(), c.dialect), ("de", Dialect::British));
        assert!(
            Config::from_lsp_config(
                std::path::Path::new("."),
                json!({ "harper-ls": { "dialect": "Klingon" } })
            )
            .is_err()
        );
    }

    #[test]
    fn language_defaults_to_auto_and_validates() {
        assert_eq!(Config::default().language, "auto");

        let ok = Config::from_lsp_config(
            std::path::Path::new("."),
            json!({ "harper-ls": { "language": "de", "pluginDirectory": "plugins" } }),
        )
        .unwrap();
        assert_eq!(ok.language, "de");
        assert!(ok.plugin_dir.ends_with("plugins"));

        for bad in [json!(""), json!("../de"), json!(3)] {
            assert!(
                Config::from_lsp_config(
                    std::path::Path::new("."),
                    json!({ "harper-ls": { "language": bad } }),
                )
                .is_err()
            );
        }
    }

    #[test]
    fn parses_diagnostic_delay() {
        let config = Config::from_lsp_config(
            std::path::Path::new("."),
            json!({
                "harper-ls": {
                    "diagnosticDelayMs": 750
                }
            }),
        )
        .unwrap();

        assert_eq!(config.diagnostic_delay_ms, 750);
    }

    #[test]
    fn defaults_diagnostic_delay_to_zero() {
        let config = Config::default();

        assert_eq!(config.diagnostic_delay_ms, 0);
    }

    #[test]
    fn parses_stats_path_without_touching_file_dict_path() {
        let config = Config::from_lsp_config(
            std::path::Path::new("."),
            json!({
                "harper-ls": {
                    "statsPath": ".harper-stats.txt"
                }
            }),
        )
        .unwrap();

        assert!(config.stats_path.ends_with(".harper-stats.txt"));
        assert!(
            config
                .file_dict_path
                .ends_with("harper-ls/file_dictionaries/"),
            "file dictionary path should keep its default value"
        );
    }
}

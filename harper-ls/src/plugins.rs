//! Connects `harper-ls` to language plugins (WebAssembly modules, see
//! `LANGUAGE_PLUGINS_PLAN.md`). All `plugins` feature handling lives in this file; the rest of
//! `harper-ls` only sees [`PluginRegistry`] and [`PluginLinter`].

/// The format a plugin is asked to parse for an editor `languageId`, or `None` if plugins cannot
/// lint that kind of document (yet). Source code comments need the `regions` request.
pub fn plugin_format(language_id: &str) -> Option<&'static str> {
    match language_id {
        "markdown" | "quarto" => Some("markdown"),
        "org" => Some("org"),
        "typst" => Some("typst"),
        "plaintext" | "text" | "mail" | "git-commit" | "gitcommit" | "scminput" => Some("plain"),
        _ => None,
    }
}

#[cfg(feature = "plugins")]
mod imp {
    use std::collections::{BTreeMap, HashMap, HashSet};
    use std::path::{Path, PathBuf};
    use std::sync::{Arc, Mutex};

    use harper_core::linting::{FlatConfig, Lint};
    use harper_plugin_api::{LintRequest, PluginConfig, plugin_to_lint};
    use harper_plugin_host::{LoadedPlugin, PluginHost};
    use tracing::{error, warn};

    /// Finds and loads plugins on demand. Shared by all documents.
    pub struct PluginRegistry {
        host: Option<PluginHost>,
        state: Mutex<State>,
    }

    #[derive(Default)]
    struct State {
        loaded: HashMap<PathBuf, Arc<LoadedPlugin>>,
        /// Files that failed to load, so that each document update does not log the same error.
        failed: HashSet<PathBuf>,
    }

    impl PluginRegistry {
        pub fn new() -> Self {
            let host = PluginHost::new()
                .map_err(|e| error!("Could not start the plugin runtime: {e}"))
                .ok();
            Self {
                host,
                state: Mutex::default(),
            }
        }

        /// Forget load failures, so that a fixed or newly added plugin is tried again.
        pub fn retry_failed(&self) {
            self.lock().failed.clear();
        }

        fn lock(&self) -> std::sync::MutexGuard<'_, State> {
            self.state.lock().unwrap_or_else(|e| e.into_inner())
        }

        /// Loads `<dir>/harper-<language>.wasm` the first time it is needed.
        fn load(&self, language: &str, dir: &Path) -> Option<Arc<LoadedPlugin>> {
            let host = self.host.as_ref()?;
            let path = dir.join(format!("harper-{language}.wasm"));

            let mut state = self.lock();
            if let Some(p) = state.loaded.get(&path) {
                return Some(p.clone());
            }
            if state.failed.contains(&path) {
                return None;
            }
            if !path.exists() {
                warn!(
                    "Language `{language}` is configured, but there is no plugin at {}.",
                    path.display()
                );
                state.failed.insert(path);
                return None;
            }

            let cache = dirs::cache_dir().map(|d| d.join("harper").join("plugins"));
            let loaded = match &cache {
                Some(cache) => host.load_file_cached(&path, cache),
                None => host.load_file(&path),
            };
            match loaded {
                Ok(plugin) if plugin.info().id == language => {
                    let plugin = Arc::new(plugin);
                    state.loaded.insert(path, plugin.clone());
                    Some(plugin)
                }
                Ok(plugin) => {
                    error!(
                        "{} is a plugin for `{}`, not `{language}`.",
                        path.display(),
                        plugin.info().id
                    );
                    state.failed.insert(path);
                    None
                }
                Err(e) => {
                    error!("Could not load plugin {}: {e}", path.display());
                    state.failed.insert(path);
                    None
                }
            }
        }

        pub fn linter_for(
            &self,
            language: &str,
            dir: &Path,
            format: &'static str,
            config: &FlatConfig,
            user_words: Vec<String>,
        ) -> Option<PluginLinter> {
            let plugin = self.load(language, dir)?;
            // `FlatConfig` serializes as a plain `rule -> Option<bool>` map, as does `PluginConfig`.
            let config = serde_json::to_value(config)
                .and_then(serde_json::from_value::<PluginConfig>)
                .unwrap_or_default();
            Some(PluginLinter {
                plugin,
                format,
                config,
                user_words,
            })
        }
    }

    /// A plugin together with the per-document settings it lints with.
    #[derive(Clone)]
    pub struct PluginLinter {
        plugin: Arc<LoadedPlugin>,
        format: &'static str,
        config: PluginConfig,
        user_words: Vec<String>,
    }

    impl PluginLinter {
        /// Lints grouped by the rule that produced them, like `LintGroup::organized_lints`.
        pub fn organized_lints(&self, text: &str) -> BTreeMap<String, Vec<Lint>> {
            let mut out = BTreeMap::<String, Vec<Lint>>::new();

            // The plugin is shared between documents; give it this document's words first.
            if let Err(e) = self.plugin.set_user_words(self.user_words.clone()) {
                error!("Plugin refused the user dictionary: {e}");
            }

            let mut req = LintRequest::plain(text);
            req.format = self.format.to_owned();
            req.config = Some(self.config.clone());

            match self.plugin.lint(&req) {
                Ok(lints) => {
                    for lint in lints {
                        out.entry(lint.rule.clone())
                            .or_default()
                            .push(plugin_to_lint(&lint));
                    }
                }
                Err(e) => error!("Plugin failed while linting: {e}"),
            }
            out
        }
    }
}

#[cfg(feature = "plugins")]
pub use imp::{PluginLinter, PluginRegistry};

/// Stand-ins so that the rest of the crate compiles without the `plugins` feature.
/// A language other than English then simply has no linter.
#[cfg(not(feature = "plugins"))]
mod stub {
    use std::collections::BTreeMap;
    use std::path::Path;

    use harper_core::linting::{FlatConfig, Lint};

    pub struct PluginRegistry;

    impl PluginRegistry {
        pub fn new() -> Self {
            Self
        }

        pub fn retry_failed(&self) {}

        pub fn linter_for(
            &self,
            language: &str,
            _dir: &Path,
            _format: &'static str,
            _config: &FlatConfig,
            _user_words: Vec<String>,
        ) -> Option<PluginLinter> {
            tracing::warn!(
                "Language `{language}` needs plugin support, which this build of harper-ls lacks."
            );
            None
        }
    }

    #[derive(Clone)]
    pub struct PluginLinter;

    impl PluginLinter {
        pub fn organized_lints(&self, _text: &str) -> BTreeMap<String, Vec<Lint>> {
            BTreeMap::new()
        }
    }
}

#[cfg(not(feature = "plugins"))]
pub use stub::{PluginLinter, PluginRegistry};

#[cfg(all(test, feature = "plugins"))]
mod tests {
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use harper_core::linting::FlatConfig;
    use harper_core::spell::MutableDictionary;
    use harper_core::{Document, parsers::PlainEnglish};

    use super::*;
    use crate::config::{CodeActionConfig, DiagnosticSeverity};
    use crate::document_state::DocumentState;

    /// Builds the `echo` example plugin and installs it as `harper-xx.wasm` in a fresh directory.
    /// Returns `None` (so the test is skipped) if the wasm target is unavailable, except in CI.
    fn install_echo() -> Option<PathBuf> {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()?
            .to_path_buf();
        let target_dir = root.join("target").join("plugin-fixtures");
        let built = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
            .current_dir(&root)
            .args(["build", "--release", "-p", "harper-plugin-api"])
            .args(["--example", "echo", "--target", "wasm32-unknown-unknown"])
            .arg("--target-dir")
            .arg(&target_dir)
            .status()
            .ok()?
            .success();
        if !built {
            assert!(
                std::env::var_os("CI").is_none(),
                "the echo plugin must build in CI"
            );
            eprintln!("skipping: could not build the echo plugin");
            return None;
        }

        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("harper-ls-plugins-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).ok()?;
        std::fs::copy(
            target_dir.join("wasm32-unknown-unknown/release/examples/echo.wasm"),
            dir.join("harper-xx.wasm"),
        )
        .ok()?;
        Some(dir)
    }

    fn state_with_plugin(dir: &Path, text: &str, words: Vec<String>) -> Option<DocumentState> {
        let registry = PluginRegistry::new();
        let plugin = registry.linter_for("xx", dir, "plain", &FlatConfig::default(), words)?;
        let dict = std::sync::Arc::new(MutableDictionary::new());
        Some(DocumentState {
            document: Document::new(text, &PlainEnglish, &dict),
            plugin: Some(plugin),
            ..Default::default()
        })
    }

    #[test]
    fn maps_editor_language_ids_to_plugin_formats() {
        assert_eq!(plugin_format("markdown"), Some("markdown"));
        assert_eq!(plugin_format("plaintext"), Some("plain"));
        assert_eq!(plugin_format("rust"), None);
    }

    #[test]
    fn plugin_lints_become_diagnostics_and_code_actions() {
        let Some(dir) = install_echo() else { return };
        let mut state = state_with_plugin(&dir, "Teh cat sat.", vec![]).unwrap();

        let diagnostics = state.generate_diagnostics(DiagnosticSeverity::Hint);
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].range.start.character, 0);
        assert_eq!(diagnostics[0].range.end.character, 3);

        let actions =
            state.generate_code_actions(diagnostics[0].range, &CodeActionConfig::default());
        assert!(!actions.is_empty());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn user_words_silence_plugin_lints() {
        let Some(dir) = install_echo() else { return };
        let mut state = state_with_plugin(&dir, "Teh cat.", vec!["Teh".into()]).unwrap();
        assert!(
            state
                .generate_diagnostics(DiagnosticSeverity::Hint)
                .is_empty()
        );
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn missing_or_mismatched_plugin_gives_no_linter() {
        let dir = std::env::temp_dir().join("harper-ls-no-plugins");
        let registry = PluginRegistry::new();
        assert!(
            registry
                .linter_for("zz", &dir, "plain", &FlatConfig::default(), vec![])
                .is_none()
        );

        let Some(dir) = install_echo() else { return };
        // The file is harper-xx.wasm, but claims to be "xx"; asking for "yy" must not load it.
        std::fs::copy(dir.join("harper-xx.wasm"), dir.join("harper-yy.wasm")).unwrap();
        assert!(
            registry
                .linter_for("yy", &dir, "plain", &FlatConfig::default(), vec![])
                .is_none()
        );
        let _ = std::fs::remove_dir_all(dir);
    }
}

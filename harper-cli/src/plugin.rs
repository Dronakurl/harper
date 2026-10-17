//! Language plugin support for `harper-cli`. All `plugins` feature handling
//! lives in this file.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use anyhow::Result;
use harper_core::linting::Lint;

/// The format a plugin parses a whole file of this name in, or `None` when it gets only parts of
/// it: the comments of source code, the text of TeX, AsciiDoc, ... (the parts `harper-cli`'s own
/// parser for the file finds; see `FileInput::get_parser`). No name (standard input, text on the
/// command line) and anything unknown is plain text.
pub fn format_for_path(path: Option<&Path>) -> Option<&'static str> {
    let Some(path) = path else {
        return Some("plain");
    };
    match path.extension().and_then(|e| e.to_str()) {
        Some("md" | "markdown" | "mkd" | "mdwn" | "mdown" | "mdtxt" | "mdtext" | "qmd") => {
            Some("markdown")
        }
        Some("org") => Some("org"),
        Some("typ") => Some("typst"),
        Some("txt") => Some("plain"),
        Some(
            "ink" | "lhs" | "tex" | "latex" | "sty" | "cls" | "dtx" | "py" | "pyi" | "adoc"
            | "asciidoc",
        ) => None,
        _ if harper_comments::CommentParser::new_from_filename(
            path,
            harper_core::parsers::MarkdownOptions::default(),
        )
        .is_some() =>
        {
            None
        }
        _ => Some("plain"),
    }
}

/// `harper-cli plugin ...`
#[derive(clap::Subcommand)]
pub enum PluginCommand {
    /// List the plugins in the plugin directory.
    List,
    /// Copy a plugin file into the plugin directory (after checking that it loads).
    Install {
        /// A `harper-<language>.wasm` file.
        file: PathBuf,
    },
    /// Remove the plugin for a language from the plugin directory.
    Remove {
        /// The language id, e.g. `de`.
        language: String,
    },
}

#[cfg(feature = "plugins")]
mod imp {
    use std::sync::{Arc, Mutex};

    use anyhow::{Context, anyhow, bail};
    use harper_plugin_api::{LintRequest, plugin_to_lint};
    use harper_plugin_host::{LoadedPlugin, PluginHost, default_plugin_dir, discover};

    use super::*;

    /// A loaded language plugin, cheap to clone and share between threads.
    #[derive(Clone)]
    pub struct PluginLinter {
        plugin: Arc<LoadedPlugin>,
        /// The user dictionary. A file's own words are added for that file's request only.
        user_words: Arc<Mutex<Vec<String>>>,
    }

    impl PluginLinter {
        /// `path` wins; otherwise `language` is looked up in the plugin directory.
        /// `None` for both means "built-in English".
        pub fn load(language: Option<&str>, path: Option<&Path>) -> Result<Option<Self>> {
            let path = match (path, language) {
                (Some(p), _) => p.to_path_buf(),
                (None, Some(l)) if l != "en" && l != "auto" => {
                    let exact = plugin_path(l)?;
                    if exact.exists() {
                        exact
                    } else {
                        // A name, alias or dialect the plugin answers to (`German`, `Swiss`).
                        match Self::load_all()?
                            .into_iter()
                            .find(|p| p.plugin.info().answers_to(l))
                        {
                            Some(found) => return Ok(Some(found)),
                            None => bail!(
                                "no language plugin for `{l}`: there is no {}, and no plugin \
                                 there answers to `{l}`",
                                exact.display()
                            ),
                        }
                    }
                }
                _ => return Ok(None),
            };
            let plugin = load_file(&path)?;
            if let Some(l) = language
                && l != "en"
                && l != "auto"
                && !plugin.info().answers_to(l)
            {
                bail!(
                    "{} is a plugin for `{}`, not `{l}`",
                    path.display(),
                    plugin.info().id
                );
            }
            Ok(Some(Self::new(plugin)))
        }

        /// Every plugin in the plugin directory, for `--language auto`.
        pub fn load_all() -> Result<Vec<Self>> {
            let mut all = Vec::new();
            for path in discover(&plugin_dir()?) {
                match load_file(&path) {
                    Ok(plugin) => all.push(Self::new(plugin)),
                    Err(err) => eprintln!("Warning: skipping {}: {err:#}", path.display()),
                }
            }
            Ok(all)
        }

        fn new(plugin: LoadedPlugin) -> Self {
            Self {
                plugin: Arc::new(plugin),
                user_words: Arc::default(),
            }
        }

        /// The language id of the plugin (`de`).
        pub fn id(&self) -> String {
            self.plugin.info().id.clone()
        }

        /// How sure the plugin is that `text` is in its language (0 to 1).
        pub fn confidence(&self, text: &str) -> f32 {
            // The start of a document says enough.
            let sample: String = text.chars().take(4000).collect();
            self.plugin.detect(&sample).ok().flatten().unwrap_or(0.0)
        }

        /// The most confident of `plugins` for `text`, if any is at least 50 % sure.
        pub fn best_for<'a>(plugins: &'a [Self], text: &str) -> Option<&'a Self> {
            plugins
                .iter()
                .map(|p| (p, p.confidence(text)))
                .filter(|(_, c)| *c >= 0.5)
                .max_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(p, _)| p)
        }

        pub fn set_user_words(&self, words: Vec<String>) -> Result<()> {
            *self.user_words.lock().unwrap_or_else(|e| e.into_inner()) = words;
            Ok(())
        }

        /// Lints grouped by rule name, like `LintGroup::organized_lints`. With `regions` (the comments
        /// of a source file, the text of a TeX file, ...) only those character ranges are linted.
        /// `file_words` are the file's own dictionary, known for this one request: the files are
        /// linted in parallel, so the words travel with the request.
        pub fn organized_lints(
            &self,
            text: &str,
            format: &str,
            regions: Option<&[(usize, usize)]>,
            file_words: &[String],
        ) -> Result<BTreeMap<String, Vec<Lint>>> {
            let mut words = self
                .user_words
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .clone();
            words.extend(file_words.iter().cloned());
            self.lint(text, format, regions, words)
        }

        fn lint(
            &self,
            text: &str,
            format: &str,
            regions: Option<&[(usize, usize)]>,
            user_words: Vec<String>,
        ) -> Result<BTreeMap<String, Vec<Lint>>> {
            let mut req = LintRequest::plain(text);
            req.format = format.to_owned();
            req.user_words = Some(user_words);
            if let Some(regions) = regions {
                req.regions = Some(
                    regions
                        .iter()
                        .map(|&(start, end)| harper_plugin_api::Region {
                            start,
                            end,
                            format: "plain".to_owned(),
                        })
                        .collect(),
                );
            }

            let mut out = BTreeMap::<String, Vec<Lint>>::new();
            for lint in self.plugin.lint(&req)? {
                out.entry(lint.rule.clone())
                    .or_default()
                    .push(plugin_to_lint(&lint));
            }
            Ok(out)
        }
    }

    fn plugin_dir() -> Result<PathBuf> {
        default_plugin_dir().ok_or_else(|| anyhow!("could not determine the config directory"))
    }

    fn plugin_path(language: &str) -> Result<PathBuf> {
        Ok(plugin_dir()?.join(format!("harper-{language}.wasm")))
    }

    fn load_file(path: &Path) -> Result<LoadedPlugin> {
        let host = PluginHost::new()?;
        // The same cache of compiled modules harper-ls uses: compiling a plugin takes seconds.
        match dirs::cache_dir().map(|d| d.join("harper").join("plugins")) {
            Some(cache) => host.load_file_cached(path, cache),
            None => host.load_file(path),
        }
        .with_context(|| format!("could not load plugin {}", path.display()))
    }

    pub fn run(command: PluginCommand) -> Result<()> {
        let dir = plugin_dir()?;
        match command {
            PluginCommand::List => {
                println!("Plugin directory: {}", dir.display());
                for path in discover(&dir) {
                    match load_file(&path) {
                        Ok(p) => {
                            let i = p.info();
                            println!(
                                "{}\t{} ({})\tv{}\t{}",
                                i.id,
                                i.name,
                                path.file_name().unwrap_or_default().to_string_lossy(),
                                i.plugin_version,
                                if i.dialects.is_empty() {
                                    String::new()
                                } else {
                                    format!(
                                        "dialects: {}",
                                        i.dialects
                                            .iter()
                                            .map(|d| d.id.as_str())
                                            .collect::<Vec<_>>()
                                            .join(", ")
                                    )
                                }
                            );
                        }
                        Err(e) => println!("{}\tinvalid: {e:#}", path.display()),
                    }
                }
            }
            PluginCommand::Install { file } => {
                let plugin = load_file(&file)?;
                let id = &plugin.info().id;
                if !id
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
                {
                    bail!("the plugin has an unusable language id `{id}`");
                }
                std::fs::create_dir_all(&dir)?;
                let target = plugin_path(id)?;
                std::fs::copy(&file, &target)?;
                println!(
                    "Installed `{id}` ({}) to {}",
                    plugin.info().name,
                    target.display()
                );
            }
            PluginCommand::Remove { language } => {
                let target = plugin_path(&language)?;
                std::fs::remove_file(&target).with_context(|| {
                    format!("no plugin for `{language}` at {}", target.display())
                })?;
                println!("Removed {}", target.display());
            }
        }
        Ok(())
    }
}

#[cfg(feature = "plugins")]
pub use imp::{PluginLinter, run};

#[cfg(not(feature = "plugins"))]
mod stub {
    use super::*;

    #[derive(Clone)]
    pub struct PluginLinter;

    impl PluginLinter {
        pub fn load(language: Option<&str>, path: Option<&Path>) -> Result<Option<Self>> {
            if path.is_some() || language.is_some_and(|l| l != "en" && l != "auto") {
                anyhow::bail!("this build of harper-cli does not support language plugins");
            }
            Ok(None)
        }

        pub fn load_all() -> Result<Vec<Self>> {
            Ok(Vec::new())
        }

        pub fn id(&self) -> String {
            String::new()
        }

        pub fn best_for<'a>(_plugins: &'a [Self], _text: &str) -> Option<&'a Self> {
            None
        }

        pub fn set_user_words(&self, _words: Vec<String>) -> Result<()> {
            Ok(())
        }

        pub fn organized_lints(
            &self,
            _text: &str,
            _format: &str,
            _regions: Option<&[(usize, usize)]>,
            _file_words: &[String],
        ) -> Result<BTreeMap<String, Vec<Lint>>> {
            Ok(BTreeMap::new())
        }
    }

    pub fn run(_command: PluginCommand) -> Result<()> {
        anyhow::bail!("this build of harper-cli does not support language plugins")
    }
}

#[cfg(not(feature = "plugins"))]
pub use stub::{PluginLinter, run};

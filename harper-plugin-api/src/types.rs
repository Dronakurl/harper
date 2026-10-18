use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// Rule name → enabled (`None` means "use the plugin's default").
/// Same shape as [`harper_core::linting::FlatConfig`] when serialized.
pub type PluginConfig = BTreeMap<String, Option<bool>>;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DialectInfo {
    pub id: String,
    pub name: String,
    /// Other names for the dialect, such as `"de-CH"` or `"Swiss"`; see [`PluginInfo::aliases`].
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginInfo {
    /// Language code, e.g. `"de"`.
    pub id: String,
    /// Display name in the language itself, e.g. `"Deutsch"`.
    pub name: String,
    pub plugin_version: String,
    pub abi: u32,
    pub harper_core_version: String,
    #[serde(default)]
    pub dialects: Vec<DialectInfo>,
    #[serde(default)]
    pub default_dialect: Option<String>,
    /// Other names people use for the language, such as `"German"`, `"Deutsch"` or `"de-DE"`.
    /// A host accepts them, and the names and aliases of the dialects, wherever a user names a
    /// language, so it needs no list of languages of its own. Matched ignoring case.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// What the plugin can do beyond the first version of the messages, such as
    /// [`FEATURE_REQUEST_WORDS`]. [`dispatch`](crate::dispatch) adds the ones the API crate the
    /// plugin was built with has, so a plugin leaves this empty. A plugin built before a feature
    /// existed does not list it, which is how a host knows to do without.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub features: Vec<String>,
}

/// The plugin reads [`LintRequest::user_words`]. Without it a host has to send the words with
/// `set_user_words` before the request.
pub const FEATURE_REQUEST_WORDS: &str = "request_words";

/// Every feature this version of the API crate gives a plugin built with it.
pub const FEATURES: &[&str] = &[FEATURE_REQUEST_WORDS];

impl PluginInfo {
    /// Whether the plugin lists `feature`.
    pub fn supports(&self, feature: &str) -> bool {
        self.features.iter().any(|f| f == feature)
    }

    /// Whether `name` names this plugin's language: its id, its name, an alias, or one of its
    /// dialects (ignoring case).
    pub fn answers_to(&self, name: &str) -> bool {
        let is = |candidate: &String| candidate.eq_ignore_ascii_case(name);
        is(&self.id)
            || is(&self.name)
            || self.aliases.iter().any(is)
            || self.dialect_named(name).is_some()
    }

    /// The dialect `name` names: its id, its name or an alias (ignoring case).
    pub fn dialect_named(&self, name: &str) -> Option<&DialectInfo> {
        let is = |candidate: &String| candidate.eq_ignore_ascii_case(name);
        self.dialects
            .iter()
            .find(|d| is(&d.id) || is(&d.name) || d.aliases.iter().any(is))
    }
}

/// A slice of the text that should be linted on its own, e.g. a code comment.
/// Offsets are in characters of the full text.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Region {
    pub start: usize,
    pub end: usize,
    pub format: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LintRequest {
    pub text: String,
    /// `"plain"`, `"markdown"`, `"org"`, `"typst"`, ...
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default)]
    pub dialect: Option<String>,
    #[serde(default)]
    pub config: Option<PluginConfig>,
    /// When present, only these regions are linted (as `format`), and spans are reported
    /// relative to `text`.
    #[serde(default)]
    pub regions: Option<Vec<Region>>,
    /// The words of the user dictionaries that apply to this text (user, workspace and the
    /// file's own), for this language only. They belong to the request, so lints of different
    /// files may run in any order without touching each other. When absent, the plugin keeps
    /// the words of the last `set_user_words`.
    #[serde(default)]
    pub user_words: Option<Vec<String>>,
}

fn default_format() -> String {
    "plain".to_owned()
}

impl LintRequest {
    pub fn plain(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            format: default_format(),
            dialect: None,
            config: None,
            regions: None,
            user_words: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginSuggestion {
    ReplaceWith(String),
    InsertAfter(String),
    Remove,
}

/// A stable lint type, so that `harper_core::Lint` may change without breaking old plugins.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginLint {
    /// Character offsets (not bytes, not UTF-16): `[start, end)`.
    pub span: (usize, usize),
    #[serde(default)]
    pub rule: String,
    /// `LintKind` key such as `"Spelling"`.
    pub kind: String,
    pub message: String,
    #[serde(default)]
    pub suggestions: Vec<PluginSuggestion>,
    #[serde(default)]
    pub priority: u8,
}

/// The envelope sent to `harper_call`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "method", content = "params", rename_all = "snake_case")]
pub enum Request {
    Info,
    Lint(LintRequest),
    LintDescriptions,
    DefaultConfig,
    SetUserWords {
        words: Vec<String>,
    },
    Detect {
        text: String,
        /// The share of the words of `text` that Harper's English dictionary knows (0 to 1), when
        /// the host has that dictionary: the best sign that a text is English after all. Hosts
        /// before it was added leave it out.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        english_ratio: Option<f32>,
    },
}

/// The envelope returned by `harper_call`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Response<T> {
    Ok(T),
    Err(String),
}

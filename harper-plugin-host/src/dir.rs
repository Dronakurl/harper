use std::path::{Path, PathBuf};

/// `<config_dir>/harper/plugins`, where plugins are looked up unless configured otherwise.
pub fn default_plugin_dir() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("harper").join("plugins"))
}

/// `<cache_dir>/harper/plugins`, where hosts keep the compiled modules (see
/// [`PluginHost::load_file_cached`](crate::PluginHost::load_file_cached)).
pub fn default_cache_dir() -> Option<PathBuf> {
    dirs::cache_dir().map(|d| d.join("harper").join("plugins"))
}

/// Paths of all `*.wasm` files directly inside `dir`, sorted. A missing directory is empty.
pub fn discover(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut found: Vec<PathBuf> = entries
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|e| e == "wasm"))
        .collect();
    found.sort();
    found
}

/// The language of the user's interface as a BCP 47 tag, from `LC_ALL`, `LC_MESSAGES` or `LANG`
/// (`de_AT.UTF-8` gives `de-AT`), for [`harper_plugin_api::LintRequest::message_language`].
/// `None` when none is set or it is `C` or `POSIX`.
pub fn message_language_from_env() -> Option<String> {
    ["LC_ALL", "LC_MESSAGES", "LANG"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find(|value| !value.is_empty())
        .and_then(|value| parse_locale(&value))
}

fn parse_locale(value: &str) -> Option<String> {
    let tag = value.split(['.', '@']).next()?.replace('_', "-");
    if tag.is_empty() || tag == "C" || tag == "POSIX" {
        None
    } else {
        Some(tag)
    }
}

#[cfg(test)]
mod locale_tests {
    use super::parse_locale;

    #[test]
    fn locales_become_bcp_47_tags() {
        assert_eq!(parse_locale("de_AT.UTF-8").as_deref(), Some("de-AT"));
        assert_eq!(parse_locale("sk_SK@euro").as_deref(), Some("sk-SK"));
        assert_eq!(parse_locale("en").as_deref(), Some("en"));
        assert_eq!(parse_locale("C.UTF-8"), None);
        assert_eq!(parse_locale("POSIX"), None);
    }
}

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

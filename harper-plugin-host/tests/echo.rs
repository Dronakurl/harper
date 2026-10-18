//! Builds the `echo` example plugin for wasm32 and drives it through the host.
//! Skipped (with a message) when the wasm target is not installed.

use std::path::PathBuf;
use std::process::Command;

use harper_plugin_api::{LintRequest, PluginSuggestion};
use harper_plugin_host::{Error, PluginHost, discover};

fn build_echo() -> Option<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()?
        .to_path_buf();
    let target_dir = root.join("target").join("plugin-fixtures");
    let status = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()))
        .current_dir(&root)
        .args([
            "build",
            "--release",
            "-p",
            "harper-plugin-api",
            "--example",
            "echo",
            "--target",
            "wasm32-unknown-unknown",
            "--target-dir",
        ])
        .arg(&target_dir)
        .status()
        .ok()?;
    status
        .success()
        .then(|| target_dir.join("wasm32-unknown-unknown/release/examples/echo.wasm"))
}

fn echo() -> Option<PathBuf> {
    let path = build_echo();
    if path.is_none() {
        assert!(
            std::env::var_os("CI").is_none(),
            "the echo plugin must build in CI"
        );
        eprintln!(
            "skipping: could not build the echo plugin (is the wasm32-unknown-unknown target installed?)"
        );
    }
    path
}

#[test]
fn info_and_lint() {
    let Some(path) = echo() else { return };
    let host = PluginHost::new().unwrap();
    let plugin = host.load_file(&path).unwrap();

    assert_eq!(plugin.info().id, "xx");
    assert_eq!(plugin.info().abi, 1);

    let lints = plugin
        .lint(&LintRequest::plain("Teh cat cat sat."))
        .unwrap();
    assert_eq!(lints.len(), 2);
    assert_eq!(lints[0].span, (0, 3));
    assert_eq!(lints[0].rule, "Teh");
    assert_eq!(
        lints[0].suggestions,
        vec![PluginSuggestion::ReplaceWith("the".into())]
    );
    assert_eq!(lints[1].span, (4, 11));
}

#[test]
fn converts_to_core_lints() {
    let Some(path) = echo() else { return };
    let plugin = PluginHost::new().unwrap().load_file(&path).unwrap();
    let lints = plugin.lint_core(&LintRequest::plain("teh")).unwrap();
    assert_eq!(lints.len(), 1);
    assert_eq!(lints[0].span.start, 0);
    assert_eq!(lints[0].span.end, 3);
    assert_eq!(lints[0].lint_kind.to_string_key(), "Spelling");
}

#[test]
fn spans_are_character_offsets() {
    let Some(path) = echo() else { return };
    let plugin = PluginHost::new().unwrap().load_file(&path).unwrap();
    // "Ünï" is 3 chars but 5 bytes; the span of "teh" must be in chars.
    let lints = plugin.lint(&LintRequest::plain("Ünï teh")).unwrap();
    assert_eq!(lints[0].span, (4, 7));
}

#[test]
fn user_words_and_config() {
    let Some(path) = echo() else { return };
    let plugin = PluginHost::new().unwrap().load_file(&path).unwrap();
    plugin.set_user_words(vec!["teh".into()]).unwrap();
    assert!(plugin.lint(&LintRequest::plain("teh")).unwrap().is_empty());

    assert_eq!(
        plugin.default_config().unwrap().get("Teh"),
        Some(&Some(true))
    );
    assert!(plugin.lint_descriptions().unwrap().contains_key("Repeat"));
    assert_eq!(plugin.detect("anything").unwrap(), None);
}

#[test]
fn cached_load_matches_plain_load() {
    let Some(path) = echo() else { return };
    let cache = std::env::temp_dir().join(format!("harper-plugin-cache-{}", std::process::id()));
    let host = PluginHost::new().unwrap();
    for _ in 0..2 {
        let plugin = host.load_file_cached(&path, &cache).unwrap();
        assert_eq!(plugin.lint(&LintRequest::plain("teh")).unwrap().len(), 1);
    }
    assert_eq!(std::fs::read_dir(&cache).unwrap().count(), 1);
    let _ = std::fs::remove_dir_all(&cache);
}

#[test]
fn rejects_non_plugins() {
    let host = PluginHost::new().unwrap();
    // An empty but valid module: (module)
    let err = host.load_bytes(b"\0asm\x01\0\0\0").err().unwrap();
    assert!(
        matches!(err, Error::MissingExport("harper_plugin_abi_version")),
        "{err}"
    );
    assert!(host.load_bytes(b"not wasm").is_err());
}

#[test]
fn discovers_wasm_files_only() {
    let dir = std::env::temp_dir().join(format!("harper-plugin-dir-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("b.wasm"), b"").unwrap();
    std::fs::write(dir.join("a.wasm"), b"").unwrap();
    std::fs::write(dir.join("notes.txt"), b"").unwrap();
    let names: Vec<_> = discover(&dir)
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["a.wasm", "b.wasm"]);
    assert!(discover(&dir.join("missing")).is_empty());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn config_switches_rules_off() {
    let Some(path) = echo() else { return };
    let plugin = PluginHost::new().unwrap().load_file(&path).unwrap();
    let mut req = LintRequest::plain("teh cat cat");
    assert_eq!(plugin.lint(&req).unwrap().len(), 2);

    req.config = Some([("Repeat".to_owned(), Some(false))].into());
    let lints = plugin.lint(&req).unwrap();
    assert_eq!(lints.len(), 1);
    assert_eq!(lints[0].rule, "Teh");
}

#[test]
fn info_lists_the_features_of_the_api_it_was_built_with() {
    let Some(path) = echo() else { return };
    let plugin = PluginHost::new().unwrap().load_file(&path).unwrap();
    assert!(
        plugin
            .info()
            .supports(harper_plugin_api::FEATURE_REQUEST_WORDS)
    );
}

/// A plugin that does not read `LintRequest::user_words` still gets the words of a request: the
/// host sets them first, in the same locked call.
#[test]
fn an_old_plugin_gets_the_words_of_the_request_through_set_user_words() {
    let Some(path) = echo() else { return };
    let plugin = PluginHost::new()
        .unwrap()
        .load_file(&path)
        .unwrap()
        .assume_old_plugin();
    let mut with = LintRequest::plain("teh");
    with.user_words = Some(vec!["teh".into()]);
    assert!(plugin.lint(&with).unwrap().is_empty());
    // Words of an earlier request are not kept for a request that brings none of its own.
    let mut without = LintRequest::plain("teh");
    without.user_words = Some(vec![]);
    assert_eq!(plugin.lint(&without).unwrap().len(), 1);
}

/// Compiled modules nobody has asked for in a month go when a new one is written.
#[test]
fn old_cache_files_are_removed_when_a_new_module_is_cached() {
    let Some(path) = echo() else { return };
    let cache = std::env::temp_dir().join(format!("harper-plugin-stale-{}", std::process::id()));
    std::fs::create_dir_all(&cache).unwrap();
    let old = cache.join("0000000000000001.cwasm");
    let fresh = cache.join("0000000000000002.cwasm");
    for file in [&old, &fresh] {
        std::fs::write(file, b"not a module").unwrap();
    }
    let long_ago = std::time::SystemTime::now() - std::time::Duration::from_secs(40 * 24 * 3600);
    std::fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(long_ago)
        .unwrap();

    PluginHost::new()
        .unwrap()
        .load_file_cached(&path, &cache)
        .unwrap();
    assert!(!old.exists());
    assert!(fresh.exists());
    std::fs::remove_dir_all(&cache).ok();
}

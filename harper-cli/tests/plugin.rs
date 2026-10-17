//! Drives `harper-cli` with the `echo` example plugin (see `harper-plugin-api/examples`).
#![cfg(feature = "plugins")]

use std::path::PathBuf;
use std::process::{Command, Output};

fn build_echo() -> Option<PathBuf> {
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
    Some(target_dir.join("wasm32-unknown-unknown/release/examples/echo.wasm"))
}

fn cli(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_harper-cli"))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn lints_with_a_plugin_file() {
    let Some(wasm) = build_echo() else { return };
    let wasm = wasm.to_str().unwrap();

    let out = cli(&[
        "lint",
        "--plugin",
        wasm,
        "--format",
        "json",
        "Teh cat cat sat.",
    ]);
    // Like with the built-in rules, finding lints makes the exit status non-zero.
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let lints = json[0]["lints"].as_array().unwrap();
    assert_eq!(lints.len(), 2, "{json}");
    assert!(
        lints
            .iter()
            .any(|l| l["rule"] == "Teh" && l["char_start"] == 0 || l["span"]["char_start"] == 0)
    );

    // --only filters by the plugin's own rule names.
    let out = cli(&[
        "lint",
        "--plugin",
        wasm,
        "--only",
        "Repeat",
        "--format",
        "json",
        "Teh cat cat sat.",
    ]);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let lints = json[0]["lints"].as_array().unwrap();
    assert_eq!(lints.len(), 1, "{json}");
    assert_eq!(lints[0]["rule"], "Repeat");
}

#[test]
fn unknown_language_is_an_error() {
    let out = cli(&["lint", "--language", "zz", "hello"]);
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("harper-zz.wasm"));
}

#[test]
fn english_still_works_without_a_plugin() {
    let out = cli(&["lint", "--language", "en", "--count", "This is a test."]);
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

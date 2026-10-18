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

/// `--only` and `--ignore` reach the plugin as its config, as they do in harper-ls, so the
/// plugin does not even run a rule that is off. A rule it does not have is reported.
#[test]
fn rule_switches_are_sent_to_the_plugin() {
    let Some(wasm) = build_echo() else { return };
    let wasm = wasm.to_str().unwrap();

    let out = cli(&[
        "lint",
        "--plugin",
        wasm,
        "--ignore",
        "Teh",
        "--format",
        "json",
        "Teh cat cat sat.",
    ]);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let lints = json[0]["lints"].as_array().unwrap();
    assert_eq!(lints.len(), 1, "{json}");
    assert_eq!(lints[0]["rule"], "Repeat");
    assert!(
        String::from_utf8_lossy(&out.stderr)
            .find("no rule named")
            .is_none()
    );

    let out = cli(&[
        "lint",
        "--plugin",
        wasm,
        "--only",
        "Typo",
        "--format",
        "json",
        "Teh cat cat sat.",
    ]);
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("no rule named Typo"), "{stderr}");
    assert!(stderr.contains("No rules are enabled"), "{stderr}");
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

/// The number of lints the plugin finds in `teh cat.`, with `dict_dir` holding the user's
/// dictionary files.
fn lints_in_teh_cat(wasm: &str, dict_dir: &std::path::Path) -> usize {
    let out = cli(&[
        "lint",
        "--plugin",
        wasm,
        "--user-dict-path",
        dict_dir.join("dictionary.txt").to_str().unwrap(),
        "--format",
        "json",
        "teh cat.",
    ]);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    json[0]["lint_count"].as_u64().unwrap() as usize
}

#[test]
fn a_plugin_gets_the_user_words_of_its_own_language_only() {
    let Some(wasm) = build_echo() else { return };
    let wasm = wasm.to_str().unwrap();
    let dir = std::env::temp_dir().join(format!("harper-cli-words-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();

    assert_eq!(lints_in_teh_cat(wasm, &dir), 1, "no words: teh is flagged");

    // The English dictionary is not the plugin's: `dictionary.txt` holds English words.
    std::fs::write(dir.join("dictionary.txt"), "teh\n").unwrap();
    assert_eq!(
        lints_in_teh_cat(wasm, &dir),
        1,
        "English words do not reach `xx`"
    );

    // The plugin's language (`xx`) has its own file next to it.
    std::fs::write(dir.join("dictionary-xx.txt"), "teh\n").unwrap();
    assert_eq!(
        lints_in_teh_cat(wasm, &dir),
        0,
        "the word of `xx` reaches the plugin"
    );

    std::fs::remove_dir_all(&dir).ok();
}

/// Files are linted in parallel with one plugin: the own dictionary of a file reaches that file
/// only, whatever the order in which the threads get to it.
#[test]
fn a_files_own_words_reach_that_file_only() {
    let Some(wasm) = build_echo() else { return };
    let wasm = wasm.to_str().unwrap();
    let dir = std::env::temp_dir().join(format!("harper-cli-file-words-{}", std::process::id()));
    // As in harper-ls, a plugin's file dictionaries have a directory of their own, `-<id>`.
    let file_dicts = dir.join("file_dicts");
    let plugin_file_dicts = dir.join("file_dicts-xx");
    std::fs::create_dir_all(&file_dicts).unwrap();
    std::fs::create_dir_all(&plugin_file_dicts).unwrap();

    let mut files = Vec::new();
    for i in 0..24usize {
        let file = dir.join(format!("f{i:02}.txt"));
        std::fs::write(&file, "teh cat.\n").unwrap();
        // Every second file knows `teh` in the plugin's language; the name is the path, each part
        // followed by a `%`. The others know it in English only, which must not reach the plugin.
        let name: String = file
            .components()
            .filter(|c| !matches!(c, std::path::Component::RootDir))
            .map(|c| format!("{}%", c.as_os_str().to_string_lossy()))
            .collect();
        let dicts = if i.is_multiple_of(2) {
            &plugin_file_dicts
        } else {
            &file_dicts
        };
        std::fs::write(dicts.join(name), "teh\n").unwrap();
        files.push(file);
    }

    let mut args = vec![
        "lint".to_owned(),
        "--plugin".into(),
        wasm.to_owned(),
        "--user-dict-path".into(),
        dir.join("dictionary.txt").to_str().unwrap().to_owned(),
        "--file-dict-path".into(),
        file_dicts.to_str().unwrap().to_owned(),
        "--format".into(),
        "json".into(),
    ];
    args.extend(files.iter().map(|f| f.to_str().unwrap().to_owned()));
    let args: Vec<&str> = args.iter().map(String::as_str).collect();

    let out = cli(&args);
    let json: serde_json::Value = serde_json::from_slice(&out.stdout).unwrap();
    let results = json.as_array().unwrap();
    assert_eq!(results.len(), files.len());
    for result in results {
        let name = result["file"].as_str().unwrap();
        let i: usize = name[1..3].parse().unwrap();
        let expected = if i.is_multiple_of(2) { 0 } else { 1 };
        assert_eq!(result["lint_count"], expected, "{name}");
    }

    std::fs::remove_dir_all(&dir).ok();
}

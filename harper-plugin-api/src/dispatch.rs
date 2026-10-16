use serde_json::{Value, json};

use crate::{LanguagePlugin, Request};

/// Handle one raw request on the guest side. Creates the plugin on first use.
/// Always returns a JSON response, never panics on malformed input.
pub fn dispatch<P: LanguagePlugin>(slot: &mut Option<P>, request: &[u8]) -> Vec<u8> {
    let response = match serde_json::from_slice::<Request>(request) {
        Ok(req) => handle(slot, req),
        Err(e) => json!({ "err": format!("invalid request: {e}") }),
    };
    serde_json::to_vec(&response).unwrap_or_else(|_| br#"{"err":"serialization failed"}"#.to_vec())
}

fn handle<P: LanguagePlugin>(slot: &mut Option<P>, req: Request) -> Value {
    // `info` and `detect` must not trigger the (possibly slow) construction of the plugin.
    match &req {
        Request::Info => return json!({ "ok": P::info() }),
        Request::Detect {
            text,
            english_ratio,
        } => return json!({ "ok": P::detect_with(text, *english_ratio) }),
        _ => {}
    }

    let plugin = slot.get_or_insert_with(P::new);

    match req {
        Request::Info | Request::Detect { .. } => unreachable!(),
        Request::Lint(r) => json!({ "ok": plugin.lint(r) }),
        Request::LintDescriptions => json!({ "ok": plugin.lint_descriptions() }),
        Request::DefaultConfig => json!({ "ok": plugin.default_config() }),
        Request::SetUserWords { words } => {
            plugin.set_user_words(words);
            json!({ "ok": null })
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::dispatch;
    use crate::{LanguagePlugin, LintRequest, PluginConfig, PluginInfo, PluginLint};

    struct Counting {
        created: usize,
    }

    thread_local! { static CREATED: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

    impl LanguagePlugin for Counting {
        fn info() -> PluginInfo {
            PluginInfo {
                id: "t".into(),
                name: "T".into(),
                plugin_version: "0".into(),
                abi: 1,
                harper_core_version: "0".into(),
                dialects: vec![],
                default_dialect: None,
            }
        }
        fn new() -> Self {
            CREATED.with(|c| c.set(c.get() + 1));
            Self { created: 1 }
        }
        fn lint(&mut self, _: LintRequest) -> Vec<PluginLint> {
            vec![]
        }
        fn lint_descriptions(&self) -> BTreeMap<String, String> {
            BTreeMap::new()
        }
        fn default_config(&self) -> PluginConfig {
            BTreeMap::new()
        }
        fn set_user_words(&mut self, _: Vec<String>) {
            self.created += 1;
        }
    }

    #[test]
    fn info_does_not_construct_the_plugin() {
        CREATED.with(|c| c.set(0));
        let mut slot: Option<Counting> = None;
        let out = dispatch(&mut slot, br#"{"method":"info"}"#);
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["ok"]["id"], "t");
        assert!(slot.is_none());
        assert_eq!(CREATED.with(|c| c.get()), 0);
    }

    #[test]
    fn detect_does_not_construct_the_plugin() {
        CREATED.with(|c| c.set(0));
        let mut slot: Option<Counting> = None;
        let out = dispatch(
            &mut slot,
            br#"{"method":"detect","params":{"text":"hallo"}}"#,
        );
        let v: serde_json::Value = serde_json::from_slice(&out).unwrap();
        assert_eq!(v["ok"], serde_json::Value::Null);
        assert!(slot.is_none());
        assert_eq!(CREATED.with(|c| c.get()), 0);
    }

    #[test]
    fn plugin_is_constructed_once() {
        CREATED.with(|c| c.set(0));
        let mut slot: Option<Counting> = None;
        for _ in 0..3 {
            dispatch(&mut slot, br#"{"method":"lint","params":{"text":"x"}}"#);
        }
        assert_eq!(CREATED.with(|c| c.get()), 1);
    }

    #[test]
    fn garbage_yields_an_error_response() {
        let mut slot: Option<Counting> = None;
        for bad in [&b"nope"[..], br#"{"method":"unknown"}"#, b""] {
            let out = String::from_utf8(dispatch(&mut slot, bad)).unwrap();
            assert!(out.starts_with(r#"{"err":"#), "{out}");
        }
    }
}

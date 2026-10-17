use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

use harper_core::linting::Lint;
use harper_plugin_api::{
    ABI_VERSION, LintRequest, PluginConfig, PluginInfo, PluginLint, Request, Response,
    plugin_to_lint,
};
use serde::de::DeserializeOwned;
use wasmtime::{Engine, Instance, Linker, Memory, Module, Store, TypedFunc};

use crate::{Error, Result};

/// Owns the WebAssembly engine. Cheap to share; create one per process.
pub struct PluginHost {
    engine: Engine,
}

impl PluginHost {
    pub fn new() -> Result<Self> {
        Ok(Self {
            engine: Engine::default(),
        })
    }

    pub fn load_file(&self, path: impl AsRef<Path>) -> Result<LoadedPlugin> {
        self.load_bytes(&std::fs::read(path)?)
    }

    pub fn load_bytes(&self, bytes: &[u8]) -> Result<LoadedPlugin> {
        let module = Module::new(&self.engine, bytes)?;
        LoadedPlugin::instantiate(&self.engine, &module)
    }

    /// Like [`Self::load_file`], but keeps the compiled module in `cache_dir`, so that the
    /// second start does not pay for compilation again.
    pub fn load_file_cached(
        &self,
        path: impl AsRef<Path>,
        cache_dir: impl AsRef<Path>,
    ) -> Result<LoadedPlugin> {
        let path = path.as_ref();
        let bytes = std::fs::read(path)?;
        let cache = cache_dir
            .as_ref()
            .join(format!("{:016x}.cwasm", fingerprint(&bytes)));

        let module = if cache.exists() {
            // SAFETY: the file was produced by `Module::serialize` of this same wasmtime
            // version (the fingerprint includes the version) in a directory we own.
            unsafe { Module::deserialize_file(&self.engine, &cache) }.ok()
        } else {
            None
        };
        let module = match module {
            Some(m) => m,
            None => {
                let m = Module::new(&self.engine, &bytes)?;
                if std::fs::create_dir_all(&cache_dir).is_ok()
                    && let Ok(ser) = m.serialize()
                {
                    // Best effort; a read-only cache dir only costs speed.
                    let tmp = cache.with_extension("tmp");
                    if std::fs::write(&tmp, ser).is_ok() {
                        let _ = std::fs::rename(&tmp, &cache);
                    }
                }
                m
            }
        };
        LoadedPlugin::instantiate(&self.engine, &module)
    }
}

fn fingerprint(bytes: &[u8]) -> u64 {
    use std::hash::{DefaultHasher, Hash, Hasher};
    let mut h = DefaultHasher::new();
    bytes.hash(&mut h);
    // A different wasmtime build must not reuse the cache.
    env!("CARGO_PKG_VERSION").hash(&mut h);
    wasmtime::Engine::default()
        .precompile_compatibility_hash()
        .hash(&mut h);
    h.finish()
}

struct Guest {
    store: Store<()>,
    memory: Memory,
    alloc: TypedFunc<u32, u32>,
    free: TypedFunc<(u32, u32), ()>,
    call: TypedFunc<(u32, u32), u64>,
}

impl Guest {
    fn roundtrip(&mut self, request: &[u8]) -> Result<Vec<u8>> {
        let len = request.len() as u32;
        let ptr = self.alloc.call(&mut self.store, len)?;
        self.memory
            .write(&mut self.store, ptr as usize, request)
            .map_err(|e| Error::Protocol(e.to_string()))?;
        let packed = self.call.call(&mut self.store, (ptr, len))?;
        let (out_ptr, out_len) = ((packed >> 32) as u32, (packed & 0xffff_ffff) as u32);

        let mut out = vec![0u8; out_len as usize];
        self.memory
            .read(&self.store, out_ptr as usize, &mut out)
            .map_err(|e| Error::Protocol(e.to_string()))?;
        self.free.call(&mut self.store, (out_ptr, out_len))?;
        self.free.call(&mut self.store, (ptr, len))?;
        Ok(out)
    }
}

/// An instantiated plugin. Calls are serialized by an internal lock.
pub struct LoadedPlugin {
    info: PluginInfo,
    guest: Mutex<Guest>,
    user_words: Mutex<Option<Vec<String>>>,
}

impl LoadedPlugin {
    fn instantiate(engine: &Engine, module: &Module) -> Result<Self> {
        let mut store = Store::new(engine, ());
        let mut linker = Linker::new(engine);
        // A well-behaved plugin has no imports. Some dependencies still drag in unused
        // `wasm-bindgen` stubs; they are never called, so make them trap instead of refusing
        // the plugin. The plugin gets no access to anything either way.
        linker.define_unknown_imports_as_traps(module)?;
        let instance = linker.instantiate(&mut store, module)?;

        let version = typed::<(), u32>(&instance, &mut store, "harper_plugin_abi_version")?
            .call(&mut store, ())?;
        if version != ABI_VERSION {
            return Err(Error::AbiMismatch {
                plugin: version,
                host: ABI_VERSION,
            });
        }

        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or(Error::MissingExport("memory"))?;
        let alloc = typed(&instance, &mut store, "harper_alloc")?;
        let free = typed(&instance, &mut store, "harper_free")?;
        let call = typed(&instance, &mut store, "harper_call")?;

        let mut guest = Guest {
            store,
            memory,
            alloc,
            free,
            call,
        };
        let info = request::<PluginInfo>(&mut guest, &Request::Info)?;
        if info.abi != ABI_VERSION {
            return Err(Error::AbiMismatch {
                plugin: info.abi,
                host: ABI_VERSION,
            });
        }

        Ok(Self {
            info,
            guest: Mutex::new(guest),
            user_words: Mutex::new(None),
        })
    }

    pub fn info(&self) -> &PluginInfo {
        &self.info
    }

    fn send<T: DeserializeOwned>(&self, req: &Request) -> Result<T> {
        let mut guest = self.guest.lock().unwrap_or_else(|e| e.into_inner());
        request(&mut guest, req)
    }

    pub fn lint(&self, req: &LintRequest) -> Result<Vec<PluginLint>> {
        self.send(&Request::Lint(req.clone()))
    }

    /// [`Self::lint`], converted into the lint type the rest of Harper uses.
    pub fn lint_core(&self, req: &LintRequest) -> Result<Vec<Lint>> {
        Ok(self.lint(req)?.iter().map(plugin_to_lint).collect())
    }

    pub fn lint_descriptions(&self) -> Result<BTreeMap<String, String>> {
        self.send(&Request::LintDescriptions)
    }

    pub fn default_config(&self) -> Result<PluginConfig> {
        self.send(&Request::DefaultConfig)
    }

    /// Replaces the plugin's list of user dictionary words. Does nothing (and does not cross
    /// into the plugin) when the list is the same as last time, so callers may call this
    /// before every lint.
    pub fn set_user_words(&self, words: Vec<String>) -> Result<()> {
        let mut last = self.user_words.lock().unwrap_or_else(|e| e.into_inner());
        if last.as_ref() == Some(&words) {
            return Ok(());
        }
        self.send::<()>(&Request::SetUserWords {
            words: words.clone(),
        })?;
        *last = Some(words);
        Ok(())
    }

    /// How sure the plugin is that `text` is in its language (0 to 1). The plugin is told how
    /// much of the text Harper's English dictionary knows.
    pub fn detect(&self, text: &str) -> Result<Option<f32>> {
        self.send(&Request::Detect {
            text: text.to_owned(),
            english_ratio: english_ratio(text),
        })
    }
}

fn typed<P: wasmtime::WasmParams, R: wasmtime::WasmResults>(
    instance: &Instance,
    store: &mut Store<()>,
    name: &'static str,
) -> Result<TypedFunc<P, R>> {
    instance
        .get_typed_func(&mut *store, name)
        .map_err(|_| Error::MissingExport(name))
}

fn request<T: DeserializeOwned>(guest: &mut Guest, req: &Request) -> Result<T> {
    let body = serde_json::to_vec(req).map_err(|e| Error::Protocol(e.to_string()))?;
    let raw = guest.roundtrip(&body)?;
    match serde_json::from_slice::<Response<T>>(&raw) {
        Ok(Response::Ok(v)) => Ok(v),
        Ok(Response::Err(e)) => Err(Error::Plugin(e)),
        Err(e) => Err(Error::Protocol(e.to_string())),
    }
}

/// The share of the words of `text` that Harper's English dictionary knows, or `None` for a text
/// without words. Words as English prose is split into them.
pub fn english_ratio(text: &str) -> Option<f32> {
    use harper_core::parsers::{Parser, PlainEnglish};
    use harper_core::spell::{Dictionary, FstDictionary};

    let chars: Vec<char> = text.chars().collect();
    let dict = FstDictionary::curated();
    let (mut words, mut known) = (0usize, 0usize);
    for token in PlainEnglish.parse(&chars) {
        if token.kind.is_word() {
            words += 1;
            if dict.contains_word(token.get_ch(&chars)) {
                known += 1;
            }
        }
    }
    (words > 0).then(|| known as f32 / words as f32)
}

//! Loads Harper language plugins and runs them in a sandbox.
//!
//! ```no_run
//! use harper_plugin_host::PluginHost;
//! use harper_plugin_api::LintRequest;
//!
//! let host = PluginHost::new().unwrap();
//! let plugin = host.load_file("harper-de.wasm").unwrap();
//! let lints = plugin.lint_core(&LintRequest::plain("Das ist ein Test.")).unwrap();
//! ```

mod dir;
mod plugin;

pub use dir::{default_plugin_dir, discover};
pub use plugin::{LoadedPlugin, PluginHost, english_ratio};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("could not read plugin: {0}")]
    Io(#[from] std::io::Error),
    #[error("could not compile or run the WebAssembly module: {0}")]
    Wasm(String),
    #[error("plugin is not a Harper plugin: missing export `{0}`")]
    MissingExport(&'static str),
    #[error("plugin was built for ABI v{plugin}, this Harper supports v{host}")]
    AbiMismatch { plugin: u32, host: u32 },
    #[error("plugin sent an invalid response: {0}")]
    Protocol(String),
    #[error("plugin reported an error: {0}")]
    Plugin(String),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<wasmtime::Error> for Error {
    fn from(e: wasmtime::Error) -> Self {
        Error::Wasm(format!("{e:#}"))
    }
}

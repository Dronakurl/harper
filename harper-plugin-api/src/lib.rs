//! The interface between Harper and language plugins.
//!
//! A language plugin is a standalone WebAssembly module with no imports. Hosts
//! (`harper-plugin-host`, `harper.js`) talk to it through a small JSON protocol over
//! four exported functions.
//!
//! Plugin authors implement [`LanguagePlugin`] and call [`export_plugin!`].
//! Hosts use the types in this crate to build requests and read responses.

mod dispatch;
mod types;

#[cfg(feature = "core")]
mod convert;

pub use dispatch::dispatch;
pub use types::*;

#[cfg(feature = "core")]
pub use convert::{lint_to_plugin, plugin_to_lint};

/// Bumped whenever the wire format changes incompatibly.
pub const ABI_VERSION: u32 = 1;

/// What a language plugin implements. The instance is created lazily on the first call that
/// needs it, so building dictionaries belongs in [`LanguagePlugin::new`].
pub trait LanguagePlugin: Sized + 'static {
    fn info() -> PluginInfo;
    fn new() -> Self;
    fn lint(&mut self, req: LintRequest) -> Vec<PluginLint>;
    fn lint_descriptions(&self) -> std::collections::BTreeMap<String, String>;
    fn default_config(&self) -> PluginConfig;
    fn set_user_words(&mut self, words: Vec<String>);
    /// How confident the plugin is that `text` is in its language (0..1), or `None` if it cannot
    /// tell. Called without an instance, so that asking does not load the plugin's dictionary.
    fn detect(_text: &str) -> Option<f32> {
        None
    }
    /// [`Self::detect`], told by the host how much of `text` is English: the share of its words
    /// in Harper's English dictionary, when the host has it. The default ignores it.
    fn detect_with(text: &str, _english_ratio: Option<f32>) -> Option<f32> {
        Self::detect(text)
    }
}

/// Generates the exports a host expects (`harper_plugin_abi_version`, `harper_alloc`,
/// `harper_free`, `harper_call`) for a [`LanguagePlugin`].
#[macro_export]
macro_rules! export_plugin {
    ($plugin:ty) => {
        const _: () = {
            thread_local! {
                static INSTANCE: ::std::cell::RefCell<::std::option::Option<$plugin>> =
                    const { ::std::cell::RefCell::new(::std::option::Option::None) };
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn harper_plugin_abi_version() -> u32 {
                $crate::ABI_VERSION
            }

            #[unsafe(no_mangle)]
            pub extern "C" fn harper_alloc(len: u32) -> *mut u8 {
                let mut buf = ::std::vec::Vec::<u8>::with_capacity(len as usize);
                let ptr = buf.as_mut_ptr();
                ::std::mem::forget(buf);
                ptr
            }

            /// # Safety
            /// `ptr`/`len` must come from `harper_alloc` or `harper_call`.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn harper_free(ptr: *mut u8, len: u32) {
                // SAFETY: guaranteed by the caller.
                drop(unsafe { ::std::vec::Vec::from_raw_parts(ptr, len as usize, len as usize) });
            }

            /// Takes a request, returns `(ptr << 32) | len` of the response.
            /// # Safety
            /// `ptr`/`len` must describe a buffer obtained from `harper_alloc`.
            #[unsafe(no_mangle)]
            pub unsafe extern "C" fn harper_call(ptr: *const u8, len: u32) -> u64 {
                // SAFETY: guaranteed by the caller.
                let request = unsafe { ::std::slice::from_raw_parts(ptr, len as usize) };
                let response = INSTANCE
                    .with(|slot| $crate::dispatch::<$plugin>(&mut slot.borrow_mut(), request));
                let mut boxed = response.into_boxed_slice();
                let out = ((boxed.as_mut_ptr() as usize as u64) << 32) | boxed.len() as u64;
                ::std::mem::forget(boxed);
                out
            }
        };
    };
}

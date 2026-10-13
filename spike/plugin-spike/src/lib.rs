//! Wegwerf-Spike: harper-core als Wasm-Modul ohne wasm-bindgen, JSON-ABI v1 (Minimalform).

use harper_core::linting::{LintGroup, Linter};
use harper_core::parsers::PlainEnglish;
use harper_core::spell::FstDictionary;
use harper_core::{Dialect, Document};

#[unsafe(no_mangle)]
pub extern "C" fn harper_plugin_abi_version() -> u32 {
    1
}

#[unsafe(no_mangle)]
pub extern "C" fn harper_alloc(len: u32) -> *mut u8 {
    let mut v = Vec::<u8>::with_capacity(len as usize);
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn harper_free(ptr: *mut u8, len: u32) {
    drop(unsafe { Vec::from_raw_parts(ptr, 0, len as usize) });
}

/// Request: UTF-8 text. Response: JSON array of lints. Returned u64 = (ptr << 32) | len.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn harper_call(ptr: *const u8, len: u32) -> u64 {
    let input = unsafe { std::slice::from_raw_parts(ptr, len as usize) };
    let text = std::str::from_utf8(input).unwrap_or("");
    let out = lint_json(text);
    let mut bytes = out.into_bytes().into_boxed_slice();
    let (p, l) = (bytes.as_mut_ptr() as u32, bytes.len() as u32);
    std::mem::forget(bytes);
    ((p as u64) << 32) | l as u64
}

pub fn lint_json(text: &str) -> String {
    let dict = FstDictionary::curated();
    let doc = Document::new(text, &PlainEnglish, &dict);
    let mut group = LintGroup::new_curated(dict, Dialect::American);
    let lints = group.lint(&doc);
    serde_json::to_string(&lints).unwrap()
}

// Harper braucht keinen echten Zufall; feste Saat reicht für den Spike.
fn fill(buf: &mut [u8]) {
    for (i, b) in buf.iter_mut().enumerate() {
        *b = (i as u8).wrapping_mul(31).wrapping_add(7);
    }
}

fn gr02(buf: &mut [u8]) -> Result<(), getrandom02::Error> {
    fill(buf);
    Ok(())
}
getrandom02::register_custom_getrandom!(gr02);

#[unsafe(no_mangle)]
unsafe extern "Rust" fn __getrandom_v03_custom(
    dest: *mut u8,
    len: usize,
) -> Result<(), getrandom03::Error> {
    fill(unsafe { std::slice::from_raw_parts_mut(dest, len) });
    Ok(())
}

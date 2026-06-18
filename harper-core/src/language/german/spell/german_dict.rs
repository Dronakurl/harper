//! German dictionary based on the LanguageTool/igerman98 Hunspell word lists.
//!
//! The word list is derived from the igerman98 dictionary (GPLv2/GPLv3),
//! expanded using Hunspell affix rules for comprehensive coverage.
//! It is embedded as gzip-compressed data and decompressed once at first use.
use crate::spell::FstDictionary;
use crate::spell::embedded_dictionary::fst_dictionary_from_gzip_bytes;
use std::sync::{Arc, LazyLock};

static GERMAN_DICT: LazyLock<Arc<FstDictionary>> = LazyLock::new(|| {
    Arc::new(fst_dictionary_from_gzip_bytes(include_bytes!(
        "../german_dictionary.dict.gz"
    )))
});

/// Returns a shared reference to the German FstDictionary.
///
/// The dictionary is loaded and built once on first access, then cached for the
/// lifetime of the process. This provides fuzzy matching, prefix search, and
/// all other `Dictionary` trait capabilities.
pub fn german_dictionary() -> Arc<FstDictionary> {
    (*GERMAN_DICT).clone()
}

pub fn curated_german_dictionary() -> Arc<FstDictionary> {
    german_dictionary()
}

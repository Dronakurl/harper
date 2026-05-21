//! Portuguese dictionary support.

use std::sync::{Arc, LazyLock};

use crate::spell::FstDictionary;
use crate::{CharString, DictWordMetadata};

// For now, we'll use a simple embedded dictionary
// The dictionary file format: one word per line, with optional metadata annotations

static PORTUGUESE_DICT: LazyLock<Arc<FstDictionary>> = LazyLock::new(|| {
    let dictionary_content = include_str!("../dictionary-portuguese.dict");
    
    let words: Vec<(CharString, DictWordMetadata)> = dictionary_content
        .lines()
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(|word| {
            let word_text = word.split('/').next().unwrap_or(word);
            let chars: CharString = word_text.chars().collect();
            (chars, DictWordMetadata::default())
        })
        .collect();
    
    Arc::new(FstDictionary::new(words))
});

/// Returns a shared reference to the Portuguese FstDictionary.
pub fn curated_portuguese_dictionary() -> Arc<FstDictionary> {
    (*PORTUGUESE_DICT).clone()
}

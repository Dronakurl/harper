//! Suggestion hints: language data describing which misspellings are plausible,
//! in the form hunspell dictionaries already ship it (`REP`, `MAP`, `KEY` and
//! `TRY` in a `.aff` file).
//!
//! The hints are an optional `suggestions` section of a dictionary's
//! `annotations.json`. A language without hints is ranked exactly as before,
//! see [`super::suggest_correct_spelling_with_hints`].

use serde::Deserialize;

use super::{Dictionary, FuzzyMatchResult};
use crate::edit_distance::edit_distance;

/// The most spellings a misspelled word is expanded to through `MAP` before
/// the expansion is skipped. 20 000 covers a long Slovak word typed entirely
/// without diacritics ("dolezitost" has 2 304 variants).
const MAX_MAP_VARIANTS: usize = 20_000;

/// Hints for ranking spelling suggestions, imported from a hunspell `.aff` file.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct SuggestionHints {
    /// `REP` pairs: a common misspelling and its replacement. `^` and `$`
    /// anchor the pattern to the start and end of the word, `_` is a space.
    pub rep: Vec<(String, String)>,
    /// `MAP` groups of related characters or character sequences, such as a
    /// letter and its variants with diacritics (`["t", "ť"]`).
    pub map: Vec<Vec<String>>,
    /// `KEY` rows of the keyboard; neighbouring keys are likely substitutions.
    pub key: Vec<String>,
    /// `TRY` characters, most frequent first.
    #[serde(rename = "try")]
    pub try_chars: String,
}

#[derive(Deserialize)]
struct AnnotationsSuggestions {
    #[serde(default)]
    suggestions: SuggestionHints,
}

/// Why a candidate could be what the writer meant. Earlier is more plausible.
///
/// Public so that a language can filter or regroup the ranked suggestions by
/// the same classification, without restating its own table in code.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum EditKind {
    /// Characters swapped within a `MAP` group (`ťo` → `to`).
    Map,
    /// One `REP` replacement.
    Rep,
    /// A key struck twice: the misspelling repeats a letter the word has once
    /// (`Worrt` → `Wort`). The likeliest slip of all when typing fast, in any
    /// language, so it comes before a neighbouring key: `Worrt` is not `Wirrt`.
    DoubledChar,
    /// One character replaced by its neighbour on the keyboard.
    KeyNeighbor,
    /// Two adjacent characters swapped.
    Transposition,
    /// The misspelling has one character too many.
    ExtraChar,
    /// The misspelling lacks one character.
    MissingChar,
    /// One character replaced by an unrelated one.
    Substitution,
    /// Anything else.
    Other,
}

impl SuggestionHints {
    /// Read the optional `suggestions` section of an `annotations.json`.
    /// A file without the section gives empty hints.
    pub fn from_annotations_json(json: &str) -> Result<Self, serde_json::Error> {
        Ok(serde_json::from_str::<AnnotationsSuggestions>(json)?.suggestions)
    }

    /// Whether there is nothing to go by, in which case ranking is unchanged.
    pub fn is_empty(&self) -> bool {
        self.rep.is_empty()
            && self.map.is_empty()
            && self.key.is_empty()
            && self.try_chars.is_empty()
    }

    /// The `MAP` group of a single character, if it has one.
    fn char_group(&self, c: char) -> Option<&Vec<String>> {
        self.map.iter().find(|group| {
            group
                .iter()
                .any(|member| member.chars().eq(std::iter::once(c)))
        })
    }

    fn same_char_group(&self, a: char, b: char) -> bool {
        self.char_group(a).is_some_and(|group| {
            group
                .iter()
                .any(|member| member.chars().eq(std::iter::once(b)))
        })
    }

    fn key_neighbors(&self, a: char, b: char) -> bool {
        self.key.iter().any(|row| {
            let row: Vec<char> = row.chars().collect();
            row.windows(2)
                .any(|pair| (pair[0] == a && pair[1] == b) || (pair[0] == b && pair[1] == a))
        })
    }

    /// Every word one `REP` replacement (or one replacement within a
    /// multi-character `MAP` group) away from `word`.
    fn single_replacements(&self, word: &str) -> Vec<String> {
        let mut out = Vec::new();
        let mut pairs: Vec<(&str, String)> = self
            .rep
            .iter()
            .map(|(from, to)| (from.as_str(), to.replace('_', " ")))
            .collect();
        for group in &self.map {
            if group.iter().all(|member| member.chars().count() == 1) {
                continue;
            }
            for from in group {
                for to in group {
                    if from != to {
                        pairs.push((from.as_str(), to.clone()));
                    }
                }
            }
        }

        for (from, to) in pairs {
            let anchored_start = from.starts_with('^');
            let anchored_end = from.len() > 1 && from.ends_with('$');
            let pattern = from.trim_start_matches('^');
            let pattern = if anchored_end {
                &pattern[..pattern.len() - 1]
            } else {
                pattern
            };
            if pattern.is_empty() {
                continue;
            }
            for (start, _) in word.match_indices(pattern) {
                let end = start + pattern.len();
                if (anchored_start && start != 0) || (anchored_end && end != word.len()) {
                    continue;
                }
                let candidate = format!("{}{}{}", &word[..start], to, &word[end..]);
                if candidate != word && !out.contains(&candidate) {
                    out.push(candidate);
                }
            }
        }
        out
    }

    /// Classify how `candidate` could have been mistyped as `misspelled`.
    /// Letter case is ignored.
    pub fn classify(&self, misspelled: &[char], candidate: &[char]) -> EditKind {
        let m: Vec<char> = misspelled.iter().flat_map(|c| c.to_lowercase()).collect();
        let c: Vec<char> = candidate.iter().flat_map(|c| c.to_lowercase()).collect();

        if m.len() == c.len() {
            let diffs: Vec<usize> = (0..m.len()).filter(|&i| m[i] != c[i]).collect();
            if !diffs.is_empty() && diffs.iter().all(|&i| self.same_char_group(m[i], c[i])) {
                return EditKind::Map;
            }
        }

        let m_str: String = m.iter().collect();
        let c_str: String = c.iter().collect();
        if self.single_replacements(&m_str).contains(&c_str) {
            let multi_char_map = self.map.iter().any(|group| {
                group.iter().any(|member| member.chars().count() > 1)
                    && group.iter().any(|a| {
                        group.iter().any(|b| {
                            a != b
                                && m_str.match_indices(a.as_str()).any(|(i, _)| {
                                    format!("{}{}{}", &m_str[..i], b, &m_str[i + a.len()..])
                                        == c_str
                                })
                        })
                    })
            });
            return if multi_char_map {
                EditKind::Map
            } else {
                EditKind::Rep
            };
        }

        if m.len() == c.len() {
            let diffs: Vec<usize> = (0..m.len()).filter(|&i| m[i] != c[i]).collect();
            match diffs.as_slice() {
                [i] => {
                    return if self.key_neighbors(m[*i], c[*i]) {
                        EditKind::KeyNeighbor
                    } else {
                        EditKind::Substitution
                    };
                }
                [i, j] if *j == i + 1 && m[*i] == c[*j] && m[*j] == c[*i] => {
                    return EditKind::Transposition;
                }
                _ => return EditKind::Other,
            }
        }

        if m.len() == c.len() + 1 && is_one_deletion(&m, &c) {
            return if repeats_a_neighbour(&m, &c) {
                EditKind::DoubledChar
            } else {
                EditKind::ExtraChar
            };
        }
        if m.len() + 1 == c.len() && is_one_deletion(&c, &m) {
            return EditKind::MissingChar;
        }
        EditKind::Other
    }

    /// Position in `TRY` of the first character in which `candidate` departs
    /// from `misspelled`: a frequent letter is the likelier correction.
    pub(crate) fn try_rank(&self, misspelled: &[char], candidate: &[char]) -> usize {
        let fallback = self.try_chars.chars().count();
        let differing = misspelled
            .iter()
            .zip(candidate)
            .find(|(a, b)| !a.eq_ignore_ascii_case(b))
            .map(|(_, b)| *b)
            .or_else(|| candidate.get(misspelled.len()).copied());
        differing
            .and_then(|c| self.try_chars.chars().position(|t| t == c))
            .unwrap_or(fallback)
    }

    /// Dictionary words that `REP` or `MAP` reach from `misspelled`, which an
    /// edit-distance search can miss: a word typed without any of its
    /// diacritics is often more than two edits away ("stastni" → "šťastní"),
    /// and a multi-letter `REP` such as "ph" → "f" costs two edits on its own.
    ///
    /// Suggestions containing a space (`REP alot a_lot`) are not produced,
    /// since a suggestion has to be a single dictionary word.
    pub(crate) fn generate_candidates<'a, D: Dictionary + ?Sized>(
        &self,
        misspelled: &[char],
        dictionary: &'a D,
    ) -> Vec<FuzzyMatchResult<'a>> {
        let lower: Vec<char> = misspelled.iter().flat_map(|c| c.to_lowercase()).collect();
        let lower_str: String = lower.iter().collect();

        let mut spellings: Vec<Vec<char>> = self
            .single_replacements(&lower_str)
            .into_iter()
            .filter(|s| !s.contains(' '))
            .map(|s| s.chars().collect())
            .collect();

        // Every combination of the single-character MAP groups.
        let options: Vec<Vec<char>> = lower
            .iter()
            .map(|&c| match self.char_group(c) {
                Some(group) => group.iter().filter_map(|m| m.chars().next()).collect(),
                None => vec![c],
            })
            .collect();
        let total = options
            .iter()
            .try_fold(1usize, |acc, o| acc.checked_mul(o.len()))
            .unwrap_or(usize::MAX);
        if total > 1 && total <= MAX_MAP_VARIANTS {
            for mut index in 0..total {
                let mut variant = Vec::with_capacity(options.len());
                for option in &options {
                    variant.push(option[index % option.len()]);
                    index /= option.len();
                }
                if variant != lower {
                    spellings.push(variant);
                }
            }
        }

        let mut out: Vec<FuzzyMatchResult<'a>> = Vec::new();
        for spelling in spellings {
            if !dictionary.contains_word(&spelling) {
                continue;
            }
            // A spelling with several capitalizations in the dictionary is
            // left to the edit-distance search.
            let Some(word) = dictionary.get_correct_capitalization_of(&spelling) else {
                continue;
            };
            if out.iter().any(|r| r.word == word) {
                continue;
            }
            let Some(metadata) = dictionary.get_word_metadata(word) else {
                continue;
            };
            out.push(FuzzyMatchResult {
                word,
                edit_distance: edit_distance(&lower, &spelling),
                metadata,
            });
        }
        out
    }
}

/// Whether the character `longer` has beyond `shorter` repeats the one next to it.
///
/// `longer` must be `shorter` with one character inserted. Where a run of equal
/// characters makes the insertion point ambiguous, the first differing position
/// is in or right after that run, so its neighbours tell.
fn repeats_a_neighbour(longer: &[char], shorter: &[char]) -> bool {
    let i = longer
        .iter()
        .zip(shorter)
        .take_while(|(a, b)| a == b)
        .count();
    (i > 0 && longer[i] == longer[i - 1]) || longer.get(i + 1) == Some(&longer[i])
}

/// Whether `shorter` is `longer` with exactly one character removed.
fn is_one_deletion(longer: &[char], shorter: &[char]) -> bool {
    let i = longer
        .iter()
        .zip(shorter)
        .take_while(|(a, b)| a == b)
        .count();
    longer[i + 1..] == shorter[i..]
}

#[cfg(test)]
mod tests {
    use super::{EditKind, SuggestionHints};
    use crate::DictWordMetadata;
    use crate::spell::MutableDictionary;

    fn chars(s: &str) -> Vec<char> {
        s.chars().collect()
    }

    fn slovak_like() -> SuggestionHints {
        SuggestionHints::from_annotations_json(
            r#"{"affixes": {}, "properties": {}, "suggestions": {
                "rep": [["ds", "c"], ["^bz", "bez"]],
                "map": [["a", "á", "ä"], ["s", "š"], ["t", "ť"], ["i", "í"], ["hc", "ch"]],
                "key": ["qwertz", "asdfgh"],
                "try": "aoeintsv"
            }}"#,
        )
        .unwrap()
    }

    #[test]
    fn missing_section_gives_empty_hints() {
        let hints =
            SuggestionHints::from_annotations_json(r#"{"affixes": {}, "properties": {}}"#).unwrap();
        assert!(hints.is_empty());
        assert_eq!(hints, SuggestionHints::default());
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let hints = SuggestionHints::from_annotations_json(
            r##"{"#": "comment", "affixes": {}, "properties": {}, "suggestions": {"try": "ab", "future": 1}}"##,
        )
        .unwrap();
        assert_eq!(hints.try_chars, "ab");
    }

    #[test]
    fn classifies_by_hint() {
        let h = slovak_like();
        let k = |m: &str, c: &str| h.classify(&chars(m), &chars(c));
        assert_eq!(k("ťo", "to"), EditKind::Map);
        assert_eq!(k("Ťo", "to"), EditKind::Map);
        assert_eq!(k("stastni", "šťastní"), EditKind::Map);
        assert_eq!(k("hcata", "chata"), EditKind::Map);
        assert_eq!(k("tidsa", "tica"), EditKind::Rep);
        assert_eq!(k("bzpečný", "bezpečný"), EditKind::Rep);
        assert_eq!(k("xbzy", "xbezy"), EditKind::MissingChar);
        assert_eq!(k("dwm", "dem"), EditKind::KeyNeighbor);
        assert_eq!(k("dxm", "dem"), EditKind::Substitution);
        assert_eq!(k("flasch", "falsch"), EditKind::Transposition);
        assert_eq!(k("domm", "dom"), EditKind::DoubledChar);
        assert_eq!(k("worrt", "wort"), EditKind::DoubledChar);
        assert_eq!(k("ddom", "dom"), EditKind::DoubledChar);
        assert_eq!(k("doxm", "dom"), EditKind::ExtraChar);
        assert_eq!(k("dm", "dom"), EditKind::MissingChar);
        assert_eq!(k("abcd", "wxyz"), EditKind::Other);
    }

    #[test]
    fn generates_candidates_beyond_edit_distance() {
        let mut dict = MutableDictionary::new();
        for w in ["šťastní", "tica", "chata", "to"] {
            dict.append_word_str(w, DictWordMetadata::default());
        }
        let h = slovak_like();
        let found = |m: &str| -> Vec<String> {
            h.generate_candidates(&chars(m), &dict)
                .into_iter()
                .map(|r| r.word.iter().collect())
                .collect()
        };
        assert_eq!(found("stastni"), vec!["šťastní"]);
        assert_eq!(found("Stastni"), vec!["šťastní"]);
        assert_eq!(found("tidsa"), vec!["tica"]);
        assert_eq!(found("hcata"), vec!["chata"]);
        assert_eq!(found("ťo"), vec!["to"]);
        assert!(found("xyz").is_empty());
        assert!(
            SuggestionHints::default()
                .generate_candidates(&chars("stastni"), &dict)
                .is_empty()
        );
    }

    #[test]
    fn try_rank_prefers_frequent_letters() {
        let h = slovak_like();
        assert!(h.try_rank(&chars("xo"), &chars("ao")) < h.try_rank(&chars("xo"), &chars("vo")));
        assert_eq!(h.try_rank(&chars("xo"), &chars("zo")), 8);
    }
}

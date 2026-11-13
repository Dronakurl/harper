//! Steps a parser for a language with dotted abbreviations and ordinals runs on its own tokens.
//!
//! [`Document`](crate::Document) only runs the English steps (`etc.`, `U.S.A.`, `et al.`) on
//! English text, so a full stop after "napr." or "bzw." ends a sentence unless the parser says
//! otherwise. These functions let a parser say so, with the data of its language.

use crate::{Span, Token, TokenKind};

/// Join the tokens of each abbreviation in `abbreviations` (`"z. B."`, `"bzw."`) into one word
/// token, so that the full stop inside it is not a sentence end and a spell check sees the
/// abbreviation as a unit. The abbreviation has to start at the start of a word and is matched
/// case-sensitively, with the spaces it is written with.
pub fn condense_abbreviations(tokens: &mut Vec<Token>, source: &[char], abbreviations: &[&str]) {
    if abbreviations.is_empty() {
        return;
    }
    let abbreviations: Vec<Vec<char>> = abbreviations.iter().map(|a| a.chars().collect()).collect();

    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        let starts_word = tokens[i].kind.is_word()
            && !out
                .last()
                .is_some_and(|t: &Token| t.kind.is_word() || t.kind.is_number());
        if starts_word {
            let start = tokens[i].span.start;
            let found = abbreviations
                .iter()
                .filter(|a| source[start..].starts_with(a))
                .filter_map(|a| {
                    let end = start + a.len();
                    // The abbreviation must end where a token ends, and not run into a word.
                    let last = tokens[i..].iter().position(|t| t.span.end == end)?;
                    let next_is_letter = source.get(end).is_some_and(|c| c.is_alphanumeric());
                    (!next_is_letter).then_some((i + last, end))
                })
                .max_by_key(|(_, end)| *end);

            if let Some((last, end)) = found {
                let mut token = tokens[i].clone();
                token.span = Span::new(start, end);
                out.push(token);
                i = last + 1;
                continue;
            }
        }
        out.push(tokens[i].clone());
        i += 1;
    }
    *tokens = out;
}

/// Join a number and the full stop after it when it is an ordinal ("8. November", "8. 11.",
/// "5. miesto"), so that the full stop does not end the sentence. `follows` decides from the
/// word or number after the spaces whether the number is an ordinal: German accepts a month
/// name or a lowercase word, for example. A number that ends the text or is followed by a
/// line break stays a number and a sentence end.
pub fn condense_ordinals(
    tokens: &mut Vec<Token>,
    source: &[char],
    follows: impl Fn(&[char]) -> bool,
) {
    let mut out: Vec<Token> = Vec::with_capacity(tokens.len());
    let mut i = 0;
    while i < tokens.len() {
        let is_ordinal = matches!(tokens[i].kind, TokenKind::Number(n) if n.suffix.is_none())
            && tokens.get(i + 1).is_some_and(|t| t.kind.is_period())
            && tokens[i].span.end == tokens[i + 1].span.start
            && tokens.get(i + 2).is_some_and(|t| t.kind.is_space())
            && tokens.get(i + 3).is_some_and(|t| {
                (t.kind.is_word() || t.kind.is_number()) && follows(t.get_ch(source))
            });
        if is_ordinal {
            let mut token = tokens[i].clone();
            token.span.end = tokens[i + 1].span.end;
            out.push(token);
            i += 2;
        } else {
            out.push(tokens[i].clone());
            i += 1;
        }
    }
    *tokens = out;
}

#[cfg(test)]
mod tests {
    use super::{condense_abbreviations, condense_ordinals};
    use crate::parsers::{Parser, PlainEnglish};

    fn texts(tokens: &[crate::Token], source: &[char]) -> Vec<String> {
        tokens
            .iter()
            .filter(|t| !t.kind.is_whitespace())
            .map(|t| t.get_str(source))
            .collect()
    }

    #[test]
    fn joins_abbreviations_with_spaces() {
        let source: Vec<char> = "Das ist z. B. so, bzw. nicht. Wortz. B.".chars().collect();
        let mut tokens = PlainEnglish.parse(&source);
        condense_abbreviations(&mut tokens, &source, &["z. B.", "bzw."]);
        assert_eq!(
            texts(&tokens, &source),
            [
                "Das", "ist", "z. B.", "so", ",", "bzw.", "nicht", ".", "Wortz", ".", "B", "."
            ]
        );
    }

    #[test]
    fn joins_ordinals_before_a_matching_word() {
        let source: Vec<char> = "Am 8. November und am 8. 11. um 5. Ende 3."
            .chars()
            .collect();
        let mut tokens = PlainEnglish.parse(&source);
        condense_ordinals(&mut tokens, &source, |w| {
            w[0].is_lowercase()
                || w[0].is_ascii_digit()
                || w == ['N', 'o', 'v', 'e', 'm', 'b', 'e', 'r']
        });
        assert_eq!(
            texts(&tokens, &source),
            [
                "Am", "8.", "November", "und", "am", "8.", "11.", "um", "5", ".", "Ende", "3", "."
            ]
        );
    }
}

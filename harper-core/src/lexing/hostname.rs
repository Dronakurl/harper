use crate::TokenKind;

use super::FoundToken;

/// Lex a hostname token.
pub fn lex_hostname_token(source: &[char]) -> Option<FoundToken> {
    let mut len = lex_hostname(source)?;

    // A full stop right after the hostname ends the sentence: "Visit example.com."
    if len > 1 && source.get(len - 1) == Some(&'.') {
        len -= 1;
    }

    // Might be word, just skip it.
    if len <= 1 {
        return None;
    }

    if !source.get(1..len - 1)?.contains(&'.') {
        return None;
    }

    if source.get(len - 1) == Some(&'.') {
        return None;
    }

    // For the sake of semantics and downstream grammar checking.
    if !ends_with_common_tld(&source[0..len]) && !source.starts_with(&['w', 'w', 'w', '.']) {
        return None;
    }

    Some(FoundToken {
        next_index: len,
        token: TokenKind::Hostname,
    })
}

pub fn lex_hostname(source: &[char]) -> Option<usize> {
    let mut passed_chars = 0;

    // The beginning has different requirements from the rest of the hostname.
    let first = source.first()?;

    if !matches!(first,  'A'..='Z' |  'a'..='z' | '0'..='9' ) {
        return None;
    }

    for label in source.split(|c| *c == '.') {
        for c in label {
            passed_chars += 1;
            if !matches!(c,  'A'..='Z' |  'a'..='z' | '0'..='9' | '-') {
                return Some(passed_chars - 1);
            }
        }

        passed_chars += 1;
    }

    if passed_chars == 0 {
        None
    } else {
        Some(passed_chars - 1)
    }
}

const COMMON_TLDS: &[&[char]] = &[
    &['c', 'o', 'm'],
    &['n', 'e', 't'],
    &['o', 'r', 'g'],
    &['e', 'd', 'u'],
    &['g', 'o', 'v'],
    &['m', 'i', 'l'],
    &['t', 'x', 't'],
    &['i', 'o'],
    &['c', 'o'],
    &['u', 's'],
    &['u', 'k'],
    &['d', 'e'],
    &['c', 'a'],
    &['a', 'u'],
    &['j', 'p'],
    &['a', 't'],
    &['c', 'h'],
    &['e', 'u'],
    &['f', 'r'],
    &['n', 'l'],
    &['i', 'n', 'f', 'o'],
    // File extensions, which look the same: "report.pdf".
    &['p', 'd', 'f'],
    &['d', 'o', 'c', 'x'],
    &['x', 'l', 's', 'x'],
    &['p', 'p', 't', 'x'],
    &['o', 'd', 't'],
    &['c', 's', 'v'],
    &['j', 'p', 'g'],
    &['p', 'n', 'g'],
    &['z', 'i', 'p'],
    &['h', 't', 'm', 'l'],
    &['j', 's', 'o', 'n'],
];

fn ends_with_common_tld(input: &[char]) -> bool {
    for tld in COMMON_TLDS {
        let n = tld.len();
        if input.len() >= n && &input[input.len() - n..] == *tld {
            return true;
        }
    }
    false
}

#[cfg(test)]
pub mod tests {
    use super::{lex_hostname, lex_hostname_token};

    fn hostname_len(text: &str) -> Option<usize> {
        let chars: Vec<char> = text.chars().collect();
        lex_hostname_token(&chars).map(|t| t.next_index)
    }

    #[test]
    fn a_full_stop_after_a_hostname_is_not_part_of_it() {
        assert_eq!(hostname_len("example.com. Next"), Some(11));
        assert_eq!(hostname_len("www.beispiel.de.\n"), Some(15));
    }

    #[test]
    fn www_makes_a_hostname_whatever_its_tld() {
        assert_eq!(hostname_len("www.example.shop"), Some(16));
    }

    #[test]
    fn file_names_with_common_extensions() {
        assert_eq!(hostname_len("report.pdf"), Some(10));
        assert_eq!(hostname_len("bericht.docx,"), Some(12));
    }

    #[test]
    fn a_missing_space_is_not_a_hostname() {
        assert_eq!(hostname_len("end.Then"), None);
        assert_eq!(hostname_len("end.then"), None);
    }

    pub fn example_domain_parts() -> impl Iterator<Item = Vec<char>> {
        [
            r"example.com",
            r"example.com",
            r"example.com",
            r"and.subdomains.example.com",
            r"example.com",
            r"example.com",
            r"example",
            r"s.example",
            r"example.org",
            r"example.org",
            r"example.org",
            r"strange.example.com",
            r"example.org",
            r"example.org",
        ]
        .into_iter()
        .map(|s| s.chars().collect())
    }

    #[test]
    fn can_parse_example_hostnames() {
        for domain in example_domain_parts() {
            dbg!(domain.iter().collect::<String>());
            assert_eq!(lex_hostname(&domain), Some(domain.len()));
        }
    }

    #[test]
    fn hyphen_cannot_open_hostname() {
        let host: Vec<_> = "-something.com".chars().collect();
        assert!(lex_hostname(&host).is_none())
    }
}

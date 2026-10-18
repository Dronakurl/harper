use std::path::Path;

use crate::comment_parsers;
use comment_parsers::{Go, JavaDoc, JsDoc, Lua, Solidity, Unit};
use harper_core::parsers::{self, MarkdownOptions, Parser};
use harper_core::spell::MutableDictionary;
use harper_core::{Document, Token};
use tree_sitter::Node;

use crate::masker::CommentMasker;

pub struct CommentParser {
    inner: parsers::Mask<CommentMasker, Box<dyn Parser>>,
}

impl CommentParser {
    pub fn create_ident_dict(&self, source: &[char]) -> Option<MutableDictionary> {
        self.inner.masker.create_ident_dict(source)
    }

    pub fn new_from_language_id(
        language_id: &str,
        markdown_options: MarkdownOptions,
    ) -> Option<Self> {
        let language = match language_id {
            "c" => tree_sitter_c::LANGUAGE,
            "clojure" => tree_sitter_clojure::LANGUAGE,
            "cmake" => tree_sitter_cmake::LANGUAGE,
            "cpp" => tree_sitter_cpp::LANGUAGE,
            "csharp" => tree_sitter_c_sharp::LANGUAGE,
            "dart" => harper_tree_sitter_dart::LANGUAGE,
            "gleam" => tree_sitter_gleam::LANGUAGE,
            "elixir" => tree_sitter_elixir::LANGUAGE,
            "go" => tree_sitter_go::LANGUAGE,
            "groovy" => tree_sitter_groovy::LANGUAGE,
            "haskell" => tree_sitter_haskell::LANGUAGE,
            "daml" => tree_sitter_haskell::LANGUAGE,
            "java" => tree_sitter_java::LANGUAGE,
            "javascript" => tree_sitter_javascript::LANGUAGE,
            "javascriptreact" => tree_sitter_typescript::LANGUAGE_TSX,
            "kotlin" => tree_sitter_kotlin_ng::LANGUAGE,
            "lua" => tree_sitter_lua::LANGUAGE,
            "nix" => tree_sitter_nix::LANGUAGE,
            "php" => tree_sitter_php::LANGUAGE_PHP,
            "powershell" => tree_sitter_powershell::LANGUAGE,
            "ruby" => tree_sitter_ruby::LANGUAGE,
            "rust" => tree_sitter_rust::LANGUAGE,
            "scala" => tree_sitter_scala::LANGUAGE,
            "shellscript" => tree_sitter_bash::LANGUAGE,
            "solidity" => tree_sitter_solidity::LANGUAGE,
            "swift" => tree_sitter_swift::LANGUAGE,
            "toml" => tree_sitter_toml_ng::LANGUAGE,
            "typescript" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT,
            "typescriptreact" => tree_sitter_typescript::LANGUAGE_TSX,
            "zig" => tree_sitter_zig::LANGUAGE,
            _ => return None,
        };

        let comment_parser: Box<dyn Parser> = match language_id {
            "go" => Box::new(Go::new_markdown(markdown_options)),
            "java" => Box::new(JavaDoc::default()),
            "javascript" | "javascriptreact" | "typescript" | "typescriptreact" => {
                Box::new(JsDoc::new_markdown(markdown_options))
            }
            "lua" => Box::new(Lua::new_markdown(markdown_options)),
            "solidity" => Box::new(Solidity::new_markdown(markdown_options)),
            _ => Box::new(Unit::new_markdown(markdown_options)),
        };

        Some(Self {
            inner: parsers::Mask::new(
                CommentMasker::new(language.into(), Self::node_condition),
                comment_parser,
            ),
        })
    }

    /// Infer the programming language from a provided filename.
    pub fn new_from_filename(filename: &Path, markdown_options: MarkdownOptions) -> Option<Self> {
        Self::new_from_language_id(Self::filename_to_filetype(filename)?, markdown_options)
    }

    /// Convert a provided path to a corresponding Language Server Protocol file
    /// type.
    ///
    /// Note to contributors: try to keep this in sync with
    /// [`Self::new_from_language_id`]
    fn filename_to_filetype(path: &Path) -> Option<&'static str> {
        Some(match path.extension()?.to_str()? {
            "c" => "c",
            "bb" | "cljc" | "cljd" | "clj" | "cljs" => "clojure",
            "cmake" => "cmake",
            "cpp" | "h" => "cpp",
            "cs" => "csharp",
            "dart" => "dart",
            "gleam" => "gleam",
            "ex" | "exs" => "elixir",
            "go" => "go",
            "groovy" | "gradle" => "groovy",
            "hs" => "haskell",
            "daml" => "daml",
            "java" => "java",
            "js" => "javascript",
            "jsx" => "javascriptreact",
            "kt" | "kts" => "kotlin",
            "lua" => "lua",
            "nix" => "nix",
            "php" => "php",
            "ps1" | "psd1" | "psm1" => "powershell",
            "rb" => "ruby",
            "rs" => "rust",
            "sbt" | "sc" | "scala" | "mill" => "scala",
            "bash" | "sh" => "shellscript",
            "sol" => "solidity",
            "swift" => "swift",
            "toml" => "toml",
            "ts" => "typescript",
            "tsx" => "typescriptreact",
            "zig" => "zig",
            _ => return None,
        })
    }

    fn node_condition(n: &Node) -> bool {
        n.kind().contains("comment")
    }
}

impl Parser for CommentParser {
    fn parse(&self, source: &[char]) -> Vec<Token> {
        self.inner.parse(source)
    }
}

#[cfg(test)]
mod tests {
    use super::CommentParser;
    use harper_core::parsers::{MarkdownOptions, StrParser};

    #[test]
    fn hang() {
        use std::sync::mpsc::channel;
        use std::thread;
        use std::time::Duration;

        let (tx, rx) = channel::<()>();

        let handle = thread::spawn(move || {
            let opts = MarkdownOptions::default();
            let parser = CommentParser::new_from_language_id("java", opts).unwrap();
            let _res = parser.parse_str("//{@j");
            tx.send(()).expect("send failed");
        });

        rx.recv_timeout(Duration::from_secs(10)).expect("timed out");
        handle.join().expect("failed to join");
    }
}

/// The stretches of a source file that are comments: runs of tokens with only white space between.
///
/// For a language plugin, which lints text rather than tokens and so has to be told where the
/// prose is.
pub fn comment_regions(document: &Document) -> Vec<(usize, usize)> {
    let source = document.get_source();
    let mut regions: Vec<(usize, usize)> = Vec::new();

    for token in document.get_tokens() {
        // The masker puts a paragraph break over the code between two comments.
        if token.kind.is_paragraph_break() {
            continue;
        }
        let (start, end) = (token.span.start, token.span.end);
        match regions.last_mut() {
            Some(last) if source[last.1..start].iter().all(|c| c.is_whitespace()) => last.1 = end,
            _ => regions.push((start, end)),
        }
    }
    regions
}

#[cfg(test)]
mod region_tests {
    use super::*;
    use harper_core::spell::FstDictionary;

    #[test]
    fn code_between_comments_is_not_a_region() {
        let text = "// eins zwei\nfn main() {\n    let x = 1; /* drei vier */\n}\n";
        let parser =
            CommentParser::new_from_filename(Path::new("a.rs"), MarkdownOptions::default())
                .unwrap();
        let document = Document::new(text, &parser, &FstDictionary::curated());
        let chars: Vec<char> = text.chars().collect();
        let regions: Vec<String> = comment_regions(&document)
            .into_iter()
            .map(|(s, e)| chars[s..e].iter().collect())
            .collect();
        assert_eq!(regions, vec!["eins zwei", "drei vier"]);
    }
}

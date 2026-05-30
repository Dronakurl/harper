---
title: Harper's Architecture
---

This document seeks to solve one simple problem:

> "Roughly, it takes 2x more time to write a patch if you are unfamiliar with the project, but it takes 10x more time to figure out **where** you should change the code." - [Alex Kladov](https://matklad.github.io/2021/02/06/ARCHITECTURE.md.html)

This document is meant to serve as a kind of table of contents for the Harper project.
Hopefully, we can reduce that 10x down to something a little more reasonable.

## What does Harper do?

Harper tries to do one thing well: find grammatical and spelling errors in prose text.
If possible, provide suggestions to correct those errors.
An error and its possible corrections together form what we call a lint.

Historically Harper started as an English-focused linter, and much of the codebase still reflects that history.
Today the live language architecture also supports German and Portuguese.

## `harper-core`

`harper-core` is where all the magic happens.
It contains the code needed to tokenize, parse, analyze and lint prose text.

At a high level, there are just a couple types you need to worry about.

- [Document](https://docs.rs/harper-core/latest/harper_core/struct.Document.html): A representation of a document being linted. Implements [`TokenStringExt`](https://docs.rs/harper-core/latest/harper_core/trait.TokenStringExt.html) to make it easier to query.
- [Parser](https://docs.rs/harper-core/latest/harper_core/parsers/trait.Parser.html): A trait that describes an object that consumes text and emits tokens. The name is somewhat of a misnomer since it is supposed to lex prose tokens (and emit [Tokens](https://docs.rs/harper-core/latest/harper_core/struct.Token.html)), not build a full syntax tree. It is called a parser since most types that implement this trait parse _other_ languages (JavaScript, Markdown, Typst, etc.) to extract the prose text Harper should lint.
  - The [Markdown parser](https://docs.rs/harper-core/latest/harper_core/parsers/struct.Markdown.html) is a great example.
- [Linter](https://docs.rs/harper-core/latest/harper_core/linting/trait.Linter.html): A trait that, provided a document, will produce zero or more [Lints](https://docs.rs/harper-core/latest/harper_core/linting/struct.Lint.html#). This is usually done using direct queries on the document or by implementing a [`PatternLinter`](https://docs.rs/harper-core/latest/harper_core/linting/trait.PatternLinter.html).

If you want to add a linter to Harper, create a new file under the `linters` module in `harper-core` and create a public struct that implements the `Linter` trait.
There are a couple places in other parts of the codebase you'll need to update before it will show up in editors and have persistent settings, but that's a problem for after you've opened your pull request.

### Language support architecture

Language support now has three layers:

1. **Dialect selection** in `harper-core/src/dict_word_metadata.rs`
2. **Language-family routing** in `harper-core/src/language/registry.rs`
3. **Per-language implementations** under `harper-core/src/language/*`

The main public selector is `Dialect`.
That type includes English dialects plus the currently supported non-English variants:

- English: American, Canadian, Australian, British, Indian
- German: German, GermanAustrian, GermanSwiss
- Portuguese: Portuguese

`Dialect` is the source of truth for runtime behavior.
It answers questions like:

- Which broad language family is this (`language_family()`)?
- Is this dialect English, German, or Portuguese?
- If automatic detection finds a language family, which concrete dialect variant should we use (`resolve_detected_language_family(...)`)?

`LanguageFamily` in `harper-core/src/languages.rs` is a smaller internal grouping layer used when code only cares about the broad language, not the exact regional variant.
For example, the Markdown and Org parsers care whether inline prose should be tokenized as English, German, or Portuguese; they do not usually care whether English is American or British.

### Parser and dictionary routing

`harper-core/src/language/registry.rs` is the central switchboard for live language behavior.
Given a `Dialect`, it decides:

- which prose parser to use for `plaintext`, `markdown`, `quarto`, and `org`
- which dictionary to load
- which language-specific linters to add
- which rules stay enabled by default for that language

That file is where the high-level language architecture comes together.
If you are trying to understand why one language behaves differently from another, start there.

The key design point is that parsing now routes through `LanguageFamily` instead of special-casing individual formats.
`harper-core/src/parsers/mod.rs` exposes `parse_inline_prose(LanguageFamily, ...)`, and both `Markdown` and `OrgMode` use that shared seam.
That keeps German and Portuguese support analogous instead of baking format-specific logic around one language.

### Automatic language detection

Automatic language detection now also lives in `harper-core`, under `harper-core/src/language_detection/`.

- `mod.rs` defines the shared registry and the English-likelihood helper
- `english.rs`, `german.rs`, and `portuguese.rs` define the detectors for each supported language family

The registry detects a `LanguageFamily`, then resolves it back to a concrete `Dialect` using the preferred configured dialect as a hint.
That means:

- detecting English preserves an English preference like British vs American
- detecting German preserves a configured German variant like Swiss German
- falling back still returns the caller's configured default dialect

This is important because detection should choose the right language module without silently discarding the user's preferred regional variant.

### What `harper-ls` does now

`harper-ls` no longer owns the language-detection implementation.
Instead, its backend decides **when** to detect and `harper-core` decides **how** to detect.

The live flow in `harper-ls/src/backend.rs` is:

1. decide whether a document looks like prose
2. if it does, and it has enough words, call the core `LanguageDetectionRegistry`
3. receive a `Dialect`
4. pass that `Dialect` back into `harper-core` routing for parser, dictionary, and linter selection

So the split of responsibilities is:

- `harper-ls`: document lifecycle, prose gating, caching, and when-to-detect policy
- `harper-core`: which language was detected and how that language changes parsing and linting

### Where to change things

If you are adding or refactoring language support, these are the main files to inspect:

| Concern | Main files |
| --- | --- |
| Top-level runtime language choice | `harper-core/src/dict_word_metadata.rs` |
| Language grouping | `harper-core/src/languages.rs` |
| Parser/dictionary/linter routing | `harper-core/src/language/registry.rs` |
| Language detection | `harper-core/src/language_detection/*` |
| Language-specific parser/dictionary/linter code | `harper-core/src/language/<language>/*` |
| Detection policy in the LSP | `harper-ls/src/backend.rs` |

If you add a new language, try to keep those seams aligned.
The architecture is easiest to reason about when:

- `Dialect` stays the public selector
- `LanguageFamily` stays the parser/detector grouping
- `language/registry.rs` stays the one routing table
- each language keeps its implementation under `harper-core/src/language/<language>/`

## `harper-ls`

`harper-ls` is a language server that wraps around `harper-core`.
In essence, it enables text editors and IDEs to access the capabilities of Harper over a network or via standard input/output.

If you aren't familiar with what a language server does, I would suggest reading [this](https://tamerlan.dev/an-introduction-to-the-language-server-protocol/) or the [official language server protocol documentation](https://microsoft.github.io/language-server-protocol/).

When Harper is used through Neovim, Visual Studio Code, Helix, Emacs or Sublime Text, `harper-ls` is the interface.

You can read more about it [here](../integrations/language-server).

## `harper.js`

`harper.js` is a JavaScript/TypeScript module that enables developers to use Harper on any platform that supports JavaScript and WebAssembly.
Most of the JavaScript code in `harper.js` exists to load and manage the underlying WebAssembly module (otherwise known as `harper-wasm`).

[There are more details about it in the documentation.](../harperjs/introduction)

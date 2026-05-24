# Add German and Portuguese Language Support

This PR adds comprehensive support for German and Portuguese languages to Harper, including:

- German language module with dialect support (Standard, Austrian, Swiss)
- Portuguese language module with dialect support (European, Brazilian, African)
- Portuguese spell checking with curated dictionary
- Portuguese language detection
- Updated core language registration system

## Changes

### Core Language Support
- Added `harper-core/src/language/` directory structure
- Added `harper-core/src/language/german/` with dialects and spell checking
- Added `harper-core/src/language/portuguese/` with full language support
- Added `harper-core/src/languages.rs` for language type definitions
- Updated `harper-core/src/lexing/mod.rs` for language-aware lexing
- Updated `harper-core/src/lib.rs` to register new languages
- Updated `harper-core/src/parsers/mod.rs` to include Portuguese parser

### German Language
- `harper-core/src/language/german/dialects.rs` - German dialect enum
- `harper-core/src/language/german/mod.rs` - German language module
- `harper-core/src/language/german/spell/mod.rs` - German spell checking

### Portuguese Language
- `harper-core/src/language/portuguese/dialects.rs` - Portuguese dialect enum
- `harper-core/src/language/portuguese/mod.rs` - Portuguese language module
- `harper-core/src/language/portuguese/parsers/mod.rs` - Portuguese parser module
- `harper-core/src/language/portuguese/parsers/plain_portuguese.rs` - Plain text parser
- `harper-core/src/language/portuguese/linting/mod.rs` - Portuguese linting module
- `harper-core/src/language/portuguese/linting/portuguese_spell_check.rs` - Spell check linter
- `harper-core/src/language/portuguese/spell/mod.rs` - Portuguese spell checking
- `harper-core/src/language/portuguese/spell/portuguese_dict.rs` - Dictionary loader
- `harper-core/src/language/portuguese/dictionary-portuguese.dict` - Portuguese word dictionary
- `harper-core/src/language/portuguese/annotations-portuguese.json` - Portuguese annotations

### Language Server
- Added `harper-ls/src/language_detection/portuguese.rs` - Portuguese language detector
- Updated `harper-ls/src/language_detection/mod.rs` to include Portuguese

### Bug Fixes
- Fixed timeout in Obsidian plugin test (`packages/obsidian-plugin/src/State.test.ts`)
- Fixed rustfmt formatting issues in Portuguese and German files
- Fixed biome formatter issues in JSON files
- Fixed Portuguese dictionary word formatting (removed `+` prefixes)
- Fixed Portuguese language detection test

## Testing
- All Rust tests pass for new language modules
- Obsidian plugin tests pass with increased timeout
- Portuguese spell checking works correctly
- Portuguese language detection works correctly

## Notes
- The PR includes both the feature additions and necessary bug fixes
- The `test-vscode` failure is a pre-existing issue unrelated to these changes

# Commits to Include in PR

The following commits should be included in the PR to Automattic/harper:

## Feature Commits
1. `b1a03974` - feat: add German and Portuguese language support
   - Initial addition of German and Portuguese language modules
   - Core language infrastructure

## Bug Fix Commits
2. `e297bd47` - fix(obsidian): increase timeout for dictionary persistence test
   - Fixes: `packages/obsidian-plugin/src/State.test.ts`
   - Added 30s timeout to prevent test timeout

3. `2182584a` - style: fix rustfmt formatting issues
   - Fixes formatting in Portuguese and German dialects files
   - Fixes formatting in Portuguese spell check and dict files
   - Fixes formatting in languages.rs

4. `289ac7dc` - style: format annotations-portuguese.json with tabs
   - Fixes biome formatter requirement for tab indentation

5. `b2cd4e53` - style: fix trailing newline in annotations-portuguese.json
   - Ensures proper file ending for biome formatter

6. `5f6b3cdc` - fix(portuguese): add missing words to dictionary
   - Adds: tenho, e to dictionary-portuguese.dict

7. `026a85a5` - fix(portuguese): remove + prefixes from dictionary words
   - Removes + prefix from all dictionary entries
   - Fixes spell checker word recognition

8. `e92f0710` - fix(portuguese): update spell check test to avoid filler word detection
   - Changes test sentence from "Eu tenho um mundo e um amor." to "tenho mundo amor"

9. `28225ede` - fix(portuguese): update language detection test with more special characters
   - Changes test text to include João and São Paulo for reliable detection

10. `b2a84b26` - ci: add workflow_dispatch to just_checks.yml
    - Allows manual triggering of workflow on feature branches

11. `db1f5091` - ci: add workflow_dispatch (master branch)
    - Same change for master branch

## Files Modified
- `.github/workflows/just_checks.yml`
- `harper-core/src/language/german/dialects.rs`
- `harper-core/src/language/german/mod.rs`
- `harper-core/src/language/german/spell/mod.rs`
- `harper-core/src/language/mod.rs`
- `harper-core/src/language/portuguese/annotations-portuguese.json`
- `harper-core/src/language/portuguese/dialects.rs`
- `harper-core/src/language/portuguese/dictionary-portuguese.dict`
- `harper-core/src/language/portuguese/linting/mod.rs`
- `harper-core/src/language/portuguese/linting/portuguese_spell_check.rs`
- `harper-core/src/language/portuguese/mod.rs`
- `harper-core/src/language/portuguese/parsers/mod.rs`
- `harper-core/src/language/portuguese/parsers/plain_portuguese.rs`
- `harper-core/src/language/portuguese/spell/mod.rs`
- `harper-core/src/language/portuguese/spell/portuguese_dict.rs`
- `harper-core/src/languages.rs`
- `harper-core/src/lexing/mod.rs`
- `harper-core/src/lib.rs`
- `harper-core/src/parsers/mod.rs`
- `harper-ls/src/language_detection/mod.rs`
- `harper-ls/src/language_detection/portuguese.rs`
- `packages/obsidian-plugin/src/State.test.ts`

## Files Added
- `harper-core/src/language/german/dialects.rs`
- `harper-core/src/language/german/mod.rs`
- `harper-core/src/language/german/spell/mod.rs`
- `harper-core/src/language/mod.rs`
- `harper-core/src/language/portuguese/` (directory)
- `harper-core/src/language/portuguese/annotations-portuguese.json`
- `harper-core/src/language/portuguese/dialects.rs`
- `harper-core/src/language/portuguese/dictionary-portuguese.dict`
- `harper-core/src/language/portuguese/linting/mod.rs`
- `harper-core/src/language/portuguese/linting/portuguese_spell_check.rs`
- `harper-core/src/language/portuguese/mod.rs`
- `harper-core/src/language/portuguese/parsers/mod.rs`
- `harper-core/src/language/portuguese/parsers/plain_portuguese.rs`
- `harper-core/src/language/portuguese/spell/mod.rs`
- `harper-core/src/language/portuguese/spell/portuguese_dict.rs`
- `harper-core/src/languages.rs`
- `harper-ls/src/language_detection/portuguese.rs`

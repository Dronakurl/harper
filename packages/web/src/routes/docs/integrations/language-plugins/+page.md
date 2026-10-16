---
title: Other Languages
---

Harper checks English out of the box.
Other languages are **language plugins**: each language is a single file, `harper-<language>.wasm`, where `<language>` is the language's id (`de` for German, for example).
Nothing else needs to be installed, and Harper itself does not change when a language is added.

The same file works everywhere Harper runs: `harper-ls` (and so Visual Studio Code, Neovim, Helix, Zed, …), `harper-cli`, Obsidian, the Chrome and Firefox extensions and `harper.js`.
Several languages can be installed side by side.

## Getting a Plugin

A plugin comes from the project that makes it, either as a released `harper-<language>.wasm` or built from its source.
For German, see [harper-german](https://github.com/Dronakurl/harper-german); building it there (`just build-wasm`) produces `target/wasm32-unknown-unknown/release/harper_german.wasm`.
The file name does not matter for installing it with `harper-cli plugin install` or in the browser extension: they read the language id from the plugin itself.

## `harper-ls` and `harper-cli`

Both look for plugins in the same place:

| System  | Plugin directory                                  |
| ------- | ------------------------------------------------- |
| Linux   | `~/.config/harper/plugins/`                       |
| macOS   | `~/Library/Application Support/harper/plugins/`   |
| Windows | `%APPDATA%\harper\plugins\`                       |

The easiest way to put a plugin there is `harper-cli`, which checks that the file is a plugin this Harper can use and names it after its language:

```bash
harper-cli plugin install path/to/plugin.wasm   # copies it to <plugin directory>/harper-<language>.wasm
harper-cli plugin list                          # the installed plugins, their names and dialects
harper-cli plugin remove de                     # removes harper-de.wasm
```

Copying the file there by hand, named `harper-<language>.wasm`, works just as well.
From a checkout of this repository, `just install` installs `harper-ls` and `harper-cli` (English only, no plugins), and `just install-plugin path/to/plugin.wasm` adds a language.

### `harper-ls`

With the default setting, `"language": "auto"`, `harper-ls` checks each document in the language of an installed plugin when the document (for source code: its comments) is written in that language, and in English otherwise.
A plugin added to the directory is used without restarting `harper-ls`.

To check every document in one language instead, set `language` to the plugin's id or one of the names it answers to (`harper-cli plugin list` shows them):

```json
{
  "harper-ls": {
    "language": "de"
  }
}
```

`"en"` turns plugins off. `pluginDirectory` points `harper-ls` at another directory.
See the [language server settings](./language-server#Configuration) for the details.

### `harper-cli`

`harper-cli lint` checks English unless told otherwise:

```bash
harper-cli lint --language de notes.md     # in the plugin's language
harper-cli lint --language auto notes.md   # each file in the language it is written in
harper-cli lint --plugin path/to/plugin.wasm notes.md   # with a plugin that is not installed
```

## Obsidian

Put the plugin file, named `harper-<language>.wasm`, into the `languages` folder of Harper's plugin folder in your vault: `.obsidian/plugins/harper/languages/`.
Then reopen Harper's settings and choose the language.
Each language keeps its own dictionary and rule settings.

## Chrome and Firefox

Open the extension's options, choose the plugin file under **Add a language plugin**, then pick the language.
The extension keeps the file, so this is needed only once.
The command **Switch to the next language** cycles through English and the installed languages; give it a key under `chrome://extensions/shortcuts`.

## `harper.js`

```js
import { PluginLinter } from 'harper.js';

// A URL, a fetch Response or the file's bytes.
const linter = new PluginLinter({ plugin: '/harper-de.wasm' });
const lints = await linter.lint('Ich habe das Buch gelesent.');
```

## Making a Plugin for a New Language

A plugin is a Rust crate compiled to `wasm32-unknown-unknown` that implements `LanguagePlugin` from the `harper-plugin-api` crate and calls `export_plugin!`.
The plugin says which language it is (its id, a name, other names it answers to and its dialects), lints text and can tell how likely a text is to be in its language, which is what `auto` uses.
[`harper-plugin-api/examples/echo.rs`](https://github.com/Dronakurl/harper/blob/claude/language-plugin-architecture-h8bw6z/harper-plugin-api/examples/echo.rs) is a complete, tiny plugin; [harper-german](https://github.com/Dronakurl/harper-german) is a full one.

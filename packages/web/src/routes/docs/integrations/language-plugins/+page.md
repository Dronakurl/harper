---
title: Other Languages
---

Harper checks English out of the box.
Other languages are **language plugins**: each language is a single file, `harper-<language>.wasm`, where `<language>` is the language's id, a short code such as the language's ISO 639-1 code.
Nothing else needs to be installed, and Harper itself does not change when a language is added.

The same file works everywhere Harper runs: `harper-ls` (and so Visual Studio Code, Neovim, Helix, Zed, …), `harper-cli`, Obsidian, the Chrome and Firefox extensions and `harper.js`.
Several languages can be installed side by side.

## Getting a Plugin

The [list of language plugins](./available-languages) says which languages there are and where each one comes from.
A plugin is published as a release of the project that makes it: a file `harper-<language>.wasm`, usually with a `.sha256` checksum next to it.
Download it from the project's releases page, or let `just language-install` do that (below).

## `harper-ls` and `harper-cli`

Both look for plugins in the same place:

| System  | Plugin directory                                |
| ------- | ----------------------------------------------- |
| Linux   | `~/.config/harper/plugins/`                     |
| macOS   | `~/Library/Application Support/harper/plugins/` |
| Windows | `%APPDATA%\harper\plugins\`                     |

From a checkout of the Harper repository, with [`just`](https://github.com/casey/just) installed:

```bash
just install                        # harper-ls and harper-cli; they check English
just language-install <owner>/<repo>   # downloads the plugin from the project's latest release and installs it
just language-install <owner>/<repo> v1.2.0   # a particular release
```

`just language-install` needs `curl`, and `harper-cli` on your `PATH`.
It checks the download against the release's `.sha256` file when there is one.

With a plugin file at hand, `harper-cli` installs it directly.
It checks that the file is a plugin this Harper can use and names it after its language, so the file name does not matter:

```bash
harper-cli plugin install path/to/plugin.wasm   # copies it to <plugin directory>/harper-<language>.wasm
harper-cli plugin list                          # the installed plugins, their names and dialects
harper-cli plugin remove <language>             # removes harper-<language>.wasm
```

Copying the file there by hand, named `harper-<language>.wasm`, works just as well.

### `harper-ls`

With the default setting, `"language": "auto"`, `harper-ls` checks each document in the language of an installed plugin when the document (for source code: its comments) is written in that language, and in English otherwise.
A plugin added to the directory is used without restarting `harper-ls`.

To check every document in one language instead, set `language` to the plugin's id or one of the names it answers to (`harper-cli plugin list` shows them):

```json
{
  "harper-ls": {
    "language": "<language>"
  }
}
```

A plugin's dialect is chosen the same way: `"language": "Swiss"` or `"language": "de-CH"`, or `"language": "de"` together with `"dialect": "Swiss"`; `harper-cli plugin list` shows the dialects a plugin has.

`"en"` turns plugins off. `pluginDirectory` points `harper-ls` at another directory.
See the [language server settings](./language-server#Configuration) for the details.

### `harper-cli`

`harper-cli lint` checks English unless told otherwise:

```bash
harper-cli lint --language <language> notes.md   # in the plugin's language
harper-cli lint --language auto notes.md         # each file in the language it is written in
harper-cli lint --plugin path/to/plugin.wasm notes.md   # with a plugin that is not installed
harper-cli lint --language <language> --dialect <dialect> notes.md   # in one of the plugin's dialects
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
const linter = new PluginLinter({ plugin: '/harper-<language>.wasm' });
const lints = await linter.lint(text);
```

## Making a Plugin for a New Language

A plugin is a Rust crate compiled to `wasm32-unknown-unknown` that implements `LanguagePlugin` from the `harper-plugin-api` crate and calls `export_plugin!`.
The plugin says which language it is (its id, a name, other names it answers to and its dialects), lints text and can tell how likely a text is to be in its language, which is what `auto` uses.
[`harper-plugin-api/examples/echo.rs`](harper-plugin-api/examples/echo.rs) is a complete, tiny plugin, and the projects in the [list of language plugins](./available-languages) are full ones.

For `just language-install` to find it, publish the plugin as a GitHub release with exactly one asset named `harper-<language>.wasm`, ideally with `harper-<language>.wasm.sha256` (the output of `sha256sum`) next to it.
Then add the language to the [list](./available-languages).

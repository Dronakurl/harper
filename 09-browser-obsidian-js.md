# 9. Browser, Obsidian, harper.js

[← Wörterbücher](08-woerterbuecher.md) · [Weiter: Das deutsche Plugin →](10-deutsches-plugin.md)

Der zweite Host: dasselbe Plugin, dieselbe JSON-Sprache, aber in
TypeScript. Es gibt kein wasmtime; der Browser (oder Node) führt
WebAssembly selbst aus.

```
harper.js  ← Kern: PluginLinter
  ▲   ▲
  │   └── Chrome / Firefox
  └────── Obsidian
```

Die Rust-Hosts (Kapitel 3 bis 6) und dieser hier sprechen exakt
denselben Vertrag. Wer Kapitel 2 verstanden hat, erkennt alles
wieder.

## harper.js: `PluginLinter`

Datei: [`packages/harper.js/src/PluginLinter.ts`][pl] (rund 520
Zeilen). Test: [`PluginLinter.test.ts`][plt].

Ziel: ein `Linter`, der sich von außen wie `LocalLinter` (Harpers
englischer Wasm-Linter) benutzen lässt.

### Laden: `PluginRuntime`

[`PluginRuntime.load`][rt]:

1. Quelle kompilieren ([`compile`][comp]): URL, `Response`, Bytes oder
   fertiges `WebAssembly.Module`.
2. **Importe** des Moduls auflisten. Funktionen werden durch Stubs
   ersetzt, die einen Fehler werfen; alles andere (Speicher, Tabellen
   …) wird abgelehnt. Nichts vom Host wird übergeben: Das Plugin
   kommt weder an die Seite noch an das Netz.
3. Instanziieren, die fünf Exporte prüfen, ABI-Version prüfen.
4. `info` abfragen.

Das ist die TS-Version von `instantiate` aus Kapitel 3.

### Aufruf: `call`

[`call`][call] ist `Guest::roundtrip` in JavaScript: UTF-8 kodieren,
`harper_alloc`, Bytes in den Speicher schreiben, `harper_call`, den
`u64` mit `BigInt` aufteilen (`>> 32n`), lesen, zweimal
`harper_free`.

### Das Lint-Objekt

Die Oberfläche von Harpers Lints erwartet Objekte mit Methoden
(`span()`, `suggestions()`, `get_problem_text()`, …). Die echten sind
wasm-bindgen-Objekte. [`PluginLintObject`][plo] ist ein Ersatz mit
denselben Methoden, aufgebaut aus dem JSON.

Zwei Eigenheiten:

- **Zeichen vs. UTF-16.** Das Plugin zählt Unicode-Zeichen,
  JavaScript zählt UTF-16-Einheiten. Enthält der Text Surrogate
  (z. B. Emojis), baut [`runLint`][run] eine Umrechnungstabelle
  (`toUtf16`). Ohne Emojis ist das ein Identitäts-Lambda.
- Die Objekte sind keine wasm-bindgen-Objekte. Darum kann
  `WorkerLinter` sie nicht über `postMessage` schicken. Der
  `PluginLinter` läuft auf dem aufrufenden Thread.

`PluginInfo` hat `aliases` und `features`; `answersTo(info, name)`
folgt derselben Regel wie `PluginInfo::answers_to` in Rust
(Issue [#31], behoben).

### Was `PluginLinter` nicht kann

Statistik, Weirpacks, Englisch-Erkennung, Dialekte (bei Plugins
benennt das Plugin sie). Diese Methoden sind leer oder liefern
Standardwerte.

### Wörter

[`importWords`][iw] sammelt Wörter und schickt **jedes Mal die
ganze Liste** per `set_user_words`. Es gibt keinen Anfrage-Weg: In
einem Browser ist immer genau ein Text und ein Nutzer im Spiel, ein
Konflikt wie im CLI kann nicht entstehen. Ignorierte Lints
(`ignoreLint`) werden im Host gefiltert (FNV-Hash des Kontexts), nicht
im Plugin.

## Chrome und Firefox

Dateien: [`pluginStore.ts`][ps] und [`background/index.ts`][bg] im
Paket `chrome-plugin`; Optionsseite `Options.svelte`.

**Wie kommt ein Plugin in den Browser?** Erweiterungen dürfen keinen
Code nachladen (Store-Regeln). Der Nutzer wählt die Datei selbst auf
der Optionsseite. [`savePlugin`][sp] legt sie in **IndexedDB** ab,
einem Speicher, den Optionsseite und Hintergrund-Prozess teilen.
Dadurch müssen Dutzende Megabyte nicht durch eine Nachricht.

**Wahl der Sprache.** `language` steht in `chrome.storage.local`. Beim
Start liest der Hintergrund-Prozess sie und
[`initializeLinter`][il] baut entweder einen `PluginLinter` (mit den
Bytes aus IndexedDB) oder den normalen `LocalLinter`. Fehlt das
Plugin, gibt es eine Warnung und Englisch.

**Pro-Sprache-Einstellungen.** [`languageKey(name)`][lk]: für Englisch
der unveränderte Schlüssel, für andere `name:<id>`. Das gilt für die Regelkonfiguration
(`lintConfig`) und das Nutzerwörterbuch (`userDictionary`). So hat
jede Sprache ihre eigenen.

**Weirpacks** (Regelpakete für Englisch) werden nur bei Englisch
geladen.

**Sprachwechsel per Tastatur.** Der Befehl `switch-language` geht im
Kreis durch Englisch und die installierten Sprachen
(`chrome://extensions/shortcuts`). Eine Sprache pro Browser, nicht
pro Text: Spracherkennung gibt es im Browser nicht (offener Punkt).

## Obsidian

Datei: [`State.ts`][st] im Paket `obsidian-plugin`.

Plugins liegen in `.obsidian/plugins/harper/languages/` im Tresor und
werden über einen `PluginLoader` gelesen
(`(language) => Promise<ArrayBuffer | null>`).

Der Kern ist [`swapLanguageSettings`][sw]: Obsidian speichert **ein**
Settings-Objekt mit `lintSettings`, `userDictionary`, `ignoredLints`.
Beim Wechsel der Sprache wird das aktuelle Set unter
`otherLanguages[alt]` abgelegt und das der neuen Sprache nach oben
geholt. Die Funktion ist rein und getestet (`State.test.ts`).
Altbestände ohne `settingsLanguage` gelten als Englisch.

[`createLinter`][cl] wählt `PluginLinter`, `WorkerLinter` oder
`LocalLinter`. Ein Wechsel des Web-Worker-Schalters oder des
englischen Dialekts baut den Linter nur bei Englisch neu.

## Was die drei gemeinsam haben

- Genau eine aktive Sprache.
- Wörter über `set_user_words`.
- Sprachgetrennte Einstellungen (Schlüssel bzw. `otherLanguages`).
- Laden aus Bytes, nie aus dem Netz zur Laufzeit.

## Tests

- `PluginLinter.test.ts`: lädt das Echo-Plugin.
- `State.test.ts` (Obsidian): `swapLanguageSettings`.
- Chrome: `tests/language_plugin.spec.ts` und
  `language_plugin_dictionary.spec.ts` (Playwright; brauchen
  `echo.wasm`, `just test-chrome-plugin` baut es; noch nicht in der
  CI).

## Merke

1. `PluginLinter` ist `PluginRuntime` (Wasm sprechen) plus Objekte
   mit der Oberfläche der englischen Lints.
2. Importe werden zu werfenden Stubs; nichts wird dem Plugin übergeben.
3. Zeichen ≠ UTF-16: bei Surrogaten wird umgerechnet.
4. Chrome: Datei per Hand wählen → IndexedDB; Obsidian: Datei im
   Tresor.
5. Pro-Sprache-Einstellungen durch Schlüsselpräfix (Chrome) bzw.
   `swapLanguageSettings` (Obsidian).

[← Wörterbücher](08-woerterbuecher.md) · [Weiter: Das deutsche Plugin →](10-deutsches-plugin.md)

[pl]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/harper.js/src/PluginLinter.ts
[plt]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/harper.js/src/PluginLinter.test.ts
[rt]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/harper.js/src/PluginLinter.ts
[comp]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/harper.js/src/PluginLinter.ts
[call]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/harper.js/src/PluginLinter.ts
[plo]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/harper.js/src/PluginLinter.ts
[run]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/harper.js/src/PluginLinter.ts
[iw]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/harper.js/src/PluginLinter.ts
[ps]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/chrome-plugin/src/pluginStore.ts
[bg]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/chrome-plugin/src/background/index.ts
[sp]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/chrome-plugin/src/pluginStore.ts
[il]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/chrome-plugin/src/background/index.ts
[lk]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/chrome-plugin/src/background/index.ts
[st]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/obsidian-plugin/src/State.ts
[sw]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/obsidian-plugin/src/State.ts
[cl]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/packages/obsidian-plugin/src/State.ts
[#31]: https://github.com/Dronakurl/harper/issues/31

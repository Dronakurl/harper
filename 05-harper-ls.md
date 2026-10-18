# 5. harper-ls im Detail

[← Ein Lint von A bis Z](04-ein-lint-von-a-bis-z.md) · [Weiter: harper-cli →](06-harper-cli.md)

Kapitel 4 hat den Weg gezeigt. Hier die Bauteile einzeln.

Dateien:

- [`harper-ls/src/plugins.rs`][plugins]: Registry, `PluginLinter`,
  Sprachwahl. Dazu die Stubs ohne Feature und die Tests.
- [`harper-ls/src/backend.rs`][backend]: `update_document` und die
  Wörterbuch-Funktionen.
- [`document_state.rs`][ds]: ein Feld `plugin`, ein Feld `language`.
- [`config.rs`][config]: Einstellungen `language`, `pluginDirectory`.

Der Grundsatz: **Alles Plugin-Spezifische steht in `plugins.rs`.**
Der Rest von `harper-ls` kennt nur `PluginRegistry` und `PluginLinter`.
Ohne das Feature `plugins` ersetzen leere Stubs beide, und die
Sprache bleibt immer Englisch.

## Einstellungen

[`config.rs`][config]:

- `language`: `"auto"` (Standard), `"en"` oder eine Plugin-Id / ein
  Name, auf den ein Plugin hört (`"de"`, `"German"`).
- `pluginDirectory`: anderer Ordner als
  `<config>/harper/plugins`.

Ein Sonderfall zeigt, wie sorgfältig auf Altlasten geachtet wird:
Der ältere Mehrsprachen-Build von harper-ls nahm `"dialect":
"German"` und `"language": {"German": "Swiss"}`. Das liest
[`from_lsp_config`][fromlsp] weiterhin.
[`english_dialect_named`][edn] erkennt englische Namen
(`uk`, `en-GB`, …); alles andere, was wie ein Sprachname aussieht,
geht an die Spracherkennung der Plugins. Dank [`valid_language`][vl]
kann nie ein Pfad (`../de`) eingeschleust werden.

## Die Registry

[`PluginRegistry`][reg]: gehört dem `Backend`, ist von allen Dokumenten
geteilt. Zustand hinter einem `Mutex`:

- `loaded`: Pfad → geladenes Plugin (`Arc<LoadedPlugin>`)
- `failed`: Pfade, die nicht geladen werden konnten

Warum `failed`? Bei jedem Tastendruck läuft `update_document`. Ohne
diese Liste würde derselbe Ladefehler tausendfach protokolliert.
[`retry_failed`][retry] leert sie, z. B. wenn die Konfiguration
neu geladen wird, damit ein später ins Verzeichnis kopiertes Plugin
gefunden wird, ohne `harper-ls` neu zu starten.

Methoden:

- [`load(language, dir)`][load]: `harper-<language>.wasm`. Prüft, dass
  `info.id` zur Dateiname-Sprache passt (eine Datei `harper-yy.wasm`,
  die sich „xx" nennt, wird abgelehnt). Benutzt den
  Kompilier-Cache.
- [`resolve(name, dir)`][resolve]: macht aus „German" oder „Swiss" die
  Id `de`: erst Dateiname-Treffer, dann alle Plugins fragen
  (`answers_to`).
- [`detect(text, dir)`][det]: siehe unten.
- [`linter_for(...)`][lf]: baut einen `PluginLinter`.

## Sprachwahl

[`choose_language`][cl]. Reine Funktion, deshalb gut testbar:

- `en` → `en`
- feste Sprache → `resolve(...)`, notfalls der Name unverändert
- `auto`:
  - Ist es Prosa (Markdown, Klartext, Kommentare aus Code) **und**
    mindestens 10 Wörter? Dann `detect`, sonst `en`.
  - Zu wenig Text oder kein Prosa-Dokument: die bisherige Sprache
    bleibt, beim ersten Mal `en`.

„Zu kurz zum Erkennen" zählt also nicht als „Englisch". Wer
„Hallo" tippt, soll nicht von Deutsch auf Englisch springen.

Quellcode wird nicht geraten: Bei `auto` und `rust`, `python` usw.
bleibt es Englisch, es sei denn, die **Kommentare** reichen zum
Erkennen. Das erledigt der Code ab [`detection_text`][dt] in
`update_document`: Harper-Parser für die Sprache, dann
`comment_regions`, dann Text der Kommentare zusammenfügen.

### Erkennung

[`PluginRegistry::detect`][det]:

- Nur die ersten 4000 Zeichen („der Anfang eines Dokuments sagt
  genug und hält es bei jedem Tastendruck billig").
- Alle Plugins, die mindestens 50 % melden, kommen in Frage. Der
  höchste Wert gewinnt.

## Formate und Regionen

Ein Plugin bekommt entweder das **ganze** Dokument oder **Regionen**.
Die Weiche:

- [`plugin_format(language_id)`][pf]: `markdown`, `quarto` →
  `markdown`; `org` → `org`; `typst` → `typst`;
  `plaintext`, `text`, `mail` → `plain`. Sonst `None`.
- Bei `None` sucht [`region_parser`][rp] einen Harper-Parser für das
  Dokument. Er findet den Text: Kommentare in Code,
  den Text zwischen TeX-Befehlen, HTML-Text, Commit-Nachrichten.
  [`comment_regions`][cr] (in `harper-comments`) liefert daraus die
  Zeichenbereiche.

Das Plugin bekommt dann den **ganzen Dateitext** und die Liste
der Bereiche (`with_regions`), prüft nur dort und meldet Spans
relativ zum Gesamttext. So bleibt ein Wort, das über zwei Zeilen
eines Kommentars geht, zusammen, und die Offsets stimmen.

Besonderheiten in `update_document`:

- Datei zu lang (`max_file_length`) → leere Regionen → keine Lints,
  und alte Lints verschwinden.
- Format nicht prüfbar (z. B. unbekannter `languageId`) → Warnung,
  das Dokument wird aus der Verwaltung entfernt.

## `DocumentState`

[`DocumentState`][ds]:

```
document      Harper-Dokument (Parser)
linter        englischer LintGroup (leer
              bei Plugin-Sprache)
plugin        Option<PluginLinter>
language      "en" oder Plugin-Id
language_id   Editor-Sprache (z. B. rust)
ignored_lints ...
```

Zwei Fälle in `generate_diagnostics` und `generate_code_actions`:
`plugin` gesetzt → Plugin fragen; sonst der bisherige englische Weg.
Das sind die einzigen Stellen außerhalb von `update_document`, die
den Unterschied kennen.

## Wo das Englische ausgespart wird

In `update_document`, [`let is_english`][ie]:

- Englisch: `LintGroup::new_curated(...)` wie immer.
- Plugin-Sprache: `LintGroup::default()` (leer). „Den englischen
  Linter für sie zu bauen würde nur Zeit und Speicher kosten."
- Wechselt die Sprache, wird der englische Linter bei Bedarf neu
  gebaut, sonst nicht.

Beachte: Das **Dokument** wird auch für Plugin-Sprachen von Harpers
(englischem) Parser gelesen. Das ist nötig für Spans und
`ignored_lints`, wirft aber die Frage auf, ob dabei unnötig der
englische Tagger läuft. Siehe Issue [#29]. (Außerdem
ist der Kopfkommentar von `plugins.rs` beschädigt, Issue [#26].)

## Merke

1. Alles Plugin-Spezifische in `plugins.rs`; Stubs ohne Feature.
2. Registry: einmal laden, Fehler merken, Neuversuch auf Befehl.
3. `auto` rät nur bei Prosa mit ≥ 10 Wörtern und hält sonst die
   alte Sprache.
4. Kein Prosa-Format → Harper findet die Regionen, das Plugin
   bekommt Text plus Bereiche.
5. Für Plugin-Dokumente gibt es keinen englischen Linter.

[← Ein Lint von A bis Z](04-ein-lint-von-a-bis-z.md) · [Weiter: harper-cli →](06-harper-cli.md)

[plugins]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/plugins.rs
[backend]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/backend.rs
[ds]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/document_state.rs
[config]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/config.rs
[fromlsp]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/config.rs#L93
[edn]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/config.rs#L294
[vl]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/config.rs#L307
[reg]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/plugins.rs#L63
[retry]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/plugins.rs#L87
[load]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/plugins.rs#L96
[resolve]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/plugins.rs#L147
[det]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/plugins.rs#L167
[lf]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/plugins.rs#L184
[cl]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/plugins.rs#L26
[dt]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/backend.rs#L399
[pf]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/plugins.rs#L9
[rp]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/backend.rs#L69
[cr]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-comments/src/comment_parser.rs#L164
[ie]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-ls/src/backend.rs#L432
[#29]: https://github.com/Dronakurl/harper/issues/29
[#26]: https://github.com/Dronakurl/harper/issues/26

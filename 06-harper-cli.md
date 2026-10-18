# 6. harper-cli im Detail

[← harper-ls](05-harper-ls.md) · [Weiter: Die Haken in harper-core →](07-core-haken.md)

Das CLI ist der einfachste Host und das beste Experimentierfeld. Es
hat keinen Editor, keine Dokumentverwaltung, nur: Dateien rein,
Lints raus. Dafür kommt etwas dazu, das `harper-ls` nicht hat:
**viele Dateien gleichzeitig**.

Dateien:

- [`harper-cli/src/plugin.rs`][plugin]: `PluginLinter`, `plugin`-Befehle
- [`lint.rs`][lint]: `LanguageChoice`, Parallelisierung
- [`lint_engine.rs`][engine]: Lint einer Eingabe
- [`main.rs`][main]: Optionen
- Tests: [`tests/plugin.rs`][tests]

Wie in `harper-ls` steht alles Plugin-Spezifische in einer Datei
(`plugin.rs`) mit Stub ohne das Feature `plugins`.

## Optionen

`harper-cli lint …`:

- `--language de`: Sprache des Plugins. `en` ist Englisch.
  (Eine Kurzform `-l` gibt es nicht; Issue von braklo, #6.)
- `--language auto`: je Eingabe das Plugin, das den Text erkennt;
  sonst Englisch.
- `--plugin datei.wasm`: ein Plugin direkt, ohne Installation.
- `--dialect german` (Altlast): wird als Sprachname behandelt, wenn
  es kein englischer Dialekt ist ([`main.rs`][main-dialect]).

Intern wird daraus eine einzige Entscheidung,
[`LanguageChoice`][lc]:

```
LanguageChoice
 ├ English
 ├ Plugin(PluginLinter)
 └ Auto(Vec<PluginLinter>)
```

Der Gedanke dahinter (Kommentar im Code): `harper-core` weiß nichts
von Plugins, also entscheidet das CLI einmal und der Lint-Code
vergleicht nur noch. `Auto` ist von `Plugin` getrennt, damit ein
**einziges** installiertes Plugin trotzdem gefragt wird, ob es den
Text erkennt, statt blind englische Texte mit deutschen Regeln zu
prüfen.

## Befehle `harper-cli plugin …`

[`PluginCommand`][pc]:

- `list`: Ordner, Ids, Namen, Versionen, Dialekte
- `install <datei>`: lädt die Datei probehalber, prüft, dass die Id
  nur aus `a-z0-9-_` besteht, und kopiert nach
  `harper-<id>.wasm`. Der Dateiname der Quelle spielt keine Rolle.
- `remove <id>`

Der Prüfschritt ist Absicht: Wer eine kaputte Datei installiert, soll
das sofort merken, nicht erst beim ersten Lint.

(Offenes Issue [#17]: `plugin` steht in der Hilfe ganz oben, soll
ans Ende.)

## Eine Datei linten

[`lint_one_input`][engine] in `lint_engine.rs`, für Plugin-Sprache:

1. **Datei-Wörterbuch laden** (`file_dict_name`), falls vorhanden.
   Standardverzeichnis ist dasselbe wie bei `harper-ls`
   (`…/harper-ls/file_dictionaries/`, Option `--file-dict-path`).
   Achtung: Anders als `harper-ls` trennt das CLI es **nicht nach
   Sprache** (Issue [#25]).
2. Dokument parsen, mit Harpers Parser.
3. **Format** wählen: [`format_for_path`][ffp] sieht die Endung.
   `.md` → `markdown`, `.org` → `org`, `.typ` → `typst`,
   `.txt` oder unbekannt → `plain`. TeX, Python, AsciiDoc und alles,
   wofür `CommentParser` zuständig ist → `None`, das heißt Regionen.
4. Bei `None`: `comment_regions(&doc)` liefert die Bereiche.
5. Bei `Auto`: Prosa (oder zusammengefügte Regionen) an
   [`best_for`][bf]. Wer mindestens 0,5 meldet und am sichersten ist,
   darf.
6. `plugin.organized_lints(text, format, regions, file_words)`.
7. `--only` / `--ignore` filtern **die Regeln des Plugins**
   (nicht die englischen).
8. Überlappende Lints entfernen (außer `--keep-overlapping-lints`),
   zählen, ausgeben.

## Parallel

Mehrere Eingaben (Verzeichnis, mehrere Dateien) werden mit `rayon`
parallel abgearbeitet ([`lint.rs`][par]; `--no-parallel` schaltet das
ab). Jeder Thread ruft `lint_one_input`. Für Englisch heißt das:
jeder Thread hat eigene Linter. Für ein Plugin heißt das: alle
Threads rufen **dasselbe** `LoadedPlugin` auf, und der `Mutex`
(Kapitel 3) reiht sie hintereinander.

Zwei Dinge folgen daraus:

- Es gibt keinen Zeitgewinn beim Linten selbst. Der Gewinn ist
  der Rest: Parsen, Lesen, Ausgabe laufen parallel.
- **Die Wörter einer Datei müssen mit der Anfrage reisen.**
  Die Threads laufen in beliebiger Reihenfolge. Hätte jede Datei
  vorher `set_user_words` gerufen, könnte Thread B zwischen A's
  `set` und A's `lint` dazwischenkommen. Mehr in
  [Kapitel 8](08-woerterbuecher.md).

Bevor das `calls`-Schloss ausgebaut wurde, gab es genau diesen Schutz
als Sperre pro Datei. Jetzt ist es überflüssig.

## Nutzerwörter

Beim Start, einmal ([`lint.rs`][uw]): für jedes mögliche Plugin die
Datei `dictionary-<id>.txt` laden (z. B. `dictionary-de.txt`, dieselbe
Namensregel wie in `harper-ls`) und mit `set_user_words` im
`PluginLinter` hinterlegen. Das speichert die Liste nur; verschickt wird
sie mit jeder Anfrage. Hinzu kommen die Wörter der Datei selbst.

Wichtig: Wörter einer Sprache erreichen nur deren Plugin. Der Test
`a_plugin_gets_the_user_words_of_its_own_language_only` hält es fest.

## Tests

[`tests/plugin.rs`][tests] baut das Echo-Plugin und startet das echte
`harper-cli`:

- `lints_with_a_plugin_file`
- `unknown_language_is_an_error`
- `english_still_works_without_a_plugin`
- `a_plugin_gets_the_user_words_of_its_own_language_only`
- [`a_files_own_words_reach_that_file_only`][t-file]: vierundzwanzig
  Dateien parallel, nur die geraden haben das Wort im eigenen
  Wörterbuch; die ungeraden dürfen es nicht sehen.

Fehlt das Wasm-Target lokal, werden sie übersprungen. In der CI
(`CI`-Variable gesetzt) schlägt das Fehlen fehl.

## Merke

1. `LanguageChoice`: einmal entscheiden, überall vergleichen.
2. `format_for_path`: Prosa ganz, alles andere als Regionen.
3. Dateien laufen parallel, ein Plugin bedient sie nacheinander.
4. Darum reisen die Wörter einer Datei mit der Anfrage.
5. `.cwasm`-Cache: größter Geschwindigkeitsgewinn (20 Dateien:
   7–8 s → 2,3 s).

[← harper-ls](05-harper-ls.md) · [Weiter: Die Haken in harper-core →](07-core-haken.md)

[#25]: https://github.com/Dronakurl/harper/issues/25
[#17]: https://github.com/Dronakurl/harper/issues/17
[plugin]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-cli/src/plugin.rs
[lint]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-cli/src/lint.rs
[engine]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-cli/src/lint_engine.rs
[main]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-cli/src/main.rs
[main-dialect]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-cli/src/main.rs#L285
[tests]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-cli/tests/plugin.rs
[t-file]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-cli/tests/plugin.rs#L144
[lc]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-cli/src/lint.rs#L109
[pc]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-cli/src/plugin.rs#L43
[ffp]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-cli/src/plugin.rs#L14
[bf]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-cli/src/plugin.rs#L150
[par]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-cli/src/lint.rs#L378
[uw]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-cli/src/lint.rs#L291

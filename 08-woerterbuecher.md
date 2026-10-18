# 8. Wörterbücher und Nutzerwörter

[← Die Haken in harper-core](07-core-haken.md) · [Weiter: Browser, Obsidian, harper.js →](09-browser-obsidian-js.md)

Das heikelste Kapitel. Hier steckt die meiste Überlegung, und hier
sind die Fehler am leichtesten zu machen. Lies es langsam.

## Drei Ebenen von Wörtern

Ein Wort kann „bekannt" sein:

1. **für mich überall** (Nutzerwörterbuch)
2. **in diesem Projekt** (Workspace-Wörterbuch)
3. **nur in dieser Datei** (Datei-Wörterbuch)

Das ist von `master` übernommen. Dazu kommt bei Quellcode eine
vierte, unsichtbare Ebene: Bezeichner aus dem Code selbst.

## Wo liegen die Dateien?

Unter Linux, `harper-ls`:

```
~/.config/harper-ls/
  dictionary.txt            Nutzer, en
  dictionary-de.txt         Nutzer, de

~/.local/share/harper-ls/
  file_dictionaries/        en, je Datei
  file_dictionaries-de/     de, je Datei

<Projekt>/
  .harper-dictionary.txt    Workspace, en
  .harper-dictionary-de.txt Workspace, de
```

Namensregel für **Sprachen** ([`language_path`][lp]): `en` behält den
Namen, jede andere Sprache hängt `-<id>` vor die Endung. Bei
Verzeichnissen: ans Ende ([`get_file_dict_path`][gfdp]).

Namensregel für **Dateien** ([`fileify_path`][fp]): der absolute
Pfad, jeder Bestandteil gefolgt von `%`. `/home/me/notiz.md` wird
`home%me%notiz.md%`. Eine Datei pro Quelldatei.

Ja: Das Wort wird **nicht** in die Quelldatei zurückgeschrieben. Es
liegt nur in diesem Verzeichnis. Verschiebt man die Quelldatei, ist das
Datei-Wörterbuch weg.

Warum pro Sprache getrennt? Ein deutsches Wort soll nicht englische
Rechtschreibung „heilen", und umgekehrt. Dasselbe gilt in Chrome und
Obsidian über eigene Speicherschlüssel (Kapitel 9).

## Wie macht es Englisch (master)?

Für jedes offene Dokument:

```
MergedDictionary (je Dokument)
 ├ curated   (geteilt, Arc)
 ├ Nutzer
 ├ Workspace
 └ Datei
```

Die Kinder sind `Arc`-Zeiger. Das große englische Wörterbuch
(`FstDictionary::curated`) existiert **einmal** im Speicher, egal wie
viele Dokumente offen sind. Pro Dokument kostet nur der kleine
Wörterbuch-Teil und ein eigener `LintGroup` (die Regeln halten das
Wörterbuch).

Daher löst Englisch das „Datei-Wort" ganz einfach: Es hängt der
Mischung des einen Dokuments ein viertes Kind an. Nichts davon sieht
ein anderes Dokument.

## Warum das beim Plugin nicht geht

Das Plugin ist ein **anderes Programm** mit eigenem Speicher. Es kann
das `MergedDictionary` des Hosts nicht ansehen. Also gehen die Wörter als
Text über die Grenze, als `Vec<String>`.

Und: Das Plugin ist **eine** Instanz mit **einem** Wörterbuch für alle
Dokumente (ein zweites wäre 330 MB mehr). Da liegt der Knackpunkt.

## Das Problem mit dem gemeinsamen Zustand

Erste Fassung: `set_user_words` schickt die Liste einmal; sie gilt
„bis zum nächsten Mal". Für den Editor geht das gut. Fürs CLI nicht.

Beispiel mit zwei Dateien gleichzeitig (rayon, zwei Threads):

```
A: set_user_words(["Quorblix"])
B: set_user_words([])          ← dazwischen
A: lint(Datei A)               ← verliert sein Wort
```

Weil das Plugin nur ein Wörterbuch hat und Threads in beliebiger
Reihenfolge ankommen, zerstört B das Wort von A. Die erste Lösung war
eine Sperre (`calls`), die jede „setze Wörter + lint"-Folge am Stück
ausführt. Das war korrekt, aber:

- Ein zusätzliches Schloss, das man nie vergessen darf.
- Es serialisiert das Linten komplett.
- Zwei Dateien werden nur nacheinander bedient, obwohl das Plugin das
  ohnehin tut.

## Die Lösung: Wörter reisen mit der Anfrage

Seit dem Umbau (Commit [`ef659a5b`][c]) trägt **jede Anfrage** ihre
Wörter ([`LintRequest::user_words`][uw]):

```
Anfrage A: Text A + Wörter für A
Anfrage B: Text B + Wörter für B
```

Zwei Anfragen können in beliebiger Reihenfolge ankommen. Jede ist für
sich vollständig. Es gibt nichts mehr zu schützen.

Regeln:

- `user_words = Some(liste)`: Diese Liste gilt für **diese**
  Anfrage. Sie hat Vorrang.
- `user_words = None`: Es gilt die Liste des letzten
  `set_user_words`. (Für Hosts mit nur einem Text, siehe unten.)
- Ältere Hosts schicken das Feld nicht: `None`, altes Verhalten.

Das Feld ist neu und additiv. Das ABI bleibt bei 1. Fallstrick war:
Ein **altes Plugin** kennt das Feld nicht und ignoriert es leise.
Dafür gibt es jetzt `PluginInfo::features`: Meldet ein Plugin
`request_words` nicht, setzt der Host die Wörter der Anfrage vorher
per `set_user_words`, im selben gesperrten Aufruf (Issue [#28],
behoben, Test `an_old_plugin_gets_the_words_of_the_request_...`).

## Was das Plugin damit macht (Deutsch)

[`German::use_words`][gw] in harper-german:

1. Gleiche Liste wie zuletzt? → nichts tun. Der Normalfall: die
   Nutzerwörter sind für alle Dateien gleich.
2. Sonst: ein neues Mini-Wörterbuch aus den Wörtern, mit dem
   unveränderlichen Basiswörterbuch zusammenmischen, dann die Regeln
   **neu bauen** (sie halten das Wörterbuch). Die vom Nutzer
   abgeschalteten Regeln bleiben erhalten.

Bei Dateien mit **unterschiedlichen** Wörtern wechselt die Liste von
Anfrage zu Anfrage, und es wird jedes Mal neu gebaut. Das kostet.
Der Test `what_words_in_the_request_cost` in
[`tests/plugin_host.rs`][cost] misst vier Fälle (keine Wörter, gleiche
Wörter, abwechselnde Listen, 5000 Wörter). Zahlen:
`cargo test -- --nocapture what_words_in_the_request_cost`.

Ein Nebeneffekt, den du kennen musst: Die Bezeichner-Wörter aus
Code-Dateien (`create_ident_dict`) werden in `harper-ls` zu den
Nutzerwörtern geschlagen, deshalb ist die Liste dort je Datei anders,
sobald sich Bezeichner ändern.

## Wer schickt wie?

- `harper-ls`: Wörter in der Anfrage
  ([`organized_lints`][ls-ol]). Eingesammelt wird bei jedem
  `update_document` ([`collect_user_words`][cuw]), nicht pro Lint.
- `harper-cli`: Nutzerwörter (einmal geladen) plus die Wörter der
  Datei, in der Anfrage ([`organized_lints`][cli-ol]).
- `harper.js`, Obsidian, Chrome: `set_user_words` (nur ein Text, nur
  ein Nutzer, kein Konflikt). Kapitel 9.

## Getestete Eigenschaften

- Sprachen sind getrennt, auch beim CLI-Nutzerwörterbuch
  (`a_plugin_gets_the_user_words_of_its_own_language_only`).
- Zwei Dateien: Datei-Wort gilt nur für die eigene
  (`a_files_own_words_reach_that_file_only`, 24 Dateien parallel).
- Im Plugin: `words_belong_to_the_request`,
  `concurrent_requests_do_not_see_each_others_words`.
- Chrome und Obsidian: eigene Tests pro Sprache (Kapitel 9).

## Früher eine Lücke

Das CLI trennte Datei-Wörterbücher nicht nach Sprache, `harper-ls`
tat es. Behoben (Issue [#25]); der Test legt die englische und die
Plugin-Variante an und prüft, dass nur die zweite zählt.

## Merke

1. Drei Ebenen: Nutzer, Workspace, Datei. Pfade tragen die Sprache.
2. Englisch teilt das Große (Arc) und baut je Dokument das Kleine.
3. Ein Plugin hat **ein** Wörterbuch; darum gehören die Wörter zur
   **Anfrage**, nicht zum Plugin.
4. `user_words = Some` hat Vorrang; `None` heißt „wie zuletzt gesetzt".
5. Wechselnde Wortlisten kosten einen Neuaufbau der Regeln.

[← Die Haken in harper-core](07-core-haken.md) · [Weiter: Browser, Obsidian, harper.js →](09-browser-obsidian-js.md)

[#25]: https://github.com/Dronakurl/harper/issues/25
[c]: https://github.com/Dronakurl/harper/commit/ef659a5b81af3981555bd01100f154ca5ebb2745
[lp]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-ls/src/backend.rs
[gfdp]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-ls/src/backend.rs
[fp]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-ls/src/io_utils.rs
[uw]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-api/src/types.rs
[gw]: https://github.com/Dronakurl/harper-german/blob/fix/single-cases-16-20/src/plugin.rs
[cost]: https://github.com/Dronakurl/harper-german/blob/fix/single-cases-16-20/tests/plugin_host.rs
[ls-ol]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-ls/src/plugins.rs
[cuw]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-ls/src/backend.rs
[cli-ol]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-cli/src/plugin.rs
[#28]: https://github.com/Dronakurl/harper/issues/28

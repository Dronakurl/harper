# 10. Das deutsche Plugin (`harper-german`)

[← Browser, Obsidian, harper.js](09-browser-obsidian-js.md) · [Weiter: Bauen, Testen, Verteilen →](11-bauen-testen-verteilen.md)

Das Repo [Dronakurl/harper-german][repo]. Eine Rust-Crate (`cdylib` +
`rlib`), die zu `harper_german.wasm` kompiliert und als
`harper-de.wasm` verteilt wird.

Es ist die **Referenz-Implementierung** des Plugin-Systems: alles,
was Kapitel 2 bis 8 beschreiben, sieht man hier von der anderen Seite.

## Landkarte

```
src/
  plugin.rs      der Einstieg (LanguagePlugin)
  module.rs      die Regeln in Rust
  linting/       32 Regel-Dateien + weir_rules
  spell/         Wörterbuch, Komposita
  parsers/       PlainGerman
  lexing.rs      deutsche Token
  grammar/       Nominalphrase, Fall, ...
  morphology/    Fall/Genus/Numerus als Bits
  language_detection.rs
  dialects.rs
data/
  dictionary.dict   Wortliste (6 MB)
  annotations.json  Grammatik je Wort
  german.pack       gebackenes Wörterbuch*
tests/plugin_host.rs
build.rs  examples/bake_pack.rs
```

*nicht im Git, wird gebacken (siehe unten).

## `plugin.rs`: der Einstieg

Datei: [`src/plugin.rs`][plugin]. Hier ist alles, was der Host sieht.

### Die Struktur

[`German`][german]:

```
German
 ├ base         Basiswörterbuch (geteilt)
 ├ dictionary   base + Nutzerwörter
 ├ words        die Nutzerwörter von eben
 └ group        LintGroup (die Regeln)
```

Wie in Kapitel 2 beschrieben, entsteht sie lazy: erst bei der ersten
Anfrage, die sie braucht ([`new`][new]).

### Der Steckbrief

[`info()`][info]: Id `de`, Name „Deutsch", drei Dialekte (Standard =
Deutschland, Austrian, Swiss) mit Aliasen (`de-AT`, `CH`, …) und
Plugin-Aliase `German`, `Deutsch`. Daher funktionieren alte
Einstellungen wie `"dialect": "German"` weiter.

`HARPER_CORE_VERSION` ist von Hand gepflegt und soll zu `Cargo.toml`
passen.

### `lint`

[`German::lint`][lint]:

1. `req.user_words` → [`use_words`][uw] (Kapitel 8)
2. `req.config` → [`apply_config`][ac]: nur Regeln, die es gibt
3. ganzes Dokument oder `regions` → [`lint_chars`][lc]

Regionen werden gefiltert (`start < end`, `end ≤ Länge`): Ein
fehlerhafter Host darf das Plugin nicht abstürzen lassen. In
[`lint_chars`][lc] bekommt jede Region ihren eigenen `Document` und
einen Offset, damit Spans zum Gesamttext passen.

### Parser

[`German::parser(format)`][parser]:

- `markdown` → `Markdown::with_inline_parser(…PlainGerman…)
  .non_english()`
- `org` → `OrgMode` entsprechend
- sonst `PlainGerman`

Das sind die Haken aus Kapitel 7.

### Spracherkennung

[`detect`][detect] und [`detect_with`][detect2] rufen
[`language_detection.rs`][ld]. Wichtig: Sie laufen ohne Instanz.

`german_confidence_given_english(text, english_ratio)`:

- viele deutsche Sonderzeichen (ä, ö, ü, ß) → 0,95
- Englisch klar vorn (`english_ratio ≥ 0,65`) → keine Aussage
- ≥ 20 % deutsche Wörter → 0,85
- englisch < 40 % und ≥ 8 % deutsch → 0,6

Die Schwellen stammen aus dem Detektor des alten Branches, der das
englische Wörterbuch zur Hand hatte. Jetzt liefert der Host die
Zahl (`english_ratio`, Kapitel 3).

## Die Regeln

[`module.rs`][module]: `rust_lint_group` fügt 33 Regeln (`group.add`)
zu einer `LintGroup` zusammen:

- zwei Regeln **aus dem Kern** (`Spaces`, `CommaFixes`) unverändert
- Rechtschreibung (`GermanSpellCheck`), Groß-/Kleinschreibung von
  Nomen und Satzanfängen
- Grammatik: Subjekt-Verb, Präposition und Fall, Genus des Artikels,
  Dativ-Plural, Verbklammer, Nebensatz-Wortstellung, …
- Kommas: Nebensatz, Relativsatz
- Stil: Füllwörter, Wiederholungen, Fusionen („auf jeden Fall")

Zusätzlich liegen **73 Weir-Regeln** als Textdateien unter
`src/linting/weir_rules/de/*.weir` (eine Regel pro Datei, mit
Test). `build.rs` macht daraus ein Rust-Modul.
[`new_curated_german`][ncg] mischt beide Gruppen und schaltet alle
Regeln an.

Zum Lesen der Regeln: Beginne mit einer kleinen, etwa
`german_wider_wieder.rs` oder `german_repeated_words.rs`. Sie zeigen
das Muster (Token durchlaufen, Lint mit Vorschlag bauen). Die
Grammatikregeln nutzen `grammar/` und `morphology/`.

## Das Wörterbuch

Das Herz und der größte Brocken (etwa 45 MB im Modul, gepackt rund
16 MB).

- Quelle: [`data/dictionary.dict`][dict] (Wörter mit Affix-Flags) und
  `annotations.json` (Fall, Genus, Wortart je Form).
- Daraus entsteht durch **Expansion** ein Wörterbuch mit allen
  Formen. Das dauert Sekunden.
- Darum gibt es das **Pack**: `just bake-pack` führt die Expansion
  einmal aus und schreibt `data/german.pack`. `build.rs` bettet die
  Datei ins Wasm ein ([`load_german_pack`][lgp]). Beim Laden spart das
  die Expansion. Fehlt das Pack, wird sie beim ersten Gebrauch
  nachgeholt, mit demselben Ergebnis, nur langsamer.
- Ein **Stempel** ([`pack_stamp`][stamp]) hasht Wortliste,
  Annotationen, den expandierenden Code und `Cargo.lock`. Ein
  altes Pack, das nicht mehr zum Code passt, wird nicht akzeptiert.
- Komposita: [`compound_aware_dict.rs`][cad] prüft Wortzusammensetzungen
  („Gemüsekuchenblech") gegen Teile, statt alle Kombinationen
  aufzulisten. `stem_only` (Kapitel 7) verhindert, dass Verbstämme
  als Wörter oder Kompositateile durchgehen.

Die lange Geschichte dazu steht in
[`docs/GERMAN_DICTIONARY_NOTES.md`][notes] (2400 Zeilen, historisch).

## Webassembly-Besonderheiten

Das Ziel `wasm32-unknown-unknown` hat kein Betriebssystem, also auch
keine Zufallszahlen. [`wasm_support.rs`][wasm] registriert einen festen
„Zufall" für `getrandom` 0.2 und 0.3. Harper braucht nur Hash-Seeds,
keine echte Zufälligkeit. Das Cfg-Flag steht in `.cargo/config.toml`.

Release-Profil: `opt-level 3`, `lto = "fat"`, `panic = "abort"`.

## Abhängigkeit von Harper

`Cargo.toml` hängt `harper-core`, `harper-plugin-api` und (nur für Tests)
`harper-plugin-host` an eine **feste Revision** von `Dronakurl/harper`.
Nach Änderungen an Harper: Revision erhöhen. Für gleichzeitige
Entwicklung gibt es den `[patch]`-Block in `.cargo/config.toml` (nicht
einchecken); danach braucht es `cargo update -p harper-core@<alte
Version>`, damit der Patch greift.

## Tests

- `cargo test --lib` (`just test`): die vielen Sprachtests in
  `src/tests/` (Batterien).
- [`tests/plugin_host.rs`][pht] (`just test-plugin`): baut das Wasm,
  lädt es über den **echten Host** und prüft: Steckbrief, Fund und
  Vorschlag, saubere Texte, Nutzerwörter, Markdown und Regionen,
  `wasm_matches_native` (das Plugin liefert wie nativ), Erkennung ohne
  Wörterbuch, und die Anfrage-Wörter aus Kapitel 8.

## Merke

1. Alles Sichtbare steht in `plugin.rs`; der Rest ist Deutsch.
2. Das Wörterbuch wird in ein Pack gebacken, damit der Start nicht
   Sekunden kostet.
3. Spracherkennung läuft ohne Instanz und bekommt den
   Englisch-Anteil vom Host.
4. Das Repo hängt an einer festen Harper-Revision.
5. Tests laufen zweimal: nativ und durch den Host.

[← Browser, Obsidian, harper.js](09-browser-obsidian-js.md) · [Weiter: Bauen, Testen, Verteilen →](11-bauen-testen-verteilen.md)

[repo]: https://github.com/Dronakurl/harper-german
[plugin]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/plugin.rs
[german]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/plugin.rs#L24
[new]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/plugin.rs#L146
[info]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/plugin.rs#L115
[lint]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/plugin.rs#L159
[uw]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/plugin.rs#L74
[ac]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/plugin.rs#L99
[lc]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/plugin.rs#L54
[parser]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/plugin.rs#L38
[detect]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/plugin.rs#L201
[detect2]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/plugin.rs#L205
[ld]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/language_detection.rs#L39
[module]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/module.rs
[ncg]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/linting/mod.rs#L48
[dict]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/data/dictionary.dict
[lgp]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/spell/german_dict.rs#L76
[stamp]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/spell/german_dict.rs#L48
[cad]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/spell/compound_aware_dict.rs
[notes]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/docs/GERMAN_DICTIONARY_NOTES.md
[wasm]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/src/wasm_support.rs
[pht]: https://github.com/Dronakurl/harper-german/blob/736635448a0e587632634b1a8cdc9ead2a1c4d91/tests/plugin_host.rs

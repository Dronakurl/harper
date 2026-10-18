# 7. Die Haken in harper-core

[← harper-cli](06-harper-cli.md) · [Weiter: Wörterbücher →](08-woerterbuecher.md)

Das Plugin ist ein eigenes Programm, aber es baut auf `harper-core`
auf (als Bibliothek, in das Wasm hineinkompiliert). Damit das geht,
musste der Kern an einigen Stellen **geöffnet** werden.

Die Regel dahinter: *Kein einziges Wort Deutsch im Kern. Englisch
verhält sich genau wie vorher.* Alles ist entweder sprachneutral
oder hinter einem Feature.

Den Überblick liefert:

```
git diff --shortstat origin/master...HEAD -- harper-core harper-comments
```

Stand dieser Doku: 24 Dateien, +605 / −94 Zeilen, davon vieles
Tests und Kommentare.

## Die fünf Haken

### 1. `Parser::is_english`

[`parsers/mod.rs`][pm]. Neue Methode im `Parser`-Trait mit Standard
`true`:

```rust
fn is_english(&self) -> bool { true }
```

Warum? In [`Document::parse`][dp] laufen für jeden Satz der
Brill-Tagger und der neuronale Nominalphrasen-Chunker. Beide sind auf
Englisch trainiert. Auf Deutsch kosten sie den größten Teil der
Parse-Zeit und liefern falsche oder fehlende Tags.

Antwortet der Parser `false`, überspringt das Dokument beide und
nutzt nur die Metadaten aus dem Wörterbuch
([`annotate_from_dictionary`][afd]). Alle bestehenden Parser sind
weiter englisch (Standard), also ändert sich nichts für sie.

Wrapper müssen die Antwort **durchreichen**, sonst verschwindet sie:
`Mask` und `OopsAllHeadings` tun das, mit Test
(`wrapping_parsers_forward_the_language`).

### 2. Markdown und Org-Mode mit fremdem Textparser

[`Markdown`][md] und [`OrgMode`][org] zerlegen Dokumente in Absätze
und Überschriften und rufen für den eigentlichen Fließtext
`PlainEnglish` auf. Das war fest eingebaut.

Neu: `with_inline_parser(fn)` setzt einen anderen Textparser ein, und
`.non_english()` meldet über `is_english`, dass dessen Text nicht
englisch ist. So bekommt Deutsch Markdown- und Org-Unterstützung, ohne
Markdown neu zu schreiben. Das Plugin ruft das so auf
([`German::parser`][gparser]):

```rust
Markdown::with_inline_parser(opts, |s| PlainGerman.parse(s))
    .non_english()
```

Wegen der Änderung ist `OrgMode` jetzt kein Einheitstyp mehr:
überall `OrgMode::default()` statt `OrgMode`.

### 3. `LanguageBits` (hinter Feature `language-bits`)

[`language_bits.rs`][lb]. 128 Bit (zwei `u64`) an jedem
Wörterbucheintrag, gesetzt vom Plugin, **nie gelesen** von Harper:

```rust
pub struct LanguageBits(pub [u64; 2]);
```

Warum? Deutsch markiert Fall, Genus und Numerus an jeder Form. Das
passt in keines der Felder von `DictWordMetadata`. Statt für jede
Sprache neue Felder anzubauen, bekommt der Eintrag einen Platz, dessen
Bedeutung das Plugin bestimmt.

Beim Zusammenführen zweier Einträge (`merge`) werden die Bits
**vereinigt** (ODER), wie Harper es mit seinen eigenen Flags tut.
Daraus folgt: Ein Plugin sollte jeder Lesart einer Form ein eigenes
Bit geben.

Das Feld existiert nur, wenn das Feature `language-bits` an ist. In
der JSON-Form wird es weggelassen, wenn leer. Englisch zahlt nichts.
`harper-german` schaltet das Feature ein.

### 4. Öffentliche Helfer

Mehrere bisher private Dinge sind `pub` geworden, damit eine Sprache
sie wiederverwenden kann statt sie zu kopieren:

- `lexing` samt `lex_word`, `lex_number`, … : die Bausteine, aus denen
  `PlainGerman` seinen Lexer baut
- `spell::rune`: Wortlisten mit Affixen (Hunspell-ähnlich) expandieren
- `spell::word_map`, `spell::fst_dictionary`: Fuzzy-Suche
  (`build_dfa`, `stream_distances_vec`, …)
- `linting::spaces`, `linting::comma_fixes`: die zwei Typografie-
  Regeln, die das deutsche Plugin unverändert übernimmt
  (`Spaces`, `CommaFixes`)
- `AdjectiveData` wird exportiert

Wichtig: Jede neue `pub`-Fläche ist Wartungslast. Sie wurde
bewusst klein gehalten.

### 5. Wörterbuch-Mechanik

Kleine Änderungen, die ein **großes** Wörterbuch (Deutsch: Millionen
Formen) handhabbar machen, ohne Englisch zu verändern:

- [`cached_word_hash`][cwh] im `Dictionary`-Trait. `MergedDictionary`
  fingerprintet jedes Kind, und das geht über jedes Wort. Ein
  Wörterbuch, das sich nie ändert, kann den Hash merken
  (Standard `None`: nichts ändert sich).
- [`hash_dictionary`][hd]: fragt zuerst
  [`FstDictionary::curated_if_loaded`][ciu], ob das englische
  Wörterbuch überhaupt gebaut ist. Sonst baute eine rein deutsche
  Sitzung das englische nur, um festzustellen, dass es nicht gemeint
  war.
- `FstDictionary`: FST und veränderliches Wörterbuch werden auf
  nicht-Wasm-Zielen in zwei Threads gebaut (Wasm bleibt seriell).
- Wortlisten dürfen **ohne** Zähler in der ersten Zeile beginnen.
- `stem_only` an Einträgen: Verbstämme wie `absperr` sind keine
  Wörter, sollen aber Endungen tragen. Ohne Flag landeten sie als
  Wörter im Wörterbuch.

Letztere zwei Punkte ändern auch den englischen Pfad leicht (schneller
Aufbau, optionaler Zähler). Tests stellen sicher, dass die Ergebnisse
gleich sind.

## Außerhalb von harper-core

- [`harper-comments`][hc]: neue Funktion
  [`comment_regions(&Document)`][cr]. Aus einem geparsten
  Quelltext-Dokument die zusammenhängenden Kommentar-Bereiche als
  Zeichen-Spannen. Gebraucht, weil ein Plugin Text prüft, nicht
  Token, und wissen muss, wo Prosa ist.

## Was man prüfen kann

Leitfrage bei jeder Änderung am Kern: „Ist das sprachneutral, und
bleibt Englisch gleich?" Ein Schnelltest:

```
git diff --stat origin/master...HEAD -- harper-core/src/
```

Alles, was dort steht, ist eine künftige Merge-Konfliktquelle gegen
upstream. Deshalb die Disziplin.

## Merke

1. `Parser::is_english`: nicht-englische Sprachen sparen Tagger und
   Chunker.
2. `with_inline_parser`/`non_english`: Markdown und Org für jede
   Sprache.
3. `LanguageBits`: ein Fach pro Wort, Inhalt gehört dem Plugin,
   Feature-gesteckt.
4. Öffentliche Helfer statt Kopien.
5. Hashes und Aufbau: damit große Wörterbücher nicht jeden Start
   verteuern.

[← harper-cli](06-harper-cli.md) · [Weiter: Wörterbücher →](08-woerterbuecher.md)

[pm]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-core/src/parsers/mod.rs
[dp]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-core/src/document.rs
[afd]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-core/src/document.rs
[md]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-core/src/parsers/markdown.rs
[org]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-core/src/parsers/org_mode.rs
[gparser]: https://github.com/Dronakurl/harper-german/blob/fix/single-cases-16-20/src/plugin.rs
[lb]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-core/src/language_bits.rs
[cwh]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-core/src/spell/dictionary.rs
[hd]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-core/src/spell/merged_dictionary.rs
[ciu]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-core/src/spell/fst_dictionary.rs
[hc]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-comments/src/comment_parser.rs
[cr]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-comments/src/comment_parser.rs

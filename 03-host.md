# 3. Der Host (`harper-plugin-host`)

[← Schnittstelle](02-schnittstelle.md) · [Weiter: Ein Lint von A bis Z →](04-ein-lint-von-a-bis-z.md)

Die Rust-Seite des Vertrags: lädt eine `.wasm`-Datei, startet sie in
einer Sandbox und spricht JSON mit ihr. Benutzt von `harper-ls` und
`harper-cli`. Rund 320 Zeilen in drei Dateien.

- [`plugin.rs`][plugin]: `PluginHost`, `LoadedPlugin`, `Guest`
- [`dir.rs`][dir]: wo Plugins liegen
- [`lib.rs`][lib]: Fehlertyp
- Test: [`tests/echo.rs`][tests]

Abhängigkeit: `wasmtime` (Cranelift-Compiler). Das ist der große
Brocken im Binary. Deshalb gibt es in `harper-ls` und `harper-cli`
das Cargo-Feature `plugins` (Standard: an). Ohne es ersetzen
Stubs den Code ([`stub` in plugins.rs][ls-stub]) und das Binary
bleibt klein.

## Drei Typen, drei Aufgaben

```
PluginHost      Engine, einmal je Prozess
   │ load_file / load_bytes /
   │ load_file_cached
   ▼
LoadedPlugin    ein geladenes Plugin
   │ hat
   ▼
Guest           Store + Speicher +
                die drei Funktionen
```

### `PluginHost`

[`PluginHost`][host] hält nur die wasmtime-`Engine`. Billig zu teilen,
einmal pro Prozess anlegen.

`load_bytes` kompiliert das Modul (`Module::new`) und instanziiert es.
**Kompilieren dauert Sekunden** bei einem 17 MB großen Plugin. Das ist
der Grund für den Cache.

### Der Kompilier-Cache

[`load_file_cached`][cached]:

1. Datei lesen, Fingerabdruck bilden ([`fingerprint`][fp]): Hash über
   die Bytes, die Crate-Version und den wasmtime-Kompatibilitäts-Hash.
2. Gibt es `<cache>/<hash>.cwasm`? Dann direkt laden
   (`Module::deserialize_file`, `unsafe`, weil wasmtime der Datei
   vertraut).
3. Sonst kompilieren, serialisieren, atomar schreiben
   (`.tmp` → `rename`).

Ort: `<cache_dir>/harper/plugins`. Beide Hosts benutzen dasselbe
Verzeichnis.

Gemessen (Kapitel 6): der Cache ist der größte einzelne
Geschwindigkeitsgewinn für das CLI. Ein Lauf über 20 Dateien:
7–8 s ohne, 2,3 s mit.

Beachte:

- Ändert sich die `.wasm`, ändert sich der Hash, es entsteht eine
  **neue** Cache-Datei. Beim Schreiben einer neuen Datei werden
  `.cwasm`/`.tmp`-Dateien entfernt, die älter als 30 Tage sind
  (`remove_stale_cache_files`, Issue [#27], behoben).
- Ein schreibgeschützter Cache kostet nur Zeit, nie Korrektheit.

### `LoadedPlugin::instantiate`

[`instantiate`][inst], in dieser Reihenfolge:

1. `Store` und `Linker` anlegen.
2. [`define_unknown_imports_as_traps`][traps]: Manche Abhängigkeiten
   ziehen unbenutzte `wasm-bindgen`-Stubs herein. Sie werden nie
   aufgerufen. Statt das Plugin abzulehnen, werden sie zu Fallen.
   Das Plugin bekommt **nichts** vom Host: kein Dateizugriff, kein
   Netz.
3. Instanz erzeugen, `harper_plugin_abi_version` rufen und prüfen.
4. `memory`, `harper_alloc`, `harper_free`, `harper_call` suchen.
   Fehlt eines: `Error::MissingExport`.
5. `info` anfragen, `info.abi` noch einmal prüfen.

Erst danach existiert ein `LoadedPlugin`. Wer ein Plugin lädt, weiß
also sofort Id, Name, Dialekte.

## `Guest::roundtrip`: ein Aufruf

[`roundtrip`][rt] setzt das Speicherprotokoll aus Kapitel 2 um:
`alloc` → Bytes schreiben → `call` → Antwort lesen → zweimal `free`.
Darüber liegt [`request`][req]: JSON bauen, `roundtrip`,
Antwort als `Response<T>` lesen. `Ok(v)` → Wert, `Err(e)` →
`Error::Plugin(e)`, kaputtes JSON → `Error::Protocol`.

## Das Schloss: ein Plugin, ein Aufruf zugleich

```rust
pub struct LoadedPlugin {
    info: PluginInfo,
    guest: Mutex<Guest>,   // Guest merkt sich auch die letzten Wörter
    request_words: bool,   // liest das Plugin user_words?
}
```

Ein `Store` ist nicht für mehrere Threads gedacht. Deshalb liegt
der `Guest` hinter einem `Mutex` ([`LoadedPlugin`][lp]).

Folge: **Aufrufe an dasselbe Plugin laufen nacheinander**, egal wie
viele Threads es gibt. Das Wasm-Modul selbst ist einfädig; echte
Parallelität bräuchte mehrere Instanzen. Eine Instanz des deutschen
Plugins belegt rund 330 MB, das lohnt sich nicht.

Das ist der Grund, warum die Wörter einer Datei **mit der Anfrage**
reisen (Kapitel 8): Zwei Anfragen für zwei Dateien kommen nacheinander
am Plugin an, ohne dass die eine das Wörterbuch der anderen
verändern darf.

Ein vergiftetes Schloss (ein Thread ist beim Halten abgestürzt) wird
ignoriert (`unwrap_or_else(|e| e.into_inner())`). Das Plugin bleibt
nutzbar.

## Die Methoden von `LoadedPlugin`

- [`lint`][lint]: Anfrage senden, `Vec<PluginLint>` zurück.
  `lint_core` wandelt zusätzlich in `harper_core::Lint`. Meldet das
  Plugin `request_words` nicht (gebaut vor dieser Funktion), nimmt
  `lint` die Wörter aus der Anfrage und setzt sie vorher per
  `set_user_words`, beides unter demselben Schloss.
- `lint_descriptions`, `default_config`: durchreichen.
- [`set_user_words`][suw]: merkt sich die letzte Liste und
  schickt nur, wenn sie sich ändert (der Zustand liegt im `Guest`,
  unter demselben Schloss wie `lint`). Wird von den Rust-Hosts nicht
  mehr benutzt (sie schicken `user_words` pro Anfrage), ist aber für
  Hosts mit nur einem Text da.
- [`detect`][detect]: schickt Text **und** `english_ratio`.

### `english_ratio`

Der Host kennt das englische Wörterbuch, das Plugin nicht.
[`english_ratio`][er] berechnet den Anteil der Wörter eines Textes, die
Harpers Englisch kennt. Das Plugin nutzt die Zahl, um englischen Text
sicher auszuschließen (`detect_with`). So kann ein Plugin deutsche
Texte mit vielen englischen Wörtern von rein englischen trennen,
ohne ein englisches Wörterbuch einzubinden. Wer mehrere Plugins
fragt, rechnet sie einmal und übergibt sie mit `detect_with_ratio`
(Issue [#30], behoben).

## Plugins finden

[`dir.rs`][dir]:

- [`default_plugin_dir`][dpd]: `<config_dir>/harper/plugins`
  (Linux: `~/.config/harper/plugins`).
- [`discover`][disc]: alle `*.wasm` direkt in einem Ordner, sortiert.
  Ein fehlender Ordner ist leer, kein Fehler.

## Fehler

[`Error`][err]: `Io`, `Wasm`, `MissingExport`, `AbiMismatch`,
`Protocol`, `Plugin`. Die Unterscheidung zählt: `Plugin` heißt „das
Plugin hat Nein gesagt", `Protocol` heißt „es hat Unsinn geantwortet".

## Was der Host nicht tut

- **Keine Zeitgrenze, keine Speichergrenze.** Ein Plugin in einer
  Endlosschleife hängt den Host. Vorschlag in PR [#22], noch nicht
  gebaut.
- Kein Parallelismus innerhalb eines Plugins (siehe oben).

## Merke

1. `PluginHost` (Engine) → `LoadedPlugin` (geladen) → `Guest`
   (Speicher und Funktionen).
2. Kompilieren ist teuer, deshalb gibt es den `.cwasm`-Cache.
3. Plugin ohne Importe: Sandbox von Haus aus.
4. Ein `Mutex` serialisiert alle Aufrufe eines Plugins.

[← Schnittstelle](02-schnittstelle.md) · [Weiter: Ein Lint von A bis Z →](04-ein-lint-von-a-bis-z.md)

[#22]: https://github.com/Dronakurl/harper/pull/22
[plugin]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/plugin.rs
[dir]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/dir.rs
[lib]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/lib.rs
[tests]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/tests/echo.rs
[ls-stub]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-ls/src/plugins.rs
[host]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/plugin.rs
[cached]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/plugin.rs
[fp]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/plugin.rs
[inst]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/plugin.rs
[traps]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/plugin.rs
[rt]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/plugin.rs
[req]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/plugin.rs
[lp]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/plugin.rs
[lint]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/plugin.rs
[suw]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/plugin.rs
[detect]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/plugin.rs
[er]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/plugin.rs
[dpd]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/dir.rs
[disc]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/dir.rs
[err]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-host/src/lib.rs
[#27]: https://github.com/Dronakurl/harper/issues/27
[#30]: https://github.com/Dronakurl/harper/issues/30

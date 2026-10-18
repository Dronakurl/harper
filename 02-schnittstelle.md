# 2. Die Schnittstelle (`harper-plugin-api`)

[← Überblick](01-ueberblick.md) · [Weiter: Der Host →](03-host.md)

Der Vertrag zwischen Host und Plugin. Kleinste Crate im System
(rund 450 Zeilen), aber alles andere hängt daran.

Dateien:

- [`lib.rs`][lib]: Trait `LanguagePlugin`, Makro `export_plugin!`,
  `ABI_VERSION`.
- [`types.rs`][types]: alle Nachrichtentypen.
- [`dispatch.rs`][dispatch]: nimmt eine JSON-Anfrage entgegen und ruft
  das Plugin.
- [`convert.rs`][convert]: `PluginLint` ⇄ `harper_core::Lint`
  (nur mit Feature `core`, also nur auf Host-Seite).
- [`examples/echo.rs`][echo]: winziges Beispiel-Plugin, Testobjekt für
  den Host.

## Die vier Exporte

Jedes Plugin exportiert genau diese Funktionen (plus `memory`):

```
harper_plugin_abi_version() -> u32
harper_alloc(len) -> ptr
harper_free(ptr, len)
harper_call(ptr, len) -> u64
```

Sie werden nie von Hand geschrieben. Das Makro
[`export_plugin!`][macro] erzeugt sie für jeden Typ, der
`LanguagePlugin` implementiert.

### So läuft ein Aufruf (Speicherprotokoll)

WebAssembly kennt nur Zahlen. Text geht über den Speicher des Plugins:

```
Host                     Plugin
────                     ──────
alloc(len)       ──────► Puffer anlegen
                 ◄────── ptr
Anfrage in den
Speicher schreiben
call(ptr, len)   ──────► JSON lesen, handeln
                 ◄────── (ptr<<32)|len
Antwort lesen
free(Antwort)
free(Anfrage)
```

Die Antwort kommt als **ein** `u64`: oben die Adresse, unten die
Länge ([`harper_call`][call]). Der Host liest die Bytes und gibt beide
Puffer wieder frei. Das macht [`Guest::roundtrip`][roundtrip] in
Kapitel 3.

## Die Nachrichten

Anfrage ([`Request`][request]) ist ein JSON-Umschlag
`{"method": …, "params": …}`. Es gibt sechs Methoden:

- `info`: wer bist du? Antwort: [`PluginInfo`][info]
- `lint`: prüfe diesen Text. Antwort: Liste von [`PluginLint`][lint]
- `lint_descriptions`: Beschreibung jeder Regel
- `default_config`: welche Regeln sind standardmäßig an
- `set_user_words`: das Nutzerwörterbuch (siehe Kapitel 8)
- `detect`: wie sicher bist du, dass das deine Sprache ist?

Antwort ([`Response`][response]) ist entweder `{"ok": …}` oder
`{"err": "…"}`.

Beispiel einer Lint-Anfrage:

```json
{"method":"lint","params":{
  "text":"Das ist ein Tset.",
  "format":"plain",
  "user_words":["Quorblix"]
}}
```

### `PluginInfo`: was ein Plugin über sich sagt

[`PluginInfo`][info]:

- `id`: `"de"`. Der Dateiname ist `harper-de.wasm`.
- `name`: `"Deutsch"`
- `aliases`: `["German", "Deutsch"]`
- `dialects`: Standard, Austrian, Swiss, jeweils mit eigenen Aliasen
- `abi`, `plugin_version`, `harper_core_version`

Wichtig ist [`answers_to`][answers]: Der Host fragt „hörst du auf
`Swiss`?" und das Plugin antwortet. So braucht kein Host eine Liste
von Sprachen.

### `LintRequest`: was beim Prüfen mitkommt

[`LintRequest`][req]:

- `text`: der **ganze** Text
- `format`: `plain`, `markdown`, `org`, `typst`
- `regions`: optional. Dann wird nur in diesen Bereichen geprüft
  (z. B. Kommentare in Quellcode), Spans bleiben relativ zum
  Gesamttext.
- `config`: Regel → an/aus
- `dialect`: vorgesehen, wird aktuell von keinem Host gefüllt
  (Issue [#24], „Noch offen")
- `user_words`: **neu**, Wörter für genau diese Anfrage
  (Kapitel 8)

Alle Felder außer `text` haben Standardwerte. Dadurch bleibt das
Format abwärtskompatibel: ein Host darf Felder weglassen, ein altes
Plugin ignoriert unbekannte.

### `PluginLint`: was zurückkommt

[`PluginLint`][lint]: `span` (Zeichen-Offsets `[start, end)`), `rule`,
`kind`, `message`, `suggestions`, `priority`.

Offsets zählen **Zeichen** (Unicode-Skalare), nicht Bytes und nicht
UTF-16-Einheiten. Das ist Absicht, weil `harper-core` selbst in
`char` rechnet. Der JS-Host rechnet für Emojis um (Kapitel 9).

Vorschläge ([`PluginSuggestion`][sugg]): ersetzen, danach einfügen,
entfernen.

## Der Trait `LanguagePlugin`

Das muss ein Plugin implementieren ([`lib.rs`][trait]):

- `info()`: Steckbrief, ohne Instanz (statisch)
- `new()`: baut das Plugin, **hier** entsteht das Wörterbuch
- `lint(&mut self, req)`
- `lint_descriptions()`, `default_config()`
- `set_user_words(&mut self, words)`
- `detect(text)` und `detect_with(text, english_ratio)`:
  optional, auch ohne Instanz

Merke den Unterschied: `info` und `detect` sind **assoziierte
Funktionen** ohne `self`. Das hat einen Grund.

### Faule Erzeugung

Ein Wörterbuch aufzubauen dauert Sekunden. `harper-ls` fragt aber
beim Start jedes installierte Plugin per `detect`, ob es zum Text
passt. Würde das jedes Mal das Wörterbuch bauen, wäre der Start
unerträglich.

Deshalb hält [`dispatch`][dispatch-fn] einen leeren Slot
(`Option<Plugin>`). `info` und `detect` werden beantwortet, **ohne**
den Slot zu füllen ([`handle`][handle]). Erst `lint`,
`lint_descriptions`, `default_config`, `set_user_words` rufen
`P::new()`. Drei Tests in `dispatch.rs` halten das fest.

`dispatch` stürzt nie bei kaputten Eingaben ab: es antwortet mit
`{"err":…}`.

### Wo lebt die Instanz?

Im Makro: eine `thread_local!`-`RefCell<Option<Plugin>>`. Ein
Wasm-Modul ist einfädig, also reicht das. Ein Modul = ein Plugin =
eine Instanz.

## ABI-Version

`ABI_VERSION = 1` ([`lib.rs`][abi]). Wird erhöht, wenn sich das
Nachrichtenformat so ändert, dass ein alter Host oder ein altes
Plugin es nicht mehr lesen kann. Der Host prüft sie zweimal:
über den Export und über `info.abi`.

Neue **optionale** Felder (wie `user_words`) brauchen keine neue
Version. Aber: ein altes Plugin ignoriert sie leise. Siehe
Issue [#28].

## `convert.rs`: Lint-Übersetzung

- Gast-Seite: [`lint_to_plugin`][l2p] macht aus einem `Lint` samt
  Regelname einen `PluginLint`.
- Host-Seite: [`plugin_to_lint`][p2l] macht daraus wieder einen `Lint`.
  Unbekannte `kind`-Werte werden zu `Miscellaneous`.

Danach braucht der Rest von Harper (Diagnosen, Code-Aktionen, CLI-
Berichte) keinen Sonderfall für Plugins.

## Merke

1. Vier Exporte, Speicher + ein `u64` als Rückgabe, JSON.
2. `info` und `detect` laufen ohne Instanz, `new()` erst bei Bedarf.
3. Neue Felder sind optional und additiv; das ABI bleibt bei 1.
4. Offsets zählen Zeichen.

[← Überblick](01-ueberblick.md) · [Weiter: Der Host →](03-host.md)

[#24]: https://github.com/Dronakurl/harper/issues/24
[lib]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/lib.rs
[types]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/types.rs
[dispatch]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/dispatch.rs
[convert]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/convert.rs
[echo]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/examples/echo.rs
[macro]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/lib.rs#L54
[call]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/lib.rs#L87
[roundtrip]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-host/src/plugin.rs#L97
[request]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/types.rs#L132
[response]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/types.rs#L153
[info]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/types.rs#L19
[answers]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/types.rs#L41
[req]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/types.rs#L68
[lint]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/types.rs#L115
[sugg]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/types.rs#L107
[trait]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/lib.rs#L28
[dispatch-fn]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/dispatch.rs#L10
[handle]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/dispatch.rs#L18
[abi]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/lib.rs#L24
[l2p]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/convert.rs#L7
[p2l]: https://github.com/Dronakurl/harper/blob/ef659a5b81af3981555bd01100f154ca5ebb2745/harper-plugin-api/src/convert.rs#L28
[#28]: https://github.com/Dronakurl/harper/issues/28

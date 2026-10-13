# Plan: Sprachen als nachinstallierbare Plugins

> Status: **Phasen 0–3 sind umgesetzt, Phasen 4–6 offen.** Dieses Dokument bleibt der
> Arbeitsauftrag; Abweichungen von der ursprünglichen Planung stehen unten in
> „Stand der Umsetzung“. Der deutsche Code liegt als Plugin im Repo
> [`dronakurl/harper-german`](https://github.com/dronakurl/harper-german); auf
> `feature/german-language-support` bleibt er unverändert erhalten.

### Stand der Umsetzung

| Phase | Stand |
|---|---|
| 0 Spike | fertig, Ergebnis in `spike/RESULTS.md` (1,5× nativ, kalt 1,17×) |
| 1 api + host + Testplugin | fertig (`harper-plugin-api`, `harper-plugin-host`, Beispiel `echo`) |
| 2 harper-ls + harper-cli | fertig (`language`, `pluginDirectory`, `--language`, `--plugin`, `plugin list/install/remove`) |
| 3 harper-german | Code umgezogen, 628 Tests grün, Modul lädt und prüft über den Host (Wasm = nativ auf allen Beispieltexten). Offen: Release-Lauf, Quellcode-Kommentare, `detect` |
| 4 harper.js, Web, Obsidian | `PluginLinter` in harper.js (getestet, auch `detect`), Demo-Seite `/plugins`, Obsidian-Einstellung „Language“ mit Plugin-Ordner. Nicht gebaut/getestet: die Browser-Tests, Obsidian, Website (brauchen `pnpm install` + `wasm-pack`) |
| 5 Chrome/Firefox | Optionsseite „Language“: Plugin-Datei hinzufügen/entfernen, Sprache wählen, Tastenkürzel zum Wechseln; Plugins in IndexedDB, Regeln und Wörterbuch je Sprache. Mit `tsc` auf Typen geprüft (nur `PluginLinter`/`pluginStore`), Erweiterung nicht gebaut. Fehlt: Begleit-Erweiterung für den Store, Spracherkennung im Browser, Harper Desktop |
| `harper-ls`/`harper-cli` | `language: auto` (Voreinstellung) und `--language auto` erkennen die Sprache über die installierten Plugins; Wörterbücher je Sprache; Kommentare im Quellcode |
| 6 Verteilung | `harper-german`: CI und Trockenlauf-Release (`workflow_dispatch`) eingerichtet; kein Tag/Release veröffentlicht; keine Index-Veröffentlichung |

Abweichungen vom ursprünglichen Plan:

- **Morphologie (§6.5, §7.1):** Statt einer Seitentabelle im Plugin gibt es in `harper-core` das
  Cargo-Feature `morphology` (aus, solange niemand es einschaltet). Es fügt `DictWordMetadata` ein
  Feld hinzu und stellt `harper_core::morphology` bereit. Englisch ändert sich nicht. Eine
  Seitentabelle hätte bedeutet, die Affix-Expansion aus `spell::rune` zu kopieren.
- **Core-Haken (§7.1)** sind als ein Commit in diesem Branch gelandet: `Parser::is_english`,
  `Markdown`/`OrgMode::with_inline_parser`, öffentliche `lexing`-, `spell::rune`-,
  `word_map`-/`fst_dictionary`-Helfer, `stem_only`, geordnete Affix-Expansion, optionale
  Anzahlzeile in Wortlisten. `OrgMode` ist dadurch keine Einheits-Struktur mehr (`OrgMode::default()`).
- **Tags** lassen sich in diesem Repo derzeit nicht pushen; `harper-german` hängt deshalb an einer
  festen Revision (`rev = …`) statt an `plugins-v1`.
- **Das Modul hat noch fünf `__wbindgen_*`-Imports**, weil `burn` über `harper-brill` `wasm-bindgen`
  mitbringt. Der Host verdrahtet sie als Fallen (sie werden nie aufgerufen). Sauber wird das erst,
  wenn `burn`/Brill in `harper-core` hinter ein Feature gelegt ist.

## 1. Ausgangslage

Auf `feature/german-language-support` sind Deutsch, Portugiesisch, Slowakisch
und Polnisch direkt in `harper-core` eingebaut (`harper-core/src/language/`,
per Cargo-Feature). Das funktioniert, hat aber drei Probleme:

1. **Jede Sprache macht jedes Programm größer.** Das deutsche Wörterbuch allein
   ist 6 MB Quelltext und wird per `include_str!` in jedes Binary und in jede
   `.wasm`-Datei gebacken, auch für Leute, die nie Deutsch schreiben. Im Browser
   ist das Downloadgröße, nativ Plattenplatz und Startzeit.
2. **Die Änderung an `master` ist riesig** (375 Dateien, ~490 000 Zeilen), und
   sie berührt viele zentrale Stellen (`harper-ls/src/backend.rs`,
   `harper-wasm`, Build-System, `DictWordMetadata`). Das ist kaum reviewbar und
   kaum upstream-fähig.
3. **Mit jeder weiteren Sprache wird es schlimmer.**

## 2. Anforderungen (vom Projektinhaber festgelegt)

| Frage | Entscheidung |
|---|---|
| Wo muss Deutsch laufen? | Überall: Browser-Erweiterung/Website (harper.js), Editoren (harper-ls), Obsidian, Desktop-App, CLI |
| Eigene „Harper mit Deutsch“-Version oder nachinstallieren? | **Nachinstallieren** in die normale Version |
| Wie kommt der Nutzer an das Paket? | **Beides**: automatischer Download beim ersten Bedarf *und* manuelles Ablegen einer Datei (offline) |
| Mischtexte (Deutsch + Englisch in einer Datei)? | **Nicht nötig.** Eine Sprache pro Dokument |
| Upstream (Automattic/harper)? | Vielleicht später. Die `master`-Änderung soll so klein und allgemein sein, dass ein Upstream-PR möglich bleibt |
| Andere Sprachen (pt, sk, pl)? | Später ebenfalls als Plugins, gleicher Weg |
| Wo liegt der deutsche Code? | Im öffentlichen Repo `dronakurl/harper-german` |

## 3. Entscheidung: Ein Sprachpaket ist ein eigenständiges WebAssembly-Modul

**Bild:** Harper wird eine Konsole mit Steckplatz. Jede Sprache ist ein
Steckmodul, eine einzelne Datei `harper-<lang>.wasm`, die in jede Konsole
passt (Browser, Obsidian, harper-ls, Desktop, CLI).

Technisch:

- Das Plugin-Repo baut **ein komplettes, auf eine Sprache spezialisiertes
  Harper** (eigene Kopie von `harper-core` + Sprachcode + Wörterbuch) für das
  Target `wasm32-unknown-unknown`, **ohne `wasm-bindgen`**, mit einer kleinen,
  festen C-artigen Schnittstelle (JSON rein, JSON raus, siehe §5).
- **Browser/Obsidian/Website:** `harper.js` instanziiert das Modul mit dem
  normalen `WebAssembly.instantiate` und ruft die Schnittstelle auf.
- **harper-ls/Desktop/CLI:** ein eingebauter Wasm-Interpreter
  ([`wasmtime`](https://crates.io/crates/wasmtime)) lädt dieselbe Datei aus
  einem Plugin-Ordner.
- `master` bekommt nur: die Schnittstellendefinition, den Lader auf beiden
  Seiten, die Sprachauswahl in den Einstellungen und die Weiche „Englisch →
  wie bisher, andere Sprache → Plugin“. **Kein deutscher Code in `master`.**

### Warum genau so (verworfene Alternativen)

| Alternative | Warum nicht |
|---|---|
| Alles fest einbauen (Status quo) | Erfüllt „nachinstallieren“ nicht, siehe §1 |
| Plugin-Crate, beim Bauen eingebunden | Einfacher, aber nicht nachinstallierbar. Bleibt als Rückfallebene, falls §8-Spike scheitert |
| Native Bibliotheken (`.so`/`.dll`/`.dylib`) mit `dlopen` | Rust hat keine stabile ABI; eine Datei pro OS × CPU; im Browser unmöglich; kein Sandboxing |
| Reines Datenpaket (Wörterbuch + Weir-Regeln), z. B. die vorhandenen **Weirpacks** (`harper-core/src/weirpack/`) | Läuft überall, aber die ~27 000 Zeilen deutscher Rust-Logik (Kompositazerlegung, Kongruenz, Morphologie) passen nicht hinein. Weirpacks bleiben nützlich, um *zusätzliche* Regeln nachzuladen |
| `wasm-bindgen`-Modul wie heute `harper-wasm` | Braucht generierten JS-Kleber, läuft deshalb nicht in `wasmtime`. Wir wollen **eine** Datei für alle Hosts |
| WebAssembly Component Model / WIT | Langfristig elegant, aber Browser-Unterstützung braucht Transpilierung (`jco`) und mehr Werkzeug. Kann später ersetzt werden; die JSON-Schnittstelle ist bewusst klein genug für einen Umstieg |

### Bekannte Kosten

- Native Programme werden durch `wasmtime` größer (Größenordnung ~10 MB).
  Deshalb hinter einem Cargo-Feature `plugins` (Standard an), damit
  Distributoren es abschalten können.
- Ein Plugin läuft im Wasm-Interpreter langsamer als nativ eingebaut (grob
  1,2–2×). Für Linting beim Tippen (Debounce in harper-ls) unkritisch, aber zu
  messen (§8).
- Jedes Plugin enthält seine eigene Kopie von `harper-core`. Speicher fällt nur
  an, solange das Plugin geladen ist.
- **Browser-Store-Regeln (wichtig):** Chrome Web Store (Manifest V3) und
  Firefox AMO verbieten „remotely hosted code“, und Wasm zählt dazu. Die
  Browser-Erweiterung darf ein Plugin **nicht** selbst aus dem Netz laden. Lösung
  siehe §6.4.

## 4. Gesamtbild

```
                  master (dronakurl/harper)                       dronakurl/harper-german
 ┌──────────────────────────────────────────────────────┐   ┌──────────────────────────────┐
 │ harper-plugin-api   (neu; Typen + Gast-Makro)         │◄──┤ harper-german (crate)        │
 │ harper-plugin-host  (neu; wasmtime-Lader, nativ)      │   │  - deutscher Code (aus       │
 │ harper-ls / harper-cli / harper-desktop               │   │    feature/german-…)         │
 │     └─ Sprache ≠ en → PluginHost                      │   │  - export_plugin!(German)    │
 │ packages/harper.js                                    │   │ build → harper-de.wasm       │
 │     └─ PluginLinter (lädt .wasm ohne wasm-bindgen)    │   │ Release-Asset + Manifest     │
 │ chrome-plugin / obsidian-plugin / web                 │   └──────────────────────────────┘
 │     └─ Spracheinstellung, Plugin-Verwaltung           │
 └──────────────────────────────────────────────────────┘
```

Das Plugin-Repo hängt von `harper-core` und `harper-plugin-api` per
Git-Abhängigkeit an einen **festen Tag/Commit** von `dronakurl/harper`. Es gibt
keine Abhängigkeit in die andere Richtung.

## 5. Die Plugin-Schnittstelle (ABI v1)

Absichtlich minimal und JSON-basiert, damit Rust-Host, JS-Host und Plugin
unabhängig voneinander gebaut werden können.

### 5.1 Exporte des Wasm-Moduls

```text
harper_plugin_abi_version() -> u32            // = 1
harper_alloc(len: u32) -> u32                 // Puffer im Plugin-Speicher anlegen
harper_free(ptr: u32, len: u32)
harper_call(ptr: u32, len: u32) -> u64        // Anfrage-JSON rein; (ptr << 32 | len) der Antwort raus
memory                                        // exportierter linearer Speicher
```

**Keine Importe.** Das Modul bekommt keinen Zugriff auf Dateien, Netz, Uhr oder
Zufall. Dadurch ist es sicher sandboxed und läuft in jedem Host gleich. (Was
daraus für `getrandom` folgt, siehe §8.)

### 5.2 Methoden (`harper_call`)

Anfrage: `{"method": "...", "params": {...}}`; Antwort: `{"ok": ...}` oder
`{"err": "..."}`.

| Methode | Zweck |
|---|---|
| `info` | `{id: "de", name: "Deutsch", plugin_version, abi: 1, harper_core_version, dialects: [{id, name}], default_dialect}` |
| `lint` | `{text, format, dialect, config?}` → `[PluginLint]` |
| `lint_descriptions` | Regelname → Beschreibung (für Einstellungs-UIs) |
| `default_config` | Standard-`FlatConfig` des Plugins (Regel an/aus) |
| `set_user_words` | Wörter aus Nutzer-/Workspace-/Datei-Wörterbuch (ersetzt die bisherige Liste) |
| `detect` *(optional, Phase 4)* | `{text}` → Konfidenz 0..1, für automatische Spracherkennung |

`format` ist eines von `"plain"`, `"markdown"`, `"org"`, `"typst"` usw. Das
Plugin parst selbst mit seinem eigenen Parser.

**Quellcode-Kommentare (harper-ls):** Der Host extrahiert die Kommentare wie
heute über `harper-comments`. Statt Tokens schickt er
`regions: [{start, end, format}]` (Zeichenoffsets im Originaltext); das Plugin
lintet jede Region einzeln und verschiebt die Spans zurück. Das ist der
heikelste Teil der Schnittstelle und sollte im Spike (§8) zuerst ausprobiert
werden.

### 5.3 `PluginLint`

Eigener, stabiler Typ in `harper-plugin-api`, **nicht** `harper_core::Lint`
direkt, damit sich `Lint` in `master` ändern darf, ohne alte Plugins zu brechen:

```rust
pub struct PluginLint {
    pub span: (usize, usize),     // Zeichen-Offsets (char, nicht Byte/UTF-16)
    pub rule: String,
    pub kind: String,             // LintKind als String
    pub message: String,
    pub suggestions: Vec<PluginSuggestion>, // ReplaceWith(String) | Remove | InsertAfter(String)
    pub priority: u8,
}
```

Der Host wandelt `PluginLint` in `harper_core::Lint` um. Dadurch funktionieren
Code-Actions, „Ignorieren“, Statistik usw. in harper-ls und harper.js ohne
Sonderfälle.

### 5.4 Versionierung

- `harper_plugin_abi_version` muss exakt passen, sonst lehnt der Host ab und
  meldet „Plugin für ABI vN, Harper unterstützt vM“.
- Neue Methoden sind abwärtskompatibel (unbekannte Methode → `err`); der Host
  prüft vorher über `info`.

## 6. Änderungen in `master` (dieses Repo)

Ziel: überschaubar, nichts Deutsch-spezifisches, in kleine PRs schneidbar.

### 6.1 Neue Crate `harper-plugin-api`

- Typen aus §5 (`PluginInfo`, `LintRequest`, `PluginLint`, …), `serde`.
- **Gast-Seite:** Trait + Makro, damit ein Sprach-Plugin nur noch das hier
  schreiben muss:

  ```rust
  pub trait LanguagePlugin: Sized + 'static {
      fn info() -> PluginInfo;
      fn new() -> Self;                                  // lazy: Wörterbuch hier bauen
      fn lint(&mut self, req: LintRequest) -> Vec<PluginLint>;
      fn lint_descriptions(&self) -> BTreeMap<String, String>;
      fn default_config(&self) -> FlatConfig;
      fn set_user_words(&mut self, words: Vec<String>);
  }
  harper_plugin_api::export_plugin!(German);
  ```

  Das Makro erzeugt `harper_alloc`/`harper_free`/`harper_call` und das
  JSON-Dispatching.
- Hilfsfunktion `harper_core::Lint` → `PluginLint` (Gast) und zurück (Host).
- Ein **Beispiel-/Testplugin** `harper-plugin-api/examples/echo` (oder ein
  winziges „Pseudo-Englisch“-Plugin), mit dem CI den ganzen Weg testet, ohne
  dass `master` eine echte Zweitsprache enthält.

### 6.2 Neue Crate `harper-plugin-host` (nativ)

- `PluginHost::load(path | bytes) -> Result<LoadedPlugin>` mit `wasmtime`
  (Cranelift; Fallback auf `wasmi`, falls `wasmtime` auf einer Zielplattform
  nicht baut).
- Prüft ABI-Version und `info`, cached kompilierte Module
  (`wasmtime`-Serialisierung im Cache-Ordner, damit der zweite Start schnell ist).
- **Lazy:** Ein Plugin wird erst instanziiert, wenn ein Dokument in seiner
  Sprache gelintet wird; optional nach N Minuten Leerlauf wieder entladen.
- **Plugin-Verzeichnis:** `<config_dir>/harper/plugins/*.wasm` (über `dirs`,
  wie heute für Wörterbücher), überschreibbar per Einstellung.
- **Download (optional, Feature `plugin-download`):** Lädt aus einem
  Plugin-Index (JSON mit `{id, version, url, sha256, abi}`, Standard-URL zeigt auf
  GitHub-Releases von `dronakurl/harper-german`), prüft SHA-256, legt die Datei
  ins Plugin-Verzeichnis. Nur nach ausdrücklicher Nutzeraktion bzw. Einstellung
  `autoDownloadPlugins`.

### 6.3 harper-ls, harper-cli, harper-desktop

- **Einstellung `language`** (`"en"` Standard, sonst Plugin-ID wie `"de"`),
  zusätzlich zu `dialect`. Optional pro Datei über die Sprache des Editors
  bzw. `detect` (Phase 4). Da Mischtexte nicht nötig sind, reicht eine Sprache
  pro Dokument.
- **Weiche an genau einer Stelle:** Wo heute `LintGroup::new_curated(...)`
  gebaut wird (`harper-ls/src/backend.rs`, `update_document` /
  `did_change_configuration`), bekommt der Dokumentzustand ein
  `enum DocLinter { Native(LintGroup), Plugin(Arc<LoadedPlugin>) }`. Der Rest von
  `backend.rs` (Diagnostics, Code-Actions, Ignorieren) arbeitet weiter auf
  `Vec<Lint>`.
- Wörterbuch-Befehle („Zum Wörterbuch hinzufügen“) rufen bei Plugin-Dokumenten
  zusätzlich `set_user_words`.
- **harper-cli:** `--language <id>` und `--plugin <pfad.wasm>`; ein
  Unterbefehl `harper-cli plugin install|list|remove`.
- **harper-desktop:** Sprachauswahl in den Einstellungen; der Highlighter nutzt
  denselben `PluginHost`. Beim Lint-Callback muss bei Plugin-Sprachen das
  Plugin statt `Config::create_linter()` benutzt werden.
- **VS-Code-Erweiterung:** nur neue Einstellungen `harper.language`,
  `harper.pluginDirectory`, `harper.autoDownloadPlugins` durchreichen.

### 6.4 harper.js und die Web-Integrationen

- **`PluginLinter`** in `packages/harper.js`: implementiert das bestehende
  `Linter`-Interface (`lint`, `getLintDescriptions`, `setLintConfig`, …), lädt
  ein Plugin per `WebAssembly.instantiate(bytes)` und spricht die JSON-ABI. Läuft
  wie `LocalLinter`/`WorkerLinter` auch im Worker.
- Spans: Plugins liefern `char`-Offsets, harper.js rechnet wie heute in
  UTF-16 um. **Dieselbe Umrechnung wiederverwenden, nicht neu schreiben.**
- **Website/Demo:** darf das Plugin per URL laden (Website ist keine
  Store-Erweiterung).
- **Obsidian:** Plugin-Datei liegt im Vault-Plugin-Ordner. Manuelles Ablegen
  geht immer; ob automatischer Download erlaubt ist, gegen die
  Obsidian-Community-Richtlinien prüfen (§9).
- **Chrome-/Firefox-Erweiterung:** Wegen des Verbots von Fremdcode **nicht**
  herunterladen. Zwei erlaubte Wege, Entscheidung beim Umsetzen (§9):
  1. **Begleit-Erweiterung** „Harper – Deutsch“ im Store, die das Wasm-Modul
     selbst enthält und über `chrome.runtime.sendMessage(extensionId, …)`
     Lint-Anfragen der Haupt-Erweiterung beantwortet. Nachinstallieren =
     Begleit-Erweiterung installieren. Hält die Haupt-Erweiterung klein.
  2. Manuelles Hochladen der `.wasm` in den Erweiterungs-Einstellungen
     (gespeichert in IndexedDB). Rechtlich zu prüfen, eher für Entwickler.

### 6.5 Was in `master` **nicht** passiert

- Kein `harper-core/src/language/`, keine Feature-Flags pro Sprache, kein
  generiertes `Dialect`-Enum in `harper-wasm`.
- Keine Änderung an `DictWordMetadata` und am englischen Linting-Pfad.
- Englisch bleibt fest eingebaut und ist kein Plugin.

## 7. Das Plugin-Repo `dronakurl/harper-german`

```
harper-german/
├── Cargo.toml               # [lib] crate-type = ["cdylib", "rlib"]
│                            # harper-core, harper-plugin-api = { git = "https://github.com/dronakurl/harper", tag = "…" }
├── src/                     # aus harper-core/src/language/german/ übernommen
│   ├── lib.rs               # impl LanguagePlugin for German; export_plugin!(German)
│   ├── spell/ linting/ grammar/ parsers/ …
│   └── morphology.rs        # bisher harper-core/src/language/morphology.rs
├── data/                    # dictionary.dict, annotations.json, compound_begin_forms.txt …
├── tests/                   # bisherige Tests + Batterien
├── scripts/                 # Recall-/Battery-/Treebank-Skripte
├── justfile                 # build-wasm, test, language-perf …
└── .github/workflows/release.yml   # baut harper-de.wasm, sha256, plugin-index.json
```

Build: `cargo build --release --target wasm32-unknown-unknown` →
`wasm-opt -Oz` → `harper-de.wasm` + `harper-de.wasm.sha256` als Release-Asset;
`plugin-index.json` wird mit veröffentlicht.

### 7.1 Was beim Umzug des deutschen Codes zu lösen ist

Der deutsche Code greift heute auf `pub(crate)`-Interna von `harper-core` zu
und auf Änderungen, die nur auf `feature/german-language-support` existieren.
Bekannte Stellen (aus dem Diff gegen `master`) und die geplante Lösung:

| Abhängigkeit | Lösung |
|---|---|
| `Parser::is_english()` / `Document::parse` überspringt Brill-Tagger und Chunker | Kleiner, allgemeiner Core-PR („Parser darf POS-Tagging abschalten“). Sinnvoll auch ohne Plugins |
| `DictWordMetadata.morphology` (Feature-Feld) | **Nicht** in Core. Plugin hält Morphologie in einer eigenen Seitentabelle (Wort → `Morphology`) in seinem `CompactDict`; `MorphologyExt` liest von dort |
| `spell::rune` (`parse_word_list`, `AttributeList`) ist privat | Core-PR: öffentlich machen (oder Plugin kopiert die Expansion; sie ist ohnehin für die Pack-Prüfsumme eingebunden) |
| Fix der Reihenfolge in `attribute_list.rs` (`new_word_order`) | Eigener Bugfix-PR an Core, unabhängig von Plugins |
| `lexing::lex_*`, `fst_dictionary`-Helfer, `merged_dictionary::word_hash`, `Dictionary::cached_word_hash` | Pro Stelle entscheiden: Core-PR „öffentlich machen“ oder ins Plugin kopieren. Kopieren ist bei Kleinkram vorzuziehen |
| `Markdown::with_inline_parser`, `OrgMode::with_inline_parser` | Core-PR (allgemein nützlich: Markdown mit beliebigem Inline-Parser) |
| `title_case`, `CompoundChecker`-bezogene Änderungen | Prüfen, ob ins Plugin verlagerbar |
| `.pack`-Beschleuniger (`HARPER_GERMAN_PACK`) | Entfällt bzw. wird ersetzt: Im Wasm-Modul gibt es kein Dateisystem. Stattdessen das vorab expandierte Wörterbuch **beim Bauen** ins Modul backen (`build.rs` erzeugt das Pack, `include_bytes!`). Das spart die Startzeit im Plugin |

Bis diese Core-PRs gemergt sind, hängt das Plugin an einem Tag des
Plugin-Branches in `dronakurl/harper`, der sie enthält.

## 8. Phasen und Reihenfolge

Jede Phase ist einzeln lieferbar und testbar.

**Phase 0: Spike (1–2 Tage, wegwerfbar).** Beweisen, dass es geht, bevor
irgendetwas Schönes gebaut wird:
- `harper-core` als `cdylib` für `wasm32-unknown-unknown` **ohne**
  `wasm-bindgen` bauen. Bekanntes Hindernis: `getrandom` 0.2/0.3 (über `burn`
  → `harper-pos-utils` → `harper-brill`) braucht dort ein Backend. Optionen:
  `getrandom_backend="custom"` mit fester Saat (Harper braucht keinen echten
  Zufall), oder Brill/Burn in `harper-core` hinter ein Feature legen (spart im
  Plugin auch Größe, da Nicht-Englisch sie nicht benutzt).
- Mini-Plugin mit `lint` exportieren, einmal aus `wasmtime` (Rust-Test) und
  einmal aus Node (`WebAssembly.instantiate`) aufrufen, gleiche Lints.
- Messen: Modulgröße, Instanziierungszeit, Lint-Zeit vs. nativ (mit dem
  deutschen Code vom Feature-Branch als Last).
- **Abbruchkriterium:** Ist Lint-Zeit > 3× nativ oder ist das Modul trotz
  `wasm-opt` unvertretbar groß, dann zurück auf die Rückfallebene
  „Plugin-Crate beim Bauen eingebunden“ (§3).

**Phase 1: `harper-plugin-api` + `harper-plugin-host` + Testplugin.** Mit
Unit-Tests und einem CI-Job, der das Testplugin baut und lädt.

**Phase 2: harper-ls + harper-cli.** Einstellung `language`, Weiche im
Backend, Plugin-Verzeichnis, manuelles Ablegen. Integrationstest mit
Testplugin.

**Phase 3: `harper-german` als Plugin.** Code vom Feature-Branch übernehmen,
§7.1 abarbeiten, Release-Workflow. Abnahme: dieselben Lints wie
`feature/german-language-support` auf den deutschen Testkorpora
(`test_sources`, Batterien); Abweichungen begründen.

**Phase 4: harper.js `PluginLinter`, Website-Demo, Obsidian.**

**Phase 5: Desktop + Browser-Erweiterung** (Begleit-Erweiterung, §6.4).

**Phase 6: Komfort.** Automatischer Download mit Prüfsumme, `detect` für
automatische Spracherkennung, Entladen bei Leerlauf, dann pt/sk/pl als eigene
Plugin-Repos nach demselben Muster.

## 9. Offene Punkte (beim Umsetzen klären)

1. Browser-Erweiterung: Begleit-Erweiterung oder manuelles Hochladen (§6.4)?
   Store-Richtlinien von Chrome und Firefox zum Zeitpunkt der Umsetzung erneut
   lesen.
2. Obsidian: Ist automatisches Nachladen von Wasm nach den
   Community-Plugin-Richtlinien erlaubt?
3. `wasmtime` vs. `wasmi` für native Hosts: nach Messung in Phase 0.
4. Signatur zusätzlich zur SHA-256-Prüfsumme (z. B. minisign) für den
   Auto-Download?
5. Upstream: Wenn gewünscht, Phase 1–2 als kleine Einzel-PRs bei
   Automattic/harper vorschlagen. Vorher mit dem Maintainer sprechen (siehe
   `AGENT_POLICY.md`: PR-Beschreibungen müssen von Menschen geschrieben sein,
   und von Agenten geschriebener Code hat deutlich schlechtere Chancen).

## 10. Abnahmekriterien für das Gesamtziel

- `master`-Diff enthält keinen sprachspezifischen Code und bleibt in der
  Größenordnung weniger tausend Zeilen (ohne `Cargo.lock`).
- Ein englischer Nutzer, der kein Plugin installiert, merkt keinen Unterschied
  (gleiche Lints, Startzeit und Speicher ±5 %; mit `just`-Benchmarks messen).
- Dieselbe Datei `harper-de.wasm` funktioniert in harper-ls, harper-cli,
  Desktop, harper.js (Node + Browser) und Obsidian.
- Ohne geladenes Plugin belegt Deutsch null Speicher; mit Plugin ist der
  Speicher nach dem Entladen wieder frei.
- `harper-german` kann ohne Änderung an `master` ein neues Release
  veröffentlichen, solange die ABI-Version gleich bleibt.

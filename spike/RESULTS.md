# Phase 0 – Ergebnis des Spikes

Wegwerf-Code (nicht Teil des Workspaces). Zwei Crates:

- `plugin-spike/`: `harper-core` (ohne Default-Features) als `cdylib` für
  `wasm32-unknown-unknown`, ohne `wasm-bindgen`-Glue. Exportiert die
  ABI-v1-Funktionen `harper_plugin_abi_version`, `harper_alloc`, `harper_free`,
  `harper_call` und `memory`. `harper_call` bekommt Text und liefert die Lints als JSON.
- `host/`: Rust-Host mit `wasmtime`, misst gegen den nativen Aufruf derselben Funktion.
- `plugin-spike/run.mjs`: derselbe Aufruf aus Node (`WebAssembly.instantiate`).

Nachbauen:

    cd spike/plugin-spike && cargo build --release --target wasm32-unknown-unknown
    node run.mjs target/wasm32-unknown-unknown/release/plugin_spike.wasm
    cd ../host && cargo run --release -- ../plugin-spike/target/wasm32-unknown-unknown/release/plugin_spike.wasm

## Messwerte (Last: englische Standard-Lints, 2 Sätze; deutscher Code wurde bewusst noch nicht portiert)

| Messung | Nativ | wasmtime | Node (V8) |
|---|---|---|---|
| Erster Aufruf (Wörterbuch + Lints aufbauen) | 642 ms | 748 ms (1,17×) | 993 ms |
| Weitere Aufrufe | 7,4 ms | 11,0 ms (1,5×) | 11–12 ms |
| Instanziieren | – | 2 ms | 4 ms |
| Kompilieren (einmalig) | – | 5,2 s (cranelift) | im `instantiate` enthalten |
| Modulgröße roh / gzip -9 | – | 9,3 MB / 2,85 MB | – |
| Speicher nach Lauf | – | – | 163 MB |

**Abbruchkriterium (> 3× nativ): nicht erreicht.** Steady State 1,5×, kalt 1,17×.

## Befunde

1. **Es geht ohne `wasm-bindgen`-Glue, aber nicht ganz ohne Imports.** `burn`
   (über `harper-pos-utils` → `harper-brill` → `harper-core`) zieht `js-sys` und
   `wasm-bindgen` ins Modul. Ergebnis: 5 Imports (`__wbindgen_*`) und ~1400
   unnütze Exports. Sie werden zur Laufzeit nie aufgerufen; mit Trap-Stubs
   (`define_unknown_imports_as_traps` in wasmtime, Stub-Objekt in Node) läuft
   alles. Für das Ziel „Modul ohne Imports“ muss `burn`/Brill in `harper-core`
   hinter ein Cargo-Feature (§6.1 der Planung). Für ein reines deutsches Plugin
   spart das auch Größe (Nicht-Englisch braucht den englischen Tagger/Chunker nicht).
2. **`getrandom` gelöst:** `.cargo/config.toml` mit
   `--cfg getrandom_backend="custom"` plus `getrandom 0.2` (Feature `custom`)
   und eine `__getrandom_v03_custom`-Funktion mit fester Saat.
3. **Kaltstart dominiert.** Der erste Aufruf kostet fast eine Sekunde (Aufbau
   des Wörterbuchs/der Lints). Das gilt auch nativ. Folge: Plugin-Instanz
   behalten (nicht pro Aufruf neu bauen) und `LintGroup` im Plugin cachen.
4. **Kompilierzeit in wasmtime (5 s)** muss gecacht werden
   (`Engine::precompile_module` → Datei neben dem Plugin), sonst startet
   `harper-ls` bei jedem Start 5 s länger. In Browsern übernimmt V8 das
   (Streaming-Compile, Cache).
5. **Größe:** 9,3 MB sind der englische Wörterbuch- und Brill-Anteil. Das
   deutsche Plugin hat ein anderes Profil (eigenes Wörterbuch, kein Brill).
   `wasm-opt -Oz` war im Spike nicht verfügbar und ist noch nicht gemessen.

## Entscheidung

Der Steckmodul-Weg trägt. Weiter mit Phase 1. Zwei Vorarbeiten für master
kommen dazu: (a) `burn`/Brill hinter ein Feature in `harper-core`, (b) Phase 0
mit dem deutschen Code als Last wiederholen, sobald der portiert ist (Phase 3).

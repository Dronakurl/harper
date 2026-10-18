# 11. Bauen, Testen, Verteilen

[← Das deutsche Plugin](10-deutsches-plugin.md) · [Weiter: Offene Punkte →](12-offene-punkte.md)

Die praktischen Befehle. Als Nachschlagekapitel gedacht.

## Voraussetzungen

- Rust (die Version steht in `rust-toolchain.toml`)
- `rustup target add wasm32-unknown-unknown`
- [`just`](https://github.com/casey/just)
- Für Browser-Pakete: `pnpm`

Ohne das Wasm-Target **überspringen** lokale Tests das Echo-Plugin
(sie bauen es selbst). In der CI (Umgebungsvariable `CI`) ist das
Fehlen ein Fehler.

## Das Echo-Plugin: der Prüfstand

[`harper-plugin-api/examples/echo.rs`][echo] ist ein vollständiges
Mini-Plugin für eine erfundene Sprache „Echo" (Id `xx`): markiert das
Wort „teh" und doppelte Wörter. Es kann Regionen, Konfiguration,
Nutzerwörter und Erkennung (jeder Text mit „teh" gilt als Echo). Es dient
als Prüfobjekt für alles, was Wasm braucht, **ohne** das 45-MB-Deutsch
bauen zu müssen:

- `harper-plugin-host/tests/echo.rs`
- `harper-ls` (Tests in `plugins.rs`)
- `harper-cli/tests/plugin.rs`
- `harper.js`, Chrome (`tests/plugin-fixtures/echo.wasm`)

Bauen von Hand:

```
cargo build --release -p harper-plugin-api \
  --example echo --target wasm32-unknown-unknown
```

## Harper (dieses Repo)

```
cargo test -p harper-plugin-api
cargo test -p harper-plugin-host
cargo test -p harper-cli --features plugins
cargo test -p harper-ls
just install            # harper-ls + harper-cli
```

Das Feature `plugins` ist in `harper-ls` und `harper-cli` standardmäßig an.
Bauen ohne: `--no-default-features`, dann gelten die Stubs und das
Binary bleibt klein.

Weitere `just`-Befehle (Datei `justfile`):

- `just test-chrome-plugin`: baut `echo.wasm` und startet die
  Playwright-Tests
- `just install-plugin FILE`: ruft `harper-cli plugin install`
- `just language-install OWNER/REPO [TAG]`: lädt die Plugin-Datei
  aus einem GitHub-Release, prüft die Prüfsumme (`.sha256`), und
  installiert sie. Erwartet **genau ein** Asset
  `harper-<id>.wasm` im Release.

CI (`.github/workflows/just_checks.yml`): hat zusätzlich das Wasm-Target,
damit die Tests mit dem Echo-Plugin nicht übersprungen werden.

## harper-german

```
just test             # cargo test --lib (nativ)
just bake-pack        # data/german.pack erzeugen
just build-wasm       # Pack backen + Wasm bauen (release)
just test-plugin      # bauen + durch den Host testen
just release-files V  # dist/ mit Wasm, sha256, plugin-index.json
```

Wichtig: **Ein Wasm ohne gebackenes Pack ist langsam beim Start.**
Nach `git clone` fehlt `data/german.pack` (in `.gitignore`). `build-wasm`
backt es automatisch. Wer `cargo build` direkt aufruft, bekommt das
Wasm ohne Pack.

CI (`ci.yml`): zwei Jobs, `just check` + `just test`, und
`just test-plugin`. Das Release (`release.yml`) läuft bei einem Tag
`v*` oder von Hand mit Version; ohne Version ist es ein Probelauf, der
die Dateien nur als Workflow-Artefakt behält. Es nimmt `wasm-opt -Oz`
zu Hilfe, falls vorhanden.

## Verteilen

Wie kommt ein Plugin zum Nutzer?

```
harper-german (Release)
   harper-de.wasm
   harper-de.wasm.sha256
   plugin-index.json
        │
        ▼
just language-install ... ──► ~/.config/harper/plugins/
harper-cli plugin install       harper-de.wasm
Obsidian: in den Tresor
Chrome: Datei wählen
```

- Der Name zählt: `harper-<id>.wasm`. (`harper-cli plugin install`
  benennt selbst nach der Id.)
- `plugin-index.json` (`id`, `version`, `abi`, `url`, `sha256`) wird
  beim Release erzeugt, aber noch von **keinem Host gelesen**. Ein
  Auto-Download wäre der nächste Schritt (offene Punkte).
- Es gibt noch keinen veröffentlichten Release in `harper-german`.

## Harper selbst installieren

Zwei Rezepte, die nur für den Fork gedacht sind:

- `just install-release`: lädt ein fertiges `harper-ls`/`harper-cli`
  aus einem GitHub-Release. Standard: neuester Release von `origin`.
  Im Fork gibt es keinen Tag, deshalb schlägt der Standard mit einer
  klaren Meldung fehl.
- `just install-from-ci`: stößt den Workflow `binaries.yml` auf dem
  Branch an, wartet (rund zehn Minuten), lädt das Artefakt und
  installiert es. Dafür hat `binaries.yml` einen
  `workflow_dispatch`-Trigger bekommen.

Beide sind für den Pull-Request **nicht gedacht**; Issue [#23] hält
fest, dass sie vor einem Upstream-PR entfernt werden. Das Rezept
`install-from-ci` ist außerdem unbefriedigend: Es baut jedes Mal neu,
auch wenn für den Commit schon ein Artefakt existiert.

## Fehlersuche

- „plugin was built for ABI vN": Host und Plugin passen nicht
  (Kapitel 2).
- „missing export `harper_call`": Datei ist kein Harper-Plugin.
- Plugin wird nicht gefunden: `harper-cli plugin list` zeigt den
  Ordner.
- `harper-ls` findet ein später hinzugefügtes Plugin nicht: die
  Fehlerliste (`failed`) wird bei Konfigurationsänderung geleert
  (Kapitel 5).
- Erstes Linten ist langsam: erst der `.cwasm`-Cache beim ersten
  Start (Kapitel 3), dann das Wörterbuch.
- Disk voll im Sandbox-Container: `target/debug` löschen.

## Merke

1. Echo-Plugin für Hosts, German für Inhalt.
2. Ohne Wasm-Target werden Tests lokal übersprungen, in der CI nicht.
3. Ein Wasm braucht das gebackene Pack, sonst langsamer Start.
4. Verteilung heute: manuell oder `language-install`; Index-Datei
   noch ohne Leser.

[← Das deutsche Plugin](10-deutsches-plugin.md) · [Weiter: Offene Punkte →](12-offene-punkte.md)

[#23]: https://github.com/Dronakurl/harper/issues/23
[echo]: https://github.com/Dronakurl/harper/blob/feature/language-plugins/harper-plugin-api/examples/echo.rs

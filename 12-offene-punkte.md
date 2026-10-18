# 12. Offene Punkte

[← Bauen, Testen, Verteilen](11-bauen-testen-verteilen.md) · [Start](README.md)

Was noch fehlt oder nicht stimmt, geordnet nach Thema. Jeder Punkt
verweist auf ein Issue, wenn es eines gibt.

Quelle der Architektur-Lücken ist Issue [#24]; die Punkte darunter
mit „neu" habe ich beim Schreiben dieser Doku gefunden.

## Muss vor dem Upstream-PR entschieden werden

- **Zeit- und Speichergrenzen für Plugins.** Der Host setzt keine.
  Ein Plugin in einer Endlosschleife hängt den Host. Vorschlag in
  PR [#22], nicht gebaut.
- **Verhalten alter Plugins** (neu): ignorieren `user_words` ohne
  Meldung. [#28]
- **`install-release` und `install-from-ci` entfernen.** [#23]
  Zusätzlich ist `install-from-ci` unbefriedigend: Es baut jedes Mal
  neu, auch wenn das Artefakt für den Commit schon existiert.
  Gewünscht war eine Prüfung auf ein vorhandenes Artefakt vor dem
  Neubau; das ist noch nicht umgesetzt.
- **Haken in `harper-core`** (Kapitel 7) müssen upstream einzeln
  begründbar sein.

## Korrektheit

- **CLI-Datei-Wörterbücher nicht nach Sprache getrennt** (neu). [#25]
- **Plugin-Dialekt wird nicht übergeben.** `LintRequest.dialect` bleibt
  leer; Deutsch nutzt ihn nicht, ein gewählter Dialekt ändert also
  nichts. [#24]
- **Eine Sprache pro Text im Browser und Obsidian.** Es gibt keine
  Erkennung pro Text und kein Mischen. [#8] beschreibt das Problem
  allgemein.

## Leistung

- **Aufruf pro Plugin ist seriell** (`Mutex`). Mehr Instanzen kosten je
  rund 330 MB. Siehe Kapitel 3.
- **Englischer Tagger für Plugin-Dokumente?** (neu, zu messen). [#29]
- **`english_ratio` mehrfach berechnet** (neu). [#30]
- **Cache wächst** (neu). [#27]
- **Wechselnde Nutzerwörter bauen die Regeln neu** (Kapitel 8): Der
  Test misst es, entschieden ist nichts.
- Der Start mit dem deutschen Plugin ist spürbar; die Angaben im README
  von harper-german sind zu prüfen. [harper-german#2]

## Plattformen und Verteilung

- **Kein veröffentlichter Release** in `harper-german`; im Fork kein
  `v*`-Tag, deshalb schlägt `just install-release` fehl.
- **`plugin-index.json` hat keinen Leser.** Auto-Download mit
  SHA-256-Prüfung fehlt.
- **Chrome Web Store:** Begleit-Erweiterung nötig, weil Erweiterungen
  keinen Code nachladen dürfen.
- **Desktop-App und Webseite `/plugins`** nicht gebaut oder getestet.
  WordPress: nicht unterstützt; empfohlen wird die Chrome-Erweiterung.
- **wasmtime auf Windows und macOS** ungeprüft.
- **`burn`/Brill in `harper-core`** liegt noch nicht hinter einem
  Feature; im Plugin-Modul stehen deshalb ein paar `__wbindgen_*`-Stubs
  (Kapitel 3: werden zu Fallen).
- **Chrome-Test mit Plugin** braucht `echo.wasm` und läuft nicht in
  der CI.

## Aufräumen

- Kopfkommentar in `plugins.rs` beschädigt. [#26]
- `harper-cli`: Hilfe zeigt `plugin` als ersten Befehl. [#17]
- `PluginInfo` in TypeScript ohne `aliases`. [#31]
- PR [#21] noch offen neben [#22].

## Weitere Sprachen

Portugiesisch, Slowakisch, Polnisch als eigene Plugin-Repos. Dafür
müssen `LanguageBits` und `morphology` verallgemeinert werden
(Issue [#11]: Lokativ, Instrumental, Vokativ, Belebtheit).

## Wie ich diese Doku aktuell halte

Sie zeigt auf feste Commits. Wenn der Branch weiterwandert:

1. Neue Commits ansehen:
   `git log ef659a5b..feature/language-plugins`
2. Nur die betroffenen Kapitel nachziehen.
3. Die Commit-Hashes im [Start](README.md) und die Link-Basis in den
   Kapiteln ersetzen (Suchen und Ersetzen der langen Hashes).

[← Bauen, Testen, Verteilen](11-bauen-testen-verteilen.md) · [Start](README.md)

[#8]: https://github.com/Dronakurl/harper/issues/8
[#11]: https://github.com/Dronakurl/harper/issues/11
[#17]: https://github.com/Dronakurl/harper/issues/17
[#21]: https://github.com/Dronakurl/harper/pull/21
[#22]: https://github.com/Dronakurl/harper/pull/22
[#23]: https://github.com/Dronakurl/harper/issues/23
[#24]: https://github.com/Dronakurl/harper/issues/24
[#25]: https://github.com/Dronakurl/harper/issues/25
[#26]: https://github.com/Dronakurl/harper/issues/26
[#27]: https://github.com/Dronakurl/harper/issues/27
[#28]: https://github.com/Dronakurl/harper/issues/28
[#29]: https://github.com/Dronakurl/harper/issues/29
[#30]: https://github.com/Dronakurl/harper/issues/30
[#31]: https://github.com/Dronakurl/harper/issues/31
[harper-german#2]: https://github.com/Dronakurl/harper-german/issues/2

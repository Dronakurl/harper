# 12. Offene Punkte

[← Bauen, Testen, Verteilen](11-bauen-testen-verteilen.md) · [Start](README.md)

Was noch fehlt oder nicht stimmt, geordnet nach Thema. Jeder Punkt
verweist auf ein Issue, wenn es eines gibt.

Quelle der Architektur-Lücken ist Issue [#24].

## Erledigt beim Schreiben dieser Doku

Behoben und geschlossen: [#25] (CLI-Datei-Wörterbücher je Sprache),
[#26] (Kopfkommentar), [#27] (Cache-Aufräumen), [#28] (alte Plugins
über `features`), [#29] (`NotEnglish`), [#30] (`english_ratio` einmal),
[#31] (TypeScript-Typ) und harper-german#2 (README-Zahlen).

## Muss vor dem Upstream-PR entschieden werden

- **Zeit- und Speichergrenzen für Plugins.** Der Host setzt keine.
  Ein Plugin in einer Endlosschleife hängt den Host. Vorschlag in
  PR [#22], nicht gebaut.
- **`install-release` und `install-from-ci` entfernen.** [#23]
  Zusätzlich ist `install-from-ci` unbefriedigend: Es baut jedes Mal
  neu, auch wenn das Artefakt für den Commit schon existiert.
  Gewünscht war eine Prüfung auf ein vorhandenes Artefakt vor dem
  Neubau; das ist noch nicht umgesetzt.
- **Haken in `harper-core`** (Kapitel 7) müssen upstream einzeln
  begründbar sein.

## Korrektheit

- **Plugin-Dialekt wird nicht übergeben.** `LintRequest.dialect` bleibt
  leer; Deutsch nutzt ihn nicht, ein gewählter Dialekt ändert also
  nichts. [#24]
- **Eine Sprache pro Text im Browser und Obsidian.** Es gibt keine
  Erkennung pro Text und kein Mischen. [#8] beschreibt das Problem
  allgemein.

## Leistung

- **Aufruf pro Plugin ist seriell** (`Mutex`). Mehr Instanzen kosten je
  rund 330 MB. Siehe Kapitel 3.
- **Wechselnde Nutzerwörter bauen die Regeln neu** (Kapitel 8): Der
  Test misst es, entschieden ist nichts.

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

- `harper-cli`: Hilfe zeigt `plugin` als ersten Befehl. [#17]
- PR [#21] noch offen neben [#22].

## Deutsche Regeln

Seit der Bestandsaufnahme (Issue
[harper-german#3](https://github.com/Dronakurl/harper-german/issues/3))
gibt es neun neue Regeln. Bewusst offen: *Sie/Ihnen* großschreiben,
doppelte Verneinung, *Der Mädchen spielt*, fehlende Artikel, Hilfsverb
bei Bewegungsverben. Gründe stehen in
`docs/GERMAN_DICTIONARY_NOTES.md`, Abschnitt „Tenth round".

## Weitere Sprachen

Portugiesisch, Slowakisch, Polnisch als eigene Plugin-Repos. Dafür
müssen `LanguageBits` und `morphology` verallgemeinert werden
(Issue [#11]: Lokativ, Instrumental, Vokativ, Belebtheit).

## Wie ich diese Doku aktuell halte

Sie zeigt auf feste Commits. Wenn der Branch weiterwandert:

Sie zeigt auf die Branches selbst, also immer auf den neuesten Code.
Zu pflegen ist nur der Text:

1. Neue Commits ansehen:
   `git log ec12f0d5..feature/language-plugins`
2. Nur die betroffenen Kapitel nachziehen.
3. Den Commit im [Start](README.md) erhöhen.

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

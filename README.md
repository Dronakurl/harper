# Sprach-Plugins in Harper: Lernpfad

Private Doku zu meinem Fork [Dronakurl/harper][fork], Branch
[`feature/language-plugins`][branch] (PR [#22]).
Ziel: Nach dem Lesen verstehe ich Code und Zusammenhänge des
Plugin-Systems so gut, dass ich jede Stelle selbst erklären kann.

Nur für mich. Dieser Branch (`private-documentation`) hat keine
gemeinsame Historie mit dem Code und wird nie gemerged.

## Stand der Doku

Alle Code-Links zeigen auf den **aktuellen Stand** der Branches
(`feature/language-plugins` in harper, `fix/single-cases-16-20` in
harper-german), nicht auf alte Commits. Links nennen deshalb Datei
und Name, keine Zeilennummern: Wer ein Symbol sucht, findet es mit
der Suche der Datei. Die Texte beschreiben den Stand von harper
[`ec12f0d5`][c-harper]. Spätere Änderungen können einzelne Absätze
überholt haben; Kapitel 12 sagt, wie ich die Doku nachziehe.

## Lesereihenfolge

Jedes Kapitel baut auf dem vorigen auf. Jedes endet mit
„Merke" und „Weiter".

1. [Überblick](01-ueberblick.md)
   Warum es Plugins gibt, und das Bild im Kopf.
2. [Die Schnittstelle](02-schnittstelle.md)
   Der Vertrag zwischen Host und Plugin (`harper-plugin-api`).
3. [Der Host](03-host.md)
   Wie ein Plugin geladen und aufgerufen wird
   (`harper-plugin-host`, wasmtime).
4. [Ein Lint von A bis Z](04-ein-lint-von-a-bis-z.md)
   Ein Satz wandert durch den ganzen Stack. Hier setzt sich alles
   zusammen.
5. [harper-ls im Detail](05-harper-ls.md)
   Sprachwahl, Registry, Dokumentzustand, Regionen.
6. [harper-cli im Detail](06-harper-cli.md)
   Parallel viele Dateien, Cache, `plugin`-Befehle.
7. [Die Haken in harper-core](07-core-haken.md)
   Was am Kern geändert wurde, und warum so wenig.
8. [Wörterbücher und Nutzerwörter](08-woerterbuecher.md)
   Das heikelste Thema: Wörter pro Sprache, pro Datei, pro Anfrage.
9. [Browser, Obsidian, harper.js](09-browser-obsidian-js.md)
   Der zweite Host: derselbe Vertrag in TypeScript.
10. [Das deutsche Plugin](10-deutsches-plugin.md)
    Das Repo [harper-german][german] von innen.
11. [Bauen, Testen, Verteilen](11-bauen-testen-verteilen.md)
    Befehle, CI, Release, Installation.
12. [Offene Punkte](12-offene-punkte.md)
    Was noch fehlt, mit Links zu den Issues.

Zum Nachschlagen: [Glossar](glossar.md).

## Wenn ich wenig Zeit habe

- 10 Minuten: Kapitel 1 und 4.
- 30 Minuten: dazu 2 und 3.
- Vollständig: alles der Reihe nach, am besten mit dem Code daneben.

## Schreibweise

- Ein Link auf `Datei` oder `Name` führt zum aktuellen Code im Branch.
- Diagramme sind schmal und stehen senkrecht, damit sie am Handy
  passen.
- „Host" ist das Programm, das ein Plugin lädt (`harper-ls`,
  `harper-cli`, Browser, Obsidian). „Gast" ist das Plugin.

[fork]: https://github.com/Dronakurl/harper
[branch]: https://github.com/Dronakurl/harper/tree/feature/language-plugins
[#22]: https://github.com/Dronakurl/harper/pull/22
[c-harper]: https://github.com/Dronakurl/harper/commit/ec12f0d5877fb19978a5e9b60a088d0c854aad48
[german]: https://github.com/Dronakurl/harper-german

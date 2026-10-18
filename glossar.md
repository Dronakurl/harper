# Glossar

[← Start](README.md)

Alphabetisch. Kapitelnummern zeigen, wo es erklärt wird.

**ABI** — Application Binary Interface. Hier: das Nachrichtenformat
zwischen Host und Plugin, Version `ABI_VERSION = 1`. (Kap. 2)

**alloc / free** — die zwei Funktionen, mit denen der Host im Speicher
des Plugins Puffer anlegt und freigibt. (Kap. 2)

**auto** — Sprachwahl: Der Host fragt jedes Plugin per `detect` und
nimmt das sicherste (mindestens 0,5). (Kap. 5)

**Cache (`.cwasm`)** — vorkompiliertes Modul im Cache-Ordner,
damit der zweite Start nicht neu kompiliert. (Kap. 3)

**Dialekt** — Variante einer Sprache (Schweiz, Österreich). Das Plugin
benennt seine Dialekte selbst. (Kap. 2, 10)

**dispatch** — die Funktion im Plugin, die ein JSON-Request liest und
die richtige Methode ruft. (Kap. 2)

**Echo-Plugin** — erfundenes Mini-Plugin für Tests. (Kap. 11)

**english_ratio** — Anteil der Wörter eines Textes, die Harpers
englisches Wörterbuch kennt; der Host rechnet ihn und gibt ihn dem
Plugin. (Kap. 3)

**Gast** — das Plugin, aus Sicht des Hosts. (Kap. 3)

**Host** — das Programm, das ein Plugin lädt und ruft. (Kap. 1)

**is_english** — Methode von `Parser`: sagt, ob der Text englisch ist;
bei `false` entfallen Tagger und Chunker. (Kap. 7)

**LanguageBits** — 128 Bit am Wörterbucheintrag für die Grammatik einer
Sprache; Harper liest sie nie. (Kap. 7)

**LintGroup** — Sammlung der Regeln. (Kap. 5, 8)

**LintRequest** — die Lint-Anfrage an das Plugin. (Kap. 2)

**Lint / PluginLint** — Fund einer Regel. `PluginLint` ist die stabile
Draht-Form, `Lint` der interne Typ von `harper-core`. (Kap. 2)

**LoadedPlugin** — im Host ein geladenes Plugin. (Kap. 3)

**MergedDictionary** — Mischung mehrerer Wörterbücher, die Kinder sind
`Arc`-Zeiger. (Kap. 8)

**pack (`german.pack`)** — das expandierte deutsche Wörterbuch, vorab
gebacken und ins Wasm eingebettet. (Kap. 10)

**PluginLinter** — im Host das Objekt, das ein Plugin samt den Einstellungen
eines Dokuments hält. In `harper-ls`, im CLI und in TypeScript je ein
eigener. (Kap. 4 bis 6, 9)

**PluginRegistry** — Verwaltung der geladenen Plugins in `harper-ls`.
(Kap. 5)

**Region** — Bereich eines Textes, der geprüft werden soll (z. B.
Kommentare in Quellcode). (Kap. 2, 5)

**Sandbox** — Das Plugin hat keine Importe und kann nichts außer
rechnen. (Kap. 1, 3)

**set_user_words** — setzt die Nutzerwörter eines Plugins „bis auf
Weiteres". (Kap. 8)

**Span** — Bereich `[start, end)` in Zeichen. (Kap. 2)

**Stub** — leere Ersatzimplementierung, wenn das Feature `plugins`
fehlt. (Kap. 3, 5)

**user_words** — Wörter, die mit einer Anfrage reisen und nur für sie
gelten. (Kap. 8)

**wasmtime** — die WebAssembly-Laufzeit der Rust-Hosts. (Kap. 3)

**Weir** — Harpers Regelsprache in Textdateien. Das deutsche Plugin hat
73 solcher Regeln. (Kap. 10)

[← Start](README.md)

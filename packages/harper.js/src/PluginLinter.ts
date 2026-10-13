import type { Dialect, Lint, Span, Suggestion, SuggestionKind } from 'harper-wasm';
import type Linter from './Linter';
import type { WeirpackTestFailures } from './Linter';
import type { LintConfig, LintOptions, StructuredLintConfig } from './main';
import type Summary from './Summary';

/**
 * A `Linter` for languages that are not built into Harper: it runs a language plugin, a
 * standalone WebAssembly module such as `harper-de.wasm` (see `LANGUAGE_PLUGINS_PLAN.md`).
 *
 * The module has no imports and is spoken to in JSON through four exported functions, so it is
 * loaded here with plain `WebAssembly.instantiate`, without Harper's own wasm-bindgen glue.
 *
 * Differences from `LocalLinter`:
 * - It runs on the calling thread. Create it inside a worker if linting should not block.
 * - Statistics, Weirpacks and English detection are not available.
 * - The lints are plain objects with the same methods as the ones `LocalLinter` returns, but they
 *   are not wasm-bindgen objects, so `WorkerLinter`'s transport cannot carry them.
 */

const ABI_VERSION = 1;

/** Everything the plugin tells about itself (`info`). */
export interface PluginInfo {
	id: string;
	name: string;
	plugin_version: string;
	abi: number;
	harper_core_version: string;
	dialects: { id: string; name: string }[];
	default_dialect: string | null;
}

/** What a plugin can be loaded from. */
export type PluginSource =
	| string
	| URL
	| Response
	| Promise<Response>
	| ArrayBuffer
	| ArrayBufferView
	| WebAssembly.Module;

interface PluginLintWire {
	span: [number, number];
	rule: string;
	kind: string;
	message: string;
	suggestions: ({ ReplaceWith: string } | { InsertAfter: string } | 'Remove')[];
	priority: number;
}

type PluginFormat = 'plain' | 'markdown' | 'typst';

function toFormat(language: LintOptions['language']): PluginFormat {
	switch (language) {
		case 'plaintext':
			return 'plain';
		case 'typst':
			return 'typst';
		default:
			return 'markdown';
	}
}

/** Pretty-printed lint kind, as `Lint.lint_kind_pretty()` gives for the built-in kinds. */
function prettyKind(kind: string): string {
	const pretty: Record<string, string> = { WordChoice: 'Word Choice', WordOrder: 'Word Order' };
	return pretty[kind] ?? kind;
}

function escapeHtml(text: string): string {
	return text
		.replaceAll('&', '&amp;')
		.replaceAll('<', '&lt;')
		.replaceAll('>', '&gt;')
		.replaceAll('"', '&quot;');
}

/** The WebAssembly side of a plugin: loading it and calling `harper_call`. */
class PluginRuntime {
	private readonly encoder = new TextEncoder();
	private readonly decoder = new TextDecoder();
	readonly info: PluginInfo;

	private constructor(private readonly exports: PluginExports) {
		this.info = this.call<PluginInfo>('info', undefined);
	}

	static async load(source: PluginSource): Promise<PluginRuntime> {
		const module = await compile(source);

		// A plugin has no imports of its own. Dependencies may still drag in unused stubs; they
		// are never called, so they are filled with functions that throw, and nothing else is
		// handed to the module: it cannot reach the page, the network or the file system.
		const imports: WebAssembly.Imports = {};
		for (const { module: ns, name, kind } of WebAssembly.Module.imports(module)) {
			if (kind !== 'function') {
				throw new Error(`Plugin imports ${ns}.${name} (${kind}), which Harper does not provide`);
			}
			(imports[ns] ??= {})[name] = () => {
				throw new Error(`The plugin called its import ${ns}.${name}`);
			};
		}

		const instance = await WebAssembly.instantiate(module, imports);
		const exports = instance.exports as unknown as Partial<PluginExports>;
		for (const name of [
			'harper_plugin_abi_version',
			'harper_alloc',
			'harper_free',
			'harper_call',
			'memory',
		] as const) {
			if (exports[name] === undefined) {
				throw new Error(`Not a Harper plugin: missing export '${name}'`);
			}
		}
		const full = exports as PluginExports;

		const abi = full.harper_plugin_abi_version();
		if (abi !== ABI_VERSION) {
			throw new Error(`Plugin was built for ABI v${abi}, this Harper supports v${ABI_VERSION}`);
		}
		return new PluginRuntime(full);
	}

	call<T>(method: string, params: unknown): T {
		const request = this.encoder.encode(
			JSON.stringify(params === undefined ? { method } : { method, params }),
		);
		const e = this.exports;

		const ptr = e.harper_alloc(request.length);
		new Uint8Array(e.memory.buffer, ptr, request.length).set(request);
		const packed = e.harper_call(ptr, request.length);
		const outPtr = Number(packed >> 32n);
		const outLen = Number(packed & 0xffffffffn);

		// Read after the call: it may have grown the memory.
		const text = this.decoder.decode(new Uint8Array(e.memory.buffer, outPtr, outLen));
		e.harper_free(outPtr, outLen);
		e.harper_free(ptr, request.length);

		const response = JSON.parse(text) as { ok: T } | { err: string };
		if ('err' in response) {
			throw new Error(`Plugin error: ${response.err}`);
		}
		return response.ok;
	}
}

interface PluginExports {
	memory: WebAssembly.Memory;
	harper_plugin_abi_version(): number;
	harper_alloc(len: number): number;
	harper_free(ptr: number, len: number): void;
	harper_call(ptr: number, len: number): bigint;
}

async function compile(source: PluginSource): Promise<WebAssembly.Module> {
	if (source instanceof WebAssembly.Module) {
		return source;
	}
	if (typeof source === 'string' || source instanceof URL) {
		return compile(fetch(source));
	}
	if (source instanceof Promise || (typeof Response !== 'undefined' && source instanceof Response)) {
		return WebAssembly.compileStreaming(source);
	}
	return WebAssembly.compile(source as BufferSource);
}

/** A suggestion with the methods of the wasm-bindgen `Suggestion`. */
class PluginSuggestion {
	constructor(private readonly wire: PluginLintWire['suggestions'][number]) {}

	kind(): SuggestionKind {
		// SuggestionKind: Replace = 0, Remove = 1, InsertAfter = 2
		if (this.wire === 'Remove') return 1 as SuggestionKind;
		return ('ReplaceWith' in this.wire ? 0 : 2) as SuggestionKind;
	}

	get_replacement_text(): string {
		if (this.wire === 'Remove') return '';
		return 'ReplaceWith' in this.wire ? this.wire.ReplaceWith : this.wire.InsertAfter;
	}

	to_json(): string {
		return JSON.stringify(this.wire);
	}

	free(): void {}
	[Symbol.dispose](): void {}
}

/** A lint with the methods of the wasm-bindgen `Lint`. */
class PluginLintObject {
	private readonly jsSpan: { start: number; end: number };

	constructor(
		readonly wire: PluginLintWire,
		private readonly text: string,
		/** Character index (code point) → UTF-16 index. */
		toUtf16: (charIndex: number) => number,
	) {
		this.jsSpan = { start: toUtf16(wire.span[0]), end: toUtf16(wire.span[1]) };
	}

	get rule(): string {
		return this.wire.rule;
	}

	span(): Span {
		return {
			...this.jsSpan,
			len: () => this.jsSpan.end - this.jsSpan.start,
			is_empty: () => this.jsSpan.end === this.jsSpan.start,
			free: () => {},
		} as unknown as Span;
	}

	get_problem_text(): string {
		return this.text.slice(this.jsSpan.start, this.jsSpan.end);
	}

	lint_kind(): string {
		return this.wire.kind;
	}

	lint_kind_pretty(): string {
		return prettyKind(this.wire.kind);
	}

	suggestion_count(): number {
		return this.wire.suggestions.length;
	}

	suggestions(): Suggestion[] {
		return this.wire.suggestions.map((s) => new PluginSuggestion(s) as unknown as Suggestion);
	}

	message(): string {
		return this.wire.message;
	}

	message_html(): string {
		return escapeHtml(this.wire.message);
	}

	to_json(): string {
		return JSON.stringify({ ...this.wire, text: this.text });
	}

	free(): void {}
	[Symbol.dispose](): void {}
}

/** FNV-1a over UTF-16 code units, as a 64-bit BigInt. */
function hash64(text: string): bigint {
	let h = 0xcbf29ce484222325n;
	for (let i = 0; i < text.length; i++) {
		h = ((h ^ BigInt(text.charCodeAt(i))) * 0x100000001b3n) & 0xffffffffffffffffn;
	}
	return h;
}

export interface PluginLinterInit {
	/** Where to get the plugin from, e.g. a URL of `harper-de.wasm`. */
	plugin: PluginSource;
}

export default class PluginLinter implements Linter {
	private runtimePromise: Promise<PluginRuntime>;
	private config: LintConfig | undefined;
	private ignored = new Set<bigint>();
	private words: string[] = [];

	constructor(init: PluginLinterInit) {
		this.runtimePromise = PluginRuntime.load(init.plugin);
		// Avoid an unhandled rejection before anyone calls a method; the error surfaces on use.
		this.runtimePromise.catch(() => {});
	}

	/** What the plugin says about itself: its language id, name and dialects. */
	async info(): Promise<PluginInfo> {
		return (await this.runtimePromise).info;
	}

	async setup(): Promise<void> {
		await this.runtimePromise;
	}

	/** How sure the plugin is that `text` is written in its language, 0 to 1, or `null` if it
	 * cannot tell. Cheap: it does not make the plugin build its dictionary. */
	async detect(text: string): Promise<number | null> {
		return (await this.runtimePromise).call<number | null>('detect', { text });
	}

	private async runLint(text: string, options?: LintOptions): Promise<PluginLintObject[]> {
		const runtime = await this.runtimePromise;
		const config = this.config ?? (await this.getLintConfig());

		const wire = runtime.call<PluginLintWire[]>('lint', {
			text,
			format: toFormat(options?.language),
			config,
		});

		// The plugin counts characters (code points); JavaScript strings count UTF-16 units.
		const needsMapping = /[\ud800-\udfff]/.test(text);
		let toUtf16 = (i: number) => i;
		if (needsMapping) {
			const units: number[] = [0];
			for (const ch of text) {
				units.push(units[units.length - 1] + ch.length);
			}
			toUtf16 = (i) => units[Math.min(i, units.length - 1)];
		}

		let lints = wire.map((w) => new PluginLintObject(w, text, toUtf16));
		const ignoredFilter = async () => {
			const kept: PluginLintObject[] = [];
			for (const l of lints) {
				if (!this.ignored.has(await this.contextHash(text, l as unknown as Lint))) kept.push(l);
			}
			return kept;
		};
		if (this.ignored.size > 0) lints = await ignoredFilter();

		if (options?.dedup ?? true) {
			lints = removeOverlaps(lints);
		}
		return lints;
	}

	async lint(text: string, options?: LintOptions): Promise<Lint[]> {
		return (await this.runLint(text, options)) as unknown as Lint[];
	}

	async organizedLints(text: string, options?: LintOptions): Promise<Record<string, Lint[]>> {
		const out: Record<string, Lint[]> = {};
		for (const lint of await this.runLint(text, options)) {
			(out[lint.rule] ??= []).push(lint as unknown as Lint);
		}
		return out;
	}

	async applySuggestion(text: string, lint: Lint, suggestion: Suggestion): Promise<string> {
		const { start, end } = lint.span();
		const replacement = suggestion.get_replacement_text();
		switch (suggestion.kind() as number) {
			case 1:
				return text.slice(0, start) + text.slice(end);
			case 2:
				return text.slice(0, end) + replacement + text.slice(end);
			default:
				return text.slice(0, start) + replacement + text.slice(end);
		}
	}

	async isLikelyEnglish(_text: string): Promise<boolean> {
		return false;
	}

	async isolateEnglish(text: string): Promise<string> {
		return text;
	}

	async getDefaultLintConfig(): Promise<LintConfig> {
		return (await this.runtimePromise).call<LintConfig>('default_config', undefined);
	}

	async getDefaultLintConfigAsJSON(): Promise<string> {
		return JSON.stringify(await this.getDefaultLintConfig());
	}

	async getLintConfig(): Promise<LintConfig> {
		return this.config ?? (await this.getDefaultLintConfig());
	}

	async getLintConfigAsJSON(): Promise<string> {
		return JSON.stringify(await this.getLintConfig());
	}

	async setLintConfig(config: LintConfig): Promise<void> {
		this.config = { ...(await this.getDefaultLintConfig()), ...config };
	}

	async setLintConfigWithJSON(config: string): Promise<void> {
		await this.setLintConfig(JSON.parse(config) as LintConfig);
	}

	async getStructuredLintConfig(): Promise<StructuredLintConfig> {
		const config = await this.getLintConfig();
		return {
			settings: Object.entries(config).map(([name, state]) => ({
				Bool: { name, state: state ?? true, label: null },
			})),
		};
	}

	async getStructuredLintConfigJSON(): Promise<string> {
		return JSON.stringify(await this.getStructuredLintConfig());
	}

	async getLintDescriptions(): Promise<Record<string, string>> {
		return (await this.runtimePromise).call<Record<string, string>>('lint_descriptions', undefined);
	}

	async getLintDescriptionsAsJSON(): Promise<string> {
		return JSON.stringify(await this.getLintDescriptions());
	}

	async getLintDescriptionsHTML(): Promise<Record<string, string>> {
		const descriptions = await this.getLintDescriptions();
		return Object.fromEntries(Object.entries(descriptions).map(([k, v]) => [k, escapeHtml(v)]));
	}

	async getLintDescriptionsHTMLAsJSON(): Promise<string> {
		return JSON.stringify(await this.getLintDescriptionsHTML());
	}

	async toTitleCase(text: string): Promise<string> {
		return text;
	}

	async dispose(): Promise<void> {
		// The module is garbage collected with this object.
	}

	async contextHash(source: string, lint: Lint): Promise<bigint> {
		const { start, end } = lint.span();
		const before = source.slice(Math.max(0, start - 20), start);
		const after = source.slice(end, end + 20);
		return hash64(`${lint.lint_kind()}\u0000${lint.message()}\u0000${before}\u0001${lint.get_problem_text()}\u0001${after}`);
	}

	async ignoreLint(source: string, lint: Lint): Promise<void> {
		this.ignored.add(await this.contextHash(source, lint));
	}

	async ignoreLints(source: string, lints: Lint[]): Promise<void> {
		for (const lint of lints) await this.ignoreLint(source, lint);
	}

	async ignoreLintHash(hash: bigint): Promise<void> {
		this.ignored.add(hash);
	}

	async exportIgnoredLints(): Promise<string> {
		return JSON.stringify([...this.ignored].map((h) => h.toString(16)));
	}

	async importIgnoredLints(json: string): Promise<void> {
		for (const h of JSON.parse(json) as string[]) this.ignored.add(BigInt(`0x${h}`));
	}

	async clearIgnoredLints(): Promise<void> {
		this.ignored.clear();
	}

	async clearWords(): Promise<void> {
		this.words = [];
		(await this.runtimePromise).call('set_user_words', { words: [] });
	}

	async importWords(words: string[]): Promise<void> {
		this.words = [...new Set([...this.words, ...words])];
		(await this.runtimePromise).call('set_user_words', { words: this.words });
	}

	async exportWords(): Promise<string[]> {
		return [...this.words];
	}

	/** Plugin dialects are named by the plugin, see `info()`. They are not English `Dialect`s. */
	async getDialect(): Promise<Dialect> {
		return 0 as Dialect;
	}

	async setDialect(_dialect: Dialect): Promise<void> {}

	async summarizeStats(): Promise<Summary> {
		throw new Error('Statistics are not available for plugin languages.');
	}

	async generateStatsFile(): Promise<string> {
		return '';
	}

	async importStatsFile(_statsFile: string): Promise<void> {}

	async loadWeirpackFromBlob(_blob: Blob): Promise<WeirpackTestFailures | undefined> {
		throw new Error('Weirpacks are not available for plugin languages.');
	}

	async loadWeirpackFromBytes(_bytes: Uint8Array): Promise<WeirpackTestFailures | undefined> {
		throw new Error('Weirpacks are not available for plugin languages.');
	}
}

/** Keep the more important lint (lower priority number) of any two that overlap. */
function removeOverlaps(lints: PluginLintObject[]): PluginLintObject[] {
	const ordered = [...lints].sort(
		(a, b) => a.wire.priority - b.wire.priority || a.span().start - b.span().start,
	);
	const kept: PluginLintObject[] = [];
	for (const lint of ordered) {
		const s = lint.span();
		if (!kept.some((k) => s.start < k.span().end && k.span().start < s.end)) kept.push(lint);
	}
	return kept.sort((a, b) => a.span().start - b.span().start);
}

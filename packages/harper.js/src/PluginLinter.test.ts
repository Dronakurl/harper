import { beforeAll, describe, expect, test } from 'vitest';
import PluginLinter from './PluginLinter';

// `just test-harperjs` builds the `echo` example plugin of harper-plugin-api into this file.
const ECHO_URL = '/test-fixtures/echo.wasm';

let bytes: ArrayBuffer;

beforeAll(async () => {
	const response = await fetch(ECHO_URL);
	if (!response.ok) {
		throw new Error(
			`${ECHO_URL} is missing. Build it with: cargo build --release -p harper-plugin-api --example echo --target wasm32-unknown-unknown`,
		);
	}
	bytes = await response.arrayBuffer();
});

describe('PluginLinter', () => {
	test('reports what the plugin is', async () => {
		const linter = new PluginLinter({ plugin: bytes });
		const info = await linter.info();
		expect(info.id).toBe('xx');
		expect(info.abi).toBe(1);
	});

	test('loads from a URL', async () => {
		const linter = new PluginLinter({ plugin: ECHO_URL });
		expect((await linter.lint('teh', { language: 'plaintext' })).length).toBe(1);
	});

	test('gives lints with the methods of the built-in ones', async () => {
		const linter = new PluginLinter({ plugin: bytes });
		const [lint] = await linter.lint('Teh cat sat.', { language: 'plaintext' });

		const span = lint.span();
		span.free();
		expect(lint.span().start).toBe(0);
		expect(lint.span().end).toBe(3);
		expect(lint.get_problem_text()).toBe('Teh');
		expect(lint.lint_kind()).toBe('Spelling');
		expect(lint.message()).toContain('the');
		expect(lint.suggestion_count()).toBe(1);
		expect(lint.suggestions()[0].get_replacement_text()).toBe('the');
	});

	test('counts spans in UTF-16 units, not characters', async () => {
		const linter = new PluginLinter({ plugin: bytes });
		const text = 'Teh 😀 cat cat sat.';
		const lints = await linter.lint(text, { language: 'plaintext' });
		const repeat = lints.find((l) => l.lint_kind() === 'Repetition');

		expect(repeat?.span().start).toBe(7);
		expect(text.slice(repeat?.span().start, repeat?.span().end)).toBe('cat cat');
	});

	test('applies a suggestion', async () => {
		const linter = new PluginLinter({ plugin: bytes });
		const text = 'Teh 😀 end';
		const [lint] = await linter.lint(text, { language: 'plaintext' });
		expect(await linter.applySuggestion(text, lint, lint.suggestions()[0])).toBe('the 😀 end');
	});

	test('groups lints by rule', async () => {
		const linter = new PluginLinter({ plugin: bytes });
		const groups = await linter.organizedLints('teh cat cat', { language: 'plaintext' });
		expect(Object.keys(groups).sort()).toEqual(['Repeat', 'Teh']);
	});

	test('can switch a rule off', async () => {
		const linter = new PluginLinter({ plugin: bytes });
		expect(await linter.getLintConfig()).toEqual({ Repeat: true, Teh: true });

		await linter.setLintConfig({ Repeat: false });
		const lints = await linter.lint('teh cat cat', { language: 'plaintext' });
		expect(lints.map((l) => l.lint_kind())).toEqual(['Spelling']);
	});

	test('takes user words', async () => {
		const linter = new PluginLinter({ plugin: bytes });
		await linter.importWords(['teh']);
		expect(await linter.lint('teh', { language: 'plaintext' })).toHaveLength(0);
		expect(await linter.exportWords()).toEqual(['teh']);

		await linter.clearWords();
		expect(await linter.lint('teh', { language: 'plaintext' })).toHaveLength(1);
	});

	test('can ignore a lint', async () => {
		const linter = new PluginLinter({ plugin: bytes });
		const text = 'teh cat';
		const [lint] = await linter.lint(text, { language: 'plaintext' });
		await linter.ignoreLint(text, lint);
		expect(await linter.lint(text, { language: 'plaintext' })).toHaveLength(0);

		const exported = await linter.exportIgnoredLints();
		const other = new PluginLinter({ plugin: bytes });
		await other.importIgnoredLints(exported);
		expect(await other.lint(text, { language: 'plaintext' })).toHaveLength(0);
	});

	test('says how sure it is about a text', async () => {
		const linter = new PluginLinter({ plugin: bytes });
		expect(await linter.detect('this is teh text')).toBeGreaterThan(0.5);
		expect(await linter.detect('nothing to see')).toBeNull();
	});

	test('rejects things that are not plugins', async () => {
		const notWasm = new PluginLinter({ plugin: new Uint8Array([1, 2, 3]) });
		await expect(notWasm.setup()).rejects.toThrow();

		// A valid but empty module: no Harper exports.
		const empty = new PluginLinter({
			plugin: new Uint8Array([0, 0x61, 0x73, 0x6d, 1, 0, 0, 0]),
		});
		await expect(empty.setup()).rejects.toThrow(/missing export/);
	});
});

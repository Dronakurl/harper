import type { StructuredLintConfig, StructuredLintSetting } from 'harper.js';
import { shuffle } from 'lodash-es';
import { expect, test } from 'vitest';
import State from './State';

function randomString(length: number): string {
	const chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz';
	let result = '';
	for (let i = 0; i < length; i++) {
		result += chars.charAt(Math.floor(Math.random() * chars.length));
	}
	return result;
}

/** Create an instance of the test class that doesn't use external persistence. */
function createEphemeralState(): State {
	return new State(
		(_) => Promise.resolve(),
		() => {},
		undefined,
	);
}

function collectStructuredRuleNames(config: StructuredLintConfig): string[] {
	const out: string[] = [];

	const visit = (setting: StructuredLintSetting) => {
		if ('Bool' in setting) {
			out.push(setting.Bool.name);
			return;
		}

		if ('OneOfMany' in setting) {
			out.push(...setting.OneOfMany.names);
			return;
		}

		for (const child of setting.Group.child.settings) {
			visit(child);
		}
	};

	for (const setting of config.settings) {
		visit(setting);
	}

	return out;
}

test('Toggling linting should change extension array.', () => {
	const state = createEphemeralState();

	const editorExtensions = state.getCMEditorExtensions();
	state.enableEditorLinter();

	expect(editorExtensions.length).toBe(1);

	state.disableEditorLinter();

	expect(editorExtensions.length).toBe(0);
});

test('Passing default settings back in should have a null net change.', async () => {
	const state = createEphemeralState();

	const initialSettings = await state.getSettings();
	await state.initializeFromSettings(initialSettings);
	const reinitSettings = await state.getSettings();

	expect(reinitSettings).toStrictEqual(initialSettings);
});

test('Default settings should have null linter configs', async () => {
	const state = createEphemeralState();

	const defaultSettings = await state.getSettings();

	const linterKeys = Object.keys(defaultSettings.lintSettings);

	expect(linterKeys.length).toBeGreaterThan(0);

	for (const key of linterKeys) {
		const setting = defaultSettings.lintSettings[key];
		expect(setting).toBeNull();
	}
});

test('Lint keys are not undefined', async () => {
	const state = createEphemeralState();

	const defaultSettings = await state.getSettings();

	expect(defaultSettings.lintSettings.ThisKeyDoesNotExist).toBeUndefined();
	expect(defaultSettings.lintSettings.RepeatedWords).toBeNull();
});

test('Lint keys can be enabled, then set to default.', async () => {
	const state = createEphemeralState();

	let settings = await state.getSettings();

	settings.lintSettings.RepeatedWords = true;
	await state.initializeFromSettings(settings);
	settings = await state.getSettings();
	expect(settings.lintSettings.RepeatedWords).toBe(true);

	settings.lintSettings.RepeatedWords = null;
	await state.initializeFromSettings(settings);
	settings = await state.getSettings();
	expect(settings.lintSettings.RepeatedWords).toBe(null);
});

test('Lint settings and descriptions have the same keys', async () => {
	const state = createEphemeralState();

	const settings = await state.getSettings();
	const descriptions = await state.getDescriptionHTML();

	const lintKeys = Object.keys(settings.lintSettings).sort();
	const descKeys = Object.keys(descriptions).sort();

	const missingInDescriptions = lintKeys.filter((k) => !descKeys.includes(k));
	const extraInDescriptions = descKeys.filter((k) => !lintKeys.includes(k));

	if (missingInDescriptions.length || extraInDescriptions.length) {
		// Print the diffs so CI/local run shows the exact keys
		console.error('Missing in descriptions (present in lintSettings):', missingInDescriptions);
		console.error('Extra in descriptions (not in lintSettings):', extraInDescriptions);
	}

	expect(missingInDescriptions.length).toBe(0);
	expect(extraInDescriptions.length).toBe(0);
});

test('Structured lint config and flat lint settings have the same keys', async () => {
	const state = createEphemeralState();

	const settings = await state.getSettings();
	const structured = await state.getStructuredLintConfig();

	const lintKeys = Object.keys(settings.lintSettings).sort();
	const structuredKeys = collectStructuredRuleNames(structured).sort();

	expect(structuredKeys).toStrictEqual(lintKeys);
});

test('Can be initialized with incomplete lint settings and retain default state.', async () => {
	const state = createEphemeralState();

	// Get the default settings
	const defaultSettings = await state.getSettings();

	// Pick just a few lint settings to keep.
	const numberToKeep = 5;
	const reducedLintSettings = Object.fromEntries(
		shuffle(Object.entries(defaultSettings.lintSettings)).slice(0, numberToKeep),
	);
	expect(Object.keys(reducedLintSettings).length).toBe(numberToKeep);

	await state.initializeFromSettings({ ...defaultSettings, lintSettings: reducedLintSettings });

	expect(await state.getSettings()).toStrictEqual(defaultSettings);
});

test('resetAllRulesToDefaults sets all overrides to null', async () => {
	const state = createEphemeralState();

	// Start with all enabled, then reset
	let settings = await state.getSettings();
	for (const key of Object.keys(settings.lintSettings)) {
		settings.lintSettings[key] = true;
	}
	await state.initializeFromSettings(settings);

	await state.resetAllRulesToDefaults();
	settings = await state.getSettings();
	for (const key of Object.keys(settings.lintSettings)) {
		expect(settings.lintSettings[key]).toBeNull();
	}
});

test('setAllRulesEnabled toggles all rules on and off', async () => {
	const state = createEphemeralState();

	await state.setAllRulesEnabled(true);
	let settings = await state.getSettings();
	for (const key of Object.keys(settings.lintSettings)) {
		expect(settings.lintSettings[key]).toBe(true);
	}

	await state.setAllRulesEnabled(false);
	settings = await state.getSettings();
	for (const key of Object.keys(settings.lintSettings)) {
		expect(settings.lintSettings[key]).toBe(false);
	}
});

test('getEffectiveLintConfig matches defaults after reset', async () => {
	const state = createEphemeralState();
	await state.resetAllRulesToDefaults();
	const effective = await state.getEffectiveLintConfig();
	const defaults = (await state.getDefaultLintConfig()) as Record<string, boolean>;
	expect(Object.keys(effective).sort()).toStrictEqual(Object.keys(defaults).sort());
	for (const k of Object.keys(defaults)) {
		expect(effective[k]).toBe(defaults[k]);
	}
});

test('getEffectiveLintConfig reflects explicit overrides', async () => {
	const state = createEphemeralState();
	const settings = await state.getSettings();
	for (const key of Object.keys(settings.lintSettings)) {
		settings.lintSettings[key] = true;
	}
	await state.initializeFromSettings(settings);
	const effective = await state.getEffectiveLintConfig();
	for (const k of Object.keys(effective)) {
		expect(effective[k]).toBe(true);
	}
});

test('can persist dictionary words in settings', async () => {
	const state = createEphemeralState();
	let settings = await state.getSettings();

	const testWord = 'ajhsbdajshdb';
	settings.userDictionary = [testWord];

	await state.initializeFromSettings(settings);

	settings = await state.getSettings();

	expect(settings.userDictionary).toStrictEqual([testWord]);
});

test('can persist dictionary order in settings', async () => {
	const state = createEphemeralState();
	let settings = await state.getSettings();

	const testDictionary: string[] = [];
	for (let i = 0; i < 200; i++) {
		testDictionary.push(randomString(10));
	}

	settings.userDictionary = testDictionary;
	await state.initializeFromSettings(settings);

	settings = await state.getSettings();

	const roundOne = settings.userDictionary;

	await state.initializeFromSettings(settings);
	settings = await state.getSettings();
	const roundTwo = settings.userDictionary;

	expect(roundOne).toStrictEqual(roundTwo);
});

test('can overwrite dictionary words in settings', async () => {
	const state = createEphemeralState();
	let settings = await state.getSettings();

	const testWord = 'ajhsbdajshdb';
	settings.userDictionary = [testWord];

	await state.initializeFromSettings(settings);
	settings = await state.getSettings();

	expect(settings.userDictionary).toStrictEqual([testWord]);

	settings.userDictionary = [];

	await state.initializeFromSettings(settings);
	settings = await state.getSettings();

	expect(settings.userDictionary).toStrictEqual([]);
});

test('A language without an installed plugin falls back to English.', async () => {
	const asked: string[] = [];
	const state = new State(
		(_) => Promise.resolve(),
		() => {},
		undefined,
		async (language) => {
			asked.push(language);
			return null;
		},
	);

	await state.initializeFromSettings({ useWebWorker: false, lintSettings: {}, language: 'de' });

	expect(asked).toContain('de');
	expect((await state.getSettings()).language).toBe('en');
});

test('English never asks for a plugin.', async () => {
	let asked = 0;
	const state = new State(
		(_) => Promise.resolve(),
		() => {},
		undefined,
		async () => {
			asked++;
			return null;
		},
	);

	await state.initializeFromSettings({ useWebWorker: false, lintSettings: {}, language: 'en' });

	expect(asked).toBe(0);
});

// The `echo` example plugin of harper-plugin-api (see `just test-obsidian`), as language `xx`.
async function echoLoader(): Promise<(language: string) => Promise<ArrayBuffer | null>> {
	const response = await fetch('/test-fixtures/echo.wasm');
	if (!response.ok) {
		throw new Error('test-fixtures/echo.wasm is missing; `just test-obsidian` builds it.');
	}
	const bytes = await response.arrayBuffer();
	return async (language) => (language === 'xx' ? bytes : null);
}

test('Each language keeps its own dictionary and rules; English is as it was.', async () => {
	const state = new State(
		(_) => Promise.resolve(),
		() => {},
		undefined,
		await echoLoader(),
	);
	await state.initializeFromSettings({
		useWebWorker: false,
		lintSettings: { SpellCheck: false },
		userDictionary: ['englishword'],
	});

	const english = await state.getSettings();
	english.language = 'xx';
	await state.initializeFromSettings(english);

	const plugin = await state.getSettings();
	expect(plugin.language).toBe('xx');
	expect(plugin.userDictionary).toStrictEqual([]);
	expect(plugin.lintSettings.SpellCheck).toBeUndefined();
	plugin.userDictionary = ['pluginword'];
	await state.initializeFromSettings(plugin);

	const back = await state.getSettings();
	back.language = 'en';
	await state.initializeFromSettings(back);

	const again = await state.getSettings();
	expect(again.language).toBe('en');
	expect(again.userDictionary).toStrictEqual(['englishword']);
	expect(again.lintSettings.SpellCheck).toBe(false);
	expect(again.otherLanguages?.xx.userDictionary).toStrictEqual(['pluginword']);
});

test('The plugin gets the words of its own language, and only those.', async () => {
	const state = new State(
		(_) => Promise.resolve(),
		() => {},
		undefined,
		await echoLoader(),
	);
	// `teh` is a word of the English dictionary only; the plugin flags it.
	await state.initializeFromSettings({
		useWebWorker: false,
		lintSettings: {},
		userDictionary: ['teh'],
	});

	const english = await state.getSettings();
	english.language = 'xx';
	await state.initializeFromSettings(english);
	expect(await state.getLinter().lint('teh', { language: 'plaintext' })).toHaveLength(1);

	const plugin = await state.getSettings();
	plugin.userDictionary = ['teh'];
	await state.initializeFromSettings(plugin);
	expect(await state.getLinter().lint('teh', { language: 'plaintext' })).toHaveLength(0);

	// Back in English the plugin's list is not applied, and the English one is back.
	const back = await state.getSettings();
	back.language = 'en';
	await state.initializeFromSettings(back);
	expect((await state.getSettings()).userDictionary).toStrictEqual(['teh']);
	expect((await state.getSettings()).otherLanguages?.xx.userDictionary).toStrictEqual(['teh']);
});

test('Settings from before language plugins are English.', async () => {
	const state = createEphemeralState();
	await state.initializeFromSettings({
		useWebWorker: false,
		lintSettings: { SpellCheck: false },
		userDictionary: ['word'],
	});
	const settings = await state.getSettings();
	expect(settings.userDictionary).toStrictEqual(['word']);
	expect(settings.lintSettings.SpellCheck).toBe(false);
	expect(settings.otherLanguages).toStrictEqual({});
});

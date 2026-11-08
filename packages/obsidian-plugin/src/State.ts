import type { Extension, StateField } from '@codemirror/state';
import type { Lint, LintConfig, Linter, StructuredLintConfig, Suggestion } from 'harper.js';
import { type Dialect, LocalLinter, PluginLinter, SuggestionKind, WorkerLinter } from 'harper.js';
import { slimBinaryInlined } from 'harper.js/slimBinaryInlined';
import { minimatch } from 'minimatch';
import type { MarkdownFileInfo } from 'obsidian';
import { type Action, linter } from './lint';
import { lintKindClass } from './lintKindColor';

export type Settings = {
	ignoredLints?: string;
	useWebWorker: boolean;
	dialect?: Dialect;
	lintSettings: LintConfig;
	userDictionary?: string[];
	delay?: number;
	ignoredGlobs?: string[];
	lintEnabled?: boolean;
	regexMask?: string;
	useWebStyleLints?: boolean;
	/** The language to check: `en` (built in, the default) or the id of a language plugin such as `de`. */
	language?: string;
	/**
	 * Whose `lintSettings`, `userDictionary` and `ignoredLints` the fields above are: the language
	 * that was being checked when the settings were saved. Missing in settings from before
	 * language plugins, which are all English.
	 */
	settingsLanguage?: string;
	/** Those three for every other language, so that each language keeps its own. */
	otherLanguages?: Record<string, LanguageSettings>;
};

/** What each language has of its own. */
export type LanguageSettings = {
	lintSettings: LintConfig;
	userDictionary?: string[];
	ignoredLints?: string;
};

/**
 * `settings` with the dictionary, rules and ignored lints of `language` in the top-level fields,
 * and those of the language they held until now put aside in `otherLanguages`. Settings that
 * already hold `language` (always, for someone who only uses English) are returned as they are.
 */
export function swapLanguageSettings(settings: Settings, language: string): Settings {
	const owner = settings.settingsLanguage ?? 'en';
	if (owner === language) {
		return { ...settings, settingsLanguage: language };
	}

	const others = { ...(settings.otherLanguages ?? {}) };
	others[owner] = {
		lintSettings: settings.lintSettings,
		userDictionary: settings.userDictionary,
		ignoredLints: settings.ignoredLints,
	};
	const mine = others[language] ?? { lintSettings: {}, userDictionary: [] };
	delete others[language];

	return {
		...settings,
		lintSettings: { ...mine.lintSettings },
		userDictionary: mine.userDictionary ?? [],
		ignoredLints: mine.ignoredLints,
		settingsLanguage: language,
		otherLanguages: others,
	};
}

/** Fetches the bytes of the plugin for a language, or `null` if it is not installed. */
export type PluginLoader = (language: string) => Promise<ArrayBuffer | null>;

const DEFAULT_DELAY = -1;

/** The centralized state for the entire Obsidian plugin.
 * Since it also contains most business logic, for testing purpose it should not interact with Obsidian directly.*/
export default class State {
	private harper: Linter;
	private saveData: (data: any) => Promise<void>;
	private delay: number;
	private onExtensionChange: () => void;
	private ignoredGlobs?: string[];
	private editorInfoField?: StateField<MarkdownFileInfo>;
	private lintEnabled?: boolean;
	private regexMask?: string;
	private useWebStyleLints = false;
	private language = 'en';
	private wantsWebWorker = true;
	private dialect?: Dialect;
	private loadPlugin?: PluginLoader;
	private otherLanguages: Record<string, LanguageSettings> = {};

	/** The CodeMirror extension objects that should be inserted by the host. */
	private editorExtensions: Extension[];

	/** @param saveDataCallback A callback which will be used to save data on disk.
	 * @param onExtensionChange A callback this class will run when the extension array is modified.
	 * @param editorViewField Needed to provide support for ignoring files based on path.*/
	constructor(
		saveDataCallback: (data: any) => Promise<void>,
		onExtensionChange: () => void,
		_editorInfoField?: StateField<MarkdownFileInfo>,
		loadPlugin?: PluginLoader,
	) {
		this.loadPlugin = loadPlugin;
		this.harper = new WorkerLinter({ binary: slimBinaryInlined });
		this.delay = DEFAULT_DELAY;
		this.saveData = saveDataCallback;
		this.onExtensionChange = onExtensionChange;
		this.editorExtensions = [];

		this.editorInfoField = _editorInfoField;
	}

	public async initializeFromSettings(settings: Settings | null) {
		if (settings == null) {
			settings = {
				useWebWorker: true,
				lintEnabled: true,
				lintSettings: {},
			};
		}

		this.dialect = settings.dialect;
		const language = await this.resolveLanguage(settings.language);
		settings = swapLanguageSettings(settings, language);
		this.otherLanguages = settings.otherLanguages ?? {};
		const languageChanged = language !== this.language;
		if (languageChanged) {
			await this.harper.dispose();
			this.harper = await this.createLinter(language, settings);
			this.language = language;
			this.wantsWebWorker = settings.useWebWorker;
		}

		const defaultConfig = await this.harper.getDefaultLintConfig();
		for (const key of Object.keys(defaultConfig)) {
			if (settings.lintSettings[key] == undefined) {
				settings.lintSettings[key] = null;
			}
		}

		const oldSettings = await this.getSettings();

		if (
			!languageChanged &&
			language === 'en' &&
			(settings.useWebWorker !== oldSettings.useWebWorker ||
				settings.dialect !== oldSettings.dialect)
		) {
			this.wantsWebWorker = settings.useWebWorker;
			if (settings.useWebWorker) {
				this.harper.dispose();
				this.harper = new WorkerLinter({ binary: slimBinaryInlined, dialect: settings.dialect });
			} else {
				this.harper.dispose();
				this.harper = new LocalLinter({ binary: slimBinaryInlined, dialect: settings.dialect });
			}
		} else {
			await this.harper.clearIgnoredLints();
		}

		if (settings.ignoredLints !== undefined) {
			await this.harper.importIgnoredLints(settings.ignoredLints);
		}

		if (settings.userDictionary != null) {
			await this.harper.clearWords();
			if (settings.userDictionary.length > 0) {
				await this.harper.importWords(settings.userDictionary);
			}
		}

		await this.harper.setLintConfig(settings.lintSettings);
		this.harper.setup();

		this.delay = settings.delay ?? DEFAULT_DELAY;
		this.ignoredGlobs = settings.ignoredGlobs;
		this.lintEnabled = settings.lintEnabled;
		this.regexMask = settings.regexMask;
		this.useWebStyleLints = settings.useWebStyleLints ?? false;

		// Reinitialize it.
		if (this.hasEditorLinter()) {
			this.disableEditorLinter(false);
			this.enableEditorLinter(false);
		}

		await this.saveData(settings);
	}

	/** The language that will really be used: `requested`, unless its plugin is not installed. */
	private async resolveLanguage(requested?: string): Promise<string> {
		if (requested == null || requested === 'en') {
			return 'en';
		}
		if (this.loadPlugin == null || (await this.loadPlugin(requested)) == null) {
			console.warn(`Harper: no plugin for language '${requested}' is installed; using English.`);
			return 'en';
		}
		return requested;
	}

	private async createLinter(language: string, settings: Settings): Promise<Linter> {
		if (language !== 'en' && this.loadPlugin != null) {
			const bytes = await this.loadPlugin(language);
			if (bytes != null) {
				return new PluginLinter({ plugin: bytes });
			}
		}
		return settings.useWebWorker
			? new WorkerLinter({ binary: slimBinaryInlined, dialect: settings.dialect })
			: new LocalLinter({ binary: slimBinaryInlined, dialect: settings.dialect });
	}

	/** Construct the linter plugin that actually shows the errors. */
	private constructEditorLinter(): Extension {
		return linter(
			async (view) => {
				const ignoredGlobs = this.ignoredGlobs ?? [];

				if (this.editorInfoField != null) {
					const mdView = view.state.field(this.editorInfoField, false);
					const file = mdView?.file;

					if (file != null) {
						const path = file.path;
						for (const glob of ignoredGlobs) {
							if (minimatch(path, glob)) {
								return [];
							}
						}
					}
				}

				const text = view.state.doc.sliceString(-1);
				const lints = await this.harper.organizedLints(text, { regex_mask: this.regexMask });

				return Object.entries(lints).flatMap(([linterName, lints]) =>
					lints.map((lint) => {
						const span = lint.span();

						const actions: Action[] = lint.suggestions().map((sug): Action => {
							return {
								kind: 'suggestion' as const,
								name:
									sug.kind() == SuggestionKind.Replace
										? sug.get_replacement_text()
										: suggestionToLabel(sug),
								title: suggestionToLabel(sug),
								apply: (view, from, to) => {
									if (sug.kind() === SuggestionKind.Remove) {
										view.dispatch({
											changes: {
												from,
												to,
												insert: '',
											},
											selection: {
												anchor: from,
											},
										});
									} else if (sug.kind() === SuggestionKind.Replace) {
										const replacement = sug.get_replacement_text();
										view.dispatch({
											changes: {
												from,
												to,
												insert: replacement,
											},
											selection: {
												anchor: from + replacement.length,
											},
										});
									} else if (sug.kind() === SuggestionKind.InsertAfter) {
										const replacement = sug.get_replacement_text();
										view.dispatch({
											changes: {
												from: to,
												to,
												insert: replacement,
											},
											selection: {
												anchor: to + replacement.length,
											},
										});
									}
								},
							};
						});

						if (lint.lint_kind() === 'Spelling') {
							const word = lint.get_problem_text();

							actions.push({
								kind: 'dictionary',
								name: '📖',
								title: `Add “${word}” to your dictionary`,
								apply: (view, _from, to) => {
									view.dispatch({
										selection: {
											anchor: to,
										},
									});
									this.harper.importWords([word]);
									this.reinitialize();
								},
							});
						}

						return {
							from: span.start,
							to: span.end,
							source: linterName,
							severity: 'error',
							markClass: `${lintKindClass(lint.lint_kind())} ${this.useWebStyleLints ? 'harper-web-style' : 'harper-squiggly-style'}`,
							title: lint.lint_kind_pretty(),
							message: lint.message(),
							renderMessage: (_view) => {
								const node = document.createElement('template');
								node.innerHTML = lint.message_html();
								return node.content;
							},
							ignore: async () => {
								await this.ignoreLints(text, [lint]);
							},
							disable: async () => {
								const lintConfig = await this.harper.getLintConfig();
								lintConfig[linterName] = false;
								await this.harper.setLintConfig(lintConfig);

								await this.reinitialize();
							},

							actions,
						};
					}),
				);
			},
			{
				delay: this.delay,
			},
		);
	}

	/** Use this method instead of interacting with the linter directly. */
	public async ignoreLints(text: string, lints: Lint[]) {
		for (const lint of lints) {
			await this.harper.ignoreLint(text, lint);
		}

		await this.reinitialize();
	}

	public async reinitialize() {
		const settings = await this.getSettings();
		await this.initializeFromSettings(settings);
	}

	public async getSettings(): Promise<Settings> {
		const usingWebWorker =
			this.language === 'en' ? this.harper instanceof WorkerLinter : this.wantsWebWorker;

		const userDictionary = await this.harper.exportWords();
		userDictionary.sort();

		return {
			ignoredLints: await this.harper.exportIgnoredLints(),
			useWebWorker: usingWebWorker,
			lintSettings: await this.harper.getLintConfig(),
			userDictionary,
			// A plugin language has dialects of its own; keep the English choice for when it is back.
			dialect: this.language === 'en' ? await this.harper.getDialect() : this.dialect,
			delay: this.delay,
			ignoredGlobs: this.ignoredGlobs,
			lintEnabled: this.lintEnabled,
			regexMask: this.regexMask,
			useWebStyleLints: this.useWebStyleLints,
			language: this.language,
			settingsLanguage: this.language,
			otherLanguages: this.otherLanguages,
		};
	}

	/**
	 * Reset all lint rule overrides back to their defaults (null).
	 * Persists and reinitializes state to apply changes.
	 */
	public async resetAllRulesToDefaults(): Promise<void> {
		const settings = await this.getSettings();
		for (const key of Object.keys(settings.lintSettings)) {
			settings.lintSettings[key] = null;
		}
		await this.initializeFromSettings(settings);
	}

	/**
	 * Enable or disable all lint rules in bulk by setting explicit values.
	 * This overrides individual rule settings until changed again.
	 */
	public async setAllRulesEnabled(enabled: boolean): Promise<void> {
		const settings = await this.getSettings();
		for (const key of Object.keys(settings.lintSettings)) {
			settings.lintSettings[key] = enabled;
		}
		await this.initializeFromSettings(settings);
	}

	public async getDescriptionHTML(): Promise<Record<string, string>> {
		return await this.harper.getLintDescriptionsHTML();
	}

	/** Expose the structured lint configuration for UI rendering only.
	 * Rule state must still be persisted through flat `lintSettings`. */
	public async getStructuredLintConfig(): Promise<StructuredLintConfig> {
		return await this.harper.getStructuredLintConfig();
	}

	/** Expose the default lint configuration for UI rendering. */
	public async getDefaultLintConfig(): Promise<LintConfig> {
		return await this.harper.getDefaultLintConfig();
	}

	/** Effective config: merges defaults with overrides (null/undefined uses default). */
	public async getEffectiveLintConfig(): Promise<Record<string, boolean>> {
		const defaults = (await this.getDefaultLintConfig()) as Record<string, boolean>;
		const overrides = (await this.getSettings()).lintSettings as Record<
			string,
			boolean | null | undefined
		>;
		const effective: Record<string, boolean> = {};
		for (const key of Object.keys(defaults)) {
			const v = overrides[key];
			effective[key] = v === null || v === undefined ? defaults[key] : Boolean(v);
		}
		return effective;
	}

	/** Determine if any rules are effectively enabled, considering defaults. */
	public async areAnyRulesEnabled(): Promise<boolean> {
		const settings = await this.getSettings();
		const defaults = await this.getDefaultLintConfig();
		for (const key of Object.keys(settings.lintSettings)) {
			const v = settings.lintSettings[key] as boolean | null | undefined;
			const def = (defaults as Record<string, boolean | undefined>)[key];
			const effective = v === null || v === undefined ? def : v;
			if (effective) return true;
		}
		return false;
	}

	/** Get a reference to the CM editor extensions.
	 * Do not mutate the returned value, except via methods on this class. */
	public getCMEditorExtensions(): Extension[] {
		return this.editorExtensions;
	}

	/** Enables the editor linter by adding an extension to the editor extensions array. */
	public enableEditorLinter(reinit = true) {
		if (!this.hasEditorLinter()) {
			this.editorExtensions.push(this.constructEditorLinter());
			this.lintEnabled = true;
			this.onExtensionChange();
			if (reinit) this.reinitialize();
			console.log('Enabled');
		}
	}

	/** Disables the editor linter by removing the extension from the editor extensions array. */
	public disableEditorLinter(reinit = true) {
		while (this.hasEditorLinter()) {
			this.editorExtensions.pop();
		}
		this.lintEnabled = false;
		this.onExtensionChange();
		if (reinit) this.reinitialize();
		console.log('Disabled');
	}

	public hasEditorLinter(): boolean {
		return this.editorExtensions.length !== 0;
	}

	public toggleAutoLint() {
		if (this.hasEditorLinter()) {
			this.disableEditorLinter();
		} else {
			this.enableEditorLinter();
		}
	}

	/** Get a reference to the current linter.
	 * It's best not to hold on to this type and to instead use this function again if another reference is needed. */
	public getLinter(): Linter {
		return this.harper;
	}
}

function suggestionToLabel(sug: Suggestion): string {
	switch (sug.kind()) {
		case SuggestionKind.Remove:
			return 'Remove';
		case SuggestionKind.Replace:
			return `Replace with “${sug.get_replacement_text()}”`;
		case SuggestionKind.InsertAfter:
			return `Insert “${sug.get_replacement_text()}” after this.`;
	}
}

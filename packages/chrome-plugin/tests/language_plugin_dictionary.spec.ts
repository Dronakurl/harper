import { existsSync } from 'node:fs';
import path from 'node:path';
import type { BrowserContext, Page } from '@playwright/test';
import { expect, test } from './fixtures';
import {
	assertPageHasNHighlights,
	getBackground,
	getExtensionId,
	getTextarea,
	replaceEditorContent,
} from './testUtils';

// The `echo` example plugin of harper-plugin-api, language `xx`: it flags "teh" (see `just
// test-chrome-plugin`). Without it these tests are skipped.
const PLUGIN =
	process.env.HARPER_TEST_ECHO_PLUGIN ??
	path.join(import.meta.dirname, 'plugin-fixtures/echo.wasm');
const TEST_PAGE_URL = 'http://localhost:8081/simple_textarea.html';

async function stored(context: BrowserContext, key: string): Promise<unknown> {
	const background = await getBackground(context);
	return background.evaluate(async (k) => (await chrome.storage.local.get({ [k]: null }))[k], key);
}

/** No highlight can also mean "not linted yet", so give the linter time before looking. */
async function assertNoHighlights(page: Page) {
	await page.waitForTimeout(6000);
	await assertPageHasNHighlights(page, 0);
}

async function addWord(context: BrowserContext, page: Page, word: string) {
	await page.goto(`chrome-extension://${await getExtensionId(context)}/options.html`);
	await page.evaluate(
		(w) => chrome.runtime.sendMessage({ kind: 'addToUserDictionary', words: [w] }),
		word,
	);
}

async function setLanguage(context: BrowserContext, page: Page, language: string) {
	await page.goto(`chrome-extension://${await getExtensionId(context)}/options.html`);
	await page
		.locator('select')
		.filter({ has: page.locator(`option[value="${language}"]`) })
		.first()
		.selectOption(language);
	await expect.poll(() => stored(context, 'language'), { timeout: 60_000 }).toBe(language);
}

test.describe('user dictionary of a language plugin', () => {
	test.skip(({ browserName }) => browserName === 'firefox', 'Uses the background service worker.');
	test.skip(!existsSync(PLUGIN), `no language plugin at ${PLUGIN}`);
	test.setTimeout(240_000);

	test('a word is saved for the active language only, and the plugin knows it', async ({
		context,
		page,
	}) => {
		await page.goto(`chrome-extension://${await getExtensionId(context)}/options.html`);
		await page.setInputFiles('#language-plugin-file', PLUGIN);
		await expect.poll(() => stored(context, 'language'), { timeout: 120_000 }).toBe('xx');

		// The plugin flags `teh` ...
		await page.goto(TEST_PAGE_URL);
		await replaceEditorContent(getTextarea(page), 'teh cat');
		await assertPageHasNHighlights(page, 1);

		// ... until the user adds it, which is saved under the language's own key.
		await addWord(context, page, 'teh');
		await expect.poll(() => stored(context, 'userDictionary:xx')).toEqual(['teh']);
		expect((await stored(context, 'userDictionary')) ?? []).toEqual([]);

		await page.goto(TEST_PAGE_URL);
		await replaceEditorContent(getTextarea(page), 'teh cat');
		await assertNoHighlights(page);

		// English keeps a list of its own, and does not get the plugin's words.
		await setLanguage(context, page, 'en');
		await addWord(context, page, 'englishword');
		await expect.poll(() => stored(context, 'userDictionary')).toEqual(['englishword']);
		expect(await stored(context, 'userDictionary:xx')).toEqual(['teh']);

		// Back in the plugin language the stored words are loaded into the plugin again.
		await setLanguage(context, page, 'xx');
		await page.goto(TEST_PAGE_URL);
		await replaceEditorContent(getTextarea(page), 'teh cat');
		await assertNoHighlights(page);
	});
});

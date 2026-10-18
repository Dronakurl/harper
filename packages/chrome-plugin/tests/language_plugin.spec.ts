import { existsSync } from 'node:fs';
import path from 'node:path';
import type { BrowserContext, Page } from '@playwright/test';
import { expect, test } from './fixtures';
import {
	assertEditorText,
	clickHarperHighlight,
	getBackground,
	getExtensionId,
	getTextarea,
	replaceEditorContent,
} from './testUtils';

// A language plugin to install, e.g. harper-german's `harper_german.wasm` (see `just
// test-chrome-plugin`). Without it these tests are skipped.
const PLUGIN =
	process.env.HARPER_TEST_LANGUAGE_PLUGIN ??
	path.join(import.meta.dirname, 'plugin-fixtures/harper-de.wasm');
const TEST_PAGE_URL = 'http://localhost:8081/simple_textarea.html';

async function storedLanguage(context: BrowserContext): Promise<string> {
	const background = await getBackground(context);
	return background.evaluate(
		async () => (await chrome.storage.local.get({ language: 'en' })).language,
	);
}

async function openOptions(context: BrowserContext, page: Page) {
	await page.goto(`chrome-extension://${await getExtensionId(context)}/options.html`);
}

test.describe('language plugins', () => {
	test.skip(({ browserName }) => browserName === 'firefox', 'Uses the background service worker.');
	test.skip(!existsSync(PLUGIN), `no language plugin at ${PLUGIN}`);
	test.setTimeout(180_000);

	test('a plugin added in the options checks its language, and English comes back unchanged', async ({
		context,
		page,
	}) => {
		await openOptions(context, page);
		await page.setInputFiles('#language-plugin-file', PLUGIN);
		await expect.poll(() => storedLanguage(context), { timeout: 120_000 }).toBe('de');

		await page.goto(TEST_PAGE_URL);
		const editor = getTextarea(page);
		await replaceEditorContent(editor, 'Ich habe das Buch gelesent.');
		expect(await clickHarperHighlight(page)).toBe(true);
		await page.getByTitle('Click to replace "gelesent" with "gelesen"').click();
		await assertEditorText(editor, 'Ich habe das Buch gelesen.');

		// Back to English, chosen in the options like any language.
		await openOptions(context, page);
		await page
			.locator('select')
			.filter({ has: page.locator('option[value="en"]') })
			.first()
			.selectOption('en');
		await expect.poll(() => storedLanguage(context), { timeout: 60_000 }).toBe('en');

		await page.goto(TEST_PAGE_URL);
		await replaceEditorContent(getTextarea(page), 'This is an test');
		expect(await clickHarperHighlight(page)).toBe(true);
		await page.getByTitle('Click to replace "an" with "a"').click();
		await assertEditorText(getTextarea(page), 'This is a test');
	});
});

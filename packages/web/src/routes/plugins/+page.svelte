<script lang="ts">
import { Textarea } from 'components';
import { type Lint, PluginLinter, type PluginInfo } from 'harper.js';

let linter: PluginLinter | null = null;
let info: PluginInfo | null = null;
let error = '';
let loading = false;

let url = '';
let text = '';
let lints: Lint[] = [];

async function use(plugin: string | Uint8Array) {
	loading = true;
	error = '';
	try {
		const next = new PluginLinter({ plugin });
		info = await next.info();
		linter = next;
		await run();
	} catch (e) {
		linter = null;
		info = null;
		error = e instanceof Error ? e.message : String(e);
	} finally {
		loading = false;
	}
}

async function onFile(e: Event) {
	const file = (e.target as HTMLInputElement).files?.[0];
	if (file != null) {
		await use(new Uint8Array(await file.arrayBuffer()));
	}
}

async function run() {
	lints = linter == null ? [] : await linter.lint(text, { language: 'plaintext' });
}

$: text, run();
</script>

<div class="[&>*]:mt-8 p-4 dark:[&>*]:text-white">
	<h1 class="text-2xl">Language Plugin Demo</h1>
	<p>
		Harper's languages other than English are WebAssembly plugins, such as
		<code>harper-de.wasm</code>. Load one from a URL or from a file; it runs entirely in your
		browser, and the text never leaves it.
	</p>

	<div class="flex gap-2 items-center">
		<input
			class="border rounded p-1 grow dark:bg-gray-800"
			placeholder="https://…/harper-de.wasm"
			bind:value={url}
		/>
		<button class="border rounded px-3 py-1" disabled={url === '' || loading} on:click={() => use(url)}>
			Load
		</button>
	</div>
	<div><input type="file" accept=".wasm" on:change={onFile} /></div>

	{#if error}
		<p class="text-red-600">{error}</p>
	{/if}
	{#if info}
		<p>Loaded <strong>{info.name}</strong> ({info.id}) {info.plugin_version}.</p>
	{/if}

	<Textarea rows={8} bind:value={text} placeholder="Text in the plugin's language" disabled={linter == null} />

	<ul class="list-disc pl-6">
		{#each lints as lint}
			<li>
				<code>{lint.get_problem_text()}</code> – {lint.message()}
				{#if lint.suggestion_count() > 0}
					→ {lint
						.suggestions()
						.map((s) => s.get_replacement_text() || '(remove)')
						.join(', ')}
				{/if}
			</li>
		{/each}
	</ul>
</div>

/**
 * Language plugins (`harper-<language>.wasm`) installed in this browser.
 *
 * They are kept in IndexedDB, which both the options page and the background worker can reach
 * (they share the extension's origin), so the options page can store a file the user picked
 * without sending tens of megabytes through a message. Extensions may not download code, so a
 * plugin gets here only because the user chose the file.
 */

const DB_NAME = 'harper-language-plugins';
const STORE = 'plugins';

export type InstalledPlugin = {
	/** The language id, as the plugin reports it: `de`. */
	id: string;
	/** The name for people: `Deutsch`. */
	name: string;
	version: string;
	/** Size in bytes. */
	size: number;
	installedAt: string;
};

type StoredPlugin = InstalledPlugin & { bytes: ArrayBuffer };

function open(): Promise<IDBDatabase> {
	return new Promise((resolve, reject) => {
		const request = indexedDB.open(DB_NAME, 1);
		request.onupgradeneeded = () => {
			request.result.createObjectStore(STORE, { keyPath: 'id' });
		};
		request.onsuccess = () => resolve(request.result);
		request.onerror = () => reject(request.error);
	});
}

async function run<T>(mode: IDBTransactionMode, work: (store: IDBObjectStore) => IDBRequest<T>) {
	const db = await open();
	try {
		return await new Promise<T>((resolve, reject) => {
			const request = work(db.transaction(STORE, mode).objectStore(STORE));
			request.onsuccess = () => resolve(request.result);
			request.onerror = () => reject(request.error);
		});
	} finally {
		db.close();
	}
}

/** Keep a plugin, replacing an older one for the same language. */
export async function savePlugin(
	info: Omit<InstalledPlugin, 'size' | 'installedAt'>,
	bytes: ArrayBuffer,
): Promise<void> {
	const record: StoredPlugin = {
		...info,
		size: bytes.byteLength,
		installedAt: new Date().toISOString(),
		bytes,
	};
	await run('readwrite', (store) => store.put(record));
}

export async function removePlugin(id: string): Promise<void> {
	await run('readwrite', (store) => store.delete(id));
}

/** The installed plugins, without their bytes. */
export async function listPlugins(): Promise<InstalledPlugin[]> {
	const records = await run('readonly', (store) => store.getAll() as IDBRequest<StoredPlugin[]>);
	return records
		.map(({ bytes: _bytes, ...info }) => info)
		.sort((a, b) => a.name.localeCompare(b.name));
}

/** The bytes of the plugin for `id`, or `null` if none is installed. */
export async function getPluginBytes(id: string): Promise<ArrayBuffer | null> {
	const record = await run('readonly', (store) => store.get(id) as IDBRequest<StoredPlugin | undefined>);
	return record?.bytes ?? null;
}

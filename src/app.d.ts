// See https://svelte.dev/docs/kit/types#app.d.ts
// for information about these interfaces
declare global {
	namespace App {
		// interface Error {}
		// interface Locals {}
		// interface PageData {}
		// interface PageState {}
		// interface Platform {}
	}

	// File System Access API (Chromium only) — not in TypeScript's DOM lib.
	interface SaveFilePickerOptions {
		suggestedName?: string;
		types?: { description?: string; accept: Record<string, string[]> }[];
	}

	interface Window {
		showSaveFilePicker?: (options?: SaveFilePickerOptions) => Promise<FileSystemFileHandle>;
	}
}

export {};

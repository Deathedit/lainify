import { sanitizeSettings, type Settings } from './schema';

const KEY = 'lainify:settings';

function load(): Settings {
	try {
		const raw = localStorage.getItem(KEY);
		return sanitizeSettings(raw ? JSON.parse(raw) : null);
	} catch {
		return sanitizeSettings(null);
	}
}

export const settings: Settings = $state(load());

$effect.root(() => {
	$effect(() => {
		const snapshot = JSON.stringify($state.snapshot(settings));
		try {
			localStorage.setItem(KEY, snapshot);
		} catch {
			// Private mode / storage disabled — settings just won't persist.
		}
	});
});

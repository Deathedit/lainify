export type ShortcutAction =
	| 'fullscreen'
	| 'mute'
	| 'volumeUp'
	| 'volumeDown'
	| 'screenshot'
	| 'record'
	| 'settings'
	| 'close';

export const SHORTCUTS: { keys: string[]; label: string }[] = [
	{ keys: ['F'], label: 'Fullscreen' },
	{ keys: ['M'], label: 'Mute' },
	{ keys: ['↑', '↓'], label: 'Volume' },
	{ keys: ['S'], label: 'Screenshot' },
	{ keys: ['R'], label: 'Start / stop recording' },
	{ keys: [','], label: 'Settings' },
	{ keys: ['Esc'], label: 'Close panel' }
];

const KEYS: Record<string, ShortcutAction> = {
	f: 'fullscreen',
	m: 'mute',
	arrowup: 'volumeUp',
	arrowdown: 'volumeDown',
	s: 'screenshot',
	r: 'record',
	',': 'settings',
	escape: 'close'
};

type KeyEventLike = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'altKey' | 'repeat'> & {
	target: EventTarget | null;
};

function isEditable(target: EventTarget | null): boolean {
	const el = target as { tagName?: string; isContentEditable?: boolean } | null;
	return !!el && (/^(INPUT|SELECT|TEXTAREA)$/.test(el.tagName ?? '') || !!el.isContentEditable);
}

export function shortcutFor(e: KeyEventLike): ShortcutAction | null {
	if (e.ctrlKey || e.metaKey || e.altKey) return null;
	const action = KEYS[e.key.toLowerCase()] ?? null;
	if (!action) return null;
	// Form controls keep their own keys (arrows on a slider, typing in a select).
	if (isEditable(e.target) && action !== 'close') return null;
	// Holding a key should only repeat volume changes.
	if (e.repeat && action !== 'volumeUp' && action !== 'volumeDown') return null;
	return action;
}

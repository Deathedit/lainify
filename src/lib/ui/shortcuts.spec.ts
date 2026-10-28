import { describe, expect, it } from 'vitest';
import { shortcutFor } from './shortcuts';

const key = (k: string, extra: Partial<Parameters<typeof shortcutFor>[0]> = {}) => ({
	key: k,
	ctrlKey: false,
	metaKey: false,
	altKey: false,
	repeat: false,
	target: null,
	...extra
});

describe('shortcutFor', () => {
	it('maps keys case-insensitively', () => {
		expect(shortcutFor(key('f'))).toBe('fullscreen');
		expect(shortcutFor(key('F'))).toBe('fullscreen');
		expect(shortcutFor(key('ArrowUp'))).toBe('volumeUp');
		expect(shortcutFor(key(','))).toBe('settings');
		expect(shortcutFor(key('Escape'))).toBe('close');
		expect(shortcutFor(key('x'))).toBeNull();
	});

	it('leaves browser shortcuts alone', () => {
		expect(shortcutFor(key('r', { ctrlKey: true }))).toBeNull();
		expect(shortcutFor(key('f', { metaKey: true }))).toBeNull();
	});

	it('ignores keys typed into form controls, except Escape', () => {
		const target = { tagName: 'INPUT' } as unknown as EventTarget;
		expect(shortcutFor(key('ArrowUp', { target }))).toBeNull();
		expect(shortcutFor(key('m', { target }))).toBeNull();
		expect(shortcutFor(key('Escape', { target }))).toBe('close');
	});

	it('only repeats volume changes', () => {
		expect(shortcutFor(key('ArrowDown', { repeat: true }))).toBe('volumeDown');
		expect(shortcutFor(key('r', { repeat: true }))).toBeNull();
	});
});

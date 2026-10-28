import { describe, expect, it } from 'vitest';
import { cleanLabel, formatBytes, formatDuration, formatFps, timestampedName } from './format';

describe('formatDuration', () => {
	it('formats minutes and seconds', () => {
		expect(formatDuration(0)).toBe('00:00');
		expect(formatDuration(42_900)).toBe('00:42');
		expect(formatDuration(61_000)).toBe('01:01');
	});

	it('adds hours once needed', () => {
		expect(formatDuration(3_723_000)).toBe('1:02:03');
	});

	it('clamps negative input', () => {
		expect(formatDuration(-5)).toBe('00:00');
	});
});

describe('timestampedName', () => {
	it('builds a sortable local-time filename', () => {
		expect(timestampedName('lainify', 'png', new Date(2026, 9, 28, 4, 5, 6))).toBe(
			'lainify-20261028-040506.png'
		);
	});
});

describe('formatFps', () => {
	it('drops needless decimals', () => {
		expect(formatFps(60)).toBe('60');
		expect(formatFps(59.94006)).toBe('59.94');
	});
});

describe('formatBytes', () => {
	it('scales units', () => {
		expect(formatBytes(512)).toBe('512 B');
		expect(formatBytes(12_345_678)).toBe('12.3 MB');
		expect(formatBytes(2_500_000_000)).toBe('2.5 GB');
	});
});

describe('cleanLabel', () => {
	it('strips the USB id browsers append', () => {
		expect(cleanLabel('Elgato HD60 X (0fd9:0082)')).toBe('Elgato HD60 X');
	});

	it('collapses V4L2 "name: name" labels', () => {
		expect(cleanLabel('Elgato HD60 X: Elgato HD60 X (0fd9:0082)')).toBe('Elgato HD60 X');
	});

	it('leaves normal labels alone', () => {
		expect(cleanLabel('Integrated Camera')).toBe('Integrated Camera');
	});
});

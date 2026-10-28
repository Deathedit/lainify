import { describe, expect, it } from 'vitest';
import { PRESETS, findMode, modeLabel, supportedModes } from './modes';

describe('supportedModes', () => {
	it('returns every preset when capabilities are unknown', () => {
		expect(supportedModes(undefined)).toEqual(PRESETS);
		expect(supportedModes(null)).toEqual(PRESETS);
	});

	it('filters by reported maxima', () => {
		const ids = supportedModes({
			width: { min: 640, max: 1920 },
			height: { min: 480, max: 1080 },
			frameRate: { min: 1, max: 60 }
		}).map((m) => m.id);
		expect(ids).toEqual(['1080p60', '1080p50', '1080p30', '720p60', '720p30', '480p60', '480p30']);
	});

	it('accepts 59.94 fps devices for 60 fps presets', () => {
		const ids = supportedModes({
			width: { max: 1280 },
			height: { max: 720 },
			frameRate: { max: 59.94 }
		}).map((m) => m.id);
		expect(ids).toContain('720p60');
	});

	it('drops frame rates the device cannot reach', () => {
		const ids = supportedModes({
			width: { max: 3840 },
			height: { max: 2160 },
			frameRate: { max: 30 }
		}).map((m) => m.id);
		expect(ids).not.toContain('2160p60');
		expect(ids).not.toContain('1080p50');
		expect(ids).toContain('2160p30');
	});
});

describe('findMode / modeLabel', () => {
	it('round-trips preset ids', () => {
		expect(findMode('1440p60')).toMatchObject({ width: 2560, height: 1440, frameRate: 60 });
		expect(findMode('auto')).toBeNull();
	});

	it('labels modes', () => {
		expect(modeLabel(findMode('1080p60'))).toBe('1080p · 60 fps');
		expect(modeLabel(null)).toBe('Auto');
	});
});

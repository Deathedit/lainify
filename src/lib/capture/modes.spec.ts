import { describe, expect, it } from 'vitest';
import {
	FRAME_RATES,
	RESOLUTIONS,
	findResolution,
	modeLabel,
	supportedFrameRates,
	supportedResolutions,
	toMode
} from './modes';

describe('supported options', () => {
	it('returns everything when capabilities are unknown', () => {
		expect(supportedResolutions(undefined)).toEqual(RESOLUTIONS);
		expect(supportedResolutions(null)).toEqual(RESOLUTIONS);
		expect(supportedFrameRates(null)).toEqual(FRAME_RATES);
	});

	it('filters resolutions by reported maxima', () => {
		const heights = supportedResolutions({
			width: { min: 640, max: 1920 },
			height: { min: 480, max: 1080 }
		}).map((r) => r.height);
		expect(heights).toEqual([1080, 720, 480]);
	});

	it('accepts 59.94 fps devices for 60 fps', () => {
		expect(supportedFrameRates({ frameRate: { max: 59.94 } })).toContain(60);
	});

	it('drops frame rates the device cannot reach', () => {
		expect(supportedFrameRates({ frameRate: { max: 30 } })).toEqual([30]);
	});
});

describe('toMode / modeLabel', () => {
	it('is null when both axes are automatic', () => {
		expect(toMode(null, null)).toBeNull();
	});

	it('ignores unknown values', () => {
		expect(toMode(999, 7)).toBeNull();
	});

	it('keeps each axis independently', () => {
		expect(toMode(1440, null)).toEqual({ resolution: findResolution(1440), frameRate: null });
		expect(toMode(null, 30)).toEqual({ resolution: null, frameRate: 30 });
	});

	it('labels modes', () => {
		expect(modeLabel(toMode(1080, 60))).toBe('1080p · 60 fps');
		expect(modeLabel(toMode(2160, null))).toBe('2160p');
		expect(modeLabel(toMode(null, 50))).toBe('50 fps');
		expect(modeLabel(null)).toBe('Auto');
	});
});

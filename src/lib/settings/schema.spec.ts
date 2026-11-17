import { describe, expect, it } from 'vitest';
import { DEFAULT_SETTINGS, sanitizeSettings } from './schema';

describe('sanitizeSettings', () => {
	it('returns defaults for missing or garbage input', () => {
		expect(sanitizeSettings(null)).toEqual(DEFAULT_SETTINGS);
		expect(sanitizeSettings('nope')).toEqual(DEFAULT_SETTINGS);
	});

	it('keeps valid values', () => {
		const s = sanitizeSettings({
			videoId: 'cam',
			audioId: 'none',
			resolution: 1080,
			frameRate: 60,
			volume: 0.4,
			muted: true,
			bitrate: 20_000_000,
			hideDelay: 0,
			recordTarget: 'file'
		});
		expect(s).toEqual({
			videoId: 'cam',
			audioId: 'none',
			resolution: 1080,
			frameRate: 60,
			volume: 0.4,
			muted: true,
			bitrate: 20_000_000,
			hideDelay: 0,
			recordTarget: 'file'
		});
	});

	it('clamps numbers and drops wrong types', () => {
		const s = sanitizeSettings({
			volume: 9,
			bitrate: 'fast',
			muted: 'yes',
			recordTarget: 'cloud',
			videoId: 3
		});
		expect(s.volume).toBe(1.5);
		expect(s.bitrate).toBe(DEFAULT_SETTINGS.bitrate);
		expect(s.muted).toBe(false);
		expect(s.recordTarget).toBe('download');
		expect(s.videoId).toBeNull();
	});

	it('migrates the old combined mode id', () => {
		expect(sanitizeSettings({ modeId: '1440p30' })).toMatchObject({
			resolution: 1440,
			frameRate: 30
		});
		expect(sanitizeSettings({ modeId: 'auto' })).toMatchObject({
			resolution: null,
			frameRate: null
		});
	});

	it('drops unknown resolutions and frame rates', () => {
		expect(sanitizeSettings({ resolution: 1234, frameRate: 144 })).toMatchObject({
			resolution: null,
			frameRate: null
		});
	});
});

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
			modeId: '1080p60',
			volume: 0.4,
			muted: true,
			bitrate: 20_000_000,
			hideDelay: 0,
			recordTarget: 'file'
		});
		expect(s).toEqual({
			videoId: 'cam',
			audioId: 'none',
			modeId: '1080p60',
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
});

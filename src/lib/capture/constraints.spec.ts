import { describe, expect, it } from 'vitest';
import { audioConstraints, videoConstraints } from './constraints';
import { toMode } from './modes';

describe('audioConstraints', () => {
	it('disables voice processing and asks for stereo 48 kHz', () => {
		expect(audioConstraints('mic-1')).toMatchObject({
			deviceId: { exact: 'mic-1' },
			echoCancellation: false,
			noiseSuppression: false,
			autoGainControl: false,
			channelCount: { ideal: 2 },
			sampleRate: { ideal: 48_000 }
		});
	});
});

describe('videoConstraints', () => {
	it('prefers 1080p60 without a mode', () => {
		expect(videoConstraints('cam', null)).toEqual({
			deviceId: { exact: 'cam' },
			width: { ideal: 1920 },
			height: { ideal: 1080 },
			frameRate: { ideal: 60 }
		});
	});

	it('requests the exact native mode when strict', () => {
		const c = videoConstraints(undefined, toMode(720, 60));
		expect(c).not.toHaveProperty('deviceId');
		expect(c).toMatchObject({
			width: { exact: 1280 },
			height: { exact: 720 },
			resizeMode: 'none'
		});
		expect((c.frameRate as ConstrainDoubleRange).min).toBeLessThan(60);
	});

	it('only expresses preferences when relaxed', () => {
		expect(videoConstraints('cam', toMode(1080, 30), false)).toEqual({
			deviceId: { exact: 'cam' },
			width: { ideal: 1920 },
			height: { ideal: 1080 },
			frameRate: { ideal: 30 }
		});
	});

	it('pins only the chosen axis and prefers defaults for the other', () => {
		const c = videoConstraints(undefined, toMode(2160, null));
		expect(c).toMatchObject({
			width: { exact: 3840 },
			height: { exact: 2160 },
			frameRate: { ideal: 60 },
			resizeMode: 'none'
		});
		expect(videoConstraints(undefined, toMode(null, 30))).toMatchObject({
			width: { ideal: 1920 },
			height: { ideal: 1080 },
			frameRate: { min: 28.5, ideal: 30 },
			resizeMode: 'none'
		});
	});
});

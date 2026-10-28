import { describe, expect, it } from 'vitest';
import { audioConstraints, videoConstraints } from './constraints';
import { findMode } from './modes';

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
		const c = videoConstraints(undefined, findMode('720p60'));
		expect(c).not.toHaveProperty('deviceId');
		expect(c).toMatchObject({
			width: { exact: 1280 },
			height: { exact: 720 },
			resizeMode: 'none'
		});
		expect((c.frameRate as ConstrainDoubleRange).min).toBeLessThan(60);
	});

	it('only expresses preferences when relaxed', () => {
		expect(videoConstraints('cam', findMode('1080p30'), false)).toEqual({
			deviceId: { exact: 'cam' },
			width: { ideal: 1920 },
			height: { ideal: 1080 },
			frameRate: { ideal: 30 }
		});
	});
});

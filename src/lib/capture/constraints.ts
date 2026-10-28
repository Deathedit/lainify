import { FPS_TOLERANCE, type CaptureMode } from './modes';

// Not yet in TypeScript's DOM lib.
type ExtendedConstraints = MediaTrackConstraints & {
	latency?: ConstrainDouble;
	resizeMode?: ConstrainDOMString;
};

/**
 * Capture cards deliver line-level game/console audio: every voice-call
 * processing stage would only damage it, and mono downmix loses the stereo image.
 */
export function audioConstraints(deviceId: string): MediaTrackConstraints {
	const c: ExtendedConstraints = {
		deviceId: { exact: deviceId },
		echoCancellation: false,
		noiseSuppression: false,
		autoGainControl: false,
		channelCount: { ideal: 2 },
		sampleRate: { ideal: 48_000 },
		latency: { ideal: 0 }
	};
	return c;
}

/**
 * `strict` asks for the exact native mode (no browser-side scaling or frame
 * dropping); the relaxed variant is the fallback when the device refuses it.
 * With no mode we ask for 1080p60, the sweet spot for most capture cards.
 */
export function videoConstraints(
	deviceId: string | undefined,
	mode: CaptureMode | null,
	strict = true
): MediaTrackConstraints {
	const c: ExtendedConstraints = {};
	if (deviceId) c.deviceId = { exact: deviceId };

	if (!mode) {
		c.width = { ideal: 1920 };
		c.height = { ideal: 1080 };
		c.frameRate = { ideal: 60 };
		return c;
	}

	if (strict) {
		c.width = { exact: mode.width };
		c.height = { exact: mode.height };
		c.frameRate = { min: mode.frameRate * FPS_TOLERANCE, ideal: mode.frameRate };
		c.resizeMode = 'none';
	} else {
		c.width = { ideal: mode.width };
		c.height = { ideal: mode.height };
		c.frameRate = { ideal: mode.frameRate };
	}
	return c;
}

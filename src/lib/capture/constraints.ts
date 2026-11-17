import { FPS_TOLERANCE, PREFERRED, type CaptureMode } from './modes';

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
 * Axes left on auto prefer 1080p / 60 fps, the sweet spot for most capture cards.
 */
export function videoConstraints(
	deviceId: string | undefined,
	mode: CaptureMode | null,
	strict = true
): MediaTrackConstraints {
	const c: ExtendedConstraints = {};
	if (deviceId) c.deviceId = { exact: deviceId };

	const resolution = mode?.resolution;
	const frameRate = mode?.frameRate;
	const exact = strict && !!mode;

	c.width =
		resolution && exact
			? { exact: resolution.width }
			: { ideal: resolution?.width ?? PREFERRED.width };
	c.height =
		resolution && exact
			? { exact: resolution.height }
			: { ideal: resolution?.height ?? PREFERRED.height };
	c.frameRate =
		frameRate && exact
			? { min: frameRate * FPS_TOLERANCE, ideal: frameRate }
			: { ideal: frameRate ?? PREFERRED.frameRate };
	if (exact) c.resizeMode = 'none';
	return c;
}

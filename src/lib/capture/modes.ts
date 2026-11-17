export interface Resolution {
	width: number;
	height: number;
}

/** `null` on either axis means "let the device pick" (we still prefer 1080p / 60 fps). */
export interface CaptureMode {
	resolution: Resolution | null;
	frameRate: number | null;
}

export const RESOLUTIONS: Resolution[] = [
	{ width: 3840, height: 2160 },
	{ width: 2560, height: 1440 },
	{ width: 1920, height: 1080 },
	{ width: 1280, height: 720 },
	{ width: 640, height: 480 }
];

export const FRAME_RATES = [60, 50, 30];

export const PREFERRED: Resolution & { frameRate: number } = {
	width: 1920,
	height: 1080,
	frameRate: 60
};

/** Select value for "Auto" on either axis. */
export const AUTO = 'auto';

/** Tolerance so a 60 fps choice matches devices that report 59.94. */
export const FPS_TOLERANCE = 0.95;

export function findResolution(height: number | null): Resolution | null {
	return RESOLUTIONS.find((r) => r.height === height) ?? null;
}

export function findFrameRate(fps: number | null): number | null {
	return FRAME_RATES.find((f) => f === fps) ?? null;
}

/** Build a mode from stored settings; `null` when both axes are automatic. */
export function toMode(height: number | null, frameRate: number | null): CaptureMode | null {
	const mode = { resolution: findResolution(height), frameRate: findFrameRate(frameRate) };
	return mode.resolution || mode.frameRate ? mode : null;
}

export function modeLabel(mode: CaptureMode | null): string {
	const parts = [
		mode?.resolution && `${mode.resolution.height}p`,
		mode?.frameRate && `${mode.frameRate} fps`
	].filter(Boolean);
	return parts.length ? parts.join(' · ') : 'Auto';
}

type SizeCapabilities = Pick<MediaTrackCapabilities, 'width' | 'height' | 'frameRate'>;

const fits = (max: number | undefined, value: number) => max === undefined || max >= value;

/**
 * Options within the device's reported capabilities. Browsers only expose
 * per-axis maxima (not the discrete format list), so a listed combination can
 * still be rejected when applied — callers handle that case.
 */
export function supportedResolutions(caps?: SizeCapabilities | null): Resolution[] {
	return RESOLUTIONS.filter(
		(r) => fits(caps?.width?.max, r.width) && fits(caps?.height?.max, r.height)
	);
}

export function supportedFrameRates(caps?: SizeCapabilities | null): number[] {
	return FRAME_RATES.filter((f) => fits(caps?.frameRate?.max, f * FPS_TOLERANCE));
}

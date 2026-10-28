export interface CaptureMode {
	id: string;
	width: number;
	height: number;
	frameRate: number;
}

const preset = (height: number, width: number, frameRate: number): CaptureMode => ({
	id: `${height}p${frameRate}`,
	width,
	height,
	frameRate
});

export const PRESETS: CaptureMode[] = [
	preset(2160, 3840, 60),
	preset(2160, 3840, 30),
	preset(1440, 2560, 60),
	preset(1440, 2560, 30),
	preset(1080, 1920, 60),
	preset(1080, 1920, 50),
	preset(1080, 1920, 30),
	preset(720, 1280, 60),
	preset(720, 1280, 30),
	preset(480, 640, 60),
	preset(480, 640, 30)
];

export const AUTO_MODE_ID = 'auto';

/** Tolerance so a 60 fps preset matches devices that report 59.94. */
export const FPS_TOLERANCE = 0.95;

export function findMode(id: string): CaptureMode | null {
	return PRESETS.find((m) => m.id === id) ?? null;
}

export function modeLabel(mode: CaptureMode | null): string {
	return mode ? `${mode.height}p · ${mode.frameRate} fps` : 'Auto';
}

type SizeCapabilities = Pick<MediaTrackCapabilities, 'width' | 'height' | 'frameRate'>;

/**
 * Presets that fit within the device's reported capabilities. Browsers only expose
 * per-axis maxima (not the discrete format list), so a listed mode can still be
 * rejected when applied — callers handle that case.
 */
export function supportedModes(caps?: SizeCapabilities | null): CaptureMode[] {
	if (!caps) return PRESETS;
	const fits = (max: number | undefined, value: number) => max === undefined || max >= value;
	return PRESETS.filter(
		(m) =>
			fits(caps.width?.max, m.width) &&
			fits(caps.height?.max, m.height) &&
			fits(caps.frameRate?.max, m.frameRate * FPS_TOLERANCE)
	);
}

export type CaptureErrorKind =
	'unsupported' | 'denied' | 'not-found' | 'busy' | 'constraints' | 'ended' | 'unknown';

export interface CaptureError {
	kind: CaptureErrorKind;
	title: string;
	message: string;
}

const MESSAGES: Record<CaptureErrorKind, Omit<CaptureError, 'kind'>> = {
	unsupported: {
		title: 'Capture not available',
		message: 'This browser can’t access capture devices here. Open the app over https or localhost.'
	},
	denied: {
		title: 'Access blocked',
		message:
			'Camera access was denied. Allow it in the site settings (the icon in the address bar), then retry.'
	},
	'not-found': {
		title: 'No capture device',
		message: 'Plug in your capture card or webcam, then retry.'
	},
	busy: {
		title: 'Device is busy',
		message:
			'Another app (OBS, Discord, another tab…) is using the device, or it failed to start. Close it and retry.'
	},
	constraints: {
		title: 'Mode not supported',
		message: 'The device can’t deliver the selected resolution / frame rate. Pick another mode.'
	},
	ended: {
		title: 'Device disconnected',
		message:
			'The capture device stopped sending video. It will resume automatically when reconnected.'
	},
	unknown: {
		title: 'Something went wrong',
		message: 'The capture device could not be opened.'
	}
};

export function captureError(kind: CaptureErrorKind, detail?: string): CaptureError {
	const base = MESSAGES[kind];
	return {
		kind,
		title: base.title,
		message: detail ? `${base.message} (${detail})` : base.message
	};
}

export function classifyError(err: unknown): CaptureError {
	const name = (err as { name?: string } | null)?.name;
	switch (name) {
		case 'NotAllowedError':
		case 'SecurityError':
			return captureError('denied');
		case 'NotFoundError':
			return captureError('not-found');
		case 'NotReadableError':
		case 'AbortError':
		case 'TrackStartError':
			return captureError('busy');
		case 'OverconstrainedError':
			return captureError('constraints');
		case 'TypeError':
			return captureError('unsupported');
		default:
			return captureError('unknown', err instanceof Error ? err.message : undefined);
	}
}

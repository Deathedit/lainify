import { cleanLabel } from '../utils/format';

type DeviceLike = Pick<MediaDeviceInfo, 'deviceId' | 'label' | 'groupId'>;

const CAPTURE_HINT =
	/elgato|hd60|4k60|cam ?link|game ?capture|avermedia|live gamer|magewell|ezcap|ripsaw|capture|hdmi|usb3\.0 video|ms2109/i;

/** Words that appear in many unrelated device labels and say nothing about identity. */
const GENERIC = new Set([
	'usb',
	'video',
	'audio',
	'camera',
	'webcam',
	'capture',
	'device',
	'input',
	'analog',
	'digital',
	'stereo',
	'mono',
	'microphone',
	'mic',
	'default',
	'communications',
	'hd',
	'pro',
	'the',
	'pci',
	'hda',
	'built',
	'in',
	'internal',
	'controller',
	'multichannel',
	'surround',
	'iec958'
]);

const PSEUDO_AUDIO = new Set(['default', 'communications']);

export function usbId(label: string): string | null {
	return label.match(/\(([0-9a-f]{4}:[0-9a-f]{4})\)/i)?.[1].toLowerCase() ?? null;
}

function tokens(label: string): Set<string> {
	return new Set(
		cleanLabel(label)
			.toLowerCase()
			.split(/[^a-z0-9]+/)
			.filter((t) => t.length >= 2 && !GENERIC.has(t))
	);
}

export function isCaptureLike(label: string): boolean {
	return CAPTURE_HINT.test(label);
}

/** Saved device → something that looks like a capture card → the one granted in the prompt → first. */
export function pickVideoDevice(
	devices: DeviceLike[],
	preferred?: string | null,
	granted?: string
): string | undefined {
	const has = (id?: string | null) => !!id && devices.some((d) => d.deviceId === id);
	if (has(preferred)) return preferred!;
	const capture = devices.find((d) => isCaptureLike(d.label));
	if (capture) return capture.deviceId;
	if (has(granted)) return granted;
	return devices[0]?.deviceId;
}

/**
 * Find the audio interface that belongs to the same physical device as `video`.
 * Returns undefined rather than guessing — falling back to a microphone would
 * loop the room back into the speakers.
 */
export function matchAudioDevice(
	video: DeviceLike | undefined,
	audios: DeviceLike[]
): string | undefined {
	if (!video) return undefined;
	const candidates = audios.filter((a) => !PSEUDO_AUDIO.has(a.deviceId));

	const id = usbId(video.label);
	if (id) {
		const byUsb = candidates.find((a) => usbId(a.label) === id);
		if (byUsb) return byUsb.deviceId;
	}

	if (video.groupId) {
		const byGroup = candidates.find((a) => a.groupId === video.groupId);
		if (byGroup) return byGroup.deviceId;
	}

	const wanted = tokens(video.label);
	let best: { id: string; score: number } | undefined;
	for (const a of candidates) {
		let score = 0;
		for (const t of tokens(a.label)) if (wanted.has(t)) score++;
		if (score > 0 && (!best || score > best.score)) best = { id: a.deviceId, score };
	}
	return best?.id;
}

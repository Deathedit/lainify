import { AUTO_MODE_ID } from '../capture/modes';

export type RecordTarget = 'download' | 'file';

export interface Settings {
	/** Last used video input. */
	videoId: string | null;
	/** `null` = match the video device automatically, `'none'` = no audio. */
	audioId: string | null;
	modeId: string;
	/** 0…1.5 */
	volume: number;
	muted: boolean;
	/** Video bits per second for recordings. */
	bitrate: number;
	/** Hide controls after this many ms without input; 0 = never. */
	hideDelay: number;
	recordTarget: RecordTarget;
}

export const NO_AUDIO = 'none';
export const MAX_VOLUME = 1.5;

export const DEFAULT_SETTINGS: Settings = {
	videoId: null,
	audioId: null,
	modeId: AUTO_MODE_ID,
	volume: 1,
	muted: false,
	bitrate: 12_000_000,
	hideDelay: 2500,
	recordTarget: 'download'
};

/** Merge untrusted persisted data onto the defaults, dropping anything malformed. */
export function sanitizeSettings(raw: unknown): Settings {
	const out: Settings = { ...DEFAULT_SETTINGS };
	if (!raw || typeof raw !== 'object') return out;
	const r = raw as Record<string, unknown>;

	const idOrNull = (v: unknown) => (v === null || typeof v === 'string' ? v : undefined);
	const num = (v: unknown, min: number, max: number) =>
		typeof v === 'number' && Number.isFinite(v) ? Math.min(max, Math.max(min, v)) : undefined;

	out.videoId = idOrNull(r.videoId) ?? out.videoId;
	out.audioId = idOrNull(r.audioId) ?? out.audioId;
	if (typeof r.modeId === 'string') out.modeId = r.modeId;
	out.volume = num(r.volume, 0, MAX_VOLUME) ?? out.volume;
	if (typeof r.muted === 'boolean') out.muted = r.muted;
	out.bitrate = num(r.bitrate, 1_000_000, 100_000_000) ?? out.bitrate;
	out.hideDelay = num(r.hideDelay, 0, 60_000) ?? out.hideDelay;
	if (r.recordTarget === 'download' || r.recordTarget === 'file') out.recordTarget = r.recordTarget;
	return out;
}

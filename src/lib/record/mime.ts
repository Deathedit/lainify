/**
 * Preference order: MP4/H.264 plays everywhere; builds without an H.264 encoder
 * (Chromium forks, Firefox) get VP9 — in MP4 when supported, else WebM.
 * Codecs are always spelled out so the settings panel can name them.
 */
export const MIME_CANDIDATES = [
	'video/mp4;codecs=avc1.640033,mp4a.40.2',
	'video/mp4;codecs=avc1,mp4a.40.2',
	'video/mp4;codecs=vp9,opus',
	'video/webm;codecs=vp9,opus',
	'video/webm;codecs=vp8,opus',
	'video/webm',
	'video/mp4'
];

export function pickMimeType(isSupported: (type: string) => boolean): string | null {
	return MIME_CANDIDATES.find((t) => isSupported(t)) ?? null;
}

export function containerOf(mimeType: string): 'mp4' | 'webm' {
	return mimeType.startsWith('video/mp4') ? 'mp4' : 'webm';
}

/** "video/mp4;codecs=avc1.640033,mp4a.40.2" → "MP4 · H.264 + AAC" */
export function describeMime(mimeType: string | null): string {
	if (!mimeType) return 'Not supported';
	const container = containerOf(mimeType).toUpperCase();
	const codecs = mimeType.match(/codecs=(.+)$/)?.[1] ?? '';
	const names = codecs
		.split(',')
		.map((c) => {
			if (c.startsWith('avc1')) return 'H.264';
			if (c.startsWith('mp4a')) return 'AAC';
			if (c === 'vp9') return 'VP9';
			if (c === 'vp8') return 'VP8';
			if (c === 'opus') return 'Opus';
			return '';
		})
		.filter(Boolean);
	return names.length ? `${container} · ${names.join(' + ')}` : container;
}

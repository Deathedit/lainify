import { describe, expect, it } from 'vitest';
import { MIME_CANDIDATES, containerOf, describeMime, pickMimeType } from './mime';

describe('pickMimeType', () => {
	it('prefers MP4 / H.264 when available', () => {
		expect(pickMimeType(() => true)).toBe(MIME_CANDIDATES[0]);
	});

	it('uses VP9 in MP4 when H.264 is unavailable (Chromium without proprietary codecs)', () => {
		expect(pickMimeType((t) => !t.includes('avc1'))).toBe('video/mp4;codecs=vp9,opus');
	});

	it('falls back to WebM (e.g. Firefox)', () => {
		expect(pickMimeType((t) => t.startsWith('video/webm'))).toBe('video/webm;codecs=vp9,opus');
	});

	it('returns null when nothing is supported', () => {
		expect(pickMimeType(() => false)).toBeNull();
	});
});

describe('containerOf / describeMime', () => {
	it('derives the file extension', () => {
		expect(containerOf('video/mp4;codecs=avc1,mp4a.40.2')).toBe('mp4');
		expect(containerOf('video/webm')).toBe('webm');
	});

	it('describes codecs for humans', () => {
		expect(describeMime('video/mp4;codecs=avc1.640033,mp4a.40.2')).toBe('MP4 · H.264 + AAC');
		expect(describeMime('video/webm;codecs=vp9,opus')).toBe('WEBM · VP9 + Opus');
		expect(describeMime('video/mp4;codecs=vp9,opus')).toBe('MP4 · VP9 + Opus');
		expect(describeMime('video/webm')).toBe('WEBM');
		expect(describeMime(null)).toBe('Not supported');
	});
});

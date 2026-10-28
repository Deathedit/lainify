import { audioConstraints, videoConstraints } from './constraints';
import { captureError, classifyError, type CaptureError } from './errors';
import type { CaptureMode } from './modes';

export type SessionStatus = 'idle' | 'starting' | 'live' | 'error';

export interface StartOptions {
	videoId?: string;
	audioId?: string;
	mode: CaptureMode | null;
}

export interface StartResult {
	ok: boolean;
	/** The audio device failed to open; the session runs video-only. */
	audioDropped: boolean;
	/** The exact mode was refused; the closest one the device offered is used. */
	modeRelaxed: boolean;
}

const nameOf = (err: unknown) => (err as { name?: string } | null)?.name;

export class CaptureSession {
	stream = $state.raw<MediaStream | null>(null);
	status = $state<SessionStatus>('idle');
	error = $state.raw<CaptureError | null>(null);
	video = $state.raw<MediaTrackSettings | null>(null);
	audio = $state.raw<MediaTrackSettings | null>(null);
	capabilities = $state.raw<MediaTrackCapabilities | null>(null);
	label = $state('');

	/** Called when the device disappears mid-session (unplugged, driver reset). */
	onended?: () => void;

	#generation = 0;

	get videoTrack(): MediaStreamTrack | null {
		return this.stream?.getVideoTracks()[0] ?? null;
	}

	async start(opts: StartOptions): Promise<StartResult> {
		const gen = ++this.#generation;
		this.#release();
		this.status = 'starting';
		this.error = null;

		const result: StartResult = { ok: false, audioDropped: false, modeRelaxed: false };
		try {
			const stream = await this.#open(opts, result);
			if (gen !== this.#generation) {
				stream.getTracks().forEach((t) => t.stop());
				return result;
			}
			this.#adopt(stream);
			result.ok = true;
		} catch (err) {
			if (gen === this.#generation) this.fail(err);
		}
		return result;
	}

	stop(): void {
		this.#generation++;
		this.#release();
		this.status = 'idle';
		this.error = null;
	}

	fail(err: unknown): void {
		this.#release();
		this.status = 'error';
		this.error = classifyError(err);
	}

	/** Switch resolution / frame rate on the running track without reopening the device. */
	async applyMode(mode: CaptureMode | null): Promise<void> {
		const track = this.videoTrack;
		if (!track) return;
		try {
			await track.applyConstraints(videoConstraints(undefined, mode));
		} finally {
			this.#readTracks();
		}
	}

	async #open({ videoId, audioId, mode }: StartOptions, result: StartResult): Promise<MediaStream> {
		const md = navigator.mediaDevices;
		if (!md?.getUserMedia) throw new TypeError('getUserMedia unavailable');

		const audio = audioId ? audioConstraints(audioId) : false;
		const open = (strict: boolean, withAudio: boolean) =>
			md.getUserMedia({
				video: videoConstraints(videoId, mode, strict),
				audio: withAudio ? audio : false
			});

		try {
			return await open(true, !!audio);
		} catch (err) {
			if (nameOf(err) === 'NotAllowedError') throw err;
			if (mode && nameOf(err) === 'OverconstrainedError') {
				result.modeRelaxed = true;
				return this.#withAudioFallback(open, false, !!audio, result);
			}
			if (!audio) throw err;
			// Audio is the usual culprit (busy, unplugged); keep the picture.
			result.audioDropped = true;
			return open(true, false);
		}
	}

	async #withAudioFallback(
		open: (strict: boolean, withAudio: boolean) => Promise<MediaStream>,
		strict: boolean,
		withAudio: boolean,
		result: StartResult
	): Promise<MediaStream> {
		try {
			return await open(strict, withAudio);
		} catch (err) {
			if (!withAudio || nameOf(err) === 'NotAllowedError') throw err;
			result.audioDropped = true;
			return open(strict, false);
		}
	}

	#adopt(stream: MediaStream): void {
		this.stream = stream;
		this.status = 'live';
		this.#readTracks();
		stream.getVideoTracks()[0]?.addEventListener('ended', () => {
			if (this.stream !== stream) return;
			this.#generation++;
			this.#release();
			this.status = 'error';
			this.error = captureError('ended');
			this.onended?.();
		});
	}

	#readTracks(): void {
		const video = this.stream?.getVideoTracks()[0];
		const audio = this.stream?.getAudioTracks()[0];
		this.video = video?.getSettings() ?? null;
		this.audio = audio?.getSettings() ?? null;
		this.capabilities = video?.getCapabilities?.() ?? null;
		this.label = video?.label ?? '';
	}

	#release(): void {
		this.stream?.getTracks().forEach((t) => t.stop());
		this.stream = null;
		this.video = null;
		this.audio = null;
		this.capabilities = null;
		this.label = '';
	}
}

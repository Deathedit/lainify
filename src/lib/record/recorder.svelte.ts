import { downloadBlob } from '../utils/download';
import { timestampedName } from '../utils/format';
import { containerOf, pickMimeType } from './mime';

export type RecorderState = 'idle' | 'recording' | 'saving';

export interface SavedRecording {
	filename: string;
	bytes: number;
}

export interface RecordOptions {
	bitrate: number;
	/** Stream straight to a file picked by the user instead of buffering in memory. */
	toFile: boolean;
}

interface Sink {
	write(chunk: Blob): Promise<void>;
	close(): Promise<void>;
}

export const canSaveToFile = () => typeof window !== 'undefined' && !!window.showSaveFilePicker;

function memorySink(filename: string, mimeType: string): Sink {
	const chunks: Blob[] = [];
	return {
		write: async (chunk) => void chunks.push(chunk),
		close: async () => downloadBlob(new Blob(chunks, { type: mimeType.split(';')[0] }), filename)
	};
}

async function fileSink(filename: string, mimeType: string): Promise<Sink> {
	const ext = containerOf(mimeType);
	const handle = await window.showSaveFilePicker!({
		suggestedName: filename,
		types: [
			{ description: `${ext.toUpperCase()} video`, accept: { [`video/${ext}`]: [`.${ext}`] } }
		]
	});
	const writable = await handle.createWritable();
	return {
		write: (chunk) => writable.write(chunk),
		close: () => writable.close()
	};
}

export class Recorder {
	state = $state<RecorderState>('idle');
	/** Milliseconds since recording started. */
	elapsed = $state(0);
	readonly mimeType =
		typeof MediaRecorder === 'undefined'
			? null
			: pickMimeType((t) => MediaRecorder.isTypeSupported(t));

	onsaved?: (info: SavedRecording) => void;
	onerror?: (message: string) => void;

	#recorder: MediaRecorder | null = null;
	#timer: ReturnType<typeof setInterval> | undefined;

	get supported(): boolean {
		return this.mimeType !== null;
	}

	async start(stream: MediaStream, { bitrate, toFile }: RecordOptions): Promise<void> {
		const mimeType = this.mimeType;
		if (this.state !== 'idle' || !mimeType) return;

		const filename = timestampedName('lainify', containerOf(mimeType));
		let sink: Sink;
		if (toFile && canSaveToFile()) {
			try {
				sink = await fileSink(filename, mimeType);
			} catch (err) {
				if ((err as Error).name === 'AbortError') return; // picker dismissed
				throw err;
			}
		} else {
			sink = memorySink(filename, mimeType);
		}

		const recorder = new MediaRecorder(stream, {
			mimeType,
			videoBitsPerSecond: bitrate,
			audioBitsPerSecond: 192_000
		});

		let bytes = 0;
		let pending = Promise.resolve();
		recorder.ondataavailable = (e) => {
			if (!e.data.size) return;
			bytes += e.data.size;
			pending = pending.then(() => sink.write(e.data));
		};
		recorder.onerror = () => this.onerror?.('Recording failed — the file may be incomplete.');
		recorder.onstop = async () => {
			clearInterval(this.#timer);
			this.state = 'saving';
			try {
				await pending;
				await sink.close();
				this.onsaved?.({ filename, bytes });
			} catch (err) {
				this.onerror?.(`Could not save recording: ${(err as Error).message}`);
			} finally {
				this.#recorder = null;
				this.state = 'idle';
				this.elapsed = 0;
			}
		};

		recorder.start(1000);
		this.#recorder = recorder;
		this.state = 'recording';
		const startedAt = performance.now();
		this.elapsed = 0;
		this.#timer = setInterval(() => (this.elapsed = performance.now() - startedAt), 250);
	}

	stop(): void {
		if (this.#recorder?.state === 'recording') this.#recorder.stop();
	}
}

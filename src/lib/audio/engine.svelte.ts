import { peakOf, peakToMeter } from './meter';

/** Per-frame peak-hold decay for the meter (≈ 20 dB/s at 60 fps). */
const DECAY = 0.94;

/**
 * Plays the capture audio through Web Audio rather than the <video> element:
 * lower latency, gain above 100 %, and a level meter tap.
 *
 *   source → input (upmix to stereo) ─┬→ output gain → speakers
 *                                     └→ splitter → analyser L / R
 */
export class AudioEngine {
	levels = $state.raw<[number, number]>([0, 0]);
	/** False while the browser's autoplay policy keeps the context suspended. */
	running = $state(false);
	/** Output latency in seconds, when the browser reports it. */
	latency = $state<number | null>(null);

	#ctx: AudioContext | null = null;
	#input: GainNode | null = null;
	#output: GainNode | null = null;
	#analysers: AnalyserNode[] = [];
	#source: MediaStreamAudioSourceNode | null = null;
	#buffer = new Float32Array(1024);
	#peaks = [0, 0];
	#raf = 0;
	#gain = 1;

	/** Create / resume the context inside a user gesture so audio can start unblocked. */
	prime = (): void => {
		this.#context();
		this.resume();
	};

	resume = (): void => {
		const ctx = this.#ctx;
		if (!ctx || ctx.state === 'running' || ctx.state === 'closed') return;
		ctx.resume().then(this.#sync, () => {});
	};

	attach(stream: MediaStream | null): void {
		this.#source?.disconnect();
		this.#source = null;

		const track = stream?.getAudioTracks()[0];
		if (!track) {
			this.#stopMeter();
			return;
		}

		const mediaStream = new MediaStream([track]);
		try {
			this.#source = new MediaStreamAudioSourceNode(this.#context(), { mediaStream });
		} catch {
			// Firefox can't connect a stream whose sample rate differs from the context's.
			this.#source = new MediaStreamAudioSourceNode(this.#context(track.getSettings().sampleRate), {
				mediaStream
			});
		}
		this.#source.connect(this.#input!);
		this.resume();
		this.#startMeter();
	}

	setVolume(volume: number, muted: boolean): void {
		this.#gain = muted ? 0 : volume;
		const ctx = this.#ctx;
		if (ctx && this.#output) this.#output.gain.setTargetAtTime(this.#gain, ctx.currentTime, 0.015);
	}

	destroy(): void {
		this.#stopMeter();
		this.#source?.disconnect();
		this.#source = null;
		void this.#ctx?.close();
		this.#ctx = null;
	}

	#context(sampleRate?: number): AudioContext {
		if (this.#ctx && (sampleRate === undefined || this.#ctx.sampleRate === sampleRate)) {
			return this.#ctx;
		}
		void this.#ctx?.close();

		const ctx = new AudioContext({ latencyHint: 'interactive', sampleRate });
		const input = new GainNode(ctx, {
			channelCount: 2,
			channelCountMode: 'explicit',
			channelInterpretation: 'speakers'
		});
		const output = new GainNode(ctx, { gain: this.#gain });
		const splitter = new ChannelSplitterNode(ctx, { numberOfOutputs: 2 });
		this.#analysers = [0, 1].map(() => new AnalyserNode(ctx, { fftSize: this.#buffer.length }));

		input.connect(output).connect(ctx.destination);
		input.connect(splitter);
		this.#analysers.forEach((a, i) => splitter.connect(a, i));

		ctx.addEventListener('statechange', this.#sync);
		this.#ctx = ctx;
		this.#input = input;
		this.#output = output;
		this.#sync();
		return ctx;
	}

	#sync = (): void => {
		const ctx = this.#ctx;
		this.running = ctx?.state === 'running';
		this.latency = ctx ? ctx.baseLatency + (ctx.outputLatency || 0) : null;
	};

	#startMeter(): void {
		if (!this.#raf) this.#raf = requestAnimationFrame(this.#tick);
	}

	#stopMeter(): void {
		cancelAnimationFrame(this.#raf);
		this.#raf = 0;
		this.#peaks = [0, 0];
		this.levels = [0, 0];
	}

	#tick = (): void => {
		for (let i = 0; i < 2; i++) {
			this.#analysers[i].getFloatTimeDomainData(this.#buffer);
			this.#peaks[i] = Math.max(peakOf(this.#buffer), this.#peaks[i] * DECAY);
		}
		this.levels = [peakToMeter(this.#peaks[0]), peakToMeter(this.#peaks[1])];
		this.#raf = requestAnimationFrame(this.#tick);
	};
}

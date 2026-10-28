/** Measures frames actually presented by a <video>, via requestVideoFrameCallback. */
export class FrameRateMeter {
	fps = $state<number | null>(null);

	observe(video: HTMLVideoElement): () => void {
		if (!('requestVideoFrameCallback' in video)) return () => {};

		let handle = 0;
		let start: { time: number; frames: number } | null = null;

		const onFrame = (now: number, meta: VideoFrameCallbackMetadata) => {
			if (!start) {
				start = { time: now, frames: meta.presentedFrames };
			} else if (now - start.time >= 1000) {
				this.fps = ((meta.presentedFrames - start.frames) * 1000) / (now - start.time);
				start = { time: now, frames: meta.presentedFrames };
			}
			handle = video.requestVideoFrameCallback(onFrame);
		};
		handle = video.requestVideoFrameCallback(onFrame);

		return () => {
			video.cancelVideoFrameCallback(handle);
			this.fps = null;
		};
	}
}

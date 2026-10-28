export class DeviceManager {
	videoInputs = $state.raw<MediaDeviceInfo[]>([]);
	audioInputs = $state.raw<MediaDeviceInfo[]>([]);
	/** Labels are only exposed once the user has granted access. */
	hasLabels = $state(false);

	async refresh(): Promise<void> {
		if (!navigator.mediaDevices?.enumerateDevices) return;
		const all = await navigator.mediaDevices.enumerateDevices();
		this.videoInputs = all.filter((d) => d.kind === 'videoinput' && d.deviceId);
		this.audioInputs = all.filter((d) => d.kind === 'audioinput' && d.deviceId);
		this.hasLabels = all.some((d) => d.label !== '');
	}

	listen(onChange: () => void): () => void {
		const md = navigator.mediaDevices;
		if (!md) return () => {};
		const handler = async () => {
			await this.refresh();
			onChange();
		};
		md.addEventListener('devicechange', handler);
		return () => md.removeEventListener('devicechange', handler);
	}

	/**
	 * Ask for camera + microphone access in a single prompt so device labels become
	 * visible. Devices are enumerated while the temporary stream is still open
	 * (Firefox hides labels otherwise). Returns the video device the user granted.
	 */
	async requestAccess(): Promise<string | undefined> {
		if (!navigator.mediaDevices?.getUserMedia) throw new TypeError('getUserMedia unavailable');
		let stream: MediaStream;
		try {
			stream = await navigator.mediaDevices.getUserMedia({ video: true, audio: true });
		} catch (err) {
			// No microphone-class device at all — video alone is still useful.
			if ((err as Error).name !== 'NotFoundError') throw err;
			stream = await navigator.mediaDevices.getUserMedia({ video: true });
		}
		try {
			await this.refresh();
			return stream.getVideoTracks()[0]?.getSettings().deviceId;
		} finally {
			stream.getTracks().forEach((t) => t.stop());
		}
	}
}

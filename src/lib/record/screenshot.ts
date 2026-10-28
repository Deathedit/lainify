/** Encode the current video frame at the stream's native resolution. */
export function grabFrame(video: HTMLVideoElement): Promise<Blob> {
	const { videoWidth: width, videoHeight: height } = video;
	if (!width || !height) return Promise.reject(new Error('No video frame yet'));

	const canvas = document.createElement('canvas');
	canvas.width = width;
	canvas.height = height;
	const ctx = canvas.getContext('2d');
	if (!ctx) return Promise.reject(new Error('Canvas unavailable'));
	ctx.drawImage(video, 0, 0, width, height);

	return new Promise((resolve, reject) =>
		canvas.toBlob((b) => (b ? resolve(b) : reject(new Error('PNG encoding failed'))), 'image/png')
	);
}

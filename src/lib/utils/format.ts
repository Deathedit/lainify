const pad = (n: number) => String(n).padStart(2, '0');

/** `mm:ss`, or `h:mm:ss` once an hour has passed. */
export function formatDuration(ms: number): string {
	const total = Math.max(0, Math.floor(ms / 1000));
	const h = Math.floor(total / 3600);
	const m = Math.floor((total % 3600) / 60);
	const s = total % 60;
	return h > 0 ? `${h}:${pad(m)}:${pad(s)}` : `${pad(m)}:${pad(s)}`;
}

/** `lainify-20261028-142233.png` */
export function timestampedName(prefix: string, ext: string, date = new Date()): string {
	const day = `${date.getFullYear()}${pad(date.getMonth() + 1)}${pad(date.getDate())}`;
	const time = `${pad(date.getHours())}${pad(date.getMinutes())}${pad(date.getSeconds())}`;
	return `${prefix}-${day}-${time}.${ext}`;
}

/** 60 → "60", 59.9401 → "59.94" */
export function formatFps(fps: number): string {
	return String(Math.round(fps * 100) / 100);
}

export function formatBytes(bytes: number): string {
	const units = ['B', 'KB', 'MB', 'GB'];
	let value = bytes;
	let unit = 0;
	while (value >= 1000 && unit < units.length - 1) {
		value /= 1000;
		unit++;
	}
	return `${value.toFixed(unit === 0 ? 0 : 1)} ${units[unit]}`;
}

/** Browsers append the USB vendor:product id to device labels — hide it in the UI. */
export function cleanLabel(label: string): string {
	return label
		.replace(/\s*\([0-9a-f]{4}:[0-9a-f]{4}\)\s*$/i, '')
		.replace(/^(.+?): \1$/, '$1')
		.trim();
}

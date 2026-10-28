/** Bottom of the meter scale. */
export const METER_FLOOR_DB = -60;

/** Linear peak amplitude (0…1) → meter position (0…1) on a dBFS scale. */
export function peakToMeter(peak: number): number {
	if (!(peak > 0)) return 0;
	const db = 20 * Math.log10(peak);
	return Math.min(1, Math.max(0, (db - METER_FLOOR_DB) / -METER_FLOOR_DB));
}

export function peakOf(samples: Float32Array): number {
	let peak = 0;
	for (let i = 0; i < samples.length; i++) {
		const v = Math.abs(samples[i]);
		if (v > peak) peak = v;
	}
	return peak;
}

import { describe, expect, it } from 'vitest';
import { peakOf, peakToMeter } from './meter';

describe('peakToMeter', () => {
	it('maps the dBFS range onto 0…1', () => {
		expect(peakToMeter(1)).toBe(1);
		expect(peakToMeter(0.001)).toBeCloseTo(0); // -60 dBFS
		expect(peakToMeter(10 ** (-30 / 20))).toBeCloseTo(0.5); // -30 dBFS
	});

	it('clamps silence and clipping', () => {
		expect(peakToMeter(0)).toBe(0);
		expect(peakToMeter(Number.NaN)).toBe(0);
		expect(peakToMeter(1e-9)).toBe(0);
		expect(peakToMeter(2)).toBe(1);
	});
});

describe('peakOf', () => {
	it('returns the largest absolute sample', () => {
		expect(peakOf(new Float32Array([0.1, -0.75, 0.5]))).toBeCloseTo(0.75);
		expect(peakOf(new Float32Array(0))).toBe(0);
	});
});

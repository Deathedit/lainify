import { describe, expect, it } from 'vitest';
import { isCaptureLike, matchAudioDevice, pickVideoDevice, usbId } from './match';

const dev = (deviceId: string, label: string, groupId = '') => ({ deviceId, label, groupId });

const webcam = dev('v-cam', 'Integrated Camera: Integrated C (04f2:b6d9)', 'g-cam');
const elgato = dev('v-elg', 'Elgato HD60 X: Elgato HD60 X (0fd9:0082)', 'g-elg-v');

describe('usbId / isCaptureLike', () => {
	it('extracts vendor:product ids', () => {
		expect(usbId(elgato.label)).toBe('0fd9:0082');
		expect(usbId('Built-in Audio Analog Stereo')).toBeNull();
	});

	it('recognises capture hardware', () => {
		expect(isCaptureLike(elgato.label)).toBe(true);
		expect(isCaptureLike('AVerMedia Live Gamer Ultra')).toBe(true);
		expect(isCaptureLike(webcam.label)).toBe(false);
	});
});

describe('pickVideoDevice', () => {
	const devices = [webcam, elgato];

	it('keeps the saved device when present', () => {
		expect(pickVideoDevice(devices, 'v-cam')).toBe('v-cam');
	});

	it('prefers a capture card over a webcam', () => {
		expect(pickVideoDevice(devices, null)).toBe('v-elg');
		expect(pickVideoDevice(devices, 'gone')).toBe('v-elg');
	});

	it('falls back to the granted device, then the first one', () => {
		expect(pickVideoDevice([webcam, dev('v-2', 'Other')], null, 'v-2')).toBe('v-2');
		expect(pickVideoDevice([webcam], null)).toBe('v-cam');
		expect(pickVideoDevice([], null)).toBeUndefined();
	});
});

describe('matchAudioDevice', () => {
	const mic = dev('a-mic', 'Built-in Audio Analog Stereo', 'g-int');

	it('matches by USB id first', () => {
		const audio = [mic, dev('a-elg', 'HD60 X Analog Stereo (0fd9:0082)')];
		expect(matchAudioDevice(elgato, audio)).toBe('a-elg');
	});

	it('matches by groupId', () => {
		const audio = [mic, dev('a-x', 'Something', 'g-elg-v')];
		expect(matchAudioDevice(elgato, audio)).toBe('a-x');
	});

	it('matches by shared label words', () => {
		const audio = [mic, dev('a-elg', 'Elgato HD60 X Analog Stereo')];
		expect(matchAudioDevice(elgato, audio)).toBe('a-elg');
	});

	it('ignores the default/communications pseudo-devices', () => {
		const audio = [
			dev('default', 'Default - Elgato HD60 X Analog Stereo'),
			dev('a-elg', 'Elgato HD60 X')
		];
		expect(matchAudioDevice(elgato, audio)).toBe('a-elg');
	});

	it('never falls back to an unrelated microphone', () => {
		expect(matchAudioDevice(elgato, [mic, dev('a-usb', 'USB Audio Device')])).toBeUndefined();
		expect(matchAudioDevice(undefined, [mic])).toBeUndefined();
	});
});

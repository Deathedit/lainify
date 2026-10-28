import { describe, expect, it } from 'vitest';
import { captureError, classifyError } from './errors';

const domError = (name: string) => Object.assign(new Error(name), { name });

describe('classifyError', () => {
	it.each([
		['NotAllowedError', 'denied'],
		['NotFoundError', 'not-found'],
		['NotReadableError', 'busy'],
		['AbortError', 'busy'],
		['OverconstrainedError', 'constraints'],
		['TypeError', 'unsupported']
	])('%s → %s', (name, kind) => {
		expect(classifyError(domError(name)).kind).toBe(kind);
	});

	it('keeps the original message for unknown errors', () => {
		const err = classifyError(new Error('boom'));
		expect(err.kind).toBe('unknown');
		expect(err.message).toContain('boom');
	});

	it('handles non-error values', () => {
		expect(classifyError(undefined).kind).toBe('unknown');
	});
});

describe('captureError', () => {
	it('provides user-facing copy', () => {
		const err = captureError('ended');
		expect(err.title).toBe('Device disconnected');
		expect(err.message.length).toBeGreaterThan(0);
	});
});

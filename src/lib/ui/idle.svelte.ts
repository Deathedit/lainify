/** Flags the pointer as idle after a delay, so controls and cursor can hide. */
export class IdleTracker {
	idle = $state(false);
	#delay: () => number;
	#timer: ReturnType<typeof setTimeout> | undefined;

	constructor(delay: () => number) {
		this.#delay = delay;
	}

	poke = (): void => {
		this.idle = false;
		clearTimeout(this.#timer);
		const delay = this.#delay();
		if (delay > 0) this.#timer = setTimeout(() => (this.idle = true), delay);
	};

	dispose(): void {
		clearTimeout(this.#timer);
	}
}

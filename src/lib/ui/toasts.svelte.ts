export type ToastTone = 'info' | 'success' | 'error';

export interface Toast {
	id: number;
	message: string;
	tone: ToastTone;
}

export class Toasts {
	items = $state<Toast[]>([]);
	#next = 0;

	show(message: string, tone: ToastTone = 'info', duration = 3500): void {
		const id = ++this.#next;
		this.items = [...this.items.slice(-2), { id, message, tone }];
		setTimeout(() => this.dismiss(id), duration);
	}

	dismiss(id: number): void {
		this.items = this.items.filter((t) => t.id !== id);
	}
}

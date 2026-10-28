<script lang="ts">
	import { CircleAlert, CircleCheck, Info } from '@lucide/svelte';
	import { flip } from 'svelte/animate';
	import { prefersReducedMotion } from 'svelte/motion';
	import { fade, fly } from 'svelte/transition';
	import type { Toasts } from '../ui/toasts.svelte';

	let { toasts }: { toasts: Toasts } = $props();

	const duration = $derived(prefersReducedMotion.current ? 0 : 200);
</script>

<div
	class="pointer-events-none absolute top-[max(1rem,env(safe-area-inset-top))] left-1/2 z-50 flex w-max max-w-[calc(100%-2rem)] -translate-x-1/2 flex-col items-center gap-2"
	role="status"
	aria-live="polite"
>
	{#each toasts.items as toast (toast.id)}
		<button
			type="button"
			class="pointer-events-auto flex max-w-full items-center gap-2.5 rounded-full glass py-2 pr-4 pl-3 text-sm text-zinc-100"
			onclick={() => toasts.dismiss(toast.id)}
			animate:flip={{ duration }}
			in:fly={{ y: -8, duration }}
			out:fade={{ duration }}
		>
			{#if toast.tone === 'success'}
				<CircleCheck size={16} class="shrink-0 text-emerald-400" />
			{:else if toast.tone === 'error'}
				<CircleAlert size={16} class="shrink-0 text-rose-400" />
			{:else}
				<Info size={16} class="shrink-0 text-accent-300" />
			{/if}
			<span class="truncate">{toast.message}</span>
		</button>
	{/each}
</div>

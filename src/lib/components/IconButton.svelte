<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLButtonAttributes } from 'svelte/elements';

	interface Props extends HTMLButtonAttributes {
		label: string;
		shortcut?: string;
		active?: boolean;
		/** Pill-shaped button that fits text next to the icon. */
		wide?: boolean;
		tooltip?: 'top' | 'bottom' | 'none';
		children: Snippet;
	}

	let {
		label,
		shortcut,
		active = false,
		wide = false,
		tooltip = 'top',
		children,
		class: className = '',
		...rest
	}: Props = $props();
</script>

<button
	type="button"
	aria-label={label}
	class={[
		'group relative shrink-0 items-center justify-center rounded-xl transition-colors duration-150 disabled:pointer-events-none disabled:opacity-35',
		wide ? 'flex h-10 gap-2 px-3 text-sm font-medium' : 'grid size-10',
		active ? 'bg-white/12 text-white' : 'text-zinc-300 hover:bg-white/8 hover:text-white',
		className
	]}
	{...rest}
>
	{@render children()}
	{#if tooltip !== 'none'}
		<span
			aria-hidden="true"
			class={[
				'pointer-events-none absolute left-1/2 z-10 flex -translate-x-1/2 items-center gap-2 rounded-lg glass px-2.5 py-1.5 text-xs font-medium whitespace-nowrap text-zinc-200 opacity-0 transition duration-150 group-hover:translate-y-0 group-hover:opacity-100 group-focus-visible:translate-y-0 group-focus-visible:opacity-100',
				tooltip === 'top' ? 'bottom-full mb-3 translate-y-1' : 'top-full mt-3 -translate-y-1'
			]}
		>
			{label}
			{#if shortcut}<kbd>{shortcut}</kbd>{/if}
		</span>
	{/if}
</button>

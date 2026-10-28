<script lang="ts">
	let { levels, active = true }: { levels: [number, number]; active?: boolean } = $props();
</script>

<!-- -60 dBFS … 0 dBFS: green up to -15, amber to -6, red above. -->
<div
	class={['flex w-14 flex-col gap-[3px] transition-opacity', !active && 'opacity-30']}
	aria-hidden="true"
>
	{#each levels as level, i (i)}
		<div class="h-[3px] overflow-hidden rounded-full bg-white/10">
			<div
				class="meter h-full rounded-full"
				style:clip-path="inset(0 {(1 - level) * 100}% 0 0)"
			></div>
		</div>
	{/each}
</div>

<style>
	.meter {
		background: linear-gradient(
			to right,
			var(--color-emerald-400) 0%,
			var(--color-emerald-400) 72%,
			var(--color-amber-300) 86%,
			var(--color-rose-500) 100%
		);
	}
</style>

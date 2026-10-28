<script lang="ts">
	import { Volume, Volume1, Volume2, VolumeX } from '@lucide/svelte';
	import { MAX_VOLUME } from '../settings/schema';
	import IconButton from './IconButton.svelte';
	import LevelMeter from './LevelMeter.svelte';

	interface Props {
		volume: number;
		muted: boolean;
		levels: [number, number];
		disabled?: boolean;
	}

	let { volume = $bindable(), muted = $bindable(), levels, disabled = false }: Props = $props();

	const percent = $derived(Math.round(volume * 100));
	const silent = $derived(muted || volume === 0);
</script>

<div class="flex items-center gap-1.5 pr-2">
	<IconButton
		label={disabled ? 'No audio input' : muted ? 'Unmute' : 'Mute'}
		shortcut={disabled ? undefined : 'M'}
		{disabled}
		onclick={() => (muted = !muted)}
	>
		{#if silent || disabled}
			<VolumeX size={18} />
		{:else if volume < 0.35}
			<Volume size={18} />
		{:else if volume < 0.8}
			<Volume1 size={18} />
		{:else}
			<Volume2 size={18} />
		{/if}
	</IconButton>
	<input
		type="range"
		class={['range hidden w-24 transition-opacity sm:block', (muted || disabled) && 'opacity-50']}
		min="0"
		max={MAX_VOLUME}
		step="0.01"
		aria-label="Volume"
		aria-valuetext="{percent}%"
		{disabled}
		bind:value={volume}
		oninput={() => (muted = false)}
		style:--fill="{(volume / MAX_VOLUME) * 100}%"
	/>
	<span class="hidden w-9 text-right text-xs text-zinc-400 tabular-nums sm:inline">{percent}%</span>
	<div class="hidden sm:block"><LevelMeter {levels} active={!disabled && !muted} /></div>
</div>

<script lang="ts">
	import {
		Camera,
		LoaderCircle,
		Maximize,
		Minimize,
		Power,
		Settings2,
		Square
	} from '@lucide/svelte';
	import { formatDuration } from '../utils/format';
	import IconButton from './IconButton.svelte';
	import VolumeControl from './VolumeControl.svelte';

	interface Props {
		visible: boolean;
		live: boolean;
		hasAudio: boolean;
		volume: number;
		muted: boolean;
		levels: [number, number];
		recording: boolean;
		saving: boolean;
		elapsed: number;
		canRecord: boolean;
		fullscreen: boolean;
		panelOpen: boolean;
		onpower: () => void;
		onscreenshot: () => void;
		onrecord: () => void;
		onsettings: () => void;
		onfullscreen: () => void;
		onhover: (hovering: boolean) => void;
	}

	let {
		visible,
		live,
		hasAudio,
		volume = $bindable(),
		muted = $bindable(),
		levels,
		recording,
		saving,
		elapsed,
		canRecord,
		fullscreen,
		panelOpen,
		onpower,
		onscreenshot,
		onrecord,
		onsettings,
		onfullscreen,
		onhover
	}: Props = $props();
</script>

{#snippet divider()}
	<span class="mx-1 h-6 w-px bg-white/10" aria-hidden="true"></span>
{/snippet}

<div
	class="pointer-events-none absolute inset-x-0 bottom-0 flex justify-center px-4 pb-[max(1.25rem,env(safe-area-inset-bottom))]"
>
	<div
		role="toolbar"
		aria-label="Capture controls"
		tabindex="-1"
		class={[
			'flex max-w-full items-center gap-0.5 rounded-2xl glass p-1.5 transition duration-300 ease-soft',
			visible ? 'pointer-events-auto' : 'translate-y-3 opacity-0'
		]}
		onpointerenter={() => onhover(true)}
		onpointerleave={() => onhover(false)}
	>
		<IconButton label={live ? 'Stop capture' : 'Start capture'} onclick={onpower}>
			<Power size={18} class={live ? 'text-emerald-400' : ''} />
		</IconButton>

		{@render divider()}
		<VolumeControl bind:volume bind:muted {levels} disabled={!hasAudio} />
		{@render divider()}

		<IconButton label="Screenshot" shortcut="S" disabled={!live} onclick={onscreenshot}>
			<Camera size={18} />
		</IconButton>
		<IconButton
			wide
			label={recording ? 'Stop recording' : 'Record'}
			shortcut="R"
			disabled={saving || (!recording && !canRecord)}
			class={recording ? 'bg-rec/90 text-white hover:bg-rec hover:text-white' : ''}
			onclick={onrecord}
		>
			{#if recording}
				<Square size={11} fill="currentColor" />
				<span class="tabular-nums">{formatDuration(elapsed)}</span>
			{:else if saving}
				<LoaderCircle size={16} class="animate-spin" />
				<span>Saving</span>
			{:else}
				<span class="size-2.5 rounded-full bg-rec ring-3 ring-rec/25"></span>
				<span>Rec</span>
			{/if}
		</IconButton>

		{@render divider()}
		<IconButton label="Settings" shortcut="," active={panelOpen} onclick={onsettings}>
			<Settings2 size={18} />
		</IconButton>
		<IconButton
			label={fullscreen ? 'Exit fullscreen' : 'Fullscreen'}
			shortcut="F"
			onclick={onfullscreen}
		>
			{#if fullscreen}<Minimize size={18} />{:else}<Maximize size={18} />{/if}
		</IconButton>
	</div>
</div>

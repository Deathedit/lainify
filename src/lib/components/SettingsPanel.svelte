<script lang="ts">
	import { AudioLines, Keyboard, MonitorPlay, PanelTop, Video, X } from '@lucide/svelte';
	import { prefersReducedMotion } from 'svelte/motion';
	import { fade, fly } from 'svelte/transition';
	import { settings } from '../settings/store.svelte';
	import { SHORTCUTS } from '../ui/shortcuts';
	import IconButton from './IconButton.svelte';
	import Select, { type SelectOption } from './Select.svelte';

	interface Props {
		open: boolean;
		videoOptions: SelectOption[];
		audioOptions: SelectOption[];
		modeOptions: SelectOption[];
		videoValue: string;
		audioValue: string;
		modeValue: string;
		/** Mode changes are blocked while recording. */
		modeLocked: boolean;
		videoInfo: string;
		audioInfo: string;
		recordFormat: string;
		canSaveToFile: boolean;
		onvideo: (id: string) => void;
		onaudio: (id: string) => void;
		onmode: (id: string) => void;
	}

	let {
		open = $bindable(),
		videoOptions,
		audioOptions,
		modeOptions,
		videoValue,
		audioValue,
		modeValue,
		modeLocked,
		videoInfo,
		audioInfo,
		recordFormat,
		canSaveToFile,
		onvideo,
		onaudio,
		onmode
	}: Props = $props();

	const duration = $derived(prefersReducedMotion.current ? 0 : 220);

	const BITRATES: SelectOption[] = [6, 12, 20, 40].map((mbps) => ({
		value: String(mbps * 1_000_000),
		label: `${mbps} Mbps${mbps === 12 ? ' (recommended)' : ''}`
	}));

	const HIDE_DELAYS: SelectOption[] = [
		{ value: '1500', label: 'After 1.5 s' },
		{ value: '2500', label: 'After 2.5 s' },
		{ value: '5000', label: 'After 5 s' },
		{ value: '0', label: 'Never' }
	];

	const TARGETS: SelectOption[] = [
		{ value: 'download', label: 'Download when stopped' },
		{ value: 'file', label: 'Stream to a file (long recordings)' }
	];
</script>

{#snippet heading(title: string, Icon: typeof Video)}
	<h3
		class="flex items-center gap-2 text-[11px] font-semibold tracking-wider text-zinc-500 uppercase"
	>
		<Icon size={13} />
		{title}
	</h3>
{/snippet}

{#snippet info(text: string)}
	{#if text}
		<p class="rounded-lg bg-white/[0.03] px-3 py-2 text-xs text-zinc-400 tabular-nums">{text}</p>
	{/if}
{/snippet}

{#if open}
	<button
		type="button"
		class="absolute inset-0 cursor-default"
		aria-label="Close settings"
		tabindex="-1"
		onclick={() => (open = false)}
		transition:fade={{ duration }}
	></button>

	<aside
		class="absolute top-3 right-3 bottom-3 flex w-[min(22rem,calc(100%-1.5rem))] flex-col overflow-hidden rounded-2xl glass"
		aria-label="Settings"
		tabindex="-1"
		transition:fly={{ x: 24, duration }}
		{@attach (el) => el.focus()}
	>
		<header class="flex items-center justify-between py-3 pr-3 pl-5">
			<h2 class="text-sm font-semibold">Settings</h2>
			<IconButton label="Close settings" tooltip="none" onclick={() => (open = false)}>
				<X size={16} />
			</IconButton>
		</header>

		<div class="flex-1 space-y-7 overflow-y-auto px-5 pt-1 pb-6">
			<section class="space-y-3">
				{@render heading('Video', Video)}
				<Select
					label="Device"
					value={videoValue}
					options={videoOptions}
					disabled={videoOptions.length === 0}
					onchange={onvideo}
				/>
				<Select
					label="Resolution & frame rate"
					value={modeValue}
					options={modeOptions}
					disabled={modeLocked}
					hint={modeLocked
						? 'Stop recording to change the mode.'
						: 'Only modes within the device’s reported limits are listed.'}
					onchange={onmode}
				/>
				{@render info(videoInfo)}
			</section>

			<section class="space-y-3">
				{@render heading('Audio', AudioLines)}
				<Select label="Device" value={audioValue} options={audioOptions} onchange={onaudio} />
				{@render info(audioInfo)}
			</section>

			<section class="space-y-3">
				{@render heading('Recording', MonitorPlay)}
				<Select
					label="Video bitrate"
					value={String(settings.bitrate)}
					options={BITRATES}
					hint={`Format: ${recordFormat}`}
					onchange={(v) => (settings.bitrate = Number(v))}
				/>
				{#if canSaveToFile}
					<Select
						label="Save"
						value={settings.recordTarget}
						options={TARGETS}
						hint={settings.recordTarget === 'file'
							? 'You pick the file when recording starts; data is written as it arrives.'
							: 'The recording is kept in memory until you stop.'}
						onchange={(v) => (settings.recordTarget = v === 'file' ? 'file' : 'download')}
					/>
				{/if}
			</section>

			<section class="space-y-3">
				{@render heading('Interface', PanelTop)}
				<Select
					label="Hide controls"
					value={String(settings.hideDelay)}
					options={HIDE_DELAYS}
					onchange={(v) => (settings.hideDelay = Number(v))}
				/>
			</section>

			<section class="space-y-3">
				{@render heading('Shortcuts', Keyboard)}
				<ul class="space-y-2">
					{#each SHORTCUTS as shortcut (shortcut.label)}
						<li class="flex items-center justify-between text-sm text-zinc-300">
							{shortcut.label}
							<span class="flex gap-1">
								{#each shortcut.keys as key (key)}<kbd>{key}</kbd>{/each}
							</span>
						</li>
					{/each}
				</ul>
			</section>
		</div>
	</aside>
{/if}

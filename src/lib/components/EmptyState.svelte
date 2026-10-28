<script module lang="ts">
	export type Phase = 'welcome' | 'requesting' | 'starting' | 'stopped' | 'error';
</script>

<script lang="ts">
	import {
		LoaderCircle,
		Plug,
		Power,
		RotateCw,
		Settings2,
		TriangleAlert,
		Unplug
	} from '@lucide/svelte';
	import type { CaptureError } from '../capture/errors';

	interface Props {
		phase: Phase;
		error: CaptureError | null;
		deviceName: string;
		onconnect: () => void;
		onsettings: () => void;
	}

	let { phase, error, deviceName, onconnect, onsettings }: Props = $props();
</script>

{#snippet primary(label: string, Icon: typeof Plug)}
	<button
		type="button"
		class="mt-8 inline-flex items-center gap-2 rounded-xl bg-accent-400 px-5 py-2.5 text-sm font-semibold text-zinc-950 shadow-[0_8px_30px_-8px_var(--color-accent-500)] transition hover:bg-accent-300 active:scale-[0.98]"
		onclick={onconnect}
	>
		<Icon size={16} />
		{label}
	</button>
{/snippet}

{#snippet spinner(title: string, detail: string)}
	<LoaderCircle size={28} class="animate-spin text-accent-400" />
	<h2 class="mt-5 text-base font-medium">{title}</h2>
	<p class="mt-1.5 text-sm text-zinc-400">{detail}</p>
{/snippet}

<div
	class="absolute inset-0 grid place-items-center bg-[radial-gradient(60%_50%_at_50%_45%,oklch(0.32_0.09_295/0.22),transparent)] p-6"
>
	<div class="flex max-w-sm flex-col items-center text-center">
		{#if phase === 'welcome'}
			<svg
				viewBox="0 0 32 32"
				class="size-16 drop-shadow-[0_0_24px_oklch(0.77_0.13_295/0.35)]"
				aria-hidden="true"
			>
				<rect width="32" height="32" rx="8" fill="#15121d" />
				<rect
					x="6"
					y="8.5"
					width="20"
					height="15"
					rx="3.5"
					fill="none"
					stroke="#b9a2ff"
					stroke-width="1.6"
				/>
				<circle cx="16" cy="16" r="3.25" fill="#b9a2ff" />
			</svg>
			<h1 class="mt-6 text-2xl font-semibold tracking-tight">lainify</h1>
			<p class="mt-2 text-sm leading-relaxed text-zinc-400">
				A clean, low-latency window into your capture card — picture and sound, straight from the
				device.
			</p>
			{@render primary('Connect capture device', Plug)}
			<p class="mt-4 text-xs text-zinc-500">
				Your browser will ask for camera &amp; microphone access. Nothing leaves this machine.
			</p>
		{:else if phase === 'requesting'}
			{@render spinner(
				'Waiting for permission…',
				'Allow camera & microphone access in the browser prompt.'
			)}
		{:else if phase === 'starting'}
			{@render spinner(
				`Opening ${deviceName || 'device'}…`,
				'Negotiating resolution and frame rate.'
			)}
		{:else if phase === 'stopped'}
			<div class="grid size-14 place-items-center rounded-2xl bg-white/5 text-zinc-400">
				<Power size={24} />
			</div>
			<h2 class="mt-5 text-base font-medium">Capture stopped</h2>
			<p class="mt-1.5 text-sm text-zinc-400">The device is released and free for other apps.</p>
			{@render primary('Start capture', Power)}
		{:else if phase === 'error' && error}
			<div
				class={[
					'grid size-14 place-items-center rounded-2xl',
					error.kind === 'ended' ? 'bg-amber-400/10 text-amber-300' : 'bg-rose-500/10 text-rose-400'
				]}
			>
				{#if error.kind === 'ended'}<Unplug size={24} />{:else}<TriangleAlert size={24} />{/if}
			</div>
			<h2 class="mt-5 text-base font-medium">{error.title}</h2>
			<p class="mt-1.5 text-sm leading-relaxed text-zinc-400">{error.message}</p>
			<div class="mt-8 flex gap-2">
				<button
					type="button"
					class="inline-flex items-center gap-2 rounded-xl bg-white/10 px-4 py-2 text-sm font-medium transition hover:bg-white/15"
					onclick={onconnect}
				>
					<RotateCw size={15} /> Retry
				</button>
				<button
					type="button"
					class="inline-flex items-center gap-2 rounded-xl px-4 py-2 text-sm font-medium text-zinc-300 transition hover:bg-white/8 hover:text-white"
					onclick={onsettings}
				>
					<Settings2 size={15} /> Choose device
				</button>
			</div>
		{/if}
	</div>
</div>

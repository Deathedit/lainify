<script lang="ts">
	import { formatDuration, formatFps } from '../utils/format';

	interface Props {
		visible: boolean;
		live: boolean;
		deviceName: string;
		video: MediaTrackSettings | null;
		audio: MediaTrackSettings | null;
		/** Measured presentation rate; falls back to the configured rate. */
		fps: number | null;
		recording: boolean;
		elapsed: number;
	}

	let { visible, live, deviceName, video, audio, fps, recording, elapsed }: Props = $props();

	const details = $derived.by(() => {
		const parts: string[] = [];
		if (video?.width && video.height) parts.push(`${video.width}×${video.height}`);
		const rate = fps ?? video?.frameRate;
		if (rate) parts.push(`${formatFps(fps ? Math.round(rate * 10) / 10 : rate)} fps`);
		if (audio?.sampleRate) parts.push(`${audio.sampleRate / 1000} kHz`);
		if (audio?.channelCount) parts.push(audio.channelCount === 1 ? 'mono' : 'stereo');
		return parts.join(' · ');
	});
</script>

<div
	class="pointer-events-none absolute top-[max(1rem,env(safe-area-inset-top))] left-4 flex flex-wrap items-center gap-2"
>
	{#if recording}
		<div
			class="flex items-center gap-2 rounded-full glass px-3 py-1.5 text-xs font-semibold tracking-wide text-white"
		>
			<span class="size-2 animate-pulse rounded-full bg-rec"></span>
			REC
			<span class="font-medium tabular-nums">{formatDuration(elapsed)}</span>
		</div>
	{/if}
	{#if live}
		<div
			class={[
				'flex items-center gap-2.5 rounded-full glass py-1.5 pr-3.5 pl-3 text-xs font-medium text-zinc-200 transition duration-300 ease-soft',
				!visible && '-translate-y-1 opacity-0'
			]}
		>
			<span class="size-2 rounded-full bg-emerald-400 shadow-[0_0_8px_var(--color-emerald-400)]"
			></span>
			<span class="max-w-52 truncate">{deviceName || 'Live'}</span>
			{#if details}
				<span class="h-3 w-px bg-white/15"></span>
				<span class="text-zinc-400 tabular-nums">{details}</span>
			{/if}
		</div>
	{/if}
</div>

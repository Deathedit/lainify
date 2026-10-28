<script lang="ts">
	interface Props {
		stream: MediaStream | null;
		video?: HTMLVideoElement;
		/** Increment to play the screenshot flash. */
		flash: number;
		ondblclick: () => void;
	}

	let { stream, video = $bindable(), flash, ondblclick }: Props = $props();

	$effect(() => {
		if (video) video.srcObject = stream;
	});
</script>

<!-- Audio is played through Web Audio (see AudioEngine), so the element stays muted. -->
<!-- svelte-ignore a11y_media_has_caption -->
<video
	bind:this={video}
	autoplay
	playsinline
	muted
	disablepictureinpicture
	class={[
		'absolute inset-0 size-full object-contain transition-opacity duration-500',
		stream ? 'opacity-100' : 'opacity-0'
	]}
	{ondblclick}
></video>

{#key flash}
	{#if flash > 0}
		<div
			class="pointer-events-none absolute inset-0 animate-[flash_450ms_ease-out_forwards] bg-white"
		></div>
	{/if}
{/key}

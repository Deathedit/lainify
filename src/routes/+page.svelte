<script lang="ts">
	import { onMount, untrack } from 'svelte';
	import { AudioEngine } from '#lib/audio/engine.svelte.ts';
	import { DeviceManager } from '#lib/capture/devices.svelte.ts';
	import { matchAudioDevice, pickVideoDevice } from '#lib/capture/match.ts';
	import { AUTO_MODE_ID, findMode, modeLabel, supportedModes } from '#lib/capture/modes.ts';
	import { CaptureSession } from '#lib/capture/session.svelte.ts';
	import ControlBar from '#lib/components/ControlBar.svelte';
	import EmptyState, { type Phase } from '#lib/components/EmptyState.svelte';
	import type { SelectOption } from '#lib/components/Select.svelte';
	import SettingsPanel from '#lib/components/SettingsPanel.svelte';
	import StatusPill from '#lib/components/StatusPill.svelte';
	import Toasts from '#lib/components/Toasts.svelte';
	import VideoStage from '#lib/components/VideoStage.svelte';
	import { describeMime } from '#lib/record/mime.ts';
	import { Recorder, canSaveToFile } from '#lib/record/recorder.svelte.ts';
	import { grabFrame } from '#lib/record/screenshot.ts';
	import { MAX_VOLUME, NO_AUDIO } from '#lib/settings/schema.ts';
	import { settings } from '#lib/settings/store.svelte.ts';
	import { FrameRateMeter } from '#lib/ui/fps.svelte.ts';
	import { IdleTracker } from '#lib/ui/idle.svelte.ts';
	import { shortcutFor } from '#lib/ui/shortcuts.ts';
	import { Toasts as ToastQueue } from '#lib/ui/toasts.svelte.ts';
	import { downloadBlob } from '#lib/utils/download.ts';
	import { cleanLabel, formatBytes, formatFps, timestampedName } from '#lib/utils/format.ts';

	const devices = new DeviceManager();
	const session = new CaptureSession();
	const audio = new AudioEngine();
	const recorder = new Recorder();
	const toasts = new ToastQueue();
	const frameRate = new FrameRateMeter();
	const idle = new IdleTracker(() => settings.hideDelay);

	let root = $state<HTMLDivElement>();
	let video = $state<HTMLVideoElement>();
	let panelOpen = $state(false);
	let fullscreen = $state(false);
	let barHovered = $state(false);
	let booted = $state(false);
	let requesting = $state(false);
	let userStopped = $state(false);
	let flash = $state(0);

	const live = $derived(session.status === 'live');
	const hasAudio = $derived(session.audio !== null);
	const recording = $derived(recorder.state === 'recording');
	const uiVisible = $derived(!live || !idle.idle || panelOpen || barHovered);
	const deviceName = $derived(
		cleanLabel(
			session.label || devices.videoInputs.find((d) => d.deviceId === settings.videoId)?.label || ''
		)
	);

	const phase = $derived.by((): Phase | null => {
		if (live || !booted) return null;
		if (requesting) return 'requesting';
		if (session.status === 'starting') return 'starting';
		if (session.status === 'error') return 'error';
		return userStopped ? 'stopped' : 'welcome';
	});

	// ── Select options ────────────────────────────────────────────────────────

	const videoOptions = $derived<SelectOption[]>(
		devices.videoInputs.map((d, i) => ({
			value: d.deviceId,
			label: cleanLabel(d.label) || `Camera ${i + 1}`
		}))
	);

	const audioOptions = $derived<SelectOption[]>([
		{ value: 'auto', label: 'Match video device' },
		{ value: NO_AUDIO, label: 'No audio' },
		...devices.audioInputs.map((d, i) => ({
			value: d.deviceId,
			label: cleanLabel(d.label) || `Input ${i + 1}`
		}))
	]);

	const modeOptions = $derived<SelectOption[]>([
		{ value: AUTO_MODE_ID, label: 'Auto (prefer 1080p · 60 fps)' },
		...supportedModes(session.capabilities).map((m) => ({ value: m.id, label: modeLabel(m) }))
	]);

	const videoInfo = $derived.by(() => {
		const v = session.video;
		if (!v?.width) return '';
		const measured = frameRate.fps ? ` · showing ${frameRate.fps.toFixed(1)} fps` : '';
		return `Delivering ${v.width}×${v.height} @ ${formatFps(v.frameRate ?? 0)} fps${measured}`;
	});

	const audioInfo = $derived.by(() => {
		const a = session.audio;
		if (!a) {
			if (settings.audioId === NO_AUDIO || !live) return '';
			return 'No matching audio input found. On Linux, check that the card’s audio profile is enabled (pavucontrol → Configuration), or pick an input above.';
		}
		const parts = [
			a.sampleRate && `${a.sampleRate / 1000} kHz`,
			a.channelCount && (a.channelCount === 1 ? 'mono' : 'stereo'),
			audio.latency !== null && `~${Math.round(audio.latency * 1000)} ms output latency`
		];
		return parts.filter(Boolean).join(' · ');
	});

	// ── Wiring ────────────────────────────────────────────────────────────────

	$effect(() => {
		const stream = session.stream;
		untrack(() => audio.attach(stream));
	});

	$effect(() => {
		audio.setVolume(settings.volume, settings.muted);
	});

	$effect(() => {
		const el = video;
		if (!el || !session.stream) return;
		return untrack(() => frameRate.observe(el));
	});

	session.onended = () => {
		recorder.stop();
		toasts.show('Capture device disconnected', 'error');
	};
	recorder.onsaved = ({ filename, bytes }) =>
		toasts.show(`Saved ${filename} · ${formatBytes(bytes)}`, 'success');
	recorder.onerror = (message) => toasts.show(message, 'error');

	onMount(() => {
		const unlisten = devices.listen(onDeviceChange);
		const onFullscreen = () => (fullscreen = !!root && document.fullscreenElement === root);
		document.addEventListener('fullscreenchange', onFullscreen);

		void boot();
		idle.poke();

		return () => {
			unlisten();
			document.removeEventListener('fullscreenchange', onFullscreen);
			recorder.stop();
			session.stop();
			audio.destroy();
			idle.dispose();
		};
	});

	async function boot() {
		try {
			await devices.refresh();
		} finally {
			booted = true;
		}
		// Access was granted on an earlier visit — go straight to the picture.
		if (devices.hasLabels) await connect();
	}

	// ── Capture lifecycle ─────────────────────────────────────────────────────

	function resolveAudio(videoId: string): string | undefined {
		if (settings.audioId === NO_AUDIO) return undefined;
		if (settings.audioId && devices.audioInputs.some((d) => d.deviceId === settings.audioId)) {
			return settings.audioId;
		}
		const videoDevice = devices.videoInputs.find((d) => d.deviceId === videoId);
		return matchAudioDevice(videoDevice, devices.audioInputs);
	}

	async function connect() {
		userStopped = false;
		recorder.stop();
		// Runs inside the click/key gesture on first use, so audio isn't autoplay-blocked.
		audio.prime();

		let granted: string | undefined;
		if (!devices.hasLabels) {
			requesting = true;
			try {
				granted = await devices.requestAccess();
			} catch (err) {
				session.fail(err);
				return;
			} finally {
				requesting = false;
			}
		}

		const videoId = pickVideoDevice(devices.videoInputs, settings.videoId, granted);
		if (!videoId) {
			session.fail(new DOMException('No video input', 'NotFoundError'));
			return;
		}

		const mode = findMode(settings.modeId);
		const audioId = resolveAudio(videoId);
		const result = await session.start({ videoId, audioId, mode });
		if (!result.ok) return;

		settings.videoId = videoId;
		if (result.modeRelaxed) {
			const v = session.video;
			toasts.show(
				`${modeLabel(mode)} unavailable — using ${v?.height}p · ${formatFps(v?.frameRate ?? 0)} fps`
			);
		}
		if (result.audioDropped) toasts.show('Audio input could not be opened — video only', 'error');
		else if (!audioId && settings.audioId !== NO_AUDIO) {
			toasts.show(
				`No audio input found for ${deviceName || 'this device'} — see Settings`,
				'info',
				6000
			);
		}
	}

	function stopCapture() {
		recorder.stop();
		session.stop();
		userStopped = true;
	}

	function onDeviceChange() {
		// The card we lost came back: resume where we left off.
		const lost = session.status === 'error' && session.error?.kind === 'ended';
		if (lost && devices.videoInputs.some((d) => d.deviceId === settings.videoId)) {
			toasts.show('Capture device reconnected', 'success');
			void connect();
		}
	}

	function selectVideo(id: string) {
		settings.videoId = id;
		void connect();
	}

	function selectAudio(value: string) {
		settings.audioId = value === 'auto' ? null : value;
		if (session.status !== 'idle') void connect();
	}

	async function selectMode(id: string) {
		const previous = settings.modeId;
		settings.modeId = id;
		if (!live) return;
		try {
			await session.applyMode(findMode(id));
		} catch {
			settings.modeId = previous;
			toasts.show(`${modeLabel(findMode(id))} isn’t supported by this device`, 'error');
		}
	}

	// ── Actions ───────────────────────────────────────────────────────────────

	async function screenshot() {
		if (!video || !live) return;
		try {
			const blob = await grabFrame(video);
			const filename = timestampedName('lainify', 'png');
			downloadBlob(blob, filename);
			flash++;
			toasts.show(`Saved ${filename}`, 'success');
		} catch (err) {
			toasts.show(`Screenshot failed: ${(err as Error).message}`, 'error');
		}
	}

	async function toggleRecording() {
		if (recording) return recorder.stop();
		if (!session.stream) return;
		if (!recorder.supported)
			return toasts.show('Recording isn’t supported in this browser', 'error');
		try {
			await recorder.start(session.stream, {
				bitrate: settings.bitrate,
				toFile: settings.recordTarget === 'file'
			});
		} catch (err) {
			toasts.show(`Could not start recording: ${(err as Error).message}`, 'error');
		}
	}

	function toggleFullscreen() {
		if (document.fullscreenElement) void document.exitFullscreen();
		else void root?.requestFullscreen({ navigationUI: 'hide' }).catch(() => {});
	}

	function nudgeVolume(delta: number) {
		settings.volume =
			Math.round(Math.min(MAX_VOLUME, Math.max(0, settings.volume + delta)) * 100) / 100;
		settings.muted = false;
	}

	function onKeydown(e: KeyboardEvent) {
		audio.resume();
		idle.poke();
		const action = shortcutFor(e);
		if (!action) return;
		e.preventDefault();
		switch (action) {
			case 'fullscreen':
				return toggleFullscreen();
			case 'mute':
				settings.muted = !settings.muted;
				return;
			case 'volumeUp':
				return nudgeVolume(0.05);
			case 'volumeDown':
				return nudgeVolume(-0.05);
			case 'screenshot':
				return void screenshot();
			case 'record':
				return void toggleRecording();
			case 'settings':
				panelOpen = !panelOpen;
				return;
			case 'close':
				panelOpen = false;
				return;
		}
	}
</script>

<svelte:head>
	<title>{live && deviceName ? `${deviceName} · lainify` : 'lainify'}</title>
</svelte:head>

<svelte:window
	onkeydown={onKeydown}
	onpointermove={idle.poke}
	onpointerdown={() => {
		audio.resume();
		idle.poke();
	}}
/>

<div
	bind:this={root}
	class={[
		'relative h-dvh w-full overflow-hidden bg-black select-none',
		!uiVisible && 'cursor-none'
	]}
>
	<VideoStage stream={session.stream} bind:video {flash} ondblclick={toggleFullscreen} />

	{#if phase}
		<EmptyState
			{phase}
			error={session.error}
			{deviceName}
			onconnect={connect}
			onsettings={() => (panelOpen = true)}
		/>
	{/if}

	<StatusPill
		visible={uiVisible}
		{live}
		{deviceName}
		video={session.video}
		audio={session.audio}
		fps={frameRate.fps}
		{recording}
		elapsed={recorder.elapsed}
	/>

	{#if live && hasAudio && !audio.running}
		<button
			type="button"
			class="absolute top-16 left-1/2 -translate-x-1/2 rounded-full glass px-4 py-2 text-sm text-zinc-200 transition hover:text-white"
			onclick={audio.resume}
		>
			Click anywhere to enable sound
		</button>
	{/if}

	{#if booted}
		<ControlBar
			visible={uiVisible}
			{live}
			{hasAudio}
			bind:volume={settings.volume}
			bind:muted={settings.muted}
			levels={audio.levels}
			{recording}
			saving={recorder.state === 'saving'}
			elapsed={recorder.elapsed}
			canRecord={live && recorder.supported}
			{fullscreen}
			{panelOpen}
			onpower={() => (live || session.status === 'starting' ? stopCapture() : connect())}
			onscreenshot={screenshot}
			onrecord={toggleRecording}
			onsettings={() => (panelOpen = !panelOpen)}
			onfullscreen={toggleFullscreen}
			onhover={(hovering) => (barHovered = hovering)}
		/>
	{/if}

	<SettingsPanel
		bind:open={panelOpen}
		{videoOptions}
		{audioOptions}
		{modeOptions}
		videoValue={session.video?.deviceId ??
			pickVideoDevice(devices.videoInputs, settings.videoId) ??
			''}
		audioValue={settings.audioId ?? 'auto'}
		modeValue={settings.modeId}
		modeLocked={recording}
		{videoInfo}
		{audioInfo}
		recordFormat={describeMime(recorder.mimeType)}
		canSaveToFile={canSaveToFile()}
		onvideo={selectVideo}
		onaudio={selectAudio}
		onmode={selectMode}
	/>

	<Toasts {toasts} />
</div>

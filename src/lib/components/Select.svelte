<script module lang="ts">
	export interface SelectOption {
		value: string;
		label: string;
		disabled?: boolean;
	}
</script>

<script lang="ts">
	interface Props {
		label: string;
		value: string;
		options: SelectOption[];
		disabled?: boolean;
		hint?: string;
		onchange: (value: string) => void;
	}

	let { label, value, options, disabled = false, hint, onchange }: Props = $props();
	const id = $props.id();
</script>

<div class="flex flex-col gap-1.5">
	<label for={id} class="text-xs font-medium text-zinc-400">{label}</label>
	<select {id} class="field" {value} {disabled} onchange={(e) => onchange(e.currentTarget.value)}>
		{#each options as option (option.value)}
			<option value={option.value} disabled={option.disabled}>{option.label}</option>
		{/each}
	</select>
	{#if hint}<p class="text-xs leading-relaxed text-zinc-500">{hint}</p>{/if}
</div>

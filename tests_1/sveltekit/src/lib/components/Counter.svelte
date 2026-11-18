<script lang="ts">
	type Props = {
		label: string;
		initial?: number;
		step?: number;
		min?: number;
		max?: number;
		disabled?: boolean;
		/** opcional: bind:value para o pai ler a quantidade */
		value?: number;
	};

	let {
		label,
		initial = 0,
		step = 1,
		min = -Infinity,
		max = Infinity,
		disabled = false,
		value = $bindable(initial)
	}: Props = $props();

	const id = $props.id();
	const noMinimo = $derived(disabled || value <= min);
	const noMaximo = $derived(disabled || value >= max);
</script>

<div class="counter" role="group" aria-labelledby="{id}-label">
	<span id="{id}-label" class="label">{label}</span>
	<button
		type="button"
		aria-label="Diminuir {label}"
		disabled={noMinimo}
		onclick={() => (value = Math.max(min, value - step))}>−</button
	>
	<output aria-live="polite">{value}</output>
	<button
		type="button"
		aria-label="Aumentar {label}"
		disabled={noMaximo}
		onclick={() => (value = Math.min(max, value + step))}>+</button
	>
</div>

<style>
	@layer components {
		.counter {
			display: inline-flex;
			align-items: center;
			gap: var(--space-2);
		}
		.label {
			color: var(--color-muted);
			margin-inline-end: var(--space-1);
		}
		button {
			width: 2.25rem;
			padding-inline: 0;
		}
		output {
			min-width: 2.5ch;
			text-align: center;
			font-weight: 600;
			font-variant-numeric: tabular-nums;
		}
	}
</style>

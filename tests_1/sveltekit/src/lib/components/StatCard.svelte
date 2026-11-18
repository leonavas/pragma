<script lang="ts">
	export type Tom = 'neutro' | 'ok' | 'alerta';

	type Props = { title: string; value: string | number; hint?: string; tone?: Tom };
	let { title, value, hint, tone = 'neutro' }: Props = $props();
</script>

<article class="card stat" data-tone={tone}>
	<h3>{title}</h3>
	<p class="value">{value}</p>
	{#if hint}<p class="hint">{hint}</p>{/if}
</article>

<style>
	@layer components {
		.stat {
			border-inline-start: 4px solid var(--tone, var(--color-border));
		}
		.stat[data-tone='ok'] {
			--tone: var(--color-ok);
		}
		.stat[data-tone='alerta'] {
			--tone: var(--color-alerta);
		}
		h3 {
			font-size: var(--text-sm);
			font-weight: 500;
			color: var(--color-muted);
		}
		.value {
			font-size: var(--text-2xl);
			font-weight: 700;
			font-variant-numeric: tabular-nums;
		}
		.hint {
			font-size: var(--text-sm);
			color: var(--tone, var(--color-muted));
		}
	}
</style>

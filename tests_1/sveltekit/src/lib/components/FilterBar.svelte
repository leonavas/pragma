<script lang="ts">
	import { page } from '$app/state';
	import { gravarParam, lerParam } from '$lib/url';

	type Opcao = { valor: string; rotulo: string };
	/** A primeira opção é o padrão e não aparece na URL. */
	type Props = { param: string; opcoes: Opcao[]; rotulo: string };

	let { param, opcoes, rotulo }: Props = $props();

	const valores = $derived(opcoes.map((o) => o.valor));
	const atual = $derived(lerParam(page.url, param, valores));
</script>

<div class="filter-bar" role="group" aria-label={rotulo}>
	{#each opcoes as opcao, i (opcao.valor)}
		<button
			type="button"
			aria-pressed={opcao.valor === atual}
			onclick={() => gravarParam(param, i === 0 ? null : opcao.valor)}>{opcao.rotulo}</button
		>
	{/each}
</div>

<style>
	@layer components {
		.filter-bar {
			display: inline-flex;
			flex-wrap: wrap;
			gap: var(--space-1);
		}
		button[aria-pressed='true'] {
			background: var(--color-primary);
			border-color: var(--color-primary);
			color: var(--color-on-primary);
		}
	}
</style>

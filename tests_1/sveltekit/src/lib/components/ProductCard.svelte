<script lang="ts">
	import Counter from './Counter.svelte';
	import { formatarBRL } from '$lib/format';
	import { carrinho } from '$lib/state/carrinho.svelte';
	import { CATEGORIAS, type Produto } from '$lib/state/catalogo.svelte';

	/** `detalhe`: variante da página /loja/:id (nome vira <h1>, sem link). */
	let { produto, detalhe = false }: { produto: Produto; detalhe?: boolean } = $props();

	let qtd = $state(1);
	const esgotado = $derived(produto.estoque === 0);
	const noCarrinho = $derived(carrinho.qtdDe(produto.id));
	const lotado = $derived(noCarrinho >= produto.estoque);
</script>

<article class="card produto" class:detalhe>
	<header>
		{#if detalhe}
			<h1>{produto.nome}</h1>
		{:else}
			<h3><a href="/loja/{produto.id}">{produto.nome}</a></h3>
		{/if}
		{#if esgotado}<span class="selo">Esgotado</span>{/if}
	</header>
	<p class="muted">{CATEGORIAS[produto.categoria]}</p>
	<p class="preco">{formatarBRL(produto.preco)}</p>
	{#if detalhe}<p class="muted">Estoque: {produto.estoque} un.</p>{/if}
	<div class="acoes">
		<Counter label="Qtd" initial={1} step={1} min={1} max={produto.estoque} disabled={esgotado} bind:value={qtd} />
		<button type="button" class="primary" disabled={esgotado || lotado} onclick={() => carrinho.adicionar(produto, qtd)}>
			Adicionar
		</button>
	</div>
	{#if noCarrinho > 0}
		<p class="hint">{noCarrinho} no carrinho{lotado ? ' (estoque máximo)' : ''}</p>
	{/if}
</article>

<style>
	@layer components {
		.produto {
			display: grid;
			gap: var(--space-2);
			align-content: start;
		}
		header {
			display: flex;
			justify-content: space-between;
			align-items: start;
			gap: var(--space-2);
		}
		h3 {
			font-size: var(--text-base);
		}
		h3 a {
			color: inherit;
		}
		.selo {
			flex: none;
			padding: 0 var(--space-2);
			border-radius: 999px;
			background: var(--color-alerta);
			color: var(--color-on-primary);
			font-size: var(--text-sm);
			font-weight: 600;
		}
		.preco {
			font-size: var(--text-lg);
			font-weight: 700;
		}
		.acoes {
			display: flex;
			flex-wrap: wrap;
			align-items: center;
			justify-content: space-between;
			gap: var(--space-2);
		}
		.hint {
			font-size: var(--text-sm);
			color: var(--color-ok);
		}
	}
</style>

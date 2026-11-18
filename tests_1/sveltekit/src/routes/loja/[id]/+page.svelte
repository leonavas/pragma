<script lang="ts">
	import { untrack } from 'svelte';
	import { page } from '$app/state';
	import CartSummary from '$lib/components/CartSummary.svelte';
	import ProductCard from '$lib/components/ProductCard.svelte';
	import { catalogo } from '$lib/state/catalogo.svelte';

	// usa o mesmo cache da loja: se já carregou, não refaz o fetch
	$effect(() => {
		untrack(() => catalogo.carregar());
	});

	const produto = $derived(catalogo.porId(page.params.id ?? ''));
	const titulo = $derived(
		produto?.nome ?? (catalogo.status === 'ok' ? 'Produto não encontrado' : 'Produto')
	);
</script>

<svelte:head><title>{titulo} · Mercadinho</title></svelte:head>

<p><a href="/loja">← Voltar para a loja</a></p>

<div class="layout-lateral">
	{#if catalogo.status === 'erro'}
		<div class="stack" role="alert">
			<p>Não foi possível carregar o produto. <span class="muted">{catalogo.erro}</span></p>
			<p><button type="button" onclick={() => catalogo.carregar()}>Tentar de novo</button></p>
		</div>
	{:else if catalogo.status === 'carregando'}
		<div class="card stack" aria-busy="true">
			<span class="visually-hidden">Carregando produto…</span>
			<span class="skeleton" style:width="60%" style:height="2rem"></span>
			<span class="skeleton" style:width="30%"></span>
			<span class="skeleton" style:height="2.25rem"></span>
		</div>
	{:else if produto}
		{#key produto.id}<ProductCard {produto} detalhe />{/key}
	{:else}
		<div class="stack">
			<h1>Produto não encontrado</h1>
			<p class="muted">Não há produto com o id “{page.params.id}”.</p>
		</div>
	{/if}
	<CartSummary />
</div>

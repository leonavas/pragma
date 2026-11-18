<script lang="ts">
	import { page } from '$app/state';
	import CartSummary from '$lib/components/CartSummary.svelte';
	import FilterBar from '$lib/components/FilterBar.svelte';
	import ProductGrid from '$lib/components/ProductGrid.svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import { CATEGORIAS } from '$lib/state/catalogo.svelte';
	import { lerParam } from '$lib/url';

	const OPCOES = [
		{ valor: 'todas', rotulo: 'Todas' },
		...Object.entries(CATEGORIAS).map(([valor, rotulo]) => ({ valor, rotulo }))
	];

	let busca = $state('');
	const categoria = $derived(lerParam(page.url, 'categoria', OPCOES.map((o) => o.valor)));
</script>

<svelte:head><title>Loja · Mercadinho</title></svelte:head>

<div class="page-head"><h1>Loja</h1></div>

<div class="toolbar">
	<SearchInput bind:value={busca} label="Buscar" placeholder="Buscar produto…" />
	<FilterBar param="categoria" opcoes={OPCOES} rotulo="Filtrar por categoria" />
</div>

<div class="layout-lateral">
	<ProductGrid {busca} {categoria} />
	<CartSummary />
</div>

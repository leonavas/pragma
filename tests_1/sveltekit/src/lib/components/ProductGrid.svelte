<script lang="ts">
	import { untrack } from 'svelte';
	import { page } from '$app/state';
	import ProductCard from './ProductCard.svelte';
	import { catalogo } from '$lib/state/catalogo.svelte';

	let { busca = '', categoria = 'todas' }: { busca?: string; categoria?: string } = $props();

	const falhar = $derived(page.url.searchParams.get('falhar') === '1');

	// Carrega ao montar (e se ?falhar mudar). O catálogo guarda cache: voltar à loja não refaz o fetch.
	$effect(() => {
		const f = falhar;
		untrack(() => catalogo.carregar({ falhar: f }));
	});

	const visiveis = $derived.by(() => {
		const termo = busca.trim().toLocaleLowerCase('pt-BR');
		return (catalogo.produtos ?? []).filter(
			(p) =>
				(categoria === 'todas' || p.categoria === categoria) &&
				p.nome.toLocaleLowerCase('pt-BR').includes(termo)
		);
	});
</script>

<section class="grid-produtos" aria-label="Produtos" aria-busy={catalogo.status === 'carregando'}>
	{#if catalogo.status === 'erro'}
		<div class="estado erro" role="alert">
			<p>Não foi possível carregar os produtos. <span class="muted">{catalogo.erro}</span></p>
			<button type="button" onclick={() => catalogo.carregar({ falhar })}>Tentar de novo</button>
		</div>
	{:else if catalogo.status === 'carregando'}
		<p class="visually-hidden">Carregando produtos…</p>
		<ul class="grade" aria-hidden="true">
			{#each { length: 6 }, i (i)}
				<li class="card esqueleto">
					<span class="skeleton" style:width="70%"></span>
					<span class="skeleton" style:width="40%"></span>
					<span class="skeleton" style:width="30%" style:height="1.5rem"></span>
					<span class="skeleton" style:height="2.25rem"></span>
				</li>
			{/each}
		</ul>
	{:else if visiveis.length === 0}
		<p class="estado muted">Nenhum produto</p>
	{:else}
		<ul class="grade">
			{#each visiveis as produto (produto.id)}
				<li><ProductCard {produto} /></li>
			{/each}
		</ul>
	{/if}
</section>

<style>
	@layer components {
		.grade {
			display: grid;
			grid-template-columns: repeat(auto-fill, minmax(15rem, 1fr));
			gap: var(--space-3);
			list-style: none;
		}
		.grade > li {
			display: grid;
		}
		.esqueleto {
			display: grid;
			gap: var(--space-3);
		}
		.estado {
			display: grid;
			justify-items: center;
			gap: var(--space-3);
			padding: var(--space-6);
			border: 1px dashed var(--color-border);
			border-radius: var(--radius);
			text-align: center;
		}
		.erro {
			border-color: var(--color-danger);
		}
	}
</style>

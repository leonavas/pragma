<script lang="ts">
	import { formatarBRL } from '$lib/format';
	import { carrinho } from '$lib/state/carrinho.svelte';
</script>

<aside class="card carrinho" aria-labelledby="carrinho-titulo">
	<h2 id="carrinho-titulo">Carrinho</h2>
	{#if carrinho.itens.length === 0}
		<p class="muted">Carrinho vazio.</p>
	{:else}
		<ul>
			{#each carrinho.itens as item (item.id)}
				<li>
					<span class="nome">{item.nome}</span>
					<span class="qtd" role="group" aria-label="Quantidade de {item.nome}">
						<button type="button" aria-label="Diminuir" disabled={item.qtd <= 1} onclick={() => carrinho.alterar(item.id, -1)}>−</button>
						<output>{item.qtd}</output>
						<button type="button" aria-label="Aumentar" disabled={item.qtd >= item.estoque} onclick={() => carrinho.alterar(item.id, 1)}>+</button>
					</span>
					<span class="subtotal">{formatarBRL(item.qtd * item.preco)}</span>
					<button type="button" class="ghost danger" onclick={() => carrinho.remover(item.id)}>
						Remover<span class="visually-hidden"> {item.nome}</span>
					</button>
				</li>
			{/each}
		</ul>
		<p class="total">Total <strong>{formatarBRL(carrinho.total)}</strong></p>
		<button type="button" class="danger" onclick={() => carrinho.esvaziar()}>Esvaziar</button>
	{/if}
</aside>

<style>
	@layer components {
		.carrinho {
			display: grid;
			gap: var(--space-3);
			align-content: start;
		}
		h2 {
			font-size: var(--text-lg);
		}
		ul {
			list-style: none;
			display: grid;
			gap: var(--space-3);
		}
		li {
			display: grid;
			grid-template-columns: 1fr auto;
			gap: var(--space-1) var(--space-2);
			align-items: center;
			padding-bottom: var(--space-3);
			border-bottom: 1px solid var(--color-border);
		}
		.nome {
			grid-column: 1 / -1;
			font-weight: 500;
		}
		.qtd {
			display: inline-flex;
			align-items: center;
			gap: var(--space-2);
		}
		.qtd button {
			width: 2rem;
			padding-inline: 0;
		}
		output {
			min-width: 2ch;
			text-align: center;
			font-variant-numeric: tabular-nums;
		}
		li > button {
			justify-self: start;
			padding-inline: 0;
		}
		.subtotal {
			text-align: end;
			font-variant-numeric: tabular-nums;
		}
		.total {
			display: flex;
			justify-content: space-between;
			font-size: var(--text-lg);
		}
	}
</style>

<script lang="ts">
	import { carrinho } from '$lib/state/carrinho.svelte';
</script>

<a class="cart-badge" href="/loja">
	Carrinho
	<span class="count">
		{carrinho.quantidade}<span class="visually-hidden"> {carrinho.quantidade === 1 ? 'item' : 'itens'}</span>
	</span>
</a>

<style>
	@layer components {
		.cart-badge {
			display: inline-flex;
			align-items: center;
			gap: var(--space-2);
			color: var(--color-text);
			text-decoration: none;
			font-weight: 500;
		}
		.count {
			min-width: 1.75rem;
			padding: 0 var(--space-2);
			border-radius: 999px;
			background: var(--color-accent);
			color: var(--color-on-primary);
			font-size: var(--text-sm);
			text-align: center;
			font-variant-numeric: tabular-nums;
		}
	}
</style>

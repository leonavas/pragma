<script lang="ts">
	import CartBadge from './CartBadge.svelte';
	import NavLink from './NavLink.svelte';
</script>

<header class="app-header">
	<div class="container bar">
		<a class="brand" href="/">Mercadinho</a>
		<nav aria-label="Principal">
			<ul>
				<li><NavLink href="/">Painel</NavLink></li>
				<li><NavLink href="/tarefas">Tarefas</NavLink></li>
				<li><NavLink href="/loja">Loja</NavLink></li>
			</ul>
		</nav>
		<CartBadge />
	</div>
</header>

<style>
	@layer components {
		.app-header {
			border-bottom: 1px solid var(--color-border);
			background: var(--color-surface);
		}
		.bar {
			display: flex;
			flex-wrap: wrap;
			align-items: center;
			gap: var(--space-3) var(--space-5);
			padding-block: var(--space-3);
		}
		.brand {
			font-weight: 700;
			font-size: var(--text-lg);
			color: var(--color-text);
			text-decoration: none;
		}
		nav {
			flex: 1;
		}
		ul {
			display: flex;
			gap: var(--space-1);
			list-style: none;
		}
	}
</style>

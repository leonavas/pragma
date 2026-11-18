<script lang="ts">
	import type { Snippet } from 'svelte';
	import { page } from '$app/state';

	let { href, children }: { href: string; children: Snippet } = $props();

	// "page" na rota exata; "true" em subrotas (ex.: /loja/cafe destaca "Loja").
	const atual = $derived.by(() => {
		const caminho = page.url.pathname;
		if (caminho === href) return 'page';
		if (href !== '/' && caminho.startsWith(href + '/')) return 'true';
		return undefined;
	});
</script>

<a {href} aria-current={atual}>{@render children()}</a>

<style>
	@layer components {
		a {
			display: inline-block;
			padding: var(--space-1) var(--space-3);
			border-radius: var(--radius-sm);
			color: var(--color-text);
			text-decoration: none;
			font-weight: 500;
		}
		a:hover {
			background: var(--color-surface-2);
		}
		a[aria-current] {
			background: var(--color-primary);
			color: var(--color-on-primary);
		}
	}
</style>

<script lang="ts">
	import '../styles/app.css';
	import AppHeader from '$lib/components/AppHeader.svelte';
	import type { LayoutProps } from './$types';

	let { children }: LayoutProps = $props();
</script>

<a class="skip-link" href="#conteudo">Pular para o conteúdo</a>
<AppHeader />
<main id="conteudo" class="container">
	{@render children()}
</main>

<script lang="ts">
	import { page } from '$app/state';

	const naoEncontrada = $derived(page.status === 404);
</script>

<svelte:head>
	<title>{naoEncontrada ? 'Página não encontrada' : 'Erro'} · Mercadinho</title>
</svelte:head>

<section class="stack">
	<h1>{naoEncontrada ? 'Página não encontrada' : 'Algo deu errado'}</h1>
	<p class="muted">
		{naoEncontrada ? `Não existe nada em ${page.url.pathname}.` : page.error?.message}
	</p>
	<p><a href="/">Voltar para o início</a></p>
</section>

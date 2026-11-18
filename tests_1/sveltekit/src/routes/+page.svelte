<script lang="ts">
	import Clock from '$lib/components/Clock.svelte';
	import Counter from '$lib/components/Counter.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import Tabs from '$lib/components/Tabs.svelte';
	import ThemeToggle from '$lib/components/ThemeToggle.svelte';
	import { formatarBRL } from '$lib/format';
	import { carrinho } from '$lib/state/carrinho.svelte';
	import { todos } from '$lib/state/todos.svelte';

	let sobreAberto = $state(false);
	const carrinhoVazio = $derived(carrinho.quantidade === 0);
</script>

<svelte:head><title>Painel · Mercadinho</title></svelte:head>

<div class="page-head">
	<h1>Painel</h1>
	<div class="cluster">
		<Clock />
		<ThemeToggle />
		<button type="button" onclick={() => (sobreAberto = true)}>Sobre o app</button>
	</div>
</div>

<section class="stack" aria-labelledby="resumo">
	<h2 id="resumo">Resumo</h2>
	<div class="grid-cards">
		<StatCard
			title="Tarefas pendentes"
			value={todos.pendentes}
			hint={todos.pendentes === 0 ? 'Nada pendente' : `de ${todos.total} tarefas`}
			tone={todos.pendentes === 0 ? 'alerta' : 'neutro'}
		/>
		<StatCard
			title="Itens no carrinho"
			value={carrinho.quantidade}
			hint={carrinhoVazio ? 'Carrinho vazio' : undefined}
			tone={carrinhoVazio ? 'alerta' : 'neutro'}
		/>
		<StatCard
			title="Total do carrinho"
			value={formatarBRL(carrinho.total)}
			tone={carrinhoVazio ? 'alerta' : 'neutro'}
		/>
	</div>
</section>

<section class="stack" aria-labelledby="contadores">
	<h2 id="contadores">Contadores</h2>
	<div class="cluster card">
		<Counter label="Cliques" initial={0} step={1} />
		<Counter label="Pulos de 5" initial={10} step={5} min={0} max={50} />
	</div>
</section>

<section class="stack" aria-labelledby="mais">
	<h2 id="mais">Mais</h2>
	<div class="card">
		<Tabs
			rotulo="Informações"
			abas={[
				{ id: 'sobre', rotulo: 'Sobre', conteudo: sobre },
				{ id: 'atalhos', rotulo: 'Atalhos', conteudo: atalhos },
				{ id: 'contador', rotulo: 'Contador', conteudo: contador }
			]}
		/>
	</div>
</section>

{#snippet sobre()}
	<p>Mercadinho é uma SPA de exemplo: painel, tarefas e uma lojinha com carrinho.</p>
{/snippet}
{#snippet atalhos()}
	<ul class="lista">
		<li><kbd>←</kbd> <kbd>→</kbd> trocam de aba (<kbd>Home</kbd>/<kbd>End</kbd> vão às pontas).</li>
		<li><kbd>Esc</kbd> fecha o diálogo.</li>
		<li>Duplo clique numa tarefa edita; <kbd>Enter</kbd> salva, <kbd>Esc</kbd> cancela.</li>
	</ul>
{/snippet}
{#snippet contador()}
	<Counter label="Na aba" initial={3} step={3} />
{/snippet}

<Modal bind:open={sobreAberto} title="Sobre o Mercadinho">
	<p>Feito com Svelte 5 (runes) e SvelteKit 2, como SPA estática.</p>
	<p class="muted">Tarefas, carrinho e tema ficam salvos no <code>localStorage</code>.</p>
</Modal>

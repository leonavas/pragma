<script lang="ts">
	import { page } from '$app/state';
	import FilterBar from '$lib/components/FilterBar.svelte';
	import Modal from '$lib/components/Modal.svelte';
	import SearchInput from '$lib/components/SearchInput.svelte';
	import StatCard from '$lib/components/StatCard.svelte';
	import TodoForm from '$lib/components/TodoForm.svelte';
	import TodoList from '$lib/components/TodoList.svelte';
	import { todos } from '$lib/state/todos.svelte';
	import { lerParam } from '$lib/url';

	const FILTROS = [
		{ valor: 'todas', rotulo: 'Todas' },
		{ valor: 'pendentes', rotulo: 'Pendentes' },
		{ valor: 'concluidas', rotulo: 'Concluídas' }
	];

	let busca = $state('');
	let confirmando = $state(false);

	const filtro = $derived(lerParam(page.url, 'filtro', FILTROS.map((f) => f.valor)));
	const visiveis = $derived.by(() => {
		const termo = busca.trim().toLocaleLowerCase('pt-BR');
		return todos.itens.filter(
			(t) =>
				(filtro === 'todas' || (filtro === 'concluidas') === t.feita) &&
				t.titulo.toLocaleLowerCase('pt-BR').includes(termo)
		);
	});
	const tudoFeito = $derived(todos.total > 0 && todos.concluidas === todos.total);
</script>

<svelte:head><title>Tarefas · Mercadinho</title></svelte:head>

<div class="page-head">
	<h1>Tarefas</h1>
	<button type="button" class="danger" disabled={todos.concluidas === 0} onclick={() => (confirmando = true)}>
		Limpar concluídas
	</button>
</div>

<div class="layout-lateral">
	<div class="stack">
		<TodoForm validar={(t) => todos.validar(t)} onadicionar={(t) => todos.adicionar(t)} />
		<div class="toolbar">
			<SearchInput bind:value={busca} label="Buscar" placeholder="Buscar tarefa…" />
			<FilterBar param="filtro" opcoes={FILTROS} rotulo="Filtrar tarefas" />
		</div>
		<TodoList itens={visiveis} />
	</div>
	<StatCard
		title="Concluídas"
		value="{todos.concluidas} de {todos.total}"
		tone={tudoFeito ? 'ok' : 'neutro'}
		hint={tudoFeito ? 'Tudo feito!' : undefined}
	/>
</div>

<Modal bind:open={confirmando} title="Limpar concluídas?">
	<p>
		{todos.concluidas === 1 ? '1 tarefa concluída será removida' : `${todos.concluidas} tarefas concluídas serão removidas`}.
		Essa ação não pode ser desfeita.
	</p>
	{#snippet acoes(fechar)}
		<button type="button" onclick={fechar}>Cancelar</button>
		<button
			type="button"
			class="danger"
			onclick={() => {
				todos.limparConcluidas();
				fechar();
			}}>Confirmar</button
		>
	{/snippet}
</Modal>

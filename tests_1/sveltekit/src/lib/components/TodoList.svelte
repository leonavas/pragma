<script lang="ts">
	import { todos, type Todo } from '$lib/state/todos.svelte';

	let { itens }: { itens: Todo[] } = $props();

	let editando = $state<string | null>(null);
	let rascunho = $state('');
	let erro = $state<string | null>(null);

	function editar(t: Todo) {
		editando = t.id;
		rascunho = t.titulo;
		erro = null;
	}

	function cancelar() {
		editando = null;
		erro = null;
	}

	function salvar(id: string) {
		erro = todos.validar(rascunho, id);
		if (erro) return;
		todos.renomear(id, rascunho);
		editando = null;
	}

	function teclas(e: KeyboardEvent, id: string) {
		if (e.key === 'Enter') {
			e.preventDefault();
			salvar(id);
		} else if (e.key === 'Escape') {
			e.preventDefault();
			cancelar();
		}
	}

	// attachment: foca e seleciona o campo de edição ao aparecer
	const focar = (el: HTMLInputElement) => {
		el.focus();
		el.select();
	};
</script>

{#if itens.length === 0}
	<p class="vazio">Nenhuma tarefa</p>
{:else}
	<ul class="todo-list">
		{#each itens as todo (todo.id)}
			<li class:feita={todo.feita}>
				{#if editando === todo.id}
					<div class="edicao">
						<input
							bind:value={rascunho}
							{@attach focar}
							onkeydown={(e) => teclas(e, todo.id)}
							onblur={cancelar}
							aria-label="Editar tarefa “{todo.titulo}”"
							aria-invalid={erro ? true : undefined}
							aria-describedby={erro ? `erro-${todo.id}` : undefined}
						/>
						{#if erro}<p id="erro-{todo.id}" class="field-error" role="alert">{erro}</p>{/if}
					</div>
				{:else}
					<input
						type="checkbox"
						checked={todo.feita}
						onchange={() => todos.alternar(todo.id)}
						aria-label="Concluir “{todo.titulo}”"
					/>
					<!-- duplo clique edita; o botão "Editar" é a alternativa por teclado -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<span class="titulo" ondblclick={() => editar(todo)}>{todo.titulo}</span>
					<button type="button" class="ghost" onclick={() => editar(todo)}>
						Editar<span class="visually-hidden"> “{todo.titulo}”</span>
					</button>
					<button type="button" class="ghost danger" onclick={() => todos.remover(todo.id)}>
						Remover<span class="visually-hidden"> “{todo.titulo}”</span>
					</button>
				{/if}
			</li>
		{/each}
	</ul>
{/if}

<style>
	@layer components {
		.todo-list {
			list-style: none;
			border: 1px solid var(--color-border);
			border-radius: var(--radius);
			background: var(--color-surface);
		}
		li {
			display: flex;
			align-items: center;
			gap: var(--space-3);
			padding: var(--space-2) var(--space-3);
			min-height: 3.25rem;
		}
		li + li {
			border-top: 1px solid var(--color-border);
		}
		input[type='checkbox'] {
			width: 1.15rem;
			height: 1.15rem;
			accent-color: var(--color-primary);
		}
		.titulo {
			flex: 1;
			cursor: text;
			user-select: none;
		}
		.feita .titulo {
			color: var(--color-muted);
			text-decoration: line-through;
		}
		.edicao {
			flex: 1;
			display: grid;
			gap: var(--space-1);
		}
		.vazio {
			padding: var(--space-5);
			text-align: center;
			color: var(--color-muted);
			border: 1px dashed var(--color-border);
			border-radius: var(--radius);
		}
	}
</style>

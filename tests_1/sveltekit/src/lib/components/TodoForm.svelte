<script lang="ts">
	type Props = {
		/** retorna a mensagem de erro ou null */
		validar: (titulo: string) => string | null;
		onadicionar: (titulo: string) => void;
	};
	let { validar, onadicionar }: Props = $props();

	const id = $props.id();
	let titulo = $state('');
	let erro = $state<string | null>(null);
	let campo: HTMLInputElement;

	function onsubmit(e: SubmitEvent) {
		e.preventDefault();
		erro = validar(titulo);
		if (!erro) {
			onadicionar(titulo);
			titulo = '';
		}
		campo.focus();
	}
</script>

<form class="todo-form" novalidate {onsubmit}>
	<label for={id}>Nova tarefa</label>
	<div class="linha">
		<input
			{id}
			bind:this={campo}
			bind:value={titulo}
			oninput={() => (erro = null)}
			autocomplete="off"
			aria-invalid={erro ? true : undefined}
			aria-describedby={erro ? `${id}-erro` : undefined}
		/>
		<button type="submit" class="primary">Adicionar</button>
	</div>
	{#if erro}<p id="{id}-erro" class="field-error" role="alert">{erro}</p>{/if}
</form>

<style>
	@layer components {
		.todo-form {
			display: grid;
			gap: var(--space-1);
		}
		.linha {
			display: flex;
			gap: var(--space-2);
		}
		input {
			flex: 1;
		}
	}
</style>

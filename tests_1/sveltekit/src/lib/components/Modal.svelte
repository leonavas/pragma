<script lang="ts">
	import type { Snippet } from 'svelte';

	type Props = {
		open?: boolean;
		title: string;
		children: Snippet;
		/** botões do rodapé; recebe `fechar`. Sem ele, mostra só "Fechar". */
		acoes?: Snippet<[fechar: () => void]>;
		onclose?: () => void;
	};

	let { open = $bindable(false), title, children, acoes, onclose }: Props = $props();

	const id = $props.id();
	let dialog: HTMLDialogElement;
	let origem: HTMLElement | null = null;

	const fechar = () => (open = false);

	// Sincroniza a prop `open` com o <dialog> nativo.
	$effect(() => {
		if (open && !dialog.open) {
			origem = document.activeElement as HTMLElement | null;
			dialog.showModal();
		} else if (!open && dialog.open) {
			dialog.close();
		}
	});

	// Dispara em qualquer fechamento (Esc, botão, fundo): avisa o pai e devolve o foco.
	function aoFechar() {
		open = false;
		origem?.focus();
		origem = null;
		onclose?.();
	}
</script>

<!-- clique no fundo: o alvo é o próprio <dialog> (o conteúdo fica num div interno);
     Esc já é tratado nativamente pelo <dialog>, por isso não há handler de teclado aqui -->
<!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_noninteractive_element_interactions -->
<dialog
	bind:this={dialog}
	aria-labelledby="{id}-titulo"
	onclose={aoFechar}
	onclick={(e) => e.target === dialog && fechar()}
>
	<div class="conteudo">
		<h2 id="{id}-titulo">{title}</h2>
		<div class="corpo">{@render children()}</div>
		<div class="acoes">
			{#if acoes}{@render acoes(fechar)}{:else}<button type="button" onclick={fechar}>Fechar</button>{/if}
		</div>
	</div>
</dialog>

<style>
	@layer components {
		dialog {
			margin: auto;
			width: min(32rem, calc(100% - 2 * var(--space-4)));
			border: 1px solid var(--color-border);
			border-radius: var(--radius);
			background: var(--color-surface);
			color: var(--color-text);
			box-shadow: var(--shadow);
		}
		dialog::backdrop {
			background: rgb(0 0 0 / 0.45);
		}
		.conteudo {
			display: grid;
			gap: var(--space-4);
			padding: var(--space-5);
		}
		h2 {
			font-size: var(--text-lg);
		}
		.acoes {
			display: flex;
			justify-content: flex-end;
			gap: var(--space-2);
		}
	}
</style>

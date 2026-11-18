<script lang="ts">
	import type { Snippet } from 'svelte';

	export type Aba = { id: string; rotulo: string; conteudo: Snippet };
	type Props = { abas: Aba[]; rotulo: string; ativa?: string };

	let { abas, rotulo, ativa = $bindable(abas[0]?.id) }: Props = $props();

	const uid = $props.id();
	const botoes: HTMLButtonElement[] = [];

	// Padrão ARIA tabs com ativação automática: ← → (circular), Home, End.
	function onkeydown(e: KeyboardEvent) {
		const n = abas.length;
		const i = abas.findIndex((a) => a.id === ativa);
		const destino =
			e.key === 'ArrowRight' ? (i + 1) % n
			: e.key === 'ArrowLeft' ? (i - 1 + n) % n
			: e.key === 'Home' ? 0
			: e.key === 'End' ? n - 1
			: -1;
		if (destino < 0) return;
		e.preventDefault();
		ativa = abas[destino].id;
		botoes[destino]?.focus();
	}
</script>

<div class="tabs">
	<div role="tablist" aria-label={rotulo}>
		{#each abas as aba, i (aba.id)}
			<button
				bind:this={botoes[i]}
				type="button"
				role="tab"
				id="{uid}-tab-{aba.id}"
				aria-controls="{uid}-painel-{aba.id}"
				aria-selected={aba.id === ativa}
				tabindex={aba.id === ativa ? 0 : -1}
				onclick={() => (ativa = aba.id)}
				{onkeydown}>{aba.rotulo}</button
			>
		{/each}
	</div>
	<!-- todos os painéis ficam montados (hidden): o Counter da aba mantém o valor ao trocar -->
	{#each abas as aba (aba.id)}
		<div
			role="tabpanel"
			id="{uid}-painel-{aba.id}"
			aria-labelledby="{uid}-tab-{aba.id}"
			hidden={aba.id !== ativa}
			tabindex="0"
		>
			{@render aba.conteudo()}
		</div>
	{/each}
</div>

<style>
	@layer components {
		[role='tablist'] {
			display: flex;
			gap: var(--space-1);
			border-bottom: 1px solid var(--color-border);
		}
		[role='tab'] {
			border: 0;
			border-bottom: 3px solid transparent;
			border-radius: var(--radius-sm) var(--radius-sm) 0 0;
			background: none;
		}
		[role='tab'][aria-selected='true'] {
			border-bottom-color: var(--color-primary);
			font-weight: 600;
		}
		[role='tabpanel'] {
			padding: var(--space-4) var(--space-1);
		}
	}
</style>

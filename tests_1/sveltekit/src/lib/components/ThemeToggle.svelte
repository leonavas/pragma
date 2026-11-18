<script lang="ts">
	import { CHAVE_TEMA, tema } from '$lib/state/theme.svelte';

	// Efeito no DOM externo: reflete o tema no <html> e persiste.
	$effect(() => {
		document.documentElement.dataset.theme = tema.atual;
		localStorage.setItem(CHAVE_TEMA, tema.atual);
	});
</script>

<button type="button" aria-pressed={tema.atual === 'dark'} onclick={() => tema.alternar()}>
	Tema escuro
</button>

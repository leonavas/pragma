<script lang="ts">
	let agora = $state(new Date());

	// Efeito com limpeza: o intervalo vive enquanto o componente estiver montado.
	$effect(() => {
		const timer = setInterval(() => (agora = new Date()), 1000);
		return () => clearInterval(timer);
	});

	const hora = $derived(
		agora.toLocaleTimeString('pt-BR', { hour: '2-digit', minute: '2-digit', second: '2-digit', hour12: false })
	);
</script>

<p class="clock">
	<span class="muted">Agora</span>
	<time datetime={agora.toISOString()}>{hora}</time>
</p>

<style>
	@layer components {
		.clock {
			display: inline-flex;
			gap: var(--space-2);
			align-items: baseline;
		}
		time {
			font-family: var(--font-mono);
			font-size: var(--text-lg);
			font-variant-numeric: tabular-nums;
		}
	}
</style>

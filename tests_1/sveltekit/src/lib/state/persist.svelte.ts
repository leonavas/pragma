/** Lê um valor JSON do localStorage, com fallback se ausente ou inválido. */
export function carregar<T>(chave: string, padrao: T): T {
	try {
		const bruto = localStorage.getItem(chave);
		return bruto === null ? padrao : (JSON.parse(bruto) as T);
	} catch {
		return padrao;
	}
}

/**
 * Salva `ler()` no localStorage sempre que o estado lido mudar.
 * $effect.root porque roda fora de componente (escopo do módulo, vive o app inteiro).
 */
export function persistir<T>(chave: string, ler: () => T) {
	$effect.root(() => {
		$effect(() => {
			localStorage.setItem(chave, JSON.stringify(ler()));
		});
	});
}

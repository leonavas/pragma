export type Categoria = 'mercearia' | 'limpeza' | 'hortifruti';
export type Produto = { id: string; nome: string; categoria: Categoria; preco: number; estoque: number };

export const CATEGORIAS: Record<Categoria, string> = {
	mercearia: 'Mercearia',
	limpeza: 'Limpeza',
	hortifruti: 'Hortifruti'
};

const ATRASO_MS = 400;
const esperar = (ms: number) => new Promise((r) => setTimeout(r, ms));

class CatalogoState {
	/** Cache: preenchido uma vez e reaproveitado em toda visita à loja. */
	produtos = $state.raw<Produto[] | null>(null);
	status = $state<'carregando' | 'ok' | 'erro'>('carregando');
	erro = $state('');
	#pendente: Promise<void> | null = null;

	/** `falhar` simula erro (via ?falhar=1) mesmo com cache. */
	carregar({ falhar = false } = {}): Promise<void> {
		if (this.produtos && !falhar) {
			this.status = 'ok';
			return Promise.resolve();
		}
		if (this.#pendente) return this.#pendente;
		this.status = 'carregando';
		this.erro = '';
		this.#pendente = (async () => {
			try {
				await esperar(ATRASO_MS);
				if (falhar) throw new Error('Falha simulada (?falhar=1).');
				const res = await fetch('/produtos.json');
				if (!res.ok) throw new Error(`HTTP ${res.status}`);
				this.produtos = await res.json();
				this.status = 'ok';
			} catch (e) {
				this.erro = e instanceof Error ? e.message : String(e);
				this.status = 'erro';
			} finally {
				this.#pendente = null;
			}
		})();
		return this.#pendente;
	}

	porId(id: string) {
		return this.produtos?.find((p) => p.id === id);
	}
}

export const catalogo = new CatalogoState();

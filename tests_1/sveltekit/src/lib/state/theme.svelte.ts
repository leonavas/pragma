export type Tema = 'light' | 'dark';

/** Mesma chave lida pelo script inline de src/app.html. */
export const CHAVE_TEMA = 'mercadinho:tema';

class TemaState {
	// o script do app.html já resolveu o tema inicial (salvo ou preferência do sistema)
	atual = $state<Tema>(document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light');

	alternar() {
		this.atual = this.atual === 'dark' ? 'light' : 'dark';
	}
}

export const tema = new TemaState();

import { carregar, persistir } from './persist.svelte';

export type Todo = { id: string; titulo: string; feita: boolean };

const CHAVE = 'mercadinho:tarefas';
const INICIAIS: Todo[] = [
	{ id: 't1', titulo: 'Comprar café', feita: false },
	{ id: 't2', titulo: 'Pagar contas', feita: true },
	{ id: 't3', titulo: 'Ligar para a farmácia', feita: false }
];

const normalizar = (s: string) => s.trim().toLocaleLowerCase('pt-BR');

class TodosState {
	itens = $state<Todo[]>(carregar(CHAVE, INICIAIS));
	total = $derived(this.itens.length);
	concluidas = $derived(this.itens.filter((t) => t.feita).length);
	pendentes = $derived(this.total - this.concluidas);

	constructor() {
		persistir(CHAVE, () => this.itens);
	}

	/** Retorna a mensagem de erro, ou null se o título é válido. */
	validar(titulo: string, ignorarId?: string): string | null {
		const t = titulo.trim();
		if (!t) return 'Informe o título da tarefa.';
		if (t.length < 3) return 'O título precisa ter pelo menos 3 caracteres.';
		if (t.length > 80) return 'O título pode ter no máximo 80 caracteres.';
		const alvo = normalizar(t);
		if (this.itens.some((x) => x.id !== ignorarId && normalizar(x.titulo) === alvo))
			return 'Já existe uma tarefa com esse título.';
		return null;
	}

	adicionar(titulo: string) {
		this.itens.push({ id: crypto.randomUUID(), titulo: titulo.trim(), feita: false });
	}

	alternar(id: string) {
		const t = this.itens.find((x) => x.id === id);
		if (t) t.feita = !t.feita;
	}

	renomear(id: string, titulo: string) {
		const t = this.itens.find((x) => x.id === id);
		if (t) t.titulo = titulo.trim();
	}

	remover(id: string) {
		this.itens = this.itens.filter((x) => x.id !== id);
	}

	limparConcluidas() {
		this.itens = this.itens.filter((x) => !x.feita);
	}
}

export const todos = new TodosState();

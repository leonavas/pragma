import type { Produto } from './catalogo.svelte';
import { carregar, persistir } from './persist.svelte';

export type ItemCarrinho = Pick<Produto, 'id' | 'nome' | 'preco' | 'estoque'> & { qtd: number };

const CHAVE = 'mercadinho:carrinho';
const limitar = (n: number, min: number, max: number) => Math.min(max, Math.max(min, n));

class CarrinhoState {
	itens = $state<ItemCarrinho[]>(carregar(CHAVE, []));
	quantidade = $derived(this.itens.reduce((s, i) => s + i.qtd, 0));
	total = $derived(this.itens.reduce((s, i) => s + i.qtd * i.preco, 0));

	constructor() {
		persistir(CHAVE, () => this.itens);
	}

	qtdDe(id: string) {
		return this.itens.find((i) => i.id === id)?.qtd ?? 0;
	}

	/** Soma `qtd` ao item; a quantidade nunca passa do estoque. */
	adicionar(p: Produto, qtd: number) {
		if (p.estoque <= 0 || qtd <= 0) return;
		const item = this.itens.find((i) => i.id === p.id);
		if (item) item.qtd = limitar(item.qtd + qtd, 1, item.estoque);
		else {
			const { id, nome, preco, estoque } = p;
			this.itens.push({ id, nome, preco, estoque, qtd: limitar(qtd, 1, estoque) });
		}
	}

	alterar(id: string, delta: number) {
		const item = this.itens.find((i) => i.id === id);
		if (item) item.qtd = limitar(item.qtd + delta, 1, item.estoque);
	}

	remover(id: string) {
		this.itens = this.itens.filter((i) => i.id !== id);
	}

	esvaziar() {
		this.itens = [];
	}
}

export const carrinho = new CarrinhoState();

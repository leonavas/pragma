import { goto } from '$app/navigation';
import { page } from '$app/state';

/** Lê um filtro da URL; valores desconhecidos caem no primeiro (o padrão). */
export function lerParam<T extends string>(url: URL, nome: string, valores: readonly T[]): T {
	const v = url.searchParams.get(nome) as T | null;
	return v !== null && valores.includes(v) ? v : valores[0];
}

/** Grava (ou remove, com null) um parâmetro da URL atual, criando uma entrada no histórico. */
export function gravarParam(nome: string, valor: string | null) {
	const url = new URL(page.url);
	if (valor === null) url.searchParams.delete(nome);
	else url.searchParams.set(nome, valor);
	return goto(url, { keepFocus: true, noScroll: true });
}

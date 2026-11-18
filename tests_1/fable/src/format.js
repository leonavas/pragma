const currency = new Intl.NumberFormat('pt-BR', { style: 'currency', currency: 'BRL' });

export const brl = (value) => currency.format(value);

/** Minúsculas e sem acentos, para busca ("maca" encontra "Maçã"). */
export const normalize = (text) =>
  text
    .normalize('NFD')
    .replace(/\p{Diacritic}/gu, '')
    .toLowerCase()
    .trim();

let nextId = 0;
/** Id único para ligar label/aria a um elemento. */
export const uid = (prefix) => `${prefix}-${++nextId}`;

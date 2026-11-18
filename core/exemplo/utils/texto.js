const normalizar = s => s.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase()

export const contem = (texto, busca) => normalizar(texto).includes(normalizar(busca.trim()))

export const marcar = (opcoes, atual) => Object.fromEntries(opcoes.map(o => [o, o === atual && 'true']))

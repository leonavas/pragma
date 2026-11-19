export const marcar = (opcoes, atual) => Object.fromEntries(opcoes.map(o => [o, o === atual && 'true']))

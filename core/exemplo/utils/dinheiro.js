const real = new Intl.NumberFormat('pt-BR', { style: 'currency', currency: 'BRL' })

export const brl = valor => real.format(valor)

import produtos from './_produtos.json' with { type: 'json' }

const comUrl = p => ({ ...p, url: `/produtos/${p.id}` })

export const listar = categoria => produtos.filter(p => !categoria || categoria === 'todas' || p.categoria === categoria).map(comUrl)
export const achar = id => produtos.find(p => p.id === id) ?? null

export async function buscar(id) {
  return { id, nome: 'Produto ' + id, preco: 10, url: null, titulo: id, tags: [] }
}

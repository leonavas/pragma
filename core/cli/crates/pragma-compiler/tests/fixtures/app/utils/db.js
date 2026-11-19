export async function find(id) {
  return { id, name: 'Product ' + id, price: 10, url: null, heading: id, tags: [] }
}

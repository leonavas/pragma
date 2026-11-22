export const ssr = true

export async function load() {
  const time = new Date().toISOString().slice(11, 19)
  return { renderedAt: time }
}

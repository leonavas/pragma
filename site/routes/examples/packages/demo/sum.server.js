import addWasm from './_add.wasm'

export const ssr = true

export async function load() {
  const adder = await WebAssembly.instantiate(addWasm)
  return { sum: adder.exports.add(2, 3) }
}

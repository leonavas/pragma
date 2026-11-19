// `export let step = 1`: the default's type decides how an attribute string is read (browser and SSR).
export function coerce(v, d) {
  if (v === undefined) return d
  switch (typeof d) {
    case 'number': return typeof v === 'number' ? v : v === '' ? d : Number(v)
    case 'boolean': return v === true || v === '' || v === 'true' ? true : v === false || v === 'false' ? false : Boolean(v)
    case 'string': return v == null ? d : String(v)
    default: return v
  }
}

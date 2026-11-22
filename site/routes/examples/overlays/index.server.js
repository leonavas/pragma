import { readExamples } from '../_source.server.js'

export const ssr = true

const component = name => [
  { file: `_components/${name}/${name}.html`, as: 'routes/index.html' },
]

export async function load() {
  const examples = await readExamples('routes/examples/overlays', {
    modal: component('about-modal'),
    alert: component('delete-file'),
    dropdown: component('file-sort'),
    stacking: component('note-editor'),
    sheet: component('side-sheet'),
  })
  return { examples }
}

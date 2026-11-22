import { readExamples } from '../_source.server.js'

export const ssr = true

const asPage = name => [
  { file: `_components/${name}/${name}.html`, as: 'routes/index.html' },
]

export async function load() {
  const examples = await readExamples('routes/examples/forms', {
    ticket: asPage('ticket'),
    sizePicker: asPage('size-picker'),
    columnPicker: asPage('column-picker'),
    stayDates: asPage('stay-dates'),
    username: ['demo/username.html', 'demo/username.server.js'],
  })
  return { examples }
}

import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/overlays', {
    modal: ['_components/about-modal/about-modal.html'],
    alert: ['_components/delete-draft/delete-draft.html'],
    dropdown: ['_components/sort-menu/sort-menu.html'],
    stacking: ['_components/rename-project/rename-project.html'],
    library: [
      '../../../lib/components/modal/modal.html',
      '../../../lib/components/alert/alert.html',
      '../../../lib/components/dropdown/dropdown.html',
    ],
  })
  return { examples }
}

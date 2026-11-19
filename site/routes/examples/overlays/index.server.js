import { readExamples } from '../_source.server.js'

export const ssr = true

export async function load() {
  const examples = await readExamples('routes/examples/overlays', {
    modal: ['_components/about-modal/about-modal.html'],
    alert: [
      '_components/delete-draft/delete-draft.html',
      '_components/delete-draft/delete-draft.js',
    ],
    dropdown: [
      '_components/sort-menu/sort-menu.html',
      '_components/sort-menu/sort-menu.js',
    ],
    stacking: [
      '_components/rename-project/rename-project.html',
      '_components/rename-project/rename-project.js',
    ],
    library: [
      '../../../lib/components/modal/modal.html',
      '../../../lib/components/alert/alert.html',
      '../../../lib/components/dropdown/dropdown.html',
      '../../../lib/components/dropdown/dropdown.css',
    ],
  })
  return { examples }
}

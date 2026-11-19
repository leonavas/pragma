import { codeKinds } from '../../../utils/code-lines.js'

const copyResetMs = 1800
const beatGapMs = 400
const timedWindowMs = 1000
const interactions = ['pointerdown', 'click', 'keydown', 'input', 'change', 'submit']
const resetTimers = new Map()
const highlights = new Map()

function examplesOf(ctx) {
  return Object.values(ctx.data?.examples ?? {})
}

function highlightFor(kind) {
  const name = 'code-' + codeKinds[kind]
  if (!highlights.has(name)) {
    highlights.set(name, new Highlight())
    CSS.highlights.set(name, highlights.get(name))
  }
  return highlights.get(name)
}

function paint(root, files) {
  if (!globalThis.CSS?.highlights) return () => {}
  const painted = []
  for (const code of root.querySelectorAll('pre[id]')) {
    const file = files.find(candidate => candidate.id === code.id)
    code.querySelectorAll('.code-line').forEach((line, index) => {
      const marks = file?.lines[index]?.marks ?? []
      if (!line.firstChild) return
      for (let i = 0; i < marks.length; i += 3) {
        const range = new StaticRange({ startContainer: line.firstChild, startOffset: marks[i], endContainer: line.firstChild, endOffset: marks[i + 1] })
        highlightFor(marks[i + 2]).add(range)
        painted.push([marks[i + 2], range])
      }
    })
  }
  return () => {
    for (const [kind, range] of painted) highlightFor(kind).delete(range)
  }
}

function formatMs(elapsedMs) {
  if (elapsedMs < 0.1) return '<0.1 ms'
  return (elapsedMs < 10 ? elapsedMs.toFixed(1) : Math.round(elapsedMs)) + ' ms'
}

function describePatch(count, elapsedMs) {
  const updated = count === 1 ? '1 update' : count + ' updates'
  return elapsedMs === null ? updated : updated + ' · ' + formatMs(elapsedMs)
}

export const state = () => ({
  expanded: false,
  announcement: '',
  patches: '',
  beat: '',
  get expandedText() {
    return this.expanded ? 'true' : 'false'
  },
})

export const actions = {
  select(ctx) {
    const chosen = ctx.scope.file
    const siblings = examplesOf(ctx).find(example => example.files.includes(chosen))?.files ?? [chosen]
    for (const file of siblings) {
      file.active = file === chosen
      file.pressed = file === chosen ? 'true' : 'false'
    }
  },
  async copy(ctx) {
    const file = ctx.scope.file
    clearTimeout(resetTimers.get(file.id))
    try {
      await navigator.clipboard.writeText(file.lines.map(line => line.text).join('\n'))
      file.copied = true
      file.blocked = false
      ctx.$.announcement = 'Copied ' + file.name + ', ' + file.lines.length + ' lines.'
    } catch (failure) {
      ctx.log.warn('source copy failed', { 'file.path': file.path, 'exception.message': failure.message })
      getSelection()?.selectAllChildren(document.getElementById(file.id))
      file.copied = false
      file.blocked = true
      ctx.$.announcement = 'The clipboard is blocked. The code of ' + file.name + ' is selected: copy it with the keyboard.'
    }
    resetTimers.set(file.id, setTimeout(() => ctx.update(current => {
      file.copied = false
      file.blocked = false
      current.announcement = ''
    }), copyResetMs))
  },
  toggleExpanded(ctx) {
    ctx.$.expanded = !ctx.$.expanded
  },
}

export function mount(ctx) {
  const unpaint = paint(ctx.el, examplesOf(ctx).flatMap(example => example.files))
  const stage = ctx.el.querySelector('.showcase-stage')
  if (!stage || stage.querySelector('[data-c="frame"]')) return unpaint
  let interactedAt = 0
  let beatAt = 0
  const noteInteraction = event => {
    if (stage.contains(event.target)) interactedAt = event.timeStamp
  }
  const observer = new MutationObserver(records => {
    if (!beatAt && !interactedAt) return
    const touched = new Set(records.map(record => record.target.nodeType === Node.ELEMENT_NODE ? record.target : record.target.parentElement))
    touched.delete(null)
    const now = performance.now()
    const timed = interactedAt > 0 && now - interactedAt < timedWindowMs
    const patches = describePatch(touched.size, timed ? now - interactedAt : null)
    const beat = now - beatAt > beatGapMs
    interactedAt = 0
    if (beat) beatAt = now
    if (!beat && patches === ctx.$.patches) return
    ctx.update(current => {
      current.patches = patches
      if (beat) current.beat = current.beat === 'a' ? 'b' : 'a'
    })
  })
  observer.observe(stage, { subtree: true, childList: true, attributes: true, characterData: true })
  for (const type of interactions) document.addEventListener(type, noteInteraction, true)
  return () => {
    unpaint()
    observer.disconnect()
    for (const type of interactions) document.removeEventListener(type, noteInteraction, true)
  }
}

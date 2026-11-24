import { H, L } from './dom.js'

// Enter/leave motion for block bodies: a new body gets data-entering until its animations end;
// a body that leaves gets data-leaving (inert) and is removed when its animations end.
const MOTION_GRACE_MS = 50
const reduced = () => matchMedia('(prefers-reduced-motion: reduce)').matches
let leaving = [], entering = []

function timing(n) {
  const finished = []
  let ms = 0
  for (const a of n.getAnimations({ subtree: true })) {
    const end = a.effect?.getComputedTiming().endTime
    if (!(end < Infinity)) continue
    finished.push(a.finished)
    ms = Math.max(ms, end - (a.currentTime ?? 0))
  }
  return [finished, ms]
}

const still = [[], 0]
function after([finished, ms], done) {
  if (!finished.length) return done()
  Promise.race([Promise.allSettled(finished), new Promise(r => setTimeout(r, ms + MOTION_GRACE_MS))]).then(done)
}

export const afterMotion = (n, done) => after(reduced() ? still : timing(n), done)

function gone(n, token) {
  if (n[L] !== token) return
  n[L] = null
  H.remove(n)
  n.remove()
}

function entered(into) {
  const waits = into.filter(n => n.hasAttribute('data-entering')).map(n => [timing(n), () => n.removeAttribute('data-entering')])
  for (const [m, done] of waits) after(m, done)
}

function settle() {
  if (!leaving.length && !entering.length) return
  const out = leaving.filter(n => n[L]), into = entering, motion = !reduced()
  leaving = []
  entering = []
  if (motion) for (const n of out) {
    n.removeAttribute('data-entering')
    n.setAttribute('data-leaving', '')
    n.inert = true
  }
  const waits = out.map(n => [motion ? timing(n) : still, gone.bind(null, n, n[L])])
  for (const [m, done] of waits) after(m, done)
  if (into.length) requestAnimationFrame(() => setTimeout(entered, 0, into))
}

export default function install() {
  H.made = (n, enter) => {
    if (!enter || reduced()) return
    n.setAttribute('data-entering', '')
    entering.push(n)
  }
  H.drop = n => {
    if (n[L]) return
    n[L] = {}
    leaving.push(n)
  }
  H.stay = n => {
    n[L] = null
    n.removeAttribute('data-leaving')
    n.inert = false
  }
  H.done = settle
}

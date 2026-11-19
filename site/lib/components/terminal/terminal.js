const resetDelayMs = 1800

export const state = () => ({
  copyStatus: 'idle',
  resetTimer: 0,
  get copied() {
    return this.copyStatus === 'copied'
  },
  get copyText() {
    return { idle: 'Copy', copied: 'Copied', failed: 'Copy failed' }[this.copyStatus]
  },
  get copyLabel() {
    return this.copyStatus === 'idle' ? 'Copy the commands' : this.copyText
  },
  get announcement() {
    return {
      idle: '',
      copied: 'Commands copied to the clipboard.',
      failed: 'Copy failed. Select the commands and copy them by hand.',
    }[this.copyStatus]
  },
})

export const actions = {
  async copyCommands(ctx) {
    clearTimeout(ctx.$.resetTimer)
    try {
      await navigator.clipboard.writeText(ctx.$.copy)
      ctx.$.copyStatus = 'copied'
    } catch (failure) {
      ctx.log.warn('command copy failed', { 'exception.message': failure.message })
      ctx.$.copyStatus = 'failed'
    }
    ctx.$.resetTimer = setTimeout(() => ctx.update(current => { current.copyStatus = 'idle' }), resetDelayMs)
  },
}

export function mount(ctx) {
  return () => clearTimeout(ctx.$.resetTimer)
}

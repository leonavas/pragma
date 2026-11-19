import { email, maxLength, nonEmpty, object, pipe, safeParse, string, trim } from 'valibot'

const signup = object({
  email: pipe(string(), trim(), nonEmpty('Type an email.'), email('That is not an email address.'), maxLength(64, 'Keep it under 64 characters.')),
})

export const state = () => ({
  email: 'ana@example',
  joined: '',
  get validation() { return safeParse(signup, { email: this.email }) },
  get valid() { return this.validation.success },
  get invalid() { return !this.valid },
  get ariaInvalid() { return this.valid ? 'false' : 'true' },
  get status() {
    if (this.valid && this.joined === this.validation.output.email) return 'Joined as ' + this.joined
    return this.valid ? 'Looks good.' : this.validation.issues[0].message
  },
})

export const actions = {
  submit(ctx) {
    if (ctx.$.valid) ctx.$.joined = ctx.$.validation.output.email
  },
}

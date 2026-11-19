import { email, object, pipe, safeParse, string } from 'valibot'

const contact = object({ email: pipe(string(), email()) })

export const state = () => ({
  email: 'ana@example.com',
  get emailStatus() {
    const validation = safeParse(contact, { email: this.email })
    return validation.success ? 'Valid email' : validation.issues[0].message
  },
})

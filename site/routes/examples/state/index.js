let visitor = ''

const name = () => visitor.trim()
const greeting = () => name() ? `Hello, ${name()}.` : 'Hello, whoever you are.'
const anonymous = () => !name()

function forget() {
  visitor = ''
}

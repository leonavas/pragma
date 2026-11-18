import { render } from './render.js'

export function createHandler({ routes, service }) {
  return { routes, service, render }
}

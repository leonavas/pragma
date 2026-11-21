export function redirect(status, location) {
  throw { status, location }
}

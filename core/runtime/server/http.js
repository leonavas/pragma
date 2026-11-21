const REDIRECT_STATUSES = [301, 302, 303, 307, 308]

export class Redirect {
  constructor(status, location) {
    this.status = status
    this.location = location
  }
}

export class HttpError {
  constructor(status, body) {
    this.status = status
    this.body = body
  }
}

export function redirect(status, location) {
  if (!REDIRECT_STATUSES.includes(status)) {
    throw new Error(`redirect(${status}, ...): status must be 301, 302, 303, 307 or 308; use 303 after an action, 307 to keep the method`)
  }
  if (typeof location !== 'string' || !location) throw new Error('redirect(status, location): location must be a non-empty string such as "/login"')
  throw new Redirect(status, location)
}

export function error(status, body) {
  if (!Number.isInteger(status) || status < 400 || status > 599) {
    throw new Error(`error(${status}, ...): status must be 400-599; use redirect() for 3xx`)
  }
  throw new HttpError(status, errorBody(status, body))
}

function errorBody(status, body) {
  if (body === undefined) return { message: `HTTP ${status}` }
  if (typeof body === 'string') return { message: body }
  if (body && typeof body === 'object') return { message: `HTTP ${status}`, ...body }
  return { message: String(body) }
}

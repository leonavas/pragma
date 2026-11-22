export const ssr = true

export async function load({ cookies }) {
  const visits = Number(cookies.get('visits') ?? 0) + 1
  const options = { maxAge: 3600 }
  cookies.set('visits', String(visits), options)
  return { visits }
}

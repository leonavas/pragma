const normalize = text => text.normalize('NFD').replace(/\p{M}/gu, '').toLowerCase()

export const contains = (text, query) => normalize(text).includes(normalize(query.trim()))

export const markCurrent = (options, current) => Object.fromEntries(options.map(option => [option, option === current && 'true']))

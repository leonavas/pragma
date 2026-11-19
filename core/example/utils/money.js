const dollars = new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' })

export const usd = amount => dollars.format(amount)

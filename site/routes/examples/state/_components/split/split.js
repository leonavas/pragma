const money = new Intl.NumberFormat('en-US', { style: 'currency', currency: 'USD' })

export const state = () => ({
  amount: '84.50',
  people: '3',
  tipPercent: '15',
  get bill() { return Math.max(Number(this.amount) || 0, 0) },
  get headcount() { return Math.max(Math.round(Number(this.people)) || 1, 1) },
  get tip() { return this.bill * (Number(this.tipPercent) || 0) / 100 },
  get total() { return this.bill + this.tip },
  get tipLabel() { return money.format(this.tip) },
  get totalLabel() { return money.format(this.total) },
  get perPersonLabel() { return money.format(this.total / this.headcount) },
})

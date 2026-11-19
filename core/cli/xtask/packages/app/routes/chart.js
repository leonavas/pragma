import { BarController, BarElement, CategoryScale, Chart, LinearScale } from 'chart.js'

Chart.register(BarController, BarElement, CategoryScale, LinearScale)

export function mount(ctx) {
  const salesChart = new Chart(ctx.el.querySelector('#sales-by-month'), {
    type: 'bar',
    data: { labels: ['Jan', 'Feb', 'Mar'], datasets: [{ label: 'Sales', data: [12, 19, 7], backgroundColor: '#2563eb' }] },
    options: { animation: false, responsive: false },
  })
  return () => salesChart.destroy()
}

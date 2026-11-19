const measured = {
  highlighter: [
    { name: "import hljs from 'highlight.js'", note: 'index entry: all 193 languages', raw: 926.7, gzip: 303.4 },
    { name: '@lezer/highlight + javascript, html, css', note: 'pure ESM, rejected: the JS grammar alone is 29.6 kB', raw: 161.5, gzip: 60.9 },
    { name: 'highlight.js/lib/core + xml, javascript, css', note: 'used on this site: on the server for every showcase, in the browser only via import() on the SEO page', raw: 40.8, gzip: 14.8, shipped: true },
  ],
  packages: [
    { name: 'es-toolkit', note: 'groupBy + debounce, ESM', raw: 0.67, gzip: 0.43, shipped: true },
    { name: 'valibot', note: 'object, pipe, string, email, safeParse, ESM', raw: 3.4, gzip: 1.35, shipped: true },
    { name: '@date-fns/tz', note: 'TZDate, ESM', raw: 4.9, gzip: 1.75 },
    { name: 'dayjs', note: 'core + relativeTime, CommonJS: ships whole', raw: 8.75, gzip: 3.69 },
    { name: 'lodash', note: 'groupBy + sumBy, CommonJS: ships whole', raw: 22.0, gzip: 6.49 },
    { name: 'date-fns', note: 'format, ESM (format pulls the locale code)', raw: 25.0, gzip: 6.69 },
    { name: 'fuse.js', note: 'Fuse class, one export', raw: 26.5, gzip: 9.21 },
    { name: 'chart.js', note: 'bar chart: 5 registered pieces', raw: 132.6, gzip: 45.99 },
  ],
}

const kilobytes = size => (size < 10 ? size.toFixed(2) : size.toFixed(1)) + ' kB'

function tableRows(rows) {
  const largest = Math.max(...rows.map(row => row.gzip))
  return rows.map(row => ({
    name: row.name,
    note: row.note,
    shipped: Boolean(row.shipped),
    share: row.gzip / largest,
    rawText: kilobytes(row.raw),
    gzipText: kilobytes(row.gzip),
  }))
}

export const sizes = {
  highlighter: tableRows(measured.highlighter),
  packages: tableRows(measured.packages),
}

const rows = ['name,price', 'Keyboard,49', 'Hub,25']

export async function GET() {
  return new Response(rows.join('\n') + '\n', {
    headers: {
      'content-type': 'text/csv',
      'content-disposition':
        'attachment; filename=products.csv',
    },
  })
}

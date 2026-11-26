const rows = ['player,score', 'Nova,9850', 'Pixel,8420', 'Biscuit,7700']

export async function GET() {
  return new Response(rows.join('\n') + '\n', {
    headers: {
      'content-type': 'text/csv',
      'content-disposition':
        'attachment; filename=highscores.csv',
    },
  })
}

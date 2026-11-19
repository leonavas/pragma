import dayjs from 'dayjs'
import relativeTime from 'dayjs/plugin/relativeTime'

dayjs.extend(relativeTime)

const start = dayjs('2024-03-10T12:00:00')

export const state = () => ({
  shiftedDate: start.add(5, 'day').format('YYYY-MM-DD'),
  relativeDate: start.from(dayjs('2024-03-15T12:00:00')),
})

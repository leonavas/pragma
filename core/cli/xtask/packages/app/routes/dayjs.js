import dayjs from 'dayjs'

export const state = () => ({ shiftedDate: dayjs('2024-03-10T12:00:00').add(5, 'day').format('YYYY-MM-DD') })

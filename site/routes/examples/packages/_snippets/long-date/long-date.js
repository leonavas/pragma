import dayjs from 'dayjs'

export function longDate() {
  return dayjs().format('dddd, D MMMM YYYY')
}

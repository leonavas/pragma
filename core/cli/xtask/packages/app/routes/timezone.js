import { TZDate } from '@date-fns/tz'
import { format } from 'date-fns/format'

export const state = () => ({ tokyoTime: format(new TZDate(Date.UTC(2024, 0, 15, 12), 'Asia/Tokyo'), 'HH:mm') })

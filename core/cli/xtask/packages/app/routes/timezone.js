import { TZDate } from '@date-fns/tz'
import { format } from 'date-fns/format'
import { ptBR } from 'date-fns/locale'

const noonUtc = Date.UTC(2024, 0, 15, 12)

export const state = () => ({
  tokyoTime: format(new TZDate(noonUtc, 'Asia/Tokyo'), 'HH:mm'),
  saoPauloTime: format(new TZDate(noonUtc, 'America/Sao_Paulo'), "EEEE, d 'de' MMMM 'de' yyyy, HH:mm", { locale: ptBR }),
  convertedTime: format(new TZDate(2024, 0, 15, 9, 0, 'America/Sao_Paulo').withTimeZone('Asia/Tokyo'), 'yyyy-MM-dd HH:mm'),
})

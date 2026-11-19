import { format } from 'date-fns'

export const state = () => ({ formattedDate: format(new Date(2024, 0, 15), 'yyyy-MM-dd') })

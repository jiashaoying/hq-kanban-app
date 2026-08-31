export interface IndexData {
  code: string
  name: string
  current: number
  prev_close: number
  open: number
  high: number
  low: number
  change: number
  change_pct: number
  volume: number
  amount: number
  market: 'a' | 'hk' | 'us'
  update_time: string
}

export interface MarketGroup {
  key: string
  label: string
  indices: IndexData[]
}

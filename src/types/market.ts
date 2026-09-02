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

// 分时单点（snake_case，与 Rust serde 默认序列化对齐）
export interface MinutePoint {
  time: string // "HH:MM"
  price: number
  avg_price: number
  volume: number
}

// 分时数据（date 为 "YYYYMMDD"，休市时可能为空字符串）
export interface MinuteData {
  code: string
  date: string
  prev_close: number
  points: MinutePoint[]
}

export type KlinePeriod = 'day' | 'week' | 'month'

export interface KlineBar {
  date: string // "YYYY-MM-DD"
  open: number
  close: number
  high: number
  low: number
  volume: number
}

export interface KlineData {
  code: string
  period: KlinePeriod
  bars: KlineBar[]
}

// 格式化价格（保留2位小数）
export function formatPrice(price: number): string {
  return price.toFixed(2)
}

// 格式化涨跌额（带正负号）
export function formatChange(change: number): string {
  const sign = change > 0 ? '+' : ''
  return `${sign}${change.toFixed(2)}`
}

// 格式化涨跌幅（带正负号和%）
export function formatChangePct(pct: number): string {
  const sign = pct > 0 ? '+' : ''
  return `${sign}${pct.toFixed(2)}%`
}

// 格式化成交量（万手 → 亿手转换）
export function formatVolume(vol: number): string {
  if (vol >= 10000) {
    return `${(vol / 10000).toFixed(2)}亿`
  }
  return `${vol.toFixed(0)}万`
}

// 格式化成交额（万元 → 亿元转换）
export function formatAmount(amount: number): string {
  if (amount >= 10000) {
    return `${(amount / 10000).toFixed(2)}亿`
  }
  return `${amount.toFixed(0)}万`
}

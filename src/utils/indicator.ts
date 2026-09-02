// 滑动平均：前 n-1 项数据不足，返回 null
export function ma(closes: number[], n: number): (number | null)[] {
  const result: (number | null)[] = new Array(closes.length).fill(null)
  if (n <= 0 || closes.length < n) return result
  let sum = 0
  for (let i = 0; i < closes.length; i++) {
    sum += closes[i]
    if (i >= n) sum -= closes[i - n]
    if (i >= n - 1) result[i] = sum / n
  }
  return result
}

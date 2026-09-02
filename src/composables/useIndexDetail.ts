import { ref, watch, onMounted, onUnmounted, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { MinuteData, KlineData, KlinePeriod } from '../types/market'

const POLL_INTERVAL = 15000
// K线缓存 TTL：盘中日K最后一根 bar 持续变动，过期即回源刷新，与分时/头部报价保持一致
const KLINE_TTL = 60_000
// 统一友好错误文案：Rust 原始错误串不直出 UI，仅保留在 console.error
const FRIENDLY_ERROR = '数据源暂不可用，请稍后重试'

// K线模块级缓存（key = `${code}:${period}`）：跨挂载复用，同周期切换不重复请求；附 fetchedAt 实现 TTL 过期
const klineCache = new Map<string, { data: KlineData; fetchedAt: number }>()

export function useIndexDetail(code: Ref<string>) {
  const minuteData = ref<MinuteData | null>(null)
  const klineData = ref<KlineData | null>(null)
  const activePeriod = ref<KlinePeriod>('day')
  const loading = ref(false)
  const error = ref('')
  // K线错误与分时 error 解耦：避免被 15s 分时轮询成功清空，导致K线错误永不渲染
  const klineError = ref('')

  let minuteTimer: ReturnType<typeof setInterval> | null = null
  let minuteInflight = false

  async function loadMinute(): Promise<void> {
    // inflight 去重守卫：上一请求未返回时跳过本次触发，避免慢网络下请求堆积
    if (minuteInflight) return
    minuteInflight = true
    const requestedCode = code.value
    try {
      const data = await invoke<MinuteData>('fetch_minute_data', { code: requestedCode })
      // 乱序防护：返回时已切换指数则丢弃，不写入状态
      if (code.value !== requestedCode) return
      minuteData.value = data
      error.value = ''
    } catch (e) {
      console.error('Failed to fetch minute data:', e)
      // 乱序防护：指数已切换时错误由新请求接管，不写入过期失败态
      if (code.value === requestedCode) {
        error.value = FRIENDLY_ERROR
      }
    } finally {
      minuteInflight = false
    }
  }

  async function loadKline(period: KlinePeriod): Promise<void> {
    const cacheKey = `${code.value}:${period}`
    const cached = klineCache.get(cacheKey)
    // TTL 命中才复用缓存，过期视为未命中重新请求并覆盖
    if (cached && Date.now() - cached.fetchedAt < KLINE_TTL) {
      klineData.value = cached.data
      klineError.value = ''
      return
    }
    // 发起新请求前清除旧错误，使 UI 回到加载中状态
    klineError.value = ''
    try {
      const data = await invoke<KlineData>('fetch_kline_data', { code: code.value, period })
      klineCache.set(cacheKey, { data, fetchedAt: Date.now() })
      // 异步返回时若已切换周期/代码，丢弃过期数据，避免错配
      if (`${code.value}:${activePeriod.value}` === cacheKey) {
        klineData.value = data
      }
    } catch (e) {
      console.error('Failed to fetch kline data:', e)
      // 乱序防护：失败请求对应的周期/代码已切换时，不写入过期失败态
      if (`${code.value}:${activePeriod.value}` === cacheKey) {
        klineError.value = FRIENDLY_ERROR
      }
    }
  }

  function setPeriod(period: KlinePeriod): void {
    if (activePeriod.value === period) return
    activePeriod.value = period
    loadKline(period)
  }

  // 重试当前周期K线：绕过 setPeriod 对相同周期的 early-return，供失败重载场景使用
  function retryKline(): void {
    loadKline(activePeriod.value)
  }

  function startPolling(): void {
    if (minuteTimer) return
    minuteTimer = setInterval(loadMinute, POLL_INTERVAL)
  }

  function stopPolling(): void {
    if (minuteTimer) {
      clearInterval(minuteTimer)
      minuteTimer = null
    }
  }

  async function initialLoad(): Promise<void> {
    loading.value = true
    await Promise.all([loadMinute(), loadKline(activePeriod.value)])
    loading.value = false
  }

  function handleVisibility(): void {
    if (document.hidden) {
      stopPolling()
    } else {
      loadMinute()
      startPolling()
    }
  }

  // code 变化（防御性）：重置状态并重新加载
  watch(code, () => {
    minuteData.value = null
    klineData.value = null
    stopPolling()
    initialLoad()
    startPolling()
  })

  onMounted(() => {
    initialLoad()
    startPolling()
    document.addEventListener('visibilitychange', handleVisibility)
  })

  onUnmounted(() => {
    stopPolling()
    document.removeEventListener('visibilitychange', handleVisibility)
  })

  return {
    minuteData,
    klineData,
    activePeriod,
    loading,
    error,
    klineError,
    setPeriod,
    retryKline,
  }
}

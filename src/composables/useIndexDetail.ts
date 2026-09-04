import { ref, watch, onMounted, onUnmounted, type Ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { MinuteData, KlineData, KlinePeriod } from '../types/market'

const POLL_INTERVAL = 15000
// K线缓存 TTL：盘中日K最后一根 bar 持续变动，过期即回源刷新，与分时/头部报价保持一致
const KLINE_TTL = 60_000
// K线柱数：与后端 fetch_kline_data 的 count.unwrap_or(320) 默认值对齐；
// 两阶段加载传相同 count，保证柱数一致、图表切换渲染不跳变
const KLINE_COUNT = 320
// 统一友好错误文案：Rust 原始错误串不直出 UI，仅保留在 console.error
const FRIENDLY_ERROR = '数据源暂不可用，请稍后重试'

// K线模块级缓存（key = `${code}:${period}`）：跨挂载复用，同周期切换不重复请求；附 fetchedAt 实现 TTL 过期
const klineCache = new Map<string, { data: KlineData; fetchedAt: number }>()
// 后台 revalidate 并发防护（同 key 去重）：模块级共享，返回首页重进后旧请求未返回时不重复回源
const klineRevalidating = new Set<string>()

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

  // 网络刷新 K 线（两阶段加载第二阶段与 SWR revalidate 共用）：
  // 成功更新缓存与当前 UI；失败时若已渲染过陈旧缓存（staleRendered）则静默保留
  // （陈旧优于报错空屏），否则写 klineError 供 UI 展示重试入口
  async function refreshKline(
    cacheKey: string,
    requestCode: string,
    period: KlinePeriod,
    staleRendered: boolean
  ): Promise<void> {
    try {
      const data = await invoke<KlineData>('fetch_kline_data', {
        code: requestCode,
        period,
        count: KLINE_COUNT,
        preferCache: false,
      })
      klineCache.set(cacheKey, { data, fetchedAt: Date.now() })
      // 乱序防护：返回时已切换周期/代码则仅更新缓存，不覆盖当前 UI 数据
      if (`${code.value}:${activePeriod.value}` === cacheKey) {
        klineData.value = data
      }
    } catch (e) {
      console.error('Failed to fetch kline data:', e)
      // 乱序防护：失败请求对应的周期/代码已切换时，不写入过期失败态
      if (!staleRendered && `${code.value}:${activePeriod.value}` === cacheKey) {
        klineError.value = FRIENDLY_ERROR
      }
    }
  }

  // SWR 后台静默刷新：仅在已有陈旧缓存兜底（UI 已渲染）时调用，失败必然静默
  async function revalidateKline(cacheKey: string, requestCode: string, period: KlinePeriod): Promise<void> {
    klineRevalidating.add(cacheKey)
    try {
      await refreshKline(cacheKey, requestCode, period, true)
    } finally {
      klineRevalidating.delete(cacheKey)
    }
  }

  async function loadKline(period: KlinePeriod): Promise<void> {
    const cacheKey = `${code.value}:${period}`
    const cached = klineCache.get(cacheKey)
    // SWR：缓存命中（无论是否过期）先即时渲染缓存数据，消除「K线加载中...」等待
    if (cached) {
      klineData.value = cached.data
      klineError.value = ''
      // 仍新鲜（< KLINE_TTL）：直接复用，不发请求
      if (Date.now() - cached.fetchedAt < KLINE_TTL) return
      // 已过期：后台静默 revalidate（preferCache=false 走网络）；同 key 已 inflight 则不重复发
      if (klineRevalidating.has(cacheKey)) return
      void revalidateKline(cacheKey, code.value, period)
      return
    }
    // L1 无缓存（冷启动/重进应用/首次切周期）：两阶段加载。
    // klineRevalidating 兼作加载 inflight 标记（同 key 去重，与 revalidate 路径共享），
    // 进行中重复的 loadKline 直接跳过，由进行中流程的乱序校验自然接管写入
    if (klineRevalidating.has(cacheKey)) return
    klineRevalidating.add(cacheKey)
    // 发起新加载前清除旧错误，使 UI 回到「K线加载中...」状态
    klineError.value = ''
    const requestCode = code.value
    // 第一阶段（缓存先行）：后端读 L2 磁盘 JSON，命中无论 TTL 直接返回（不发网络），
    // 立即写缓存并渲染，消除加载态；无磁盘缓存 Err 属预期（该周期首次访问），
    // 静默忽略（仅记录日志，不写 klineError），照常进入第二阶段
    let staleRendered = false
    try {
      const disk = await invoke<KlineData>('fetch_kline_data', {
        code: requestCode,
        period,
        count: KLINE_COUNT,
        preferCache: true,
      })
      klineCache.set(cacheKey, { data: disk, fetchedAt: Date.now() })
      // 乱序防护：返回时已切换周期/代码则仅写缓存，不覆盖当前 UI 数据
      if (`${code.value}:${activePeriod.value}` === cacheKey) {
        klineData.value = disk
        staleRendered = true
      }
    } catch (e) {
      console.error('No disk cache for kline, fallback to network:', e)
    }
    // 第二阶段（最新数据）：串行发起（第一阶段完成后立即发起，不并发重复网络请求）。
    // fire-and-forget：不 await 网络完成，避免 initialLoad 的 loading 被网络耗时阻塞
    // （磁盘缓存已先行渲染）；klineRevalidating 标记保持到第二阶段结束，
    // 期间同 key 的 loadKline/revalidate 一律跳过。刷新失败时已渲染过磁盘
    // 缓存则静默，否则走 klineError + 重试路径（见 refreshKline）
    void refreshKline(cacheKey, requestCode, period, staleRendered).finally(() => {
      klineRevalidating.delete(cacheKey)
    })
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

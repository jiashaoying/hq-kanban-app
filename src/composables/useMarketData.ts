import { ref, computed, onMounted, onUnmounted } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { IndexData, MarketGroup } from '../types/market'

const POLL_INTERVAL_DESKTOP = 3000
const POLL_INTERVAL_MOBILE = 15000

function getPollInterval(): number {
  return window.innerWidth < 768 ? POLL_INTERVAL_MOBILE : POLL_INTERVAL_DESKTOP
}

export function useMarketData() {
  const indices = ref<IndexData[]>([])
  const loading = ref(false)
  const lastUpdateTime = ref<string>('')
  let timer: ReturnType<typeof setInterval> | null = null

  const marketGroups = computed<MarketGroup[]>(() => {
    const aIndices = indices.value.filter(i => i.market === 'a')
    const hkIndices = indices.value.filter(i => i.market === 'hk')
    const usIndices = indices.value.filter(i => i.market === 'us')

    return [
      { key: 'a', label: 'A 股', indices: aIndices },
      { key: 'hk', label: '港 股', indices: hkIndices },
      { key: 'us', label: '美 股', indices: usIndices },
    ]
  })

  async function refresh() {
    loading.value = true
    try {
      const data = await invoke<IndexData[]>('fetch_indices')
      indices.value = data
      lastUpdateTime.value = new Date().toLocaleTimeString('zh-CN', { hour12: false })
    } catch (e) {
      console.error('Failed to fetch indices:', e)
    } finally {
      loading.value = false
    }
  }

  function startPolling() {
    refresh()
    timer = setInterval(refresh, getPollInterval())
  }

  function stopPolling() {
    if (timer) {
      clearInterval(timer)
      timer = null
    }
  }

  onMounted(() => {
    startPolling()

    const handleVisibility = () => {
      if (document.hidden) {
        stopPolling()
      } else {
        refresh()
        startPolling()
      }
    }
    document.addEventListener('visibilitychange', handleVisibility)
    ;(window as any).__cleanupVisibility = handleVisibility
  })

  onUnmounted(() => {
    stopPolling()
    const handler = (window as any).__cleanupVisibility
    if (handler) {
      document.removeEventListener('visibilitychange', handler)
      delete (window as any).__cleanupVisibility
    }
  })

  return {
    indices,
    marketGroups,
    loading,
    lastUpdateTime,
    refresh,
  }
}

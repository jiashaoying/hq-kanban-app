<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref } from 'vue'
import type { IndexData, KlinePeriod } from '../types/market'
import { formatPrice, formatChange, formatChangePct } from '../utils/format'
import { useIndexDetail } from '../composables/useIndexDetail'
import MinuteChart from './charts/MinuteChart.vue'
import KlineChart from './charts/KlineChart.vue'

const props = defineProps<{ data: IndexData }>()
const emit = defineEmits<{ back: [] }>()

// 红涨绿跌配色（参考 IndexCard.vue trendColor）
const trendColor = computed(() => {
  if (props.data.change > 0) return 'text-red-500'
  if (props.data.change < 0) return 'text-green-500'
  return 'text-gray-400'
})

const code = computed(() => props.data.code)
const { minuteData, klineData, loading, error, klineError, setPeriod, retryKline } = useIndexDetail(code)

type Tab = 'minute' | KlinePeriod
interface TabItem {
  key: Tab
  label: string
}

const tabs: TabItem[] = [
  { key: 'minute', label: '分时' },
  { key: 'day', label: '日K' },
  { key: 'week', label: '周K' },
  { key: 'month', label: '月K' },
]

const activeTab = ref<Tab>('minute')

function switchTab(tab: TabItem): void {
  activeTab.value = tab.key
  if (tab.key !== 'minute') setPeriod(tab.key)
}

// ESC 键返回
function onKeydown(e: KeyboardEvent): void {
  if (e.key === 'Escape') emit('back')
}

onMounted(() => {
  window.addEventListener('keydown', onKeydown)
})

onUnmounted(() => {
  window.removeEventListener('keydown', onKeydown)
})
</script>

<template>
  <div>
    <!-- 头部：返回按钮 + 指数名/代码 + 实时报价（随看板轮询联动） -->
    <div class="flex items-center gap-3 mb-4">
      <button
        class="shrink-0 w-8 h-8 flex items-center justify-center rounded-md border border-gray-700 text-gray-300 hover:text-white hover:bg-gray-800 transition-colors"
        title="返回 (Esc)"
        @click="emit('back')"
      >
        ←
      </button>
      <div class="min-w-0">
        <div class="text-lg font-semibold text-gray-100 truncate">{{ data.name }}</div>
        <div class="text-xs text-gray-500">{{ data.code }}</div>
      </div>
      <div class="ml-auto text-right shrink-0">
        <div class="text-2xl font-bold tabular-nums" :class="trendColor">
          {{ formatPrice(data.current) }}
        </div>
        <div class="flex items-center justify-end gap-3 text-sm tabular-nums" :class="trendColor">
          <span>{{ formatChange(data.change) }}</span>
          <span>{{ formatChangePct(data.change_pct) }}</span>
        </div>
      </div>
    </div>

    <!-- Tab 栏 -->
    <div class="inline-flex gap-1 mb-4 p-1 rounded-lg bg-gray-800/70 border border-gray-700/50">
      <button
        v-for="tab in tabs"
        :key="tab.key"
        class="px-4 py-1.5 text-xs rounded-md transition-colors"
        :class="activeTab === tab.key
          ? 'bg-gray-700 text-white font-medium'
          : 'text-gray-400 hover:text-gray-200'"
        @click="switchTab(tab)"
      >
        {{ tab.label }}
      </button>
    </div>

    <!-- 内容区：relative 容器在 loading 期间即渲染并稳定布局，图层挂载时
         无同帧过渡态（clientWidth 恒可读真实值） -->
    <div class="relative h-64 md:h-80">
      <div v-if="loading" class="absolute inset-0 flex items-center justify-center">
        <div class="text-center text-gray-500">
          <div class="text-sm mb-1">加载中...</div>
          <div class="text-xs">正在获取分时与K线数据</div>
        </div>
      </div>
      <template v-else>
        <!-- 叠放布局：两图层 absolute 叠放，切 tab 仅翻转 opacity（不用 visibility /
             display 切换），图层始终不脱离布局、盒子尺寸恒定，显隐切换不触发 RO、
             chart 实例零重建；不加 opacity 过渡，切换即时翻转避免过渡中间态。
             注：图表能否在隐藏层内正确绘制上屏与叠放 / opacity 无关——真正的渲染
             保障是子组件建图后的 forceSyncPaint()（强制同步绘制以绑定 canvas 位图
             尺寸，否则画布会停留在默认 300x150 而空白），详见 KlineChart.vue /
             MinuteChart.vue 内注释 -->
        <div
          class="absolute inset-0"
          :class="activeTab === 'minute'
            ? 'opacity-100 pointer-events-auto'
            : 'opacity-0 pointer-events-none'"
        >
          <MinuteChart
            v-if="minuteData && minuteData.points.length >= 2"
            :data="minuteData"
            :visible="activeTab === 'minute'"
          />
          <div v-else class="h-full flex flex-col items-center justify-center text-gray-500">
            <div class="text-sm mb-1">暂无分时数据</div>
            <div class="text-xs">{{ error || '市场休市中或数据源暂不可用' }}</div>
          </div>
        </div>
        <!-- 日/周/月K 共用 KlineChart：day 数据到达即在隐藏层内预建图（容器
             恒有尺寸），首次切到任一 K 线 tab 零初始化直接显示；切无缓存周期
             时短暂显示上一周期数据，watch(bars) 数据到达后自动更新 -->
        <div
          class="absolute inset-0"
          :class="activeTab !== 'minute'
            ? 'opacity-100 pointer-events-auto'
            : 'opacity-0 pointer-events-none'"
        >
          <KlineChart
            v-if="klineData && klineData.bars.length > 0"
            :bars="klineData.bars"
            :visible="activeTab !== 'minute'"
            :debug-tab="activeTab"
          />
          <div v-else-if="klineData" class="h-full flex flex-col items-center justify-center text-gray-500">
            <div class="text-sm mb-1">暂无K线数据</div>
            <div class="text-xs">{{ klineError || '数据源暂不可用' }}</div>
          </div>
          <!-- 加载失败：独立于分时 error 的 klineError，支持点击重试（绕过 setPeriod 同周期 early-return） -->
          <div v-else-if="klineError" class="h-full flex flex-col items-center justify-center text-gray-500">
            <div class="text-sm mb-1">K线加载失败</div>
            <div class="text-xs mb-3">{{ klineError }}</div>
            <button
              class="px-4 py-1.5 text-xs rounded-md border border-gray-700 text-gray-300 hover:text-white hover:bg-gray-800 transition-colors"
              @click="retryKline"
            >
              点击重试
            </button>
          </div>
          <div v-else class="h-full flex flex-col items-center justify-center text-gray-500">
            <div class="text-sm mb-1">K线加载中...</div>
          </div>
        </div>
      </template>
    </div>
  </div>
</template>

<script setup lang="ts">
import type { IndexData } from '../types/market'
import { formatPrice, formatChange, formatChangePct, formatVolume, formatAmount } from '../utils/format'
import { computed } from 'vue'

const emit = defineEmits<{ select: [data: IndexData] }>()

const props = defineProps<{
  data: IndexData
}>()

const trend = computed(() => {
  if (props.data.change > 0) return 'up'
  if (props.data.change < 0) return 'down'
  return 'flat'
})

const trendColor = computed(() => {
  switch (trend.value) {
    case 'up': return 'text-red-500'
    case 'down': return 'text-green-500'
    default: return 'text-gray-400'
  }
})

const trendBg = computed(() => {
  switch (trend.value) {
    case 'up': return 'bg-red-500/5'
    case 'down': return 'bg-green-500/5'
    default: return 'bg-gray-800'
  }
})

const trendBorder = computed(() => {
  switch (trend.value) {
    case 'up': return 'border-l-red-500'
    case 'down': return 'border-l-green-500'
    default: return 'border-l-gray-600'
  }
})
</script>

<template>
  <div
    class="rounded-lg border border-gray-700 border-l-4 p-4 transition-colors cursor-pointer hover:border-gray-600"
    :class="[trendBg, trendBorder]"
    @click="emit('select', props.data)"
  >
    <!-- 指数名称 -->
    <div class="text-sm text-gray-400 mb-1">{{ data.name }}</div>

    <!-- 当前价格 -->
    <div class="text-2xl font-bold mb-1" :class="trendColor">
      {{ formatPrice(data.current) }}
    </div>

    <!-- 涨跌额 + 涨跌幅 -->
    <div class="flex items-center gap-3 mb-2">
      <span class="text-sm" :class="trendColor">
        {{ formatChange(data.change) }}
      </span>
      <span class="text-sm" :class="trendColor">
        {{ formatChangePct(data.change_pct) }}
      </span>
    </div>

    <!-- 成交量/成交额 -->
    <div class="flex justify-between text-xs text-gray-500">
      <span>成交量: {{ formatVolume(data.volume) }}</span>
      <span>成交额: {{ formatAmount(data.amount) }}</span>
    </div>
  </div>
</template>

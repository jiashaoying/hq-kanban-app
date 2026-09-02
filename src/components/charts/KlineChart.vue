<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import {
  createChart,
  CandlestickSeries,
  LineSeries,
  HistogramSeries,
  ColorType,
  CrosshairMode,
  type IChartApi,
  type ISeriesApi,
  type Time,
} from 'lightweight-charts'
import type { KlineBar } from '../../types/market'
import { ma } from '../../utils/indicator'

const props = defineProps<{ bars: KlineBar[] }>()

const UP_COLOR = '#ef232a'
const DOWN_COLOR = '#14b143'
const GRID_COLOR = '#374151'
const TEXT_COLOR = '#9ca3af'

const MA_CONFIG = [
  { n: 5, color: '#f5a623' },
  { n: 10, color: '#4a90e2' },
  { n: 20, color: '#9b59b6' },
]

const container = ref<HTMLDivElement | null>(null)
let chart: IChartApi | null = null
let candleSeries: ISeriesApi<'Candlestick'> | null = null
let volumeSeries: ISeriesApi<'Histogram'> | null = null
let maSeries: ISeriesApi<'Line'>[] = []
let resizeObserver: ResizeObserver | null = null
// 上次渲染的柱数：首次渲染或柱数变化（如新交易日/新周期柱出现）时自适应视野，
// 其余更新（缓存 TTL 回源刷新）保留用户缩放/平移状态
let prevBarCount = -1

// 图例：显示最新一根的 MA 值
const maLegend = ref<{ n: number; color: string; value: string }[]>([])

function createChartInstance(): void {
  if (!container.value) return
  chart = createChart(container.value, {
    layout: {
      background: { type: ColorType.Solid, color: 'transparent' },
      textColor: TEXT_COLOR,
      fontSize: 10,
      attributionLogo: false,
    },
    grid: {
      vertLines: { visible: false },
      horzLines: { color: GRID_COLOR },
    },
    rightPriceScale: { borderColor: GRID_COLOR },
    timeScale: { borderColor: GRID_COLOR },
    crosshair: { mode: CrosshairMode.Normal },
  })

  // 蜡烛主图（pane 0）：独占上部区域，与量区物理隔离
  candleSeries = chart.addSeries(CandlestickSeries, {
    upColor: UP_COLOR,
    downColor: DOWN_COLOR,
    borderVisible: false,
    wickUpColor: UP_COLOR,
    wickDownColor: DOWN_COLOR,
    priceLineVisible: false,
  })
  candleSeries.priceScale().applyOptions({ scaleMargins: { top: 0.08, bottom: 0.08 } })

  // MA5/10/20 与蜡烛共享同一价格轴
  maSeries = MA_CONFIG.map(cfg =>
    chart!.addSeries(LineSeries, {
      color: cfg.color,
      lineWidth: 1,
      priceLineVisible: false,
      lastValueVisible: false,
      crosshairMarkerVisible: false,
    })
  )

  // 成交量副图（pane 1）：v5 多 pane 物理隔离，与主图价格坐标系彻底解耦（涨红跌绿）。
  // 不采用 overlay priceScaleId 方案：实测（截图证据）该配置在 v5.2.1 运行时
  // 未能将量柱挂到独立 scale，量值（万手级）与价格（点位）共轴导致主图被压扁。
  volumeSeries = chart.addSeries(
    HistogramSeries,
    {
      priceFormat: { type: 'volume' },
      priceLineVisible: false,
      lastValueVisible: false,
    },
    1
  )
  // 主图:量区高度比 = 4:1（对齐 MinuteChart）
  const panes = chart.panes()
  if (panes.length > 1) {
    panes[0].setStretchFactor(4)
    panes[1].setStretchFactor(1)
    // 量区隐藏自身价格刻度，Y 轴仅保留主图价格刻度
    chart.priceScale('right', 1).applyOptions({ visible: false })
  }
}

function updateData(): void {
  if (!chart || !candleSeries || !volumeSeries || props.bars.length === 0) return

  // lightweight-charts 要求数据按时间升序，防御性排序（YYYY-MM-DD 字符串序即时间序）
  const bars = [...props.bars].sort((a, b) => a.date.localeCompare(b.date))

  candleSeries.setData(
    bars.map(b => ({
      time: b.date as Time,
      open: b.open,
      high: b.high,
      low: b.low,
      close: b.close,
    }))
  )

  // 成交量柱涨红跌绿
  volumeSeries.setData(
    bars.map(b => ({
      time: b.date as Time,
      value: b.volume,
      color: b.close >= b.open ? UP_COLOR : DOWN_COLOR,
    }))
  )

  // MA5/10/20：前 n-1 项数据不足为 null，用 whitespace 占位保持时间轴对齐
  const closes = bars.map(b => b.close)
  const maValues = MA_CONFIG.map(cfg => ma(closes, cfg.n))
  maSeries.forEach((s, i) => {
    s.setData(
      bars.map((b, j) => {
        const v = maValues[i][j]
        return v === null ? { time: b.date as Time } : { time: b.date as Time, value: v }
      })
    )
  })

  // 图例取最新 MA 值
  maLegend.value = MA_CONFIG.map((cfg, i) => {
    const values = maValues[i]
    const last = values[values.length - 1]
    return {
      n: cfg.n,
      color: cfg.color,
      value: last === null || last === undefined ? '--' : last.toFixed(2),
    }
  })

  // 首次渲染或柱数变化时自适应视野，同柱数的数据更新不重置用户缩放/平移。
  // 两端各留少量逻辑 bar 空白：首尾日期标签以边缘 bar 为中心绘制，
  // fitContent 会将标签半截裁切，留白后完整可见（同 MinuteChart 留白做法）
  if (prevBarCount !== bars.length) {
    chart.timeScale().setVisibleLogicalRange({ from: -3, to: bars.length + 2 })
    prevBarCount = bars.length
  }
}

function handleResize(): void {
  if (!chart || !container.value) return
  chart.applyOptions({
    width: container.value.clientWidth,
    height: container.value.clientHeight,
  })
}

onMounted(() => {
  if (props.bars.length > 0) {
    createChartInstance()
    updateData()
  }
  if (container.value && typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(handleResize)
    resizeObserver.observe(container.value)
    handleResize()
  }
})

watch(
  () => props.bars,
  () => {
    if (props.bars.length === 0) return
    if (!chart) createChartInstance()
    updateData()
  }
)

onUnmounted(() => {
  resizeObserver?.disconnect()
  resizeObserver = null
  chart?.remove()
  chart = null
  candleSeries = null
  volumeSeries = null
  maSeries = []
})
</script>

<template>
  <div class="relative w-full h-full">
    <!-- MA 图例 -->
    <div class="absolute top-1 left-2 z-10 flex gap-3 text-[10px] pointer-events-none">
      <span v-for="m in maLegend" :key="m.n" :style="{ color: m.color }">MA{{ m.n }} {{ m.value }}</span>
    </div>
    <div ref="container" class="w-full h-full"></div>
  </div>
</template>

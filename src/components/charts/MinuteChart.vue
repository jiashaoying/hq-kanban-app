<script setup lang="ts">
import { onMounted, onUnmounted, ref, watch } from 'vue'
import {
  createChart,
  AreaSeries,
  LineSeries,
  HistogramSeries,
  ColorType,
  LineStyle,
  CrosshairMode,
  type IChartApi,
  type ISeriesApi,
  type IPriceLine,
  type UTCTimestamp,
  type Time,
} from 'lightweight-charts'
import type { MinuteData } from '../../types/market'

const props = defineProps<{ data: MinuteData }>()

const UP_COLOR = '#ef232a'
const DOWN_COLOR = '#14b143'
const GRID_COLOR = '#374151'
const TEXT_COLOR = '#9ca3af'

// 合成时间轴基准：固定日期 2024-01-01 00:00 UTC，index*60s 等间隔
// （消除 A 股 11:30→13:00 午休及停牌造成的时间轴空洞）
const BASE_TS = Date.UTC(2024, 0, 1) / 1000

// 各市场全天分时点数（兜底时间轴预留宽度）：
// A 股 4 小时 ≈ 242 点；港股 5.5 小时 ≈ 330 点；美股 6.5 小时 ≈ 390 点
function fullDayPoints(code: string): number {
  if (/^(sh|sz|bj)/i.test(code)) return 242
  if (/^hk/i.test(code)) return 330
  return 390
}

const container = ref<HTMLDivElement | null>(null)
let chart: IChartApi | null = null
let priceSeries: ISeriesApi<'Area'> | null = null
let percentSeries: ISeriesApi<'Line'> | null = null
let volumeSeries: ISeriesApi<'Histogram'> | null = null
let prevCloseLine: IPriceLine | null = null
let pctZeroLine: IPriceLine | null = null
let resizeObserver: ResizeObserver | null = null
// index → 真实 "HH:MM" 映射，供 tickMarkFormatter 闭包读取
let timeLabels: string[] = []
// 以昨收（0%）为中心的对称涨跌幅半幅（percent 空间）。主图（价格空间）与
// 右轴（百分比空间）的 autoscaleInfoProvider 闭包共同读取：两轴同边距 +
// 等价线性映射 ⇒ 刻度一一对应；每次 updateData 随盘中新高/新低重算
let pctHalf: number | null = null

// 右轴格式化：正负号均显示（0 不带符号），与左轴刻度线严格对齐
const pctAxisFormatter = (v: number): string =>
  `${v > 0 ? '+' : ''}${v.toFixed(2)}%`

function syntheticTime(index: number): UTCTimestamp {
  return (BASE_TS + index * 60) as UTCTimestamp
}

// 分钟量柱红涨绿跌：与上一分钟价比较，首根对照昨收
function volumeColorAt(price: number, prevPrice: number): string {
  return price >= prevPrice ? UP_COLOR : DOWN_COLOR
}

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
    // 左轴/右轴/overlay 量柱轴必须在建图时一次性给足——v5 中
    // leftPriceScale.visible 默认 false，且 chart.priceScale('right', N)
    // 的运行时 applyOptions 会走 chart 级合并分支污染全局 options（model 与
    // widget 共享同一对象），曾导致双轴刻度全部消失，故这里全部初始化声明
    leftPriceScale: {
      visible: true,
      borderColor: GRID_COLOR,
      scaleMargins: { top: 0.08, bottom: 0.08 },
    },
    rightPriceScale: {
      visible: true,
      borderColor: GRID_COLOR,
      scaleMargins: { top: 0.08, bottom: 0.08 },
    },
    // 量柱挂自定义 overlay scale（'vol'）：其轴不可见性由 v5 默认值保证
    // （overlayPriceScales 默认 visible=false，类型已移除该字段），无需显式配置
    timeScale: {
      borderColor: GRID_COLOR,
      rightOffset: 0,
      barSpacing: 6,
      // 合成时间戳反查 index，映射回真实 "HH:MM"
      tickMarkFormatter: (time: Time) => {
        const idx = Math.round((Number(time) - BASE_TS) / 60)
        return timeLabels[idx] ?? ''
      },
    },
    crosshair: { mode: CrosshairMode.Normal },
  })

  // 价格主图（pane 0，左轴，数据直接存指数点位——放弃"存涨跌幅 + formatter
  // 换算"的双层换算方案，两层换算难以维护且易错）：垂直范围由
  // autoscaleInfoProvider 接管，以昨收为中心上下对称缩放，昨收虚线恒居正中
  priceSeries = chart.addSeries(AreaSeries, {
    priceScaleId: 'left',
    lineWidth: 2,
    priceLineVisible: false,
    lastValueVisible: true,
    priceFormat: { type: 'custom', minMove: 0.01, formatter: (v: number) => v.toFixed(2) },
    autoscaleInfoProvider: () =>
      pctHalf !== null
        ? {
            priceRange: {
              minValue: props.data.prev_close * (1 - pctHalf / 100),
              maxValue: props.data.prev_close * (1 + pctHalf / 100),
            },
          }
        : null,
  })

  // 涨跌幅右轴（pane 0）：不可见 LineSeries 作为右轴数据载体，直接喂百分比
  // 数值（formatter 只处理显示格式）。全透明颜色 + 禁用标记实现"隐形"；
  // 与左轴相同对称范围（等价线性映射）+ 相同 scaleMargins ⇒ 刻度严格对齐
  percentSeries = chart.addSeries(LineSeries, {
    priceScaleId: 'right',
    color: 'rgba(0, 0, 0, 0)',
    lineWidth: 1,
    priceLineVisible: false,
    lastValueVisible: false,
    crosshairMarkerVisible: false,
    pointMarkersVisible: false,
    priceFormat: { type: 'custom', minMove: 0.01, formatter: pctAxisFormatter },
    autoscaleInfoProvider: () =>
      pctHalf !== null ? { priceRange: { minValue: -pctHalf, maxValue: pctHalf } } : null,
  })

  // 分钟成交量副图（pane 1）：挂 overlay scale 'vol'，轴不可见由 v5 默认值
  // 保证（overlay scale 默认 visible=false；不可运行时 applyOptions 修改
  // 'right'/'left' 默认 scale，会污染 chart 级 options 导致布局异常）
  volumeSeries = chart.addSeries(
    HistogramSeries,
    {
      priceScaleId: 'vol',
      priceFormat: { type: 'volume' },
      priceLineVisible: false,
      lastValueVisible: false,
    },
    1
  )
  // 主图:量区 = 4:1（量区约占底部 20%）
  const panes = chart.panes()
  if (panes.length > 1) {
    panes[0].setStretchFactor(4)
    panes[1].setStretchFactor(1)
  }
}

function updateData(): void {
  const data = props.data
  const points = data.points
  if (!chart || !priceSeries || !percentSeries || !volumeSeries || points.length < 2) return

  timeLabels = points.map(p => p.time)

  // 红涨绿跌：按最新价相对昨收决定整条 area 颜色
  const lastPrice = points[points.length - 1].price
  const up = lastPrice >= data.prev_close
  priceSeries.applyOptions({
    lineColor: up ? UP_COLOR : DOWN_COLOR,
    topColor: up ? 'rgba(239, 35, 42, 0.25)' : 'rgba(20, 177, 67, 0.25)',
    bottomColor: 'rgba(0, 0, 0, 0)',
  })

  // 主图直接喂价格值；右轴直接喂百分比值（两序列同源同长度，经等价对称
  // 缩放后刻度恒对齐：昨收价 ↔ 0.00% 同一高度）
  priceSeries.setData(points.map((p, i) => ({ time: syntheticTime(i), value: p.price })))
  percentSeries.setData(
    points.map((p, i) => ({
      time: syntheticTime(i),
      value: data.prev_close > 0 ? ((p.price - data.prev_close) / data.prev_close) * 100 : 0,
    }))
  )

  // 对称缩放半幅（percent 空间）：以昨收 0% 为轴心，涨跌两侧等幅展开；
  // 最小 0.2% 防横盘时范围塌缩，12% 缓冲避免价格线贴边
  if (data.prev_close > 0) {
    let hi = 0
    let lo = 0
    for (const p of points) {
      const v = ((p.price - data.prev_close) / data.prev_close) * 100
      if (v > hi) hi = v
      if (v < lo) lo = v
    }
    pctHalf = Math.max(hi, -lo, 0.2) * 1.12
  }

  // 昨收基准虚线（左轴昨收价标签 + 右轴 0.00% 标签，两条线几何同高）。
  // 对称缩放下昨收恒居正中，对齐新浪/百度分时版式
  if (prevCloseLine) {
    priceSeries.removePriceLine(prevCloseLine)
    prevCloseLine = null
  }
  if (pctZeroLine) {
    percentSeries.removePriceLine(pctZeroLine)
    pctZeroLine = null
  }
  if (data.prev_close > 0) {
    prevCloseLine = priceSeries.createPriceLine({
      price: data.prev_close,
      color: TEXT_COLOR,
      lineWidth: 1,
      lineStyle: LineStyle.Dashed,
      axisLabelVisible: true,
      title: '昨收',
    })
    pctZeroLine = percentSeries.createPriceLine({
      price: 0,
      color: 'rgba(0, 0, 0, 0)',
      lineWidth: 1,
      lineStyle: LineStyle.Dashed,
      axisLabelVisible: true,
      title: '',
    })
  }

  // 分钟成交量：首条 volume=0 属预期（无上条累计量参照），正常渲染
  volumeSeries.setData(
    points.map((p, i) => ({
      time: syntheticTime(i),
      value: p.volume,
      color: volumeColorAt(p.price, i > 0 ? points[i - 1].price : data.prev_close),
    }))
  )

  // 每次轮询都重设为全天视野：走势线从左向右生长，右侧预留至收盘
  // （09:30~15:00 约 242 点，A 股；其他市场按各自全天点数兜底）。
  // 两端各留 4 根 bar 空白：首尾 "09:30"/"15:00" 标签以边缘 bar 为中心
  // 绘制，不留白会被裁切半截（此前截图 ":30"/"14:5" 问题）
  const total = Math.max(points.length, fullDayPoints(data.code))
  chart.timeScale().setVisibleLogicalRange({ from: -4, to: total + 4 })
}

function handleResize(): void {
  if (!chart || !container.value) return
  chart.applyOptions({
    width: container.value.clientWidth,
    height: container.value.clientHeight,
  })
}

onMounted(() => {
  if (props.data.points.length >= 2) {
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
  () => props.data,
  () => {
    if (props.data.points.length < 2) return
    if (!chart) createChartInstance()
    updateData()
  }
)

onUnmounted(() => {
  resizeObserver?.disconnect()
  resizeObserver = null
  chart?.remove()
  chart = null
  priceSeries = null
  percentSeries = null
  volumeSeries = null
  prevCloseLine = null
  pctZeroLine = null
})
</script>

<template>
  <div ref="container" class="w-full h-full"></div>
</template>

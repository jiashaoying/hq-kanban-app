<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
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

// visible：v-show 页面化的显隐状态（与 wrapper 的 v-show 同一表达式，保证一致）——
// 建图只在 visible === true 时发生（隐藏期不建图）；隐藏期已存在的实例在转为可见时
// 销毁重建，回归唯一被证明可靠的渲染路径——「图层可见状态下创建图表实例」。
// debugTab：IndexDetail 传入的当前 tab 标识（保留 prop 契约，组件内不消费）
const props = defineProps<{ bars: KlineBar[]; debugTab?: string; visible: boolean }>()

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
// 可见态补建图的一帧 rAF id：卸载/再次隐藏时取消，避免回调触碰已销毁实例
let visibleRafId = 0
// initChart 执行时图层不可见 → 该实例属「隐藏期间创建」，转为可见时必须销毁重建
// （隐藏期创建的实例在 WKWebView 中 canvas 空白，数据/尺寸/视野全对仍不上屏）
let createdWhileHidden = false
// 上次渲染的柱数：首次渲染或柱数变化（如新交易日/新周期柱出现）时自适应视野，
// 其余更新（缓存 TTL 回源刷新）保留用户缩放/平移状态
let prevBarCount = -1

// 图例：显示最新一根的 MA 值
const maLegend = ref<{ n: number; color: string; value: string }[]>([])

function createChartInstance(): void {
  if (!container.value) return
  chart = createChart(container.value, {
    // 显式传入尺寸，不依赖 ResizeObserver——切 tab 路径下 RO 初始回调可能早于建图
    // 触发（chart 为 null 直接 return），之后容器尺寸不变 RO 永不回调，画布会停留
    // 默认 300x150 → 空白
    width: container.value.clientWidth,
    height: container.value.clientHeight,
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
  // 不采用 overlay priceScaleId 方案：该配置在 v5.2.1 运行时未能将量柱挂到独立 scale，
  // 量值（万手级）与价格（点位）共轴导致主图被压扁。
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
  // fitContent 会将标签半截裁切，留白后完整可见（同 MinuteChart 留白做法）。
  // 0 宽守卫失败（布局过渡态）时视野设置未生效，不得更新 prevBarCount——
  // 否则后续同柱数更新永不再重试视野 → 永久空白
  if (prevBarCount !== bars.length) {
    const ok = applyVisibleRange(bars.length)
    if (ok) prevBarCount = bars.length
  }
  // 数据更新后若位图仍未绑定（300x150）则补一次同步绘制（已绑定则 no-op，零开销）
  ensurePainted()
}

// 视野设置守卫：容器宽度 <= 0 时禁止调用——lightweight-charts 内部按
// width/(to-from) 折算 barSpacing 且 correctBarSpacing 的上限为 width*0.5，
// 0 宽度下折算结果被 clamp 成 0 存入 timeScale，后续 resize 不重算
// （保留缩放语义），所有 bar 宽度为 0 → canvas 空白。
// 返回 boolean 标记是否实际生效，失败态由调用方决定重试
// （updateData 的 prevBarCount 只在成功时推进；ensureVisibleRange 轮询重试）
function applyVisibleRange(barCount: number): boolean {
  if (!chart || !container.value || container.value.clientWidth <= 0) {
    return false
  }
  chart.timeScale().setVisibleLogicalRange({ from: -3, to: barCount + 2 })
  return true
}

// 视野重放轮询——叠放布局下 visibility 切换不触发 RO，「0→正 宽度恢复重放」兜底
// （handleResize）走不到；布局过渡态（分时↔K线同帧互切）下 clientWidth 可持续多帧
// 为 0。逐帧轮询至宽度为正立即重放视野，60 帧（约 1 秒）上限防死循环；
// 卸载/未建图直接放弃。幂等、失败无害
function ensureVisibleRange(framesLeft = 60): void {
  if (!chart || !container.value) return
  if (container.value.clientWidth > 0) {
    applyVisibleRange(props.bars.length)
    return
  }
  if (framesLeft <= 0) return
  requestAnimationFrame(() => ensureVisibleRange(framesLeft - 1))
}

/**
 * 强制同步绘制：lightweight-charts v5 中 canvas 位图尺寸只在 _internal_paint →
 * applySuggestedBitmapSize 时绑定，而全库唯一的同步绘制入口是
 * resize(w, h, forceRepaint=true) 且尺寸必须与已存值不同（相同则库内 _internal_resize
 * 第一行 early-return，forceRepaint 被吞）。suggestChartSize 会把宽高向下取偶数，故
 * nudge 必须用 +2（+1 会被取偶抹平，等于没变）。两次调用在同一同步块内完成，无可见闪烁。
 */
function forceSyncPaint(): void {
  const el = container.value
  if (!el || !chart) return
  const w = el.clientWidth
  const h = el.clientHeight
  if (w <= 0 || h <= 0) return
  try {
    chart.resize(w + 2, h, true) // 同步绘制 #1（尺寸不同 → 不被 early-return）
    chart.resize(w, h, true) // 同步绘制 #2（回到真实尺寸）
  } catch {
    /* 无害 */
  }
}

/** 画布仍为 HTML 默认尺寸（未绑定位图）时补一次同步绘制（幂等，可频繁调用） */
function ensurePainted(): void {
  const el = container.value
  if (!el || !chart) return
  const cv = el.querySelector('canvas')
  if (!cv) return
  // 300x150 = HTML canvas 默认尺寸，说明库从未 paint 过
  if (cv.width === 300 && cv.height === 150) forceSyncPaint()
}

function handleResize(): void {
  if (!chart || !container.value) return
  const w = container.value.clientWidth
  const h = container.value.clientHeight
  // RO 回调做尺寸同步；forceRepaint=true 强制重绘
  chart.resize(w, h, true)
  // RO resize 后补一次自检（若仍 300x150 未绑定位图则强制同步绘制）
  ensurePainted()
}

// 延迟建图助手：containerReady 判断容器是否已有真实宽高。
// 隐藏期不建图后已不存在 0 宽建图路径——visible → true 时容器必然已完成布局，
// 仅保留 containerReady 检查 + 一帧 rAF 兜底
function containerReady(): boolean {
  return !!container.value && container.value.clientWidth > 0 && container.value.clientHeight > 0
}

// 唯一建图入口，调用方（onMounted / watch(visible)）均已以 props.visible 门控，
// createdWhileHidden 正常路径恒为 false；置 true 仅出现在边缘态（如 HMR 保留实例），
// 由 watch(visible) → true 的重建分支销毁重建。
// 建图同帧的布局未稳，updateData 延迟一帧执行——下一帧布局已定，首喂与视野折算
// 均基于真实尺寸；ensureVisibleRange 兜底重放
function initChart(): void {
  if (!container.value || chart) return
  createdWhileHidden = !props.visible
  createChartInstance()
  // 建图后立刻强制同步绘制：createChart 传的宽高与库内已存值相同，
  // chart.resize(w,h,true) 会被 _internal_resize 第一行 early-return 吞掉
  // forceRepaint，位图从不绑定；forceSyncPaint 用 +2 nudge 触发两次同步绘制。
  forceSyncPaint()
  if (props.bars.length > 0) {
    requestAnimationFrame(() => {
      if (!chart || !container.value) return // 卸载防护
      updateData()
      // 数据喂入只排队 rAF，再强制同步绘制一次确保位图绑定
      forceSyncPaint()
      ensureVisibleRange()
    })
  }
}

onMounted(() => {
  if (container.value && typeof ResizeObserver !== 'undefined') {
    resizeObserver = new ResizeObserver(handleResize)
    resizeObserver.observe(container.value)
  }
  // 建图门控：图层不可见时不建图——隐藏期创建的实例在 WKWebView 中 canvas 空白
  // （数据/尺寸/视野全对仍不上屏）。数据留在 props.bars，watch(visible) → true 时
  // 在可见状态下建图并首喂
  if (!props.visible) return
  if (containerReady()) {
    initChart()
  } else {
    // 一帧 rAF 兜底：极端布局过渡（可见态下理论不发生）时下一帧补建
    visibleRafId = requestAnimationFrame(() => {
      visibleRafId = 0
      if (!props.visible || !container.value || chart) return
      initChart()
    })
  }
})

watch(
  () => props.bars,
  () => {
    if (props.bars.length === 0) return
    // 未建图（隐藏期不建图）——只记录不建图，props.bars 本身即最新引用无需暂存，
    // visible → true 建图时 updateData 自然以最新数据首喂
    if (!chart) return
    // 已建图（必为可见态创建）：照常 setData + 视野推进
    updateData()
  }
)

watch(
  () => props.visible,
  (vis) => {
    if (!vis) {
      // 变为隐藏：取消待执行的可见态补建图 rAF（实例保留，转为可见时
      // 按 createdWhileHidden 判定是否重建）
      if (visibleRafId !== 0) {
        cancelAnimationFrame(visibleRafId)
        visibleRafId = 0
      }
      return
    }
    // 可见态重建架构：唯一被证明可靠的渲染路径是「图层可见状态下创建图表实例」。
    // 隐藏期创建/存在过的实例转为可见时销毁重建，而非跑展示触发链（DOM 触发/
    // crosshair 复刻/数据重喂/原生窗口 resize 均不能使其上屏）
    nextTick(() => {
      if (!props.visible) return // 翻转后立即又隐藏（极快切换）
      if (chart && createdWhileHidden) {
        // 销毁隐藏期实例，在可见状态下重建 + 喂数据（走可靠路径）
        try {
          chart.remove()
        } catch { /* 已销毁等异常无害 */ }
        chart = null
        candleSeries = null
        volumeSeries = null
        maSeries = []
        prevBarCount = 0
        createdWhileHidden = false
        initChart()
        return
      }
      if (chart) {
        // 可见态创建且一直存活的实例转回可见——强制同步绘制补画
        // （v-show 隐藏期 rAF 停摆，位图可能仍未绑定），再重放视野
        forceSyncPaint()
        ensureVisibleRange()
        return
      }
      // 未建图（隐藏期挂载，建图门控拦下）：此刻图层已可见，走首次进入
      // 详情页的可靠路径建图 + 首喂
      if (!container.value) return
      if (containerReady()) {
        initChart()
      } else {
        // 一帧 rAF 兜底：布局尚未完成时下一帧再建（不再逐帧轮询）
        if (visibleRafId !== 0) cancelAnimationFrame(visibleRafId)
        visibleRafId = requestAnimationFrame(() => {
          visibleRafId = 0
          if (!props.visible || !container.value || chart) return
          initChart()
        })
      }
    })
  }
)

onUnmounted(() => {
  if (visibleRafId !== 0) {
    cancelAnimationFrame(visibleRafId)
    visibleRafId = 0
  }
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

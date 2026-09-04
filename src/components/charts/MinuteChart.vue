<script setup lang="ts">
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue'
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

// visible：v-show 页面化的显隐状态（与 wrapper 的 v-show 同一表达式，保证一致）——
// 建图只在 visible === true 时发生（隐藏期不建图）；隐藏期已存在的实例转为可见时
// 销毁重建，回归唯一被证明可靠的渲染路径——「图层可见状态下创建图表实例」。
// 15s 轮询的数据层不受影响：隐藏期照常拉数据更新 props.data
const props = defineProps<{ data: MinuteData; visible: boolean }>()

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
// 可见态补建图的一帧 rAF id：卸载/再次隐藏时取消，避免回调触碰已销毁实例
let visibleRafId = 0
// initChart 执行时图层不可见 → 该实例属「隐藏期间创建」，转为可见时必须销毁重建
// （隐藏期创建的实例在 WKWebView 中 canvas 空白，数据/尺寸/视野全对仍不上屏）
let createdWhileHidden = false
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
  // 绘制，不留白会被裁切半截
  applyVisibleRange()
  // 数据更新后若位图仍未绑定（300x150）则补一次同步绘制（已绑定则 no-op）
  ensurePainted()
}

// 视野设置守卫：容器宽度 <= 0 时禁止调用（v-if 互斥渲染下挂载时容器必然
// 可见，此为异常布局兜底）——0 宽度会把内部 barSpacing 折算成 0，且后续
// 外部 resize 不重算（保留缩放语义），显示后 canvas 空白。
// 返回 boolean 标记是否实际生效，失败态由调用方重试
// （本组件无 prevBarCount 守卫，每次数据更新天然重试；ensureVisibleRange 轮询兜底）
function applyVisibleRange(): boolean {
  if (!chart || !container.value || container.value.clientWidth <= 0) {
    return false
  }
  const total = Math.max(props.data.points.length, fullDayPoints(props.data.code))
  chart.timeScale().setVisibleLogicalRange({ from: -4, to: total + 4 })
  return true
}

// 视野重放轮询（与 KlineChart 同构）——叠放布局下
// visibility 切换不触发 RO，布局过渡态（分时↔K线同帧互切）下 clientWidth
// 可持续多帧为 0。逐帧轮询至宽度为正立即重放全天视野，60 帧上限防死循环；
// 卸载/未建图直接放弃。幂等、失败无害
function ensureVisibleRange(framesLeft = 60): void {
  if (!chart || !container.value) return
  if (container.value.clientWidth > 0) {
    applyVisibleRange()
    return
  }
  if (framesLeft <= 0) return
  requestAnimationFrame(() => ensureVisibleRange(framesLeft - 1))
}

/**
 * 强制同步绘制：lightweight-charts v5 中 canvas 位图尺寸只在
 * _internal_paint → applySuggestedBitmapSize 时绑定，而全库唯一的同步绘制入口是
 * resize(w, h, forceRepaint=true) 且尺寸必须与已存值不同（相同则 _internal_resize
 * 第一行 return，forceRepaint 被吞）。suggestChartSize 会把宽高向下取偶数，故 nudge
 * 必须用 +2（+1 会被取偶抹平，等于没变）。两次调用在同一同步块内完成，无可见闪烁。
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
  // RO 回调只做尺寸同步（可见态建图下不存在 0 宽实例）；forceRepaint=true 强制重绘
  chart.resize(container.value.clientWidth, container.value.clientHeight, true)
  // RO resize 后补一次自检（若仍 300x150 未绑定位图则强制同步绘制）
  ensurePainted()
}

// 延迟建图两助手（与 KlineChart 同构）：containerReady 判断容器是否已有
// 真实宽高；initChart 在可见状态下建图并以当前 points 首喂数据。
// 隐藏期不建图后已不存在 0 宽建图路径；15s 轮询的数据层不受影响
// （隐藏期照常拉数据更新 props.data）
function containerReady(): boolean {
  return !!container.value && container.value.clientWidth > 0 && container.value.clientHeight > 0
}

// createdWhileHidden 在此登记——本函数是唯一建图入口，调用方（onMounted /
// watch(visible)）均已以 props.visible 门控，正常路径恒为 false；置 true 仅出现在
// 边缘态（如 HMR 保留实例），由 watch(visible) → true 的重建分支销毁重建。
// 建图同帧布局未稳，updateData 延迟一帧执行——下一帧布局已定，首喂与视野折算
// 均基于真实尺寸；ensureVisibleRange 兜底重放
function initChart(): void {
  if (!container.value || chart) return
  createdWhileHidden = !props.visible
  createChartInstance()
  // 建图后立刻强制同步绘制：createChart 传的宽高与库内已存值相同，
  // chart.resize(w,h,true) 会被 _internal_resize 第一行 early-return 吞掉
  // forceRepaint，位图从不绑定；forceSyncPaint 用 +2 nudge 触发两次同步绘制。
  forceSyncPaint()
  if (props.data.points.length >= 2) {
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
  // （数据/尺寸/视野全对仍不上屏）。15s 轮询照常更新 props.data，
  // watch(visible) → true 时在可见状态下建图并首喂
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
  () => props.data,
  () => {
    if (props.data.points.length < 2) return
    // 未建图（隐藏期不建图）——只记录不建图，props.data 本身即最新引用无需暂存，
    // visible → true 建图时 updateData 自然以最新数据首喂
    if (!chart) return
    // 已建图（必为可见态创建）：15s 轮询更新照常 setData + 全天视野重设
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
    // 可见态重建架构（与 KlineChart 同构）：唯一被证明可靠的渲染路径是
    // 「图层可见状态下创建图表实例」。隐藏期创建/存在过的实例转为可见时销毁重建，
    // 而非跑展示触发链（DOM 触发/crosshair 复刻/数据重喂/原生窗口 resize 均无效）
    nextTick(() => {
      if (!props.visible) return // 翻转后立即又隐藏（极快切换）
      if (chart && createdWhileHidden) {
        // 销毁隐藏期实例，在可见状态下重建 + 喂数据（走可靠路径）。
        // series 与两条基准线引用必须全部清空（IPriceLine 随 chart 一同销毁）
        try {
          chart.remove()
        } catch { /* 已销毁等异常无害 */ }
        chart = null
        priceSeries = null
        percentSeries = null
        volumeSeries = null
        prevCloseLine = null
        pctZeroLine = null
        pctHalf = null
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

---
kind: frontend_style
name: 基于 Tailwind CSS v4 的暗色主题桌面 UI 风格
category: frontend_style
scope:
    - '**'
source_files:
    - src/styles.css
    - src/App.vue
    - src/components/IndexCard.vue
    - src/components/MarketSection.vue
    - src/components/TitleBar.vue
---

## 1. 使用的系统/方法
- 样式框架：Tailwind CSS v4（通过 `@import "tailwindcss"` 引入，见 `src/styles.css`）。
- 无独立 SCSS/Less、CSS Modules、CSS-in-JS 或第三方 UI 组件库；所有视觉样式以原子化 utility class 直接写在 Vue 模板中。
- 主题色调：全局暗色主题，主背景为 `bg-gray-900`，文字默认白色，次要信息使用 `text-gray-400/500` 等灰色阶。
- 响应式策略：未使用媒体查询，布局依赖 Flexbox/Grid 与 Tailwind 的栅格类（如 `grid grid-cols-3 gap-3`），整体采用固定窗口尺寸适配桌面端。

## 2. 关键文件
- `src/styles.css`：唯一的全局样式入口，仅导入 Tailwind。
- `src/App.vue`：应用根容器，定义全屏暗色背景、标题栏 + 滚动内容区的基本骨架。
- `src/components/IndexCard.vue`：行情卡片，集中体现涨跌态的颜色约定（涨红 `text-red-500` / `bg-red-500/5`，跌绿 `text-green-500` / `bg-green-500/5`）。
- `src/components/MarketSection.vue`：分组容器，使用 `grid grid-cols-3 gap-3` 排列卡片。
- `src/components/TitleBar.vue`：自定义 Tauri 标题栏，统一 `bg-gray-900 border-b border-gray-700` 风格，并集成拖拽、最小化、关闭按钮。

## 3. 架构与约定
- 样式组织：单文件全局样式 + 组件内联 utility class 的组合方式。没有独立的 `.scss`/`.css` 模块，样式职责完全落在 Vue SFC 的 `<template>` 中。
- 设计 token：项目未定义 CSS 变量或 design tokens，颜色、间距、字号全部直接使用 Tailwind 预设值（如 `gray-900`、`text-2xl`、`p-4`、`gap-3`）。
- 状态驱动样式：通过 computed 派生 `trendColor`、`trendBg`、`trendBorder` 三类 class 名，再绑定到元素上，实现涨跌态的视觉切换。
- 交互反馈：统一使用 `transition-colors`、`hover:text-*`、`hover:bg-gray-700` 提供 hover 过渡效果；加载状态用 `animate-pulse` 脉冲指示器表示。
- 图标：不使用图标库，直接在模板中内联 SVG（如 TitleBar 的最小化、关闭图标）。
- 布局模式：App 根层 `h-screen flex flex-col`，标题栏固定高度 `h-10`，内容区 `flex-1 overflow-y-auto`，形成典型的桌面应用“顶栏 + 可滚动主体”结构。

## 4. 约定与约束
- 颜色语义约定（描述性，非强制校验）：A 股涨跌沿用国内惯例——上涨用红色（`text-red-500`）、下跌用绿色（`text-green-500`），中性/平盘用灰色（`text-gray-400`）。
- 卡片边框强调：通过 `border-l-4` 配合趋势色实现左侧彩色条，作为涨跌状态的视觉锚点。
- 文本层级：名称/标签用小字号灰色（`text-sm text-gray-400`），价格用大字号加粗（`text-2xl font-bold`），成交量/成交额用更小字号灰色（`text-xs text-gray-500`）。
- 组件边界：每个组件只关注自身样式，不引入外部 CSS；跨组件一致性由复用相同的 Tailwind 预设值保证。
- 无构建期样式检查：仓库中未发现 ESLint/Prettier 对样式类的规则配置，样式约束主要依靠团队约定和 Tailwind 内置类名。
- 平台适配：样式面向桌面端窗口，未包含移动端断点或响应式前缀；Tauri 自定义标题栏通过 `select-none` 禁用选中等手段模拟原生体验。
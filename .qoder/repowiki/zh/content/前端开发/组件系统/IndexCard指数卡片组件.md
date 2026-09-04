# IndexCard指数卡片组件

<cite>
**本文引用的文件**
- [IndexCard.vue](file://src/components/IndexCard.vue)
- [MarketSection.vue](file://src/components/MarketSection.vue)
- [App.vue](file://src/App.vue)
- [IndexDetail.vue](file://src/components/IndexDetail.vue)
- [useIndexDetail.ts](file://src/composables/useIndexDetail.ts)
- [market.ts](file://src/types/market.ts)
- [format.ts](file://src/utils/format.ts)
- [useMarketData.ts](file://src/composables/useMarketData.ts)
- [styles.css](file://src/styles.css)
</cite>

## 更新摘要
**变更内容**
- 新增点击事件处理功能，支持跳转到指数详情页
- 添加了select事件的emit机制
- 实现了完整的事件冒泡链：IndexCard → MarketSection → App
- 集成了异步加载的详情页面组件
- 增强了用户体验，提供流畅的导航交互

## 目录
1. [简介](#简介)
2. [项目结构](#项目结构)
3. [核心组件与职责](#核心组件与职责)
4. [架构总览](#架构总览)
5. [详细组件分析](#详细组件分析)
6. [事件处理机制](#事件处理机制)
7. [依赖关系分析](#依赖关系分析)
8. [性能考量](#性能考量)
9. [故障排查指南](#故障排查指南)
10. [结论](#结论)
11. [附录：Props接口与数据字段映射](#附录props接口与数据字段映射)

## 简介
IndexCard 是一个用于展示单个指数信息的交互式卡片组件，负责呈现指数名称、当前价格、涨跌额、涨跌幅以及成交量/成交额等关键指标。组件不仅具备基础的展示功能，还新增了点击事件处理，支持用户点击卡片后跳转到对应的指数详情页，查看分时线和K线图等详细信息。组件通过计算属性根据涨跌情况动态决定颜色、背景与左侧边框样式，并使用统一的格式化函数对数值进行精度控制与单位转换。

## 项目结构
本项目的 UI 层由 Vue 3 + TypeScript 构建，使用 Tailwind CSS 进行样式管理。IndexCard 作为基础展示单元，被 MarketSection 按市场分组渲染；数据由 useMarketData 组合式函数定时拉取并分组，最终在 App.vue 中组织页面布局。新增的详情页面通过异步加载实现，确保首屏性能。

```mermaid
graph TB
App["App.vue"] --> MarketSection["MarketSection.vue"]
MarketSection --> IndexCard["IndexCard.vue"]
IndexCard --> Event["事件冒泡"]
Event --> App
App --> IndexDetail["IndexDetail.vue (异步加载)"]
IndexDetail --> UseIndexDetail["composables/useIndexDetail.ts"]
IndexCard --> Format["utils/format.ts"]
IndexCard --> Types["types/market.ts"]
App --> UseMarketData["composables/useMarketData.ts"]
UseMarketData --> Types
App --> Styles["styles.css (Tailwind)"]
```

图表来源
- [App.vue:1-68](file://src/App.vue#L1-L68)
- [MarketSection.vue:1-26](file://src/components/MarketSection.vue#L1-L26)
- [IndexCard.vue:1-74](file://src/components/IndexCard.vue#L1-L74)
- [IndexDetail.vue:1-140](file://src/components/IndexDetail.vue#L1-L140)
- [useIndexDetail.ts:1-144](file://src/composables/useIndexDetail.ts#L1-L144)
- [format.ts:1-33](file://src/utils/format.ts#L1-L33)
- [market.ts:1-54](file://src/types/market.ts#L1-L54)
- [useMarketData.ts:1-66](file://src/composables/useMarketData.ts#L1-L66)
- [styles.css:1-2](file://src/styles.css#L1-L2)

章节来源
- [App.vue:1-68](file://src/App.vue#L1-L68)
- [MarketSection.vue:1-26](file://src/components/MarketSection.vue#L1-L26)
- [IndexCard.vue:1-74](file://src/components/IndexCard.vue#L1-L74)
- [IndexDetail.vue:1-140](file://src/components/IndexDetail.vue#L1-L140)
- [useIndexDetail.ts:1-144](file://src/composables/useIndexDetail.ts#L1-L144)
- [format.ts:1-33](file://src/utils/format.ts#L1-L33)
- [market.ts:1-54](file://src/types/market.ts#L1-L54)
- [useMarketData.ts:1-66](file://src/composables/useMarketData.ts#L1-L66)
- [styles.css:1-2](file://src/styles.css#L1-L2)

## 核心组件与职责
- IndexCard.vue：接收一个指数数据对象，计算涨跌趋势与对应样式，调用格式化函数输出可读的数值文本，并处理点击事件向父组件发送选择信号。
- MarketSection.vue：按市场分组渲染一组 IndexCard，转发子组件的选择事件到父组件。
- App.vue：组织整体布局，管理视图切换状态（看板/详情），监听选择事件并切换到对应的详情页面。
- IndexDetail.vue：显示指数的详细信息，包括实时报价头部和分时线/K线图。
- useIndexDetail.ts：管理详情页面的数据获取、轮询和缓存逻辑。
- format.ts：提供价格、涨跌额、涨跌幅、成交量、成交额的统一格式化逻辑，确保精度和单位一致。
- market.ts：定义指数数据结构 IndexData 与市场分组 MarketGroup，以及分时和K线相关的数据类型。
- useMarketData.ts：定时获取指数数据，按市场分组，暴露给上层组件使用。

章节来源
- [IndexCard.vue:1-74](file://src/components/IndexCard.vue#L1-L74)
- [MarketSection.vue:1-26](file://src/components/MarketSection.vue#L1-L26)
- [App.vue:1-68](file://src/App.vue#L1-L68)
- [IndexDetail.vue:1-140](file://src/components/IndexDetail.vue#L1-L140)
- [useIndexDetail.ts:1-144](file://src/composables/useIndexDetail.ts#L1-L144)
- [format.ts:1-33](file://src/utils/format.ts#L1-L33)
- [market.ts:1-54](file://src/types/market.ts#L1-L54)
- [useMarketData.ts:1-66](file://src/composables/useMarketData.ts#L1-L66)

## 架构总览
IndexCard 处于展示层的叶子节点，向上依赖格式化函数与类型定义，向下无子组件依赖。数据流自上而下：App → useMarketData → MarketSection → IndexCard。事件流自下而上：IndexCard → MarketSection → App → IndexDetail。

```mermaid
sequenceDiagram
participant User as "用户"
participant Card as "IndexCard.vue"
participant Section as "MarketSection.vue"
participant App as "App.vue"
participant Detail as "IndexDetail.vue"
participant Hook as "useIndexDetail.ts"
User->>Card : 点击指数卡片
Card->>Section : emit('select', data)
Section->>App : emit('select', data)
App->>App : selectedCode = data.code
App->>Detail : 渲染详情组件
Detail->>Hook : 初始化数据获取
Hook-->>Detail : 返回分时/K线数据
Detail-->>User : 显示详细信息
```

图表来源
- [IndexCard.vue:47](file://src/components/IndexCard.vue#L47)
- [MarketSection.vue:21](file://src/components/MarketSection.vue#L21)
- [App.vue:35](file://src/App.vue#L35)
- [IndexDetail.vue:20](file://src/components/IndexDetail.vue#L20)
- [useIndexDetail.ts:14-144](file://src/composables/useIndexDetail.ts#L14-L144)

## 详细组件分析

### 组件功能与显示逻辑
- 指数名称：直接展示 data.name。
- 当前价格：使用 formatPrice 保留两位小数。
- 涨跌额与涨跌幅：分别使用 formatChange 与 formatChangePct，带正负号与百分比。
- 成交量/成交额：使用 formatVolume 与 formatAmount，自动在"万"和"亿"之间切换单位。
- 涨跌趋势与颜色：
  - 根据 change > 0 / < 0 / = 0 计算趋势 up/down/flat。
  - 文字颜色：涨红（text-red-500）、跌绿（text-green-500）、平盘灰（text-gray-400）。
  - 背景色：涨/跌使用低透明度红色/绿色背景，平盘使用深灰背景。
  - 左侧边框：涨红/跌绿/平灰，突出趋势。

**新增功能**：点击事件处理
- 卡片根元素添加 @click 事件处理器
- 通过 emit('select', props.data) 向父组件发送选择事件
- 添加 cursor-pointer 和 hover:border-gray-600 提供视觉反馈

章节来源
- [IndexCard.vue:12-40](file://src/components/IndexCard.vue#L12-L40)
- [IndexCard.vue:44-72](file://src/components/IndexCard.vue#L44-L72)

### 数据格式化函数
- formatPrice：保留两位小数，保证价格显示一致性。
- formatChange：为正值添加"+"前缀，保留两位小数。
- formatChangePct：为正值添加"+"前缀，保留两位小数并追加"%"。
- formatVolume：当数值≥10000时转换为"亿"，否则显示"万"，均保留相应精度。
- formatAmount：同 volume 的单位换算策略。

这些函数集中处理精度与单位，避免在组件内重复实现，提升可维护性与一致性。

章节来源
- [format.ts:1-33](file://src/utils/format.ts#L1-L33)

### Props 接口与数据字段映射
IndexCard 仅接收一个 data 属性，类型为 IndexData，包含以下关键字段：
- code：指数代码（唯一标识）
- name：指数名称
- current：当前价
- prev_close：昨收（未在当前卡片直接展示）
- open：开盘价（未在当前卡片直接展示）
- high：最高价（未在当前卡片直接展示）
- low：最低价（未在当前卡片直接展示）
- change：涨跌点
- change_pct：涨跌幅（百分比）
- volume：成交量
- amount：成交额
- market：市场标识（a/hk/us）
- update_time：更新时间

组件内部将上述字段映射到模板中的展示位置，并通过格式化函数输出。

**新增事件接口**：
- select 事件：当用户点击卡片时触发，传递完整的 IndexData 对象给父组件

章节来源
- [IndexCard.vue:6-10](file://src/components/IndexCard.vue#L6-L10)
- [market.ts:1-15](file://src/types/market.ts#L1-L15)
- [IndexCard.vue:44-72](file://src/components/IndexCard.vue#L44-L72)

### 响应式设计
- 网格布局：MarketSection 使用 grid-cols-1 md:grid-cols-3 在三列网格中排列卡片，适配桌面端宽度，移动端单列显示。
- 间距与留白：gap-3 与 p-4 提供舒适的视觉间距。
- 字体层级：指数名称较小、价格较大且加粗，涨跌信息次级，成交量/成交额最小，形成清晰的信息层次。
- 颜色对比：深色背景配合高对比度文字与强调色，确保可读性。
- 交互反馈：cursor-pointer 和 hover:border-gray-600 提供鼠标悬停效果。

章节来源
- [MarketSection.vue:16](file://src/components/MarketSection.vue#L16)
- [IndexCard.vue:44-72](file://src/components/IndexCard.vue#L44-L72)

### 样式定制选项
- 边框：默认使用灰色边框与左侧强调边框（border-l-*），可通过修改 trendBorder 的计算结果或模板 class 自定义。
- 阴影：当前未显式设置阴影，可在卡片根元素添加 shadow-* 类以实现悬浮阴影效果。
- 悬停效果：已添加 hover:border-gray-600 类，增强交互反馈。
- 主题色：通过 tailwind 的颜色类（red/green/gray）统一管理，便于全局替换。
- 光标样式：cursor-pointer 表明卡片可点击。

章节来源
- [IndexCard.vue:44-47](file://src/components/IndexCard.vue#L44-L47)
- [IndexCard.vue:12-40](file://src/components/IndexCard.vue#L12-L40)
- [styles.css:1-2](file://src/styles.css#L1-L2)

### 可复用性设计
- 单一职责：IndexCard 只负责单个指数的展示和点击事件处理，不关心数据来源与分组逻辑。
- 纯展示：通过 props 接收数据，通过 emit 抛出事件，无副作用，易于在其他页面或列表中使用。
- 格式化解耦：所有数值格式化集中在 format.ts，便于统一调整精度与单位策略。
- 类型约束：通过 TypeScript 接口保障数据结构稳定，降低误用风险。
- 事件驱动：通过标准的事件机制与父组件通信，保持组件的解耦性。

章节来源
- [IndexCard.vue:1-74](file://src/components/IndexCard.vue#L1-L74)
- [format.ts:1-33](file://src/utils/format.ts#L1-L33)
- [market.ts:1-54](file://src/types/market.ts#L1-L54)

### 无障碍访问支持与键盘导航
- 语义化：当前模板使用 div 与 span，建议为卡片根元素添加 role="button" 和 tabindex="0" 以支持键盘操作。
- 焦点管理：若需要支持键盘操作（如 Enter 查看详情），可为卡片添加键盘事件处理。
- 颜色对比：已使用高对比度颜色，但应确保在辅助技术或高对比度模式下仍可识别涨跌含义（可结合图标或文字标签）。
- 可访问性标签：建议添加 aria-label 描述卡片的指数名称和涨跌状态。

**改进建议**：
- 添加 `role="button"` 和 `tabindex="0"` 使卡片可聚焦
- 添加键盘事件监听器处理 Enter 键
- 添加 `aria-label` 提供屏幕阅读器支持

章节来源
- [IndexCard.vue:44-72](file://src/components/IndexCard.vue#L44-L72)

### 与父组件的数据绑定与事件处理机制
- 数据绑定：MarketSection 通过 v-for 遍历 group.indices，将每个 index 作为 data 传递给 IndexCard。
- 键值：使用 index.code 作为 key，确保列表渲染稳定性。
- 事件处理：IndexCard 通过 emit('select', props.data) 向父组件发送选择事件。
- 事件冒泡：MarketSection 监听 IndexCard 的 select 事件，并通过 emit('select', $event) 继续向上传递。
- 状态管理：App.vue 监听 MarketSection 的 select 事件，更新 selectedCode 状态以切换视图。

**新增事件流程**：
1. 用户点击 IndexCard
2. IndexCard emit('select', data)
3. MarketSection 接收并重新 emit('select', data)
4. App.vue 接收事件，设置 selectedCode = data.code
5. 视图切换到 IndexDetail 组件

章节来源
- [MarketSection.vue:17-22](file://src/components/MarketSection.vue#L17-L22)
- [IndexCard.vue:6-10](file://src/components/IndexCard.vue#L6-L10)
- [App.vue:31-36](file://src/App.vue#L31-L36)

## 事件处理机制

### 事件冒泡链
IndexCard 的点击事件通过标准的 Vue 事件机制向上传播：

```mermaid
flowchart TD
A[IndexCard.vue] --> B[emit('select', data)]
B --> C[MarketSection.vue]
C --> D[emit('select', $event)]
D --> E[App.vue]
E --> F[selectedCode = data.code]
F --> G[视图切换到 IndexDetail]
```

图表来源
- [IndexCard.vue:47](file://src/components/IndexCard.vue#L47)
- [MarketSection.vue:21](file://src/components/MarketSection.vue#L21)
- [App.vue:35](file://src/App.vue#L35)

### 详情页面集成
- 异步加载：IndexDetail 组件通过 defineAsyncComponent 实现按需加载，减少首屏体积。
- 状态管理：App.vue 使用 selectedCode 状态跟踪当前选中的指数。
- 数据传递：通过 selectedIndex computed 属性找到对应的指数数据传递给详情组件。
- 返回机制：详情页面通过 emit('back') 事件返回看板视图。

章节来源
- [App.vue:7-14](file://src/App.vue#L7-L14)
- [App.vue:47-52](file://src/App.vue#L47-L52)
- [IndexDetail.vue:9-10](file://src/components/IndexDetail.vue#L9-L10)

## 依赖关系分析
- IndexCard 依赖：
  - types/market.ts：IndexData 类型定义
  - utils/format.ts：数值格式化函数
  - Tailwind CSS：样式类
- MarketSection 依赖：
  - types/market.ts：MarketGroup 类型定义
  - IndexCard：子组件
- App 依赖：
  - composables/useMarketData.ts：数据获取与分组
  - components/IndexDetail.vue：详情页面（异步加载）
  - TitleBar：标题栏（外部组件）
  - MarketSection：市场分组展示

**新增依赖**：
- IndexDetail.vue：详情页面组件
- useIndexDetail.ts：详情数据管理组合式函数

```mermaid
graph LR
Types["types/market.ts"] --> IndexCard["IndexCard.vue"]
Format["utils/format.ts"] --> IndexCard
IndexCard --> MarketSection["MarketSection.vue"]
UseMarketData["composables/useMarketData.ts"] --> MarketSection
App["App.vue"] --> MarketSection
App --> IndexDetail["components/IndexDetail.vue"]
App --> UseMarketData
IndexDetail --> UseIndexDetail["composables/useIndexDetail.ts"]
```

图表来源
- [market.ts:1-54](file://src/types/market.ts#L1-L54)
- [format.ts:1-33](file://src/utils/format.ts#L1-L33)
- [IndexCard.vue:1-74](file://src/components/IndexCard.vue#L1-L74)
- [MarketSection.vue:1-26](file://src/components/MarketSection.vue#L1-L26)
- [useMarketData.ts:1-66](file://src/composables/useMarketData.ts#L1-L66)
- [App.vue:1-68](file://src/App.vue#L1-L68)
- [IndexDetail.vue:1-140](file://src/components/IndexDetail.vue#L1-L140)
- [useIndexDetail.ts:1-144](file://src/composables/useIndexDetail.ts#L1-L144)

章节来源
- [IndexCard.vue:1-74](file://src/components/IndexCard.vue#L1-L74)
- [MarketSection.vue:1-26](file://src/components/MarketSection.vue#L1-L26)
- [useMarketData.ts:1-66](file://src/composables/useMarketData.ts#L1-L66)
- [App.vue:1-68](file://src/App.vue#L1-L68)
- [IndexDetail.vue:1-140](file://src/components/IndexDetail.vue#L1-L140)
- [useIndexDetail.ts:1-144](file://src/composables/useIndexDetail.ts#L1-L144)

## 性能考量
- 计算属性优化：trend、trendColor、trendBg、trendBorder 均为 computed，仅在依赖变化时重新计算，减少模板重渲染开销。
- 格式化函数轻量：format.* 函数只做简单数学运算与字符串拼接，时间复杂度 O(1)。
- 列表渲染：MarketSection 使用 v-for 与稳定的 key（index.code），有助于 Vue 高效更新 DOM。
- 数据轮询：useMarketData 每 3 秒刷新一次，避免频繁请求造成性能压力。可根据业务需求调整 POLL_INTERVAL。
- **新增性能优化**：
  - 详情页面异步加载：通过 defineAsyncComponent 实现按需加载，减少首屏体积
  - lightweight-charts 独立 chunk：图表库拆分为独立模块，首屏零体积增量
  - 事件冒泡优化：Vue 的事件系统高效处理事件传播

章节来源
- [IndexCard.vue:12-40](file://src/components/IndexCard.vue#L12-L40)
- [format.ts:1-33](file://src/utils/format.ts#L1-L33)
- [MarketSection.vue:16-22](file://src/components/MarketSection.vue#L16-L22)
- [useMarketData.ts:5-41](file://src/composables/useMarketData.ts#L5-L41)
- [App.vue:7-8](file://src/App.vue#L7-L8)

## 故障排查指南
- 数据为空或加载失败：
  - 检查 useMarketData 的 refresh 是否成功调用 invoke('fetch_indices')，并查看控制台错误日志。
  - 确认 App 的空状态提示是否正确显示。
- 颜色异常：
  - 确认 IndexData.change 的正负值是否符合预期；若为 0，则显示灰色。
  - 检查 Tailwind 颜色类是否生效（确保 styles.css 引入 Tailwind）。
- 数值格式不符合预期：
  - 检查 format.ts 中的精度与单位换算逻辑，必要时调整 toFixed 的位数或阈值。
- 布局错乱：
  - 检查 MarketSection 的 grid-cols-* 是否与目标屏幕匹配；如需移动端适配，可增加断点类。
- **新增问题排查**：
  - 点击无响应：检查 IndexCard 的 @click 事件是否正确绑定，确认 emit('select', props.data) 是否执行。
  - 详情页面不显示：检查 App.vue 的 selectedCode 状态是否正确更新，确认 IndexDetail 组件是否正确渲染。
  - 事件冒泡中断：检查 MarketSection 是否正确转发 select 事件到 App.vue。
  - 详情数据加载失败：检查 useIndexDetail 的错误处理逻辑，确认 API 调用是否正常。

章节来源
- [useMarketData.ts:25-36](file://src/composables/useMarketData.ts#L25-L36)
- [App.vue:39-44](file://src/App.vue#L39-L44)
- [IndexCard.vue:12-40](file://src/components/IndexCard.vue#L12-L40)
- [format.ts:1-33](file://src/utils/format.ts#L1-L33)
- [MarketSection.vue:16-22](file://src/components/MarketSection.vue#L16-L22)
- [styles.css:1-2](file://src/styles.css#L1-L2)
- [IndexCard.vue:47](file://src/components/IndexCard.vue#L47)
- [App.vue:35](file://src/App.vue#L35)
- [MarketSection.vue:21](file://src/components/MarketSection.vue#L21)
- [useIndexDetail.ts:37-42](file://src/composables/useIndexDetail.ts#L37-L42)

## 结论
IndexCard 以简洁清晰的职责划分与一致的格式化策略，实现了指数卡片的稳定展示和交互功能。通过计算属性驱动样式、集中化的格式化函数与类型约束，组件具备良好的可维护性与可复用性。新增的点击事件处理功能为用户提供了便捷的导航体验，通过完整的事件冒泡链实现了从卡片到详情页面的无缝跳转。建议在后续迭代中补充无障碍属性与键盘交互，并根据实际设备尺寸进一步优化响应式布局。

## 附录：Props接口与数据字段映射
- 组件 Props：
  - data: IndexData（必需）
- IndexData 字段说明：
  - code：指数代码（用于列表 key 和详情页面导航）
  - name：指数名称（展示）
  - current：当前价（格式化后展示）
  - prev_close：昨收（未展示）
  - open：开盘价（未展示）
  - high：最高价（未展示）
  - low：最低价（未展示）
  - change：涨跌点（格式化后展示）
  - change_pct：涨跌幅（格式化后展示）
  - volume：成交量（格式化后展示）
  - amount：成交额（格式化后展示）
  - market：市场标识（a/hk/us）
  - update_time：更新时间（未展示）
- **新增事件接口**：
  - select: 当用户点击卡片时触发，参数为完整的 IndexData 对象

章节来源
- [IndexCard.vue:6-10](file://src/components/IndexCard.vue#L6-L10)
- [market.ts:1-15](file://src/types/market.ts#L1-L15)
- [IndexCard.vue:44-72](file://src/components/IndexCard.vue#L44-L72)
- [IndexCard.vue:47](file://src/components/IndexCard.vue#L47)
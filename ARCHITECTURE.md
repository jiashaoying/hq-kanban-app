# 大盘行情看板 — 技术架构文档

## 1. 项目概述

**项目名称：** 大盘行情看板（hq-kanban-app）

**项目标识：** `com.jiashaoying.hqkanbanapp`

**版本：** 0.1.0

**项目目标：** 构建一款轻量级跨平台桌面应用，实时展示全球主要股指行情数据（A 股、港股、美股），支持自动轮询刷新，采用暗色主题看板风格呈现，适合作为桌面常驻工具使用。

**核心功能：**

- 实时获取 A 股（上证指数、深证成指、创业板指、沪深 300、上证 50、中证 500）、港股（恒生指数、恒生科技指数）、美股（道琼斯、纳斯达克、标普 500）共 11 个主要指数行情
- 每 3 秒自动轮询刷新数据
- 自定义无边框窗口 + 可拖动标题栏
- 红涨绿跌配色，左侧彩色边框指示涨跌方向
- 成交量/成交额自动单位换算（万/亿）

---

## 2. 技术栈

### 前端

| 技术 | 版本 | 说明 |
|------|------|------|
| Vue | ^3.5.41 | 前端框架（Composition API） |
| TypeScript | ^5.9.3 | 类型安全 |
| Vite | ^8.2.2 | 构建工具与开发服务器 |
| Tailwind CSS | ^4.3.3 | 原子化 CSS 样式方案 |
| @vitejs/plugin-vue | ^6.0.8 | Vite 的 Vue 3 插件 |
| vue-tsc | ^3.3.11 | Vue 模板类型检查 |
| @tauri-apps/api | ^2.11.1 | Tauri 前端 JS/TS SDK |

### 后端（Rust / Tauri）

| 技术 | 版本 | 说明 |
|------|------|------|
| Tauri | 2 | 跨平台桌面应用框架 |
| tauri-build | 2 | Tauri 构建脚本 |
| tauri-plugin-opener | 2 | 系统打开器插件 |
| reqwest | 0.12（blocking） | HTTP 客户端，用于请求行情 API |
| encoding_rs | 0.8 | 字符编码转换（GBK → UTF-8） |
| serde | 1（derive） | Rust 序列化框架 |
| serde_json | 1 | JSON 序列化/反序列化 |
| tokio | 1（full） | 异步运行时 |

### 桌面框架

| 技术 | 版本 | 说明 |
|------|------|------|
| Tauri | 2.11.4（CLI） | 基于 Rust + WebView 的轻量桌面框架 |

---

## 3. 项目结构

```
hq-kanban-app/
├── index.html                    # 前端入口 HTML
├── package.json                  # 前端依赖与脚本配置
├── pnpm-lock.yaml                # pnpm 锁文件
├── vite.config.ts                # Vite 构建配置
├── tsconfig.json                 # TypeScript 编译配置
├── dist/                         # 前端构建产物目录
├── public/                       # 静态资源
│   ├── tauri.svg
│   └── vite.svg
├── src/                          # 前端源码
│   ├── main.ts                   # Vue 应用入口
│   ├── App.vue                   # 根组件（页面布局）
│   ├── styles.css                # 全局样式
│   ├── vite-env.d.ts             # Vite 类型声明
│   ├── assets/                   # 静态资源
│   │   └── vue.svg
│   ├── components/               # Vue 组件
│   │   ├── TitleBar.vue          # 自定义标题栏（窗口拖动/最小化/关闭）
│   │   ├── IndexCard.vue         # 单个指数卡片（价格/涨跌/成交信息）
│   │   └── MarketSection.vue     # 市场分区（标题 + 指数卡片网格）
│   ├── composables/              # Vue 组合式函数
│   │   └── useMarketData.ts      # 行情数据轮询与分组逻辑
│   ├── types/                    # TypeScript 类型定义
│   │   └── market.ts             # IndexData / MarketGroup 接口
│   └── utils/                    # 工具函数
│       └── format.ts             # 数值格式化（价格/涨跌/成交量/成交额）
└── src-tauri/                    # Tauri / Rust 后端
    ├── Cargo.toml                # Rust 依赖配置
    ├── Cargo.lock                # Rust 锁文件
    ├── tauri.conf.json           # Tauri 应用配置
    ├── build.rs                  # Tauri 构建脚本
    ├── capabilities/             # Tauri 权限配置
    │   └── default.json          # 默认窗口权限（拖动/最小化/关闭）
    ├── icons/                    # 应用图标（多尺寸/多平台）
    ├── src/                      # Rust 源码
    │   ├── main.rs               # 程序入口
    │   ├── lib.rs                # 库入口（命令注册、Tauri Builder）
    │   └── market.rs             # 行情数据模块（API 请求/解析/数据模型）
    ├── gen/                      # Tauri 自动生成的 schema 文件
    └── target/                   # Rust 编译产物
```

---

## 4. 系统架构

### 4.1 整体架构

本应用采用经典的 **前端（Vue 3）+ 后端（Rust/Tauri）** 双层架构，通过 Tauri 的 `invoke` 机制进行 IPC 通信。

```
┌─────────────────────────────────────────────────┐
│                   Tauri 窗口                      │
│  ┌─────────────────────────────────────────────┐  │
│  │              前端 (Vue 3 + WebView)          │  │
│  │                                             │  │
│  │  App.vue                                    │  │
│  │    ├── TitleBar.vue  (窗口控制)              │  │
│  │    └── MarketSection.vue × N               │  │
│  │          └── IndexCard.vue × N             │  │
│  │                                             │  │
│  │  useMarketData (composable)                 │  │
│  │    ↓ invoke('fetch_indices')                │  │
│  └─────────────┬───────────────────────────────┘  │
│                │ IPC (Tauri invoke)               │
│  ┌─────────────▼───────────────────────────────┐  │
│  │              后端 (Rust)                      │  │
│  │                                             │  │
│  │  lib.rs → fetch_indices 命令                 │  │
│  │    └── market.rs → fetch_all_indices()      │  │
│  │          ├── reqwest → qt.gtimg.cn API      │  │
│  │          ├── encoding_rs → GBK 解码          │  │
│  │          └── 字段解析 → Vec<IndexData>       │  │
│  └─────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────┘
```

### 4.2 数据流

1. **前端发起请求：** `useMarketData` composable 在组件挂载时启动定时器，每 3 秒通过 `invoke('fetch_indices')` 调用 Rust 后端命令
2. **后端请求数据：** Rust 端 `fetch_all_indices()` 使用 `reqwest` 向 `qt.gtimg.cn` 发起 HTTP GET 请求，获取原始字节流
3. **编码转换：** 使用 `encoding_rs` 将 GBK 编码的响应体解码为 UTF-8 文本
4. **数据解析：** 按行分割响应文本，提取每行双引号内的内容，按 `~` 分隔符拆分为字段数组，映射到 `IndexData` 结构体
5. **序列化返回：** `IndexData` 通过 `serde` 序列化为 JSON，经 Tauri IPC 通道返回前端
6. **前端渲染：** Vue 响应式系统接收数据，`computed` 按市场类型分组，组件自动更新视图

### 4.3 Tauri 命令注册

在 `lib.rs` 中通过 `tauri::generate_handler!` 宏注册了两个命令：

- `greet` — 示例命令（未在前端使用）
- `fetch_indices` — 核心命令，返回 `Vec<IndexData>`，前端通过 `invoke<IndexData[]>('fetch_indices')` 调用

---

## 5. 数据源接口

### 5.1 API 地址

```
https://qt.gtimg.cn/q={指数代码列表}
```

### 5.2 请求格式

- **协议：** HTTPS GET
- **参数：** URL 路径中直接拼接指数代码，多个代码用逗号分隔
- **当前请求的指数代码：**

```
sh000001,sz399001,sz399006,sh000300,sh000016,sh000905,r_hkHSI,r_hkHSTECH,r_us.DJI,r_us.IXIC,r_us.INX
```

### 5.3 市场代码前缀

| 前缀 | 市场 | 示例 |
|------|------|------|
| `sh` | 上海证券交易所 | `sh000001`（上证指数） |
| `sz` | 深圳证券交易所 | `sz399001`（深证成指） |
| `r_hk` | 香港交易所 | `r_hkHSI`（恒生指数） |
| `r_us` | 美国交易所 | `r_us.DJI`（道琼斯工业指数） |

### 5.4 响应格式

- **字符编码：** GBK（需转换为 UTF-8）
- **内容类型：** 纯文本，每行一个指数数据
- **每行格式：** `v_{指数代码}="字段1~字段2~...~字段N";`

示例（简化）：

```
v_sh000001="1~上证指数~000001~3261.56~3245.31~3248.00~...";
```

### 5.5 字段映射

响应中双引号内以 `~` 分隔的字段，以下为项目使用的字段索引：

| 索引 | 含义 | 对应字段 |
|------|------|----------|
| 1 | 指数名称 | `name` |
| 3 | 当前价格 | `current` |
| 4 | 昨收价 | `prev_close` |
| 5 | 开盘价 | `open` |
| 30 | 更新时间 | `update_time` |
| 31 | 涨跌额 | `change` |
| 32 | 涨跌幅（%） | `change_pct` |
| 33 | 最高价 | `high` |
| 34 | 最低价 | `low` |
| 36 | 成交量（万手） | `volume` |
| 37 | 成交额（万元） | `amount` |

---

## 6. 核心模块说明

### 6.1 Rust 后端：`market.rs`

**数据模型 `IndexData`：**

```rust
pub struct IndexData {
    pub code: String,       // 指数代码（如 sh000001）
    pub name: String,       // 指数名称
    pub current: f64,       // 当前价格
    pub prev_close: f64,    // 昨收价
    pub open: f64,          // 开盘价
    pub high: f64,          // 最高价
    pub low: f64,           // 最低价
    pub change: f64,        // 涨跌额
    pub change_pct: f64,    // 涨跌幅（百分比）
    pub volume: f64,        // 成交量
    pub amount: f64,        // 成交额
    pub market: String,     // 市场类型：a / hk / us
    pub update_time: String,// 更新时间
}
```

**核心函数 `fetch_all_indices()`：**

1. 拼接请求 URL，使用 `reqwest::get()` 发起异步 HTTP 请求
2. 获取响应的原始字节流 `bytes()`
3. 使用 `encoding_rs::GBK.decode()` 将 GBK 字节解码为 UTF-8 字符串
4. 逐行解析：
   - 按 `"` 分割提取双引号内的数据内容
   - 按 `=` 分割提取指数代码（去除 `v_` 前缀）
   - 按 `~` 分割数据内容为字段数组
   - 校验字段数量 ≥ 38，防止解析越界
5. 通过 `market_type()` 函数根据代码前缀判定市场类型
6. 通过 `parse_f64()` 安全地将字符串转为 f64（空字符串返回 0.0）

### 6.2 前端 Composable：`useMarketData`

**轮询机制：**

- 使用 `setInterval` 实现定时轮询，间隔 **3000ms（3 秒）**
- `onMounted` 时立即执行一次 `refresh()` 并启动定时器
- `onUnmounted` 时清除定时器，避免内存泄漏
- `refresh()` 通过 `invoke<IndexData[]>('fetch_indices')` 调用 Rust 后端
- 请求期间设置 `loading = true`，完成后恢复为 `false`
- 失败时打印错误日志，不中断轮询

**数据分组：**

- `marketGroups` 为 `computed` 属性，根据 `market` 字段将指数分为三组：
  - A 股（`market === 'a'`）：上证、深证、创业板、沪深 300、上证 50、中证 500
  - 港股（`market === 'hk'`）：恒生指数、恒生科技
  - 美股（`market === 'us'`）：道琼斯、纳斯达克、标普 500

### 6.3 组件层次关系

```
App.vue（根组件）
├── TitleBar.vue
│   ├── 显示应用标题
│   ├── 显示最后更新时间 + 加载脉动指示器
│   ├── 手动刷新按钮（emit 'refresh' → useMarketData.refresh）
│   └── 窗口控制：最小化 / 关闭（调用 Tauri Window API）
│
├── MarketSection.vue（v-for 遍历 marketGroups）
│   ├── 分区标题（A 股 / 港 股 / 美 股）+ 分隔线
│   └── IndexCard.vue（v-for 遍历 group.indices，3 列网格）
│       ├── 指数名称
│       ├── 当前价格（大号加粗）
│       ├── 涨跌额 + 涨跌幅
│       └── 成交量 + 成交额
│
└── 空状态提示（所有分组无数据且非加载中时显示）
```

**组件通信：**

- `App.vue` → `TitleBar.vue`：通过 props 传递 `lastUpdateTime` 和 `loading`，通过 `@refresh` 事件监听刷新操作
- `App.vue` → `MarketSection.vue`：通过 props 传递 `MarketGroup` 对象
- `MarketSection.vue` → `IndexCard.vue`：通过 props 传递 `IndexData` 对象

---

## 7. UI 设计

### 7.1 主题风格

- **暗色主题：** 全局使用 `bg-gray-900` 深灰背景 + `text-white` 白色文字
- **无边框窗口：** Tauri 配置 `decorations: false`，由前端 `TitleBar.vue` 实现自定义标题栏
- **标题栏高度：** 40px（`h-10`），底部 1px 分隔线（`border-gray-700`）

### 7.2 颜色方案（红涨绿跌）

| 状态 | 文字颜色 | 背景色 | 左边框颜色 |
|------|----------|--------|------------|
| 上涨 | `text-red-500` | `bg-red-500/5` | `border-l-red-500` |
| 下跌 | `text-green-500` | `bg-green-500/5` | `border-l-green-500` |
| 平盘 | `text-gray-400` | `bg-gray-800` | `border-l-gray-600` |

每张指数卡片采用左侧 4px 彩色边框（`border-l-4`）直观标识涨跌方向，配合极淡的背景色渲染。

### 7.3 布局

- **整体：** `flex flex-col` 纵向布局，标题栏固定 + 主内容区可滚动
- **指数卡片：** 每个 `MarketSection` 内使用 `grid grid-cols-3 gap-3` 三列等宽网格
- **卡片内部：** 纵向排列（名称 → 价格 → 涨跌 → 成交信息），成交量/成交额底部两端对齐

### 7.4 交互细节

- 标题栏支持鼠标拖动移动窗口（`startDragging`），双击不触发拖动
- 加载中标题栏显示蓝色脉动圆点（`animate-pulse`）
- 窗口控制按钮（最小化/关闭）使用 `@mousedown.stop` 阻止事件冒泡到拖动处理
- 关闭按钮 hover 时变为红色（`hover:text-red-400`）

---

## 8. 构建与部署

### 8.1 开发模式

```bash
# 启动 Tauri 开发环境（同时启动 Vite dev server + Rust 后端）
pnpm tauri dev
```

- Vite 开发服务器运行在 `http://localhost:1420`（固定端口）
- Rust 后端通过 `tauri-build` 编译
- 支持前端 HMR 热更新

### 8.2 Release 构建

```bash
# 构建生产版本
pnpm tauri build
```

- 前端先执行 `vue-tsc --noEmit`（类型检查）+ `vite build`（构建到 `dist/`）
- Rust 后端编译为 release 模式
- 产物位于 `src-tauri/target/release/bundle/`

### 8.3 构建产物

| 平台 | 产物格式 | 说明 |
|------|----------|------|
| macOS | `.dmg` / `.app` | DMG 安装包 / 应用程序目录 |
| Windows | `.msi` / `.exe` | MSI 安装包 / NSIS 安装包 |
| Linux | `.deb` / `.AppImage` | Debian 包 / AppImage 便携包 |

### 8.4 窗口配置

- 默认尺寸：900 × 680
- 居中显示：是
- 可调整大小：是
- 系统装饰：否（自定义标题栏）
- 始终置顶：否

---

## 9. 关键设计决策

### 9.1 为什么选择 Rust 后端请求而非前端直接 fetch

- **GBK 编码问题：** `qt.gtimg.cn` API 返回 GBK 编码数据，浏览器 `fetch` 无法直接处理 GBK 解码（`TextDecoder` 对 GBK 支持不稳定），而 Rust 的 `encoding_rs` 库可可靠地完成 GBK → UTF-8 转换
- **跨域限制（CORS）：** 行情 API 可能未配置 CORS 头，前端直接请求会被浏览器拦截；Rust 后端作为本地原生请求不受 CORS 约束
- **安全性：** Tauri 的 CSP 策略可设为 `null`（无限制），但通过后端请求避免了暴露 API 地址到浏览器网络层

### 9.2 为什么选择 Vue 3

- **Composition API：** `useMarketData` 等 composable 函数天然适合数据轮询逻辑的封装与复用
- **响应式系统：** `ref` + `computed` 自动追踪数据依赖，行情数据更新后视图自动刷新
- **轻量：** Vue 3 核心库体积小，适合 Tauri 这类追求轻量的桌面应用
- **生态成熟：** Vite 对 Vue 的支持最为完善（同为尤雨溪生态）

### 9.3 为什么选择红涨绿跌配色

- 符合中国大陆股票市场的主流显示惯例（A 股用户习惯）
- 通过左侧彩色边框 + 文字颜色双重标识，即使在快速浏览时也能迅速判断涨跌

### 9.4 为什么使用自定义标题栏

- Tauri 配置 `decorations: false` 移除系统原生标题栏
- 自定义标题栏可实现与应用风格统一的拖动区域和窗口控制按钮
- 通过 Tauri 权限配置 `core:window:allow-start-dragging` 实现窗口拖动
- 保留最小化和关闭功能，通过 `@tauri-apps/api/window` 的 `getCurrentWindow()` 控制

### 9.5 3 秒轮询策略

- 兼顾数据实时性与服务器压力
- 对于非高频交易场景，3 秒间隔足以捕捉行情变化
- 使用 `setInterval` 而非 WebSocket，因为 `qt.gtimg.cn` 为 HTTP 接口，无需维持长连接
- 组件卸载时自动清除定时器，避免后台资源浪费
